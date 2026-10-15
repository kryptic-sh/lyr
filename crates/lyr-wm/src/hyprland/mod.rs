//! The platform-free half of the Hyprland backend.
//!
//! Hyprland exposes two sockets in
//! `$XDG_RUNTIME_DIR/hypr/$HYPRLAND_INSTANCE_SIGNATURE/`: `.socket2.sock`
//! streams `event>>data` lines, and `.socket.sock` takes one request per
//! connection. [`parse_line`] turns an event line into an [`Event`],
//! [`State::apply`] folds events into what the bar shows, and
//! [`focus_request`] builds the bytes a click writes to the request socket.

mod event;
mod request;
mod state;

pub use event::{Event, ParseError, parse_line};
pub use request::{Dialect, focus_request};
pub use state::{State, Workspace};

/// A Hyprland workspace id. Numbered workspaces are positive; named and
/// special workspaces get negative ids, so the sign says nothing about
/// whether a workspace is shown.
pub type WorkspaceId = i64;
