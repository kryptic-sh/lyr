//! Workspace backends for lyr.
//!
//! Will own the workspace backends for every platform: Hyprland, sway and
//! niri IPC on Linux, Windows virtual desktops and AeroSpace on macOS
//! (`docs/plan/06-workspaces.md`).
//!
//! So far it holds the Hyprland ([`hyprland`]), sway ([`sway`]) and niri
//! ([`niri`]) backends. Each produces the same [`State`], the buttons the
//! workspace module draws.

pub mod hyprland;
pub mod niri;
mod state;
pub mod sway;

pub use state::{State, Step, Workspace, WorkspaceId};
