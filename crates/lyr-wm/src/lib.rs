//! Workspace backends for lyr.
//!
//! Will own the workspace backends for every platform: Hyprland, sway and
//! niri IPC on Linux, Windows virtual desktops and AeroSpace on macOS
//! (`docs/plan/06-workspaces.md`).
//!
//! So far it holds the Hyprland ([`hyprland`]), sway ([`sway`]) and niri
//! ([`niri`]) backends. Each produces the same [`State`], the buttons the
//! workspace module draws, and on Unix `Compositor` picks the one the
//! session runs under.

#[cfg(unix)]
mod compositor;
pub mod hyprland;
pub mod niri;
mod state;
pub mod sway;

#[cfg(unix)]
pub use compositor::{Changes, Compositor, Error};
pub use state::{State, Step, Workspace, WorkspaceId};
