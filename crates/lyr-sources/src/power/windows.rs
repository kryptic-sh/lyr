//! The power menu's actions on Windows, as plan 10 names them:
//! `LockWorkStation`, `SetSuspendState`, and `ExitWindowsEx`, with
//! `SE_SHUTDOWN_NAME` enabled on the process token first for a reboot or a
//! power-off ([`super::windows_call`] says which).
//!
//! Verified: the privilege is enabled by a test on this machine. The calls
//! themselves end the session or suspend the machine, so no test makes them.

use std::io;
use std::ptr;

use lyr_core::power_menu::Entry;
use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_NOT_ALL_ASSIGNED, GetLastError, HANDLE, LUID,
};
use windows_sys::Win32::Security::{
    AdjustTokenPrivileges, LUID_AND_ATTRIBUTES, LookupPrivilegeValueW, SE_PRIVILEGE_ENABLED,
    SE_SHUTDOWN_NAME, TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES, TOKEN_QUERY,
};
use windows_sys::Win32::System::Power::SetSuspendState;
use windows_sys::Win32::System::Shutdown::{
    EWX_LOGOFF, EWX_POWEROFF, EWX_REBOOT, ExitWindowsEx, LockWorkStation,
    SHTDN_REASON_FLAG_PLANNED, SHTDN_REASON_MAJOR_OTHER, SHTDN_REASON_MINOR_OTHER,
};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

use super::{Exit, WindowsCall, windows_call};

