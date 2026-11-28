//! Bringing an application's window forward on Windows, for a click on one
//! of its notifications in the clock panel: [`focus`] finds a window whose
//! program the caller recognises and brings it forward, and
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

/// What [`focus`] did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Focused {
    /// A window came forward.
    Forward,
    /// A window was found, but Windows kept it back, as it does for a
    /// program the user has not just clicked.
    Refused,
    /// No window was found.
    NoWindow,
}

/// Brings forward the first window `EnumWindows` lists that is an
/// application's own ([`is_app_window`]), not lyr's, and whose program's
/// name, its file without `.exe`, `is_sender` accepts; a minimised one is
/// restored. `EnumWindows` lists in Z order, frontmost first, in practice;
/// its documentation does not promise an order.
///
/// # Errors
///
/// When the windows cannot be listed.
pub fn focus(is_sender: impl Fn(&str) -> bool) -> io::Result<Focused> {
    // SAFETY: a plain call.
    let own = unsafe { GetCurrentProcessId() };
    let found = top_level()?
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
        return Ok(Focused::NoWindow);
    };
    // SAFETY: plain calls; Windows checks the handle on each, so one whose
    // window closed since it was listed fails rather than misbehaving.
    let forward = unsafe {
        if IsIconic(window) != 0 {
            ShowWindow(window, SW_RESTORE);
        }
        SetForegroundWindow(window) != 0
    };
    Ok(if forward {
        Focused::Forward
    } else {
        Focused::Refused
    })
}

/// What Explorer opens to start the application whose AppUserModelID is
/// `aumid`, as the Start menu does: its entry in the shell's Applications
/// folder, `shell:AppsFolder\<id>`. The id comes from whichever program sent
/// the notification, so one not made as an id is ([`launchable`]) is
/// refused rather than handed to Explorer.
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

/// Whether `aumid` is made only as an AppUserModelID is: letters, digits,
/// spaces, backslashes and [`ID_PUNCTUATION`] (a packaged app's
/// `Family_hash!App`, or a known folder's GUID and a path under it), with no
/// `..` to climb out of a folder and no backslash first. That leaves out
/// what would make Explorer open something other than the Applications
/// folder's entry: a drive or scheme's `:`, a quote, a forward slash, and
/// the comma Explorer splits its arguments at. A path-like id naming a
/// folder with any other character is refused too, so its sender is not
/// started.
fn launchable(aumid: &str) -> bool {
    !aumid.trim().is_empty()
        && !aumid.starts_with('\\')
        && !aumid.contains("..")
        && aumid
            .chars()
            .all(|c| c.is_alphanumeric() || c == ' ' || c == '\\' || ID_PUNCTUATION.contains(c))
}

/// The punctuation [`launchable`] lets through: what packaged apps' ids and
/// program folders' names use, none of which Explorer reads as a separator.
const ID_PUNCTUATION: &str = "._-!{}()+&'#";

/// Every top-level window, in the order `EnumWindows` lists them.
fn top_level() -> io::Result<Vec<HWND>> {
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
    let listed = unsafe { EnumWindows(Some(collect), (&raw mut windows) as LPARAM) };
    if listed == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(windows)
}

/// Whether `window` is one an application shows as its own: visible, owned
/// by no other window, and not a tool window. A cloaked window is kept: one
/// on another virtual desktop is cloaked, and bringing it forward switches
/// to that desktop, as a click on a notification switches workspace on
/// Linux.
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
    use super::{launchable, start_target};

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
        assert!(launchable(
            "{7C5A40EF-A0FB-4BFC-874A-C0F2E0B9FA8E}\\Microsoft Office\\root\\Office16\\OUTLOOK.EXE"
        ));
        assert!(launchable(
            "{6D809377-6AF0-444B-8957-A3773F02200E}\\Notepad++\\notepad++.exe"
        ));
        assert!(!launchable(""));
        assert!(!launchable("..\\..\\Windows\\System32\\calc.exe"));
        assert!(!launchable("C:\\evil.exe"));
        assert!(!launchable("https://example.com"));
        assert!(!launchable("a\" /c calc"));
        assert!(!launchable("a\nb"));
        assert!(
            !launchable("x,\\\\host\\share\\evil.exe"),
            "a second object"
        );
        assert!(!launchable("\\\\host\\share\\evil.exe"), "a UNC path");
    }
}
