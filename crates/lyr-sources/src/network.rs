//! The network module's source (`docs/plan/07-modules.md#network`) and the
//! Wi-Fi menu's (`docs/plan/10-popups-and-menus.md#wi-fi-menu`).
//!
//! [`linux`] reads NetworkManager over D-Bus into
//! [`lyr_core::network::Reading`] and [`lyr_core::wifi_menu::Wifi`]. The nl80211 fallback for a
//! machine without NetworkManager, and Windows and macOS, are not built.

use lyr_core::wifi_menu::Security;

#[cfg(target_os = "linux")]
pub mod linux;

/// How to join a network.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Join {
    /// With a saved profile, through the access point at `point`.
    Saved { profile: String, point: String },
    /// With a new profile, made from the scan.
    New {
        ssid: String,
        security: Security,
        /// For a network that needs one.
        password: Option<String>,
        point: String,
    },
}

/// Joining failed: the device's state reason, and the profile made for it,
/// if one was.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Failed {
    pub reason: u32,
    pub profile: Option<String>,
}
