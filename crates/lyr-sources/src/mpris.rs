//! The clock panel's media players' source: what each MPRIS player on the
//! session bus is playing ([`lyr_core::media`]), and its controls. [`linux`]
//! reads them over D-Bus; Windows and macOS come later.

#[cfg(target_os = "linux")]
pub mod linux;
