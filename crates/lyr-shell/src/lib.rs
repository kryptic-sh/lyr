//! Bar and popup windows for lyr.
//!
//! Will own the bar policy over crcbl-shell: one bar per output, space
//! reservation, popup placement, outputs coming and going, and turning pointer
//! input into hit tests (`docs/plan/03-surfaces-and-input.md`).
//!
//! So far it holds the bars ([`bars`]): one panel per output, created and
//! closed as outputs and the config change. Drawing into them needs crcbl's
//! window-plus-device crate (`crcbl-app`, crcbl 0.4.0); popups come with M5.

pub mod bars;
