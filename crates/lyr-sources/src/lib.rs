//! Module data sources for lyr.
//!
//! Will own every module's source except workspaces: audio, battery,
//! bluetooth, network, power profile, tray, notifications and the clock, each
//! with its per-platform backends, each pushing state changes rather than
//! polling (`docs/plan/07-modules.md`).
//!
//! So far it holds the battery ([`battery`]), Bluetooth ([`bluetooth`]),
//! clock ([`clock`]), network and Wi-Fi ([`network`]), power-profile
//! ([`power_profile`]), media-player ([`mpris`]) and tray ([`tray`])
//! sources and the notification daemon ([`notifications`]), with their
//! Linux backends, and the power menu's actions ([`power`]), which are not a
//! module's source but sit with the other system-bus clients; Windows has
//! the battery, network, power-profile, audio ([`audio`]) and foreground
//! window ([`window`]) sources and the power menu's actions. Linux audio waits on a dependency decision
//! (`docs/backlog.md`). [`program`] names the program a process runs, and
//! [`locale`] gives the calendar's first weekday.

pub mod audio;
pub mod battery;
pub mod bluetooth;
pub mod clock;
pub mod locale;
#[cfg(target_os = "macos")]
mod macos_runloop;
pub mod mpris;
pub mod network;
pub mod notifications;
pub mod power;
pub mod power_profile;
pub mod program;
pub mod tray;
pub mod window;
