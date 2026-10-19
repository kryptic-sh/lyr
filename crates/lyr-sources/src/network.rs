//! The network module's source (`docs/plan/07-modules.md#network`).
//!
//! [`linux`] reads NetworkManager over D-Bus into
//! [`lyr_core::network::Reading`]. The nl80211 fallback for a machine
//! without NetworkManager, and Windows and macOS, are not built.

#[cfg(target_os = "linux")]
pub mod linux;
