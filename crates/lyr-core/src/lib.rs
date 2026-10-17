//! The platform-free core of lyr.
//!
//! Will own module state and the `Event` / `Action` types, format strings and
//! icon ramps, building draw lists, hit testing and calendar maths
//! (`docs/plan/02-architecture.md`). It must never contain platform code, so
//! that all of it is testable on every CI runner without a display.
//!
//! So far it holds the zone layout ([`layout`]), icon ramps ([`icon`]), hit
//! testing ([`hit`]) and the control socket's requests ([`control`]). The rest
//! lands with milestone M1 (`docs/plan/15-milestones.md`).

pub mod control;
pub mod hit;
pub mod icon;
pub mod layout;
