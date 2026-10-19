//! Rendering for lyr (`docs/plan/04-rendering-and-theme.md`).
//!
//! Turns `lyr-core`'s draw list into crcbl-ui's commands: the bar's font found
//! on the system ([`fonts`]), its text measured and drawn ([`text`]), colours
//! converted to linear light ([`colour`]), and the translation itself
//! ([`translate`]). Presenting the commands on a window, through
//! `crcbl-app`'s presenter, comes next in milestone M1.

pub mod colour;
pub mod fonts;
pub mod text;
pub mod translate;
