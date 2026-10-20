//! The volume and mic modules' source (`docs/plan/07-modules.md#volume`).
//!
//! [`level`] is a scalar volume as the views' percent, [`win32`] turns what
//! Windows' audio endpoints report into the views' sink and source, and
//! [`coreaudio`] what CoreAudio's devices report, all on every platform's
//! tests; [`windows`] reads them through WASAPI and `macos` through
//! CoreAudio. Linux waits on the PulseAudio dependency decision
//! (`docs/backlog.md`).

pub mod coreaudio;
pub mod level;
#[cfg(target_os = "macos")]
pub mod macos;
pub mod win32;
#[cfg(windows)]
pub mod windows;
