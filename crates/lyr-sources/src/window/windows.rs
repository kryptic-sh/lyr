//! The foreground window on Windows: its title (`GetWindowTextW`) and its
//! program's name ([`crate::program`]). [`Watch`] re-reads it when the
//! foreground changes or the foreground window's title does, through
//! `SetWinEventHook` on a thread of its own, which runs the message loop the
//! hooks are delivered through. lyr's own windows are skipped
//! (`WINEVENT_SKIPOWNPROCESS`), so opening a menu keeps the window it was
//! opened over.

use std::cell::RefCell;
use std::io;
use std::sync::mpsc;
use std::thread::JoinHandle;

use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::Accessibility::{HWINEVENTHOOK, SetWinEventHook, UnhookWinEvent};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CHILDID_SELF, DispatchMessageW, EVENT_OBJECT_NAMECHANGE, EVENT_SYSTEM_FOREGROUND,
    GetForegroundWindow, GetMessageW, GetWindowTextW, GetWindowThreadProcessId, MSG, OBJID_WINDOW,
    PM_NOREMOVE, PeekMessageW, PostThreadMessageW, WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS,
    WM_QUIT,
};

use super::Active;
use crate::program;

/// The longest title read, in UTF-16 units; a longer one is cut, which the
/// module's own `max_length` cuts further anyway.
const TITLE_LENGTH: usize = 512;

/// The foreground window, or `None` when there is none, as while the
/// desktop is switching.
#[must_use]
pub fn read() -> Option<Active> {
    // SAFETY: plain calls; the title buffer and its length agree.
    unsafe {
        let window = GetForegroundWindow();
        if window.is_null() {
            return None;
        }
        let mut title = [0_u16; TITLE_LENGTH];
        let length = GetWindowTextW(window, title.as_mut_ptr(), TITLE_LENGTH as i32);
        let mut pid = 0;
        GetWindowThreadProcessId(window, &raw mut pid);
        Some(Active {
            class: program::windows::path(pid)
                .map(|path| program::name(&path).to_owned())
                .unwrap_or_default(),
            title: String::from_utf16_lossy(&title[..usize::try_from(length).unwrap_or(0)]),
        })
    }
}

thread_local! {
    /// What a hook calls on the watch's thread.
    static ON_CHANGE: RefCell<Option<Box<dyn Fn()>>> = RefCell::new(None);
}

/// Where a hook lands: a new foreground window, or a new title on the
/// foreground window itself rather than on anything inside it.
unsafe extern "system" fn changed(
    _hook: HWINEVENTHOOK,
    event: u32,
    window: HWND,
    object: i32,
    child: i32,
    _thread: u32,
    _time: u32,
) {
    // SAFETY: a plain call.
    let foreground = unsafe { GetForegroundWindow() };
    let ours = event == EVENT_SYSTEM_FOREGROUND
        || (window == foreground && object == OBJID_WINDOW && child == CHILDID_SELF as i32);
    if ours {
        ON_CHANGE.with(|on_change| {
            if let Some(on_change) = on_change.borrow().as_ref() {
                on_change();
            }
        });
    }
}

/// The hook thread, calling its callback while it lives.
#[derive(Debug)]
pub struct Watch {
    /// The thread's id, which its message loop is stopped through.
    thread_id: u32,
    thread: Option<JoinHandle<()>>,
}

impl Watch {
    /// Calls `on_change` with the foreground window whenever it changes or
    /// its title does, on a thread of the watch's own, until dropped.
    ///
    /// # Errors
    ///
    /// When the thread cannot start or Windows refuses the hooks.
    pub fn new(on_change: impl Fn(Active) + Send + 'static) -> io::Result<Self> {
        let (started, start) = mpsc::channel();
        let thread = std::thread::Builder::new()
            .name("lyr window".to_owned())
            .spawn(move || {
                // SAFETY: the message queue is made before the id goes out,
                // so a quit posted to it is not lost; each hook is checked,
                // and unhooked once, after the loop ends; `msg` is a live
                // buffer for each call.
                unsafe {
                    let mut msg: MSG = std::mem::zeroed();
                    PeekMessageW(&raw mut msg, std::ptr::null_mut(), 0, 0, PM_NOREMOVE);
                    ON_CHANGE.with(|slot| {
                        *slot.borrow_mut() = Some(Box::new(move || {
                            if let Some(active) = read() {
                                on_change(active);
                            }
                        }));
                    });
                    let hook = |event| {
                        SetWinEventHook(
                            event,
                            event,
                            std::ptr::null_mut(),
                            Some(changed),
                            0,
                            0,
                            WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
                        )
                    };
                    let hooks = [hook(EVENT_SYSTEM_FOREGROUND), hook(EVENT_OBJECT_NAMECHANGE)];
                    if hooks.iter().any(|hook| hook.is_null()) {
                        for hook in hooks.into_iter().filter(|hook| !hook.is_null()) {
                            UnhookWinEvent(hook);
                        }
                        let _gone = started.send(Err(io::Error::last_os_error()));
                        return;
                    }
                    let _gone = started.send(Ok(GetCurrentThreadId()));
                    // 0 is WM_QUIT and -1 an error; either ends the loop.
                    while GetMessageW(&raw mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                        DispatchMessageW(&raw const msg);
                    }
                    for hook in hooks {
                        UnhookWinEvent(hook);
                    }
                }
            })?;
        let thread_id = start
            .recv()
            .map_err(|_| io::Error::other("the window thread stopped"))??;
        Ok(Self {
            thread_id,
            thread: Some(thread),
        })
    }
}

impl Drop for Watch {
    fn drop(&mut self) {
        // SAFETY: a plain call to the watch's own thread.
        let posted = unsafe { PostThreadMessageW(self.thread_id, WM_QUIT, 0, 0) };
        // Joined only when the quit went: a thread that missed it would
        // block the drop for good, and is left to the process's end.
        if posted != 0
            && let Some(thread) = self.thread.take()
        {
            let _joined = thread.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Watch, read};

    #[test]
    fn the_foreground_window_reads_and_a_watch_starts_and_stops() {
        // A test run may have no foreground window (a service session);
        // whatever there is reads.
        if let Some(active) = read() {
            assert!(active.title.len() <= super::TITLE_LENGTH * 3);
        }
        let watch = Watch::new(|_| {}).expect("hooked");
        assert_ne!(watch.thread_id, 0);
        drop(watch);
    }
}
