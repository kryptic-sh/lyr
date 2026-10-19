//! The platform-free core of lyr.
//!
//! Will own module state and the `Event` / `Action` types, format strings and
//! icon ramps, building draw lists, hit testing and calendar maths
//! (`docs/plan/02-architecture.md`). It must never contain platform code, so
//! that all of it is testable on every CI runner without a display.
//!
//! So far it holds the zone layout ([`layout`]), the draw list ([`draw`]), icon
//! ramps ([`icon`]), hit testing ([`hit`]), the calendar ([`calendar`]), what
//! a click asks for ([`pointer`]), the control socket's requests
//! ([`control`]), the notification store ([`notifications`]) and the views of
//! the battery, power-profile, network, bluetooth, volume, mic, clock and
//! notifications ([`bell`]) modules. The rest lands with later milestones
//! (`docs/plan/15-milestones.md`).

pub mod audio;
pub mod battery;
pub mod bell;
pub mod bluetooth;
pub mod bluetooth_menu;
pub mod calendar;
pub mod centre;
pub mod clock;
pub mod control;
pub mod draw;
pub mod hit;
pub mod icon;
pub mod layout;
pub mod list_menu;
pub mod markup;
pub mod media;
pub mod menu;
pub mod network;
pub mod notifications;
pub mod pointer;
pub mod power_menu;
pub mod power_profile;
pub mod toast;
pub mod tooltip;
pub mod tray;
pub mod wifi_menu;
pub mod window;
