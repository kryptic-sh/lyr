//! The battery module's source (`docs/plan/07-modules.md#battery`).
//!
//! [`upower`] turns UPower's properties, [`win32`] Windows' power status and
//! [`iops`] a macOS power source's description into the view's reading on
//! every platform's tests; [`linux`] reads UPower over D-Bus, [`windows`] the
//! power status, and `macos` IOKit's power sources.

pub mod iops;
#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "macos")]
pub mod macos;
pub mod upower;
pub mod win32;
#[cfg(windows)]
pub mod windows;
