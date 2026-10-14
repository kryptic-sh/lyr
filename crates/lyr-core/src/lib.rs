//! The platform-free core of lyr.
//!
//! Will own module state and the `Event` / `Action` types, format strings and
//! icon ramps, zone layout, building draw lists, hit testing and calendar maths
//! (`docs/plan/02-architecture.md`). It must never contain platform code, so
//! that all of it is testable on every CI runner without a display.
//!
//! Empty in the scaffold: nothing is implemented yet. The core lands in
//! milestone M1 (`docs/plan/15-milestones.md`).
