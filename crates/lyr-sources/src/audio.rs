//! The volume and mic modules' source (`docs/plan/07-modules.md#volume`).
//!
//! [`level`] is a scalar volume as the views' percent, and [`coreaudio`]
//! turns what CoreAudio's devices report into the views' sink and source,
//! both on every platform's tests; `macos` reads them through CoreAudio, and
//! `mixer` on Linux and Windows through crcbl's mixer, which speaks the sound
//! server's own protocol on Linux and WASAPI on Windows.

pub mod coreaudio;
pub mod level;
#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(any(target_os = "linux", windows))]
pub mod mixer;
