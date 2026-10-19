//! The notification daemon's D-Bus side (`docs/plan/09-notifications.md`).
//! The store it fills is `lyr_core::notifications`. Linux only: Windows and
//! macOS keep their own notification centres.

#[cfg(target_os = "linux")]
pub mod linux;
