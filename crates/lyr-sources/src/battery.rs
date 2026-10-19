//! The battery module's source (`docs/plan/07-modules.md#battery`).
//!
//! [`upower`] turns UPower's properties, and [`win32`] Windows' power status,
//! into the view's reading on every platform's tests; [`linux`] reads UPower
//! over D-Bus, and [`windows`] the power status. macOS comes with milestone
//! M10.

#[cfg(target_os = "linux")]
pub mod linux;
pub mod upower;
pub mod win32;
#[cfg(windows)]
pub mod windows;