/// `Ok` when a Win32 call reported success, else the thread's last error.
fn check(succeeded: bool) -> io::Result<()> {
    if succeeded {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

/// The process token, open to adjust its privileges, closed when dropped.
struct Token(HANDLE);

impl Token {
    fn open() -> io::Result<Self> {
        let mut token: HANDLE = ptr::null_mut();
        // SAFETY: `GetCurrentProcess` returns a pseudo-handle that needs no
        // closing, and `token` is a local the call writes once.
        let opened = unsafe {
            OpenProcessToken(
                GetCurrentProcess(),
                TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY,
                &raw mut token,
            )
        };
        check(opened != 0)?;
        Ok(Self(token))
    }
}

impl Drop for Token {
    fn drop(&mut self) {
        // SAFETY: the handle came from `OpenProcessToken` and is closed once.
        // A failed close leaks a handle in a process that keeps running, with
        // nothing to do about it.
        unsafe { CloseHandle(self.0) };
    }
}

/// Enables `SE_SHUTDOWN_NAME` on the process token, which `ExitWindowsEx`
/// needs to reboot or power off.
///
/// # Errors
///
/// When the token cannot be opened or adjusted, or the user does not hold
/// the privilege.
pub fn enable_shutdown_privilege() -> io::Result<()> {
    let token = Token::open()?;
    let mut luid = LUID::default();
    // SAFETY: the name is a static wide string and `luid` a local.
    let found = unsafe { LookupPrivilegeValueW(ptr::null(), SE_SHUTDOWN_NAME, &raw mut luid) };
    check(found != 0)?;
    let privileges = TOKEN_PRIVILEGES {
        PrivilegeCount: 1,
        Privileges: [LUID_AND_ATTRIBUTES {
            Luid: luid,
            Attributes: SE_PRIVILEGE_ENABLED,
        }],
    };
    // SAFETY: `privileges` is a local the call only reads; no previous state
    // is asked for, so its buffer and length are null and zero.
    let adjusted = unsafe {
        AdjustTokenPrivileges(
            token.0,
            0,
            &raw const privileges,
            0,
            ptr::null_mut(),
            ptr::null_mut(),
        )
    };
    check(adjusted != 0)?;
    // It succeeds without enabling a privilege the user does not hold, and
    // says so only here.
    // SAFETY: reads the calling thread's last error, nothing else.
    if unsafe { GetLastError() } == ERROR_NOT_ALL_ASSIGNED {
        return Err(io::Error::other(
            "the user does not hold SeShutdownPrivilege",
        ));
    }
    Ok(())
}

/// Carries out the power menu's `entry`.
///
/// # Errors
///
/// When Windows refuses, as it does a hibernate with hibernation turned off.
pub fn carry_out(entry: Entry) -> io::Result<()> {
    match windows_call(entry) {
        // SAFETY: takes no arguments.
        WindowsCall::Lock => check(unsafe { LockWorkStation() } != 0),
        WindowsCall::Suspend { hibernate } => {
            // SAFETY: plain values; it returns once the machine wakes.
            check(unsafe { SetSuspendState(hibernate, false, false) })
        }
        WindowsCall::Exit(exit) => {
            let flags = match exit {
                Exit::LogOff => EWX_LOGOFF,
                Exit::Reboot => EWX_REBOOT,
                Exit::PowerOff => EWX_POWEROFF,
            };
            if exit.needs_privilege() {
                enable_shutdown_privilege()?;
            }
            let reason =
                SHTDN_REASON_MAJOR_OTHER | SHTDN_REASON_MINOR_OTHER | SHTDN_REASON_FLAG_PLANNED;
            // SAFETY: plain values.
            check(unsafe { ExitWindowsEx(flags, reason) } != 0)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{ptr, slice};

    use windows_sys::Win32::Foundation::LUID;
    use windows_sys::Win32::Security::{
        AdjustTokenPrivileges, GetTokenInformation, LUID_AND_ATTRIBUTES, LookupPrivilegeValueW,
        SE_PRIVILEGE_ENABLED, SE_SHUTDOWN_NAME, TOKEN_PRIVILEGES, TokenPrivileges,
    };

    use super::{Token, check, enable_shutdown_privilege};

    fn shutdown_luid() -> LUID {
        let mut luid = LUID::default();
        // SAFETY: a static wide string and a local.
        let found = unsafe { LookupPrivilegeValueW(ptr::null(), SE_SHUTDOWN_NAME, &raw mut luid) };
        check(found != 0).expect("the privilege has a LUID");
        luid
    }

    /// Turns the privilege off again, so enabling it can be seen.
    fn disable() {
        let token = Token::open().expect("the token");
        let privileges = TOKEN_PRIVILEGES {
            PrivilegeCount: 1,
            Privileges: [LUID_AND_ATTRIBUTES {
                Luid: shutdown_luid(),
                Attributes: 0,
            }],
        };
        // SAFETY: a local the call only reads; no previous state asked for.
        let adjusted = unsafe {
            AdjustTokenPrivileges(
                token.0,
                0,
                &raw const privileges,
                0,
                ptr::null_mut(),
                ptr::null_mut(),
            )
        };
        check(adjusted != 0).expect("disabled");
    }

    /// Whether the process token has the privilege enabled now.
    fn enabled() -> bool {
        let token = Token::open().expect("the token");
        let mut needed = 0_u32;
        // SAFETY: asks only for the size: no buffer, length zero.
        unsafe {
            GetTokenInformation(
                token.0,
                TokenPrivileges,
                ptr::null_mut(),
                0,
                &raw mut needed,
            )
        };
        // u64s, so the buffer is aligned for TOKEN_PRIVILEGES.
        let mut buffer = vec![0_u64; (needed as usize).div_ceil(8)];
        // SAFETY: the buffer holds `needed` bytes, which the call fills.
        let read = unsafe {
            GetTokenInformation(
                token.0,
                TokenPrivileges,
                buffer.as_mut_ptr().cast(),
                needed,
                &raw mut needed,
            )
        };
        check(read != 0).expect("the token's privileges");
        let header = buffer.as_ptr().cast::<TOKEN_PRIVILEGES>();
        // SAFETY: the buffer holds a TOKEN_PRIVILEGES, its array running
        // `PrivilegeCount` entries past its one declared one.
        let privileges = unsafe {
            slice::from_raw_parts(
                (&raw const (*header).Privileges).cast::<LUID_AND_ATTRIBUTES>(),
                (*header).PrivilegeCount as usize,
            )
        };
        let luid = shutdown_luid();
        privileges
            .iter()
            .find(|privilege| {
                privilege.Luid.LowPart == luid.LowPart && privilege.Luid.HighPart == luid.HighPart
            })
            .is_some_and(|privilege| privilege.Attributes & SE_PRIVILEGE_ENABLED != 0)
    }

    #[test]
    fn the_shutdown_privilege_is_enabled() {
        disable();
        assert!(!enabled(), "off to start with");
        enable_shutdown_privilege().expect("an ordinary user holds SeShutdownPrivilege");
        assert!(enabled());
        enable_shutdown_privilege().expect("enabling it again is no error");
    }
}
