//! The power menu's actions (`docs/plan/10-popups-and-menus.md#power-menu`).
//!
//! [`reboot_flags`] chooses the flags `systemctl reboot` would,
//! [`windows_call`] the Win32 call each entry is, and [`macos_command`] the
//! command each is on macOS, on every platform's tests; [`linux`] makes
//! logind's calls as systemctl makes them, [`windows`] the Win32 ones, and
//! `macos` runs the commands.

use lyr_core::power_menu::Entry;

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(windows)]
pub mod windows;

/// How `ExitWindowsEx` ends the session.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Exit {
    LogOff,
    Reboot,
    PowerOff,
}

impl Exit {
    /// Whether the process token needs `SE_SHUTDOWN_NAME` enabled first: a
    /// reboot and a power-off do, a log-off does not.
    #[must_use]
    pub const fn needs_privilege(self) -> bool {
        matches!(self, Self::Reboot | Self::PowerOff)
    }
}

/// The Win32 call a power menu entry is (plan 10's table).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowsCall {
    /// `LockWorkStation`.
    Lock,
    /// `SetSuspendState`, hibernating or sleeping.
    Suspend { hibernate: bool },
    /// `ExitWindowsEx`.
    Exit(Exit),
}

/// The Win32 call `entry` is.
#[must_use]
pub const fn windows_call(entry: Entry) -> WindowsCall {
    match entry {
        Entry::Lock => WindowsCall::Lock,
        Entry::Sleep => WindowsCall::Suspend { hibernate: false },
        Entry::Hibernate => WindowsCall::Suspend { hibernate: true },
        Entry::Logout => WindowsCall::Exit(Exit::LogOff),
        Entry::Reboot => WindowsCall::Exit(Exit::Reboot),
        Entry::Shutdown => WindowsCall::Exit(Exit::PowerOff),
    }
}

/// The command `entry` is on macOS (plan 10's table), program first, or
/// `None` for Hibernate, which macOS does not offer. Locking puts the
/// display to sleep, which locks it when the Mac asks for a password on
/// waking; the session's end goes through System Events, as the Apple menu's
/// items do.
#[must_use]
pub const fn macos_command(entry: Entry) -> Option<&'static [&'static str]> {
    match entry {
        Entry::Lock => Some(&["pmset", "displaysleepnow"]),
        Entry::Sleep => Some(&["pmset", "sleepnow"]),
        Entry::Hibernate => None,
        Entry::Logout => Some(&[
            "osascript",
            "-e",
            r#"tell application "System Events" to log out"#,
        ]),
        Entry::Reboot => Some(&[
            "osascript",
            "-e",
            r#"tell application "System Events" to restart"#,
        ]),
        Entry::Shutdown => Some(&[
            "osascript",
            "-e",
            r#"tell application "System Events" to shut down"#,
        ]),
    }
}

/// logind's `SD_LOGIND_REBOOT_VIA_KEXEC`: reboot into a loaded kexec kernel
/// when there is one (`src/basic/login-util.h` at systemd v262.1).
pub const REBOOT_VIA_KEXEC: u64 = 1 << 1;
/// logind's `SD_LOGIND_SOFT_REBOOT_IF_NEXTROOT_SET_UP`: soft-reboot into
/// `/run/nextroot` when it holds an OS tree.
pub const SOFT_REBOOT_IF_NEXTROOT_SET_UP: u64 = 1 << 3;

/// Whether an environment variable's value is true to systemd's
/// `getenv_bool`: `parse_boolean`'s true words, in any case.
fn is_true(value: Option<&str>) -> bool {
    value.is_some_and(|value| {
        ["1", "yes", "y", "true", "t", "on"]
            .iter()
            .any(|word| value.eq_ignore_ascii_case(word))
    })
}

