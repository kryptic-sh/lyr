//! The power menu's actions (`docs/plan/10-popups-and-menus.md#power-menu`).
//!
//! [`reboot_flags`] chooses the flags `systemctl reboot` would, on every
//! platform's tests; [`linux`] makes logind's calls as systemctl makes them.
//! Windows and macOS come with milestones M9 and M10.

#[cfg(target_os = "linux")]
pub mod linux;

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
    use super::{REBOOT_VIA_KEXEC, SOFT_REBOOT_IF_NEXTROOT_SET_UP, reboot_flags};

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
