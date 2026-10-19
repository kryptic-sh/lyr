//! Module data sources for lyr.
//!
//! Will own every module's source except workspaces: audio, battery,
//! bluetooth, network, power profile, tray, notifications and the clock, each
//! with its per-platform backends, each pushing state changes rather than
//! polling (`docs/plan/07-modules.md`).
//!
//! So far it holds the battery ([`battery`]), Bluetooth ([`bluetooth`]) and
//! power-profile ([`power_profile`]) sources and the notification daemon
//! ([`notifications`]), with their Linux backends, and the power menu's
//! actions ([`power`]), which are not a module's source but sit with the
//! other system-bus clients. The rest land in milestones M4 and M6 to M8
//! (`docs/plan/15-milestones.md`).

pub mod battery;
pub mod bluetooth;
pub mod notifications;
pub mod power;
pub mod power_profile;
