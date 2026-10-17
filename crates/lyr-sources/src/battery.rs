//! The battery module's source (`docs/plan/07-modules.md#battery`).
//!
//! Linux reads UPower ([`upower`]). Windows and macOS come with milestones M9
//! and M10.

pub mod upower;
