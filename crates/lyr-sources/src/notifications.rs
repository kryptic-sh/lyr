//! Notifications. On Linux, the notification daemon's D-Bus side
//! (`docs/plan/09-notifications.md`), whose store is
//! `lyr_core::notifications`. On Windows, Windows' own notifications read
//! for the clock panel. macOS keeps its own notification centre.

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(windows)]
pub mod windows;
