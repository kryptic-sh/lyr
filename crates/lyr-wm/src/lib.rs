//! Workspace backends for lyr.
//!
//! Will own the `WorkspaceBackend` trait and its backends: Hyprland IPC,
//! Windows virtual desktops and AeroSpace on macOS
//! (`docs/plan/06-workspaces.md`).
//!
//! So far it holds only the platform-free half of the Hyprland backend
//! ([`hyprland`]): parsing event lines, folding them into workspace state, and
//! building dispatch requests. Nothing here opens a socket yet. The socket
//! I/O, the initial-state query and the trait land with the rest of milestone
//! M2 (`docs/plan/15-milestones.md`).

pub mod hyprland;
