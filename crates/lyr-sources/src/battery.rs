//! The battery module's source (`docs/plan/07-modules.md#battery`).
//!
//! [`upower`] turns UPower's properties into the view's reading on every
//! platform's tests; [`linux`] reads them over D-Bus. Windows and macOS come
//! with milestones M9 and M10.

#[cfg(target_os = "linux")]
pub mod linux;
pub mod upower;
