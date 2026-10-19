//! The tray's source (`docs/plan/08-tray.md`): StatusNotifierItems on
//! Linux ([`linux`]). Windows and macOS have no tray to host.

#[cfg(target_os = "linux")]
pub mod linux;
