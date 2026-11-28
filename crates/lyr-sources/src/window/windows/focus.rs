//! Bringing an application's window forward on Windows, for a click on one
//! of its notifications in the clock panel: [`focus`] finds the frontmost
//! window whose program the caller recognises and brings it forward, and
//! [`start_target`] says what starts the application by its AppUserModelID
//! when it has none.

use std::io;

use windows_sys::Win32::Foundation::{HWND, LPARAM};
use windows_sys::Win32::System::Threading::GetCurrentProcessId;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GW_OWNER, GWL_EXSTYLE, GetWindow, GetWindowLongPtrW, GetWindowThreadProcessId,
    IsIconic, IsWindowVisible, SW_RESTORE, SetForegroundWindow, ShowWindow, WS_EX_TOOLWINDOW,
};
use windows_sys::core::BOOL;

use crate::program;

/// Brings forward the frontmost window that is an application's own (shown,
/// owned by no other window, not a tool window, not lyr's) and whose
/// program's name, its file without `.exe`, `is_sender` accepts; a minimised
/// one is restored. Returns whether one was found and Windows let it come
/// forward: it does when lyr has just been clicked, as here.
pub fn focus(is_sender: impl Fn(&str) -> bool) -> bool {
    // SAFETY: plain calls; `own` is the process's own id.
    let own = unsafe { GetCurrentProcessId() };
    let found = top_level()
        .into_iter()
        .filter(|&window| is_app_window(window))
        .find(|&window| {
            let mut pid = 0;
            // SAFETY: a plain call writing the window's process id.
            unsafe { GetWindowThreadProcessId(window, &raw mut pid) };
            pid != own
                && program::windows::path(pid).is_some_and(|path| is_sender(program::name(&path)))
        });
    let Some(window) = found else {
        return false;
    };
    // SAFETY: plain calls on a window just listed; one closed since fails
    // and says so.
    unsafe {
        if IsIconic(window) != 0 {
            ShowWindow(window, SW_RESTORE);
        }
        SetForegroundWindow(window) != 0
    }
}

/// What Explorer opens to start the application whose AppUserModelID is
/// `aumid`, as the Start menu does: its entry in the shell's Applications
/// folder, `shell:AppsFolder\<id>`. An id that could name anything outside
/// that folder is refused ([`launchable`]).
///
/// # Errors
///
/// When the id is refused.
pub fn start_target(aumid: &str) -> io::Result<String> {
    if launchable(aumid) {
        Ok(format!("shell:AppsFolder\\{aumid}"))
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("not an application's id: {aumid}"),
        ))
    }
}

/// Whether `aumid` names only an entry of the Applications folder: no
/// drive, scheme or quote (`:`, `"`), no forward slash, no `..` to climb out
/// of the folder, no control character, and not empty. The id comes from
/// whichever program sent the notification.
fn launchable(aumid: &str) -> bool {
    !aumid.trim().is_empty()
        && !aumid.contains("..")
        && !aumid
            .chars()
            .any(|c| c == ':' || c == '"' || c == '/' || c.is_control())
}

/// Every top-level window, frontmost first.
fn top_level() -> Vec<HWND> {
    unsafe extern "system" fn collect(window: HWND, windows: LPARAM) -> BOOL {
        // SAFETY: `windows` is the `Vec` `top_level` lends for the length of
        // the `EnumWindows` call that calls this.
        let windows = unsafe { &mut *(windows as *mut Vec<HWND>) };
        windows.push(window);
        1
    }
    let mut windows: Vec<HWND> = Vec::new();
    // SAFETY: `collect` reads the pointer only during this call, while
    // `windows` lives and nothing else borrows it.
    unsafe { EnumWindows(Some(collect), (&raw mut windows) as LPARAM) };
    windows
}

/// Whether `window` is one an application shows as its own: visible, owned
/// by no other window, and not a tool window.
fn is_app_window(window: HWND) -> bool {
    // SAFETY: plain calls on a window just listed.
    unsafe {
        IsWindowVisible(window) != 0
            && GetWindow(window, GW_OWNER).is_null()
            && GetWindowLongPtrW(window, GWL_EXSTYLE) & WS_EX_TOOLWINDOW as isize == 0
    }
}

#[cfg(test)]
mod tests {
    use super::{focus, launchable, start_target, top_level};

    #[test]
    fn only_an_applications_folder_entry_is_launched() {
        assert_eq!(
            start_target("Chrome").expect("an id"),
            "shell:AppsFolder\\Chrome"
        );
        assert!(start_target("C:\\evil.exe").is_err());
        assert!(launchable("Chrome"));
        assert!(launchable("Microsoft.WindowsTerminal_8wekyb3d8bbwe!App"));
        assert!(launchable(
            "{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}\\WindowsPowerShell\\v1.0\\powershell.exe"
        ));
        assert!(!launchable(""));
        assert!(!launchable("..\\..\\Windows\\System32\\calc.exe"));
        assert!(!launchable("C:\\evil.exe"));
        assert!(!launchable("https://example.com"));
        assert!(!launchable("a\" /c calc"));
        assert!(!launchable("a\nb"));
    }

    #[test]
    fn windows_list_and_an_unknown_program_is_not_focused() {
        // A test run may have no desktop (a service session); whatever
        // there is lists.
        let _windows = top_level();
        assert!(!focus(|_| false), "nothing accepted, nothing focused");
    }
}
