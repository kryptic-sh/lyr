//! The clock module's source (`docs/plan/07-modules.md#clock`): the system
//! time zone's name. [`linux`] follows systemd's timedate1, so a zone changed
//! with `timedatectl set-timezone` (or `krypt menu time`) redraws the clock
//! at once. Windows and macOS come with milestones M9 and M10.

#[cfg(target_os = "linux")]
pub mod linux;
