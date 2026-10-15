//! Workspace backends for lyr.
//!
//! Will own the workspace backends for every platform: Hyprland IPC,
//! Windows virtual desktops and AeroSpace on macOS
//! (`docs/plan/06-workspaces.md`).
//!
//! So far it holds the Hyprland backend ([`hyprland`]). The common
//! `WorkspaceBackend` trait the plan sketches is not written yet: it gets
//! extracted when the second backend (Windows, milestone M9) shows what the
//! two actually share.

pub mod hyprland;
