//! The volume and mic modules' source (`docs/plan/07-modules.md#volume`).
//!
//! [`level`] is a scalar volume as the views' percent, and [`win32`] turns
//! what Windows' audio endpoints report into the views' sink and source,
//! both on every platform's tests; [`windows`] reads them
//! through WASAPI. Linux waits on the PulseAudio dependency decision
//! (`docs/backlog.md`), and macOS comes with milestone M10.

pub mod level;
pub mod win32;
#[cfg(windows)]
pub mod windows;