/// The flags `systemctl reboot` passes to logind's `RebootWithFlags`, given
/// `SYSTEMCTL_SKIP_AUTO_KEXEC`, `SYSTEMCTL_SKIP_AUTO_SOFT_REBOOT` and whether
/// `/run/nextroot` is a mount point (`logind_reboot` in systemd v262.1's
/// `src/systemctl/systemctl-logind.c`). Inhibitors are left to logind's own
/// policy, as systemctl's default `--check-inhibitors=auto` does.
#[must_use]
pub fn reboot_flags(
    skip_auto_kexec: Option<&str>,
    skip_auto_soft_reboot: Option<&str>,
    nextroot_mounted: bool,
) -> u64 {
    let mut flags = 0;
    if !is_true(skip_auto_kexec) {
        flags |= REBOOT_VIA_KEXEC;
    }
    if !is_true(skip_auto_soft_reboot) && nextroot_mounted {
        flags |= SOFT_REBOOT_IF_NEXTROOT_SET_UP;
    }
    flags
}

#[cfg(test)]
mod tests {
    use lyr_core::power_menu::Entry;

    use super::{
        Exit, REBOOT_VIA_KEXEC, SOFT_REBOOT_IF_NEXTROOT_SET_UP, WindowsCall, macos_command,
        reboot_flags, windows_call,
    };

    #[test]
    fn each_entry_is_plan_tens_macos_command() {
        let joined = |entry| macos_command(entry).map(|argv| argv.join(" "));
        assert_eq!(
            joined(Entry::Lock).as_deref(),
            Some("pmset displaysleepnow")
        );
        assert_eq!(joined(Entry::Sleep).as_deref(), Some("pmset sleepnow"));
        assert_eq!(joined(Entry::Hibernate), None, "macOS offers no hibernate");
        for (entry, verb) in [
            (Entry::Logout, "log out"),
            (Entry::Reboot, "restart"),
            (Entry::Shutdown, "shut down"),
        ] {
            assert_eq!(
                joined(entry),
                Some(format!(
                    r#"osascript -e tell application "System Events" to {verb}"#
                ))
            );
        }
    }

    #[test]
    fn each_entry_is_its_win32_call() {
        assert_eq!(windows_call(Entry::Lock), WindowsCall::Lock);
        assert_eq!(
            windows_call(Entry::Sleep),
            WindowsCall::Suspend { hibernate: false }
        );
        assert_eq!(
            windows_call(Entry::Hibernate),
            WindowsCall::Suspend { hibernate: true }
        );
        assert_eq!(windows_call(Entry::Logout), WindowsCall::Exit(Exit::LogOff));
        assert_eq!(windows_call(Entry::Reboot), WindowsCall::Exit(Exit::Reboot));
        assert_eq!(
            windows_call(Entry::Shutdown),
            WindowsCall::Exit(Exit::PowerOff)
        );
        assert!(Exit::Reboot.needs_privilege() && Exit::PowerOff.needs_privilege());
        assert!(!Exit::LogOff.needs_privilege());
    }

    #[test]
    fn a_reboot_takes_a_loaded_kexec_kernel_unless_told_not_to() {
        assert_eq!(reboot_flags(None, None, false), REBOOT_VIA_KEXEC);
        assert_eq!(reboot_flags(Some("YES"), None, false), 0);
        assert_eq!(reboot_flags(Some("on"), None, false), 0);
        // Not one of systemd's true words, so not set.
        assert_eq!(reboot_flags(Some("0"), None, false), REBOOT_VIA_KEXEC);
        assert_eq!(reboot_flags(Some("maybe"), None, false), REBOOT_VIA_KEXEC);
    }

    #[test]
    fn a_reboot_soft_reboots_only_into_a_mounted_nextroot() {
        assert_eq!(
            reboot_flags(Some("1"), None, true),
            SOFT_REBOOT_IF_NEXTROOT_SET_UP
        );
        assert_eq!(reboot_flags(Some("1"), Some("true"), true), 0);
        assert_eq!(reboot_flags(Some("1"), None, false), 0);
    }
}
