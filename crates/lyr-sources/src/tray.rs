//! The tray's source (`docs/plan/08-tray.md`): StatusNotifierItems on
//! Linux ([`linux`]) and their icons ([`icon`]). Windows and macOS have no
//! tray to host.

#[cfg(target_os = "linux")]
pub mod icon;
#[cfg(target_os = "linux")]
pub mod linux;
