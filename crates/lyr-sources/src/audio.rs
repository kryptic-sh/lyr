//! The volume and mic modules' source (`docs/plan/07-modules.md#volume`).
//!
//! [`level`] is a scalar volume as the views' percent, [`win32`] turns what
//! Windows' audio endpoints report into the views' sink and source, and
//! [`coreaudio`] what CoreAudio's devices report, all on every platform's
//! tests; [`windows`] reads them through WASAPI, `macos` through CoreAudio,
//! and `mixer` on Linux through crcbl's mixer, which speaks the sound
//! server's own protocol.

pub mod coreaudio;
pub mod level;
#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "linux")]
pub mod mixer;
pub mod win32;
#[cfg(windows)]
pub mod windows;
