//! Workspace backends for lyr.
//!
//! Will own the `WorkspaceBackend` trait and its backends: Hyprland IPC (the
//! event-socket parser and the Lua dispatch form), Windows virtual desktops
//! and AeroSpace on macOS (`docs/plan/06-workspaces.md`).
//!
//! Empty in the scaffold: nothing is implemented yet. The Hyprland backend
//! lands in milestone M2 (`docs/plan/15-milestones.md`).
