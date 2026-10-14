//! Module data sources for lyr.
//!
//! Will own every module's source except workspaces: audio, battery,
//! bluetooth, network, power profile, tray, notifications and the clock, each
//! with its per-platform backends, each pushing state changes rather than
//! polling (`docs/plan/07-modules.md`).
//!
//! Empty in the scaffold: nothing is implemented yet. The Linux sources land
//! in milestones M4 and M6 (`docs/plan/15-milestones.md`).
