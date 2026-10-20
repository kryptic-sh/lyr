//! Workspace backends for lyr.
//!
//! Will own the workspace backends for every platform: Hyprland IPC,
//! Windows virtual desktops and AeroSpace on macOS
//! (`docs/plan/06-workspaces.md`).
//!
//! So far it holds the Hyprland backend ([`hyprland`]). Each backend
//! produces the same [`State`], the buttons the workspace module draws.

pub mod hyprland;
mod state;

pub use state::{State, Step, Workspace, WorkspaceId};
