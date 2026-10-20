//! Reading the program a Windows process runs.

use windows_sys::Win32::Foundation::CloseHandle;
use windows_sys::Win32::System::Threading::{
    OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};

/// The path of the program process `pid` runs, when it can be read: a
/// process of another user's or a protected one cannot.
#[must_use]
pub fn path(pid: u32) -> Option<String> {
    // SAFETY: the handle is checked, read through, and closed once; the
    // buffer and its length agree.
    unsafe {
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if process.is_null() {
            return None;
        }
        let mut path = [0_u16; 1024];
        let mut length = path.len() as u32;
        let read =
            QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, path.as_mut_ptr(), &mut length);
        // A failed close leaks one handle; the path is read either way.
        let _closed = CloseHandle(process);
        (read != 0).then(|| String::from_utf16_lossy(&path[..length as usize]))
    }
}
