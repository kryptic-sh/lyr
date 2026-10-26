//! Rendering for lyr (`docs/plan/04-rendering-and-theme.md`).
//!
//! Turns `lyr-core`'s draw list into crcbl-ui's commands: the bar's font found
//! on the system ([`fonts`]), its text measured and drawn ([`text`]), colours
//! converted to linear light ([`colour`]), pictures placed on a window's
//! image atlas ([`pictures`]), the translation itself ([`translate`]), and the GPU
//! remembered between runs ([`gpu`]). Presenting the commands on a window, through
//! `crcbl-app`'s presenter, comes next in milestone M1.

pub mod colour;
pub mod fonts;
pub mod gpu;
pub mod pictures;
pub mod surface;
pub mod text;
pub mod translate;
