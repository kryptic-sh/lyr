//! The niri workspace backend.
//!
//! niri takes one JSON request per line on the Unix socket `$NIRI_SOCKET`
//! names and answers each with one JSON line, `{"Ok": …}` or `{"Err": "…"}`.
//! After `"EventStream"` the connection carries only events, one JSON object
//! per line, starting with the full state (`niri-ipc`'s `lib.rs`, read at
//! niri v26.04).
//!
//! [`focus_request`] and [`QUIT_REQUEST`] are the requests a click and the
//! power menu's Log out send; those are platform-free. `Tracker` folds the
//! events into the bar's [`crate::State`] and `Niri` does the socket I/O.
//! Those exist only on Unix, where niri runs, and where lyr has its JSON
//! parser.

#[cfg(unix)]
mod ipc;
#[cfg(unix)]
mod tracker;

#[cfg(unix)]
pub use ipc::{Error, Events, Niri};
#[cfg(unix)]
pub use tracker::Tracker;

use crate::Workspace;

/// Turns the connection into an event stream.
pub const EVENT_STREAM_REQUEST: &str = r#""EventStream""#;

/// Ends the session without niri's own confirmation: the power menu has
/// already asked.
pub const QUIT_REQUEST: &str = r#"{"Action":{"Quit":{"skip_confirmation":true}}}"#;

/// Lists the windows.
pub const WINDOWS_REQUEST: &str = r#""Windows""#;

/// The request that focuses the window niri gave `id`, switching to the
/// workspace it is on (tried on niri 26.04, 2026-10-29). niri answers
/// `Handled` even for an id it has no window for.
#[must_use]
pub fn focus_window_request(id: u64) -> String {
    format!(r#"{{"Action":{{"FocusWindow":{{"id":{id}}}}}}}"#)
}

/// The request that switches to `workspace`, by niri's id for it, so no
/// name is ever quoted.
#[must_use]
pub fn focus_request(workspace: &Workspace) -> String {
    format!(
        r#"{{"Action":{{"FocusWorkspace":{{"reference":{{"Id":{}}}}}}}}}"#,
        workspace.id
    )
}

#[cfg(test)]
mod tests {
    use super::{focus_request, focus_window_request};
    use crate::Workspace;

    #[test]
    fn a_window_is_focused_by_id() {
        assert_eq!(
            focus_window_request(12),
            r#"{"Action":{"FocusWindow":{"id":12}}}"#
        );
    }

    #[test]
    fn a_switch_names_the_workspace_by_id() {
        let workspace = Workspace {
            id: 5,
            name: "web".into(),
            monitor: None,
            urgent: false,
        };
        assert_eq!(
            focus_request(&workspace),
            r#"{"Action":{"FocusWorkspace":{"reference":{"Id":5}}}}"#
        );
    }
}
