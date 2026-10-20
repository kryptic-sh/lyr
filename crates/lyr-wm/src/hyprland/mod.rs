//! The Hyprland workspace backend.
//!
//! Hyprland exposes two sockets in
//! `$XDG_RUNTIME_DIR/hypr/$HYPRLAND_INSTANCE_SIGNATURE/`: `.socket2.sock`
//! streams `event>>data` lines, and `.socket.sock` takes one request per
//! connection. [`parse_line`] turns an event line into an [`Event`],
//! [`Tracker::apply`] folds events into what the bar shows, and
//! [`focus_request`] builds the bytes a click writes to the request socket
//! ([`exit_request`] those of the power menu's Log out).
//! Those are platform-free. [`Hyprland`] does the socket I/O, and exists only
//! on Unix, where Hyprland runs.

mod event;
#[cfg(unix)]
mod ipc;
mod request;
#[cfg(unix)]
mod snapshot;
mod state;

pub use event::{Event, ParseError, parse_line};
#[cfg(unix)]
pub use ipc::{Error, Events, Hyprland};
pub use request::{Dialect, exit_request, focus_request};
pub use state::Tracker;

pub use crate::{State, Step, Workspace, WorkspaceId};
