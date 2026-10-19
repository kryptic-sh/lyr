//! The initial state, from the JSON replies to `j/workspaces`,
//! `j/activeworkspace` and `j/clients`.
//!
//! Only the fields lyr reads are declared; serde skips the rest. The shapes
//! are `CHyprCtl`'s JSON output (`src/debug/HyprCtl.cpp`, read at Hyprland
//! v0.56.2).

use serde::Deserialize;

use super::{State, Workspace, WorkspaceId};

#[derive(Deserialize)]
struct WorkspaceReply {
    id: WorkspaceId,
    name: String,
    monitor: String,
}

#[derive(Deserialize)]
struct ActiveReply {
    id: WorkspaceId,
    monitor: String,
    /// The workspace's focused window and its title, what waybar's
    /// `hyprland/window` shows; `0x0` and empty with no window.
    #[serde(default)]
    lastwindow: String,
    #[serde(default)]
    lastwindowtitle: String,
}

#[derive(Deserialize)]
struct ClientReply {
    address: String,
    workspace: ClientWorkspace,
    #[serde(default)]
    class: String,
}

#[derive(Deserialize)]
struct ClientWorkspace {
    id: WorkspaceId,
}

/// What Hyprland writes for a workspace with no monitor.
const NO_MONITOR: &str = "?";

/// Builds the state the three replies describe.
pub(super) fn parse(workspaces: &str, active: &str, clients: &str) -> serde_json::Result<State> {
    let workspaces: Vec<WorkspaceReply> = serde_json::from_str(workspaces)?;
    let active: ActiveReply = serde_json::from_str(active)?;
    let clients: Vec<ClientReply> = serde_json::from_str(clients)?;
    // The focused window: its title from the active workspace, its class
    // from the client at that address.
    let window = (!active.lastwindowtitle.is_empty()).then(|| {
        let class = clients
            .iter()
            .find(|client| client.address == active.lastwindow)
            .map(|client| client.class.clone())
            .unwrap_or_default();
        (class, active.lastwindowtitle.clone())
    });
    Ok(State::seeded(
        workspaces.into_iter().map(|workspace| Workspace {
            id: workspace.id,
            name: workspace.name,
            monitor: (workspace.monitor != NO_MONITOR).then_some(workspace.monitor),
            urgent: false,
        }),
        Some((active.id, active.monitor)),
        // The JSON writes an address as `0x…`, events write the same pointer
        // as bare hex, and `urgent` events are matched against these keys.
        clients.into_iter().map(|client| {
            let address = client
                .address
                .strip_prefix("0x")
                .map_or_else(|| client.address.clone(), str::to_owned);
            (address, client.workspace.id)
        }),
        window,
    ))
}

#[cfg(test)]
mod tests {
    use super::parse;
    use crate::hyprland::{Event, Workspace};

    const WORKSPACES: &str = r#"[
        {"id": 1, "name": "1", "monitor": "DP-1", "monitorID": 0, "windows": 2,
         "hasfullscreen": false, "lastwindow": "0x55d0c8a4b1e0",
         "lastwindowtitle": "kitty", "ispersistent": false},
        {"id": -1337, "name": "mail", "monitor": "HDMI-A-1", "monitorID": 1,
         "windows": 0, "hasfullscreen": false, "lastwindow": "0x0",
         "lastwindowtitle": "", "ispersistent": false},
        {"id": -98, "name": "special:scratch", "monitor": "?", "monitorID": null,
         "windows": 0, "hasfullscreen": false, "lastwindow": "0x0",
         "lastwindowtitle": "", "ispersistent": false}
    ]"#;
    const ACTIVE: &str = r#"{"id": 1, "name": "1", "monitor": "DP-1", "monitorID": 0,
        "windows": 2, "hasfullscreen": false, "lastwindow": "0x55d0c8a4b1e0",
        "lastwindowtitle": "kitty", "ispersistent": false}"#;
    const CLIENTS: &str = r#"[
        {"address": "0x55d0c8a4b1e0", "mapped": true,
         "workspace": {"id": -1337, "name": "mail"}, "class": "kitty"}
    ]"#;

    #[test]
    fn the_replies_seed_workspaces_active_and_windows() {
        let mut state = parse(WORKSPACES, ACTIVE, CLIENTS).expect("valid replies");
        assert_eq!(
            state.workspaces().cloned().collect::<Vec<_>>(),
            [
                Workspace {
                    id: -1337,
                    name: "mail".into(),
                    monitor: Some("HDMI-A-1".into()),
                    urgent: false
                },
                Workspace {
                    id: 1,
                    name: "1".into(),
                    monitor: Some("DP-1".into()),
                    urgent: false
                },
            ]
        );
        assert_eq!(state.active(), Some(1));
        assert_eq!(
            state.active_window(),
            Some(("kitty", "kitty")),
            "the active workspace's last window, its class from the clients"
        );

        // The client's `0x…` address matches the bare hex an event carries.
        assert!(state.apply(Event::Urgent {
            window: "55d0c8a4b1e0".into()
        }));
    }

    #[test]
    fn a_reply_of_the_wrong_shape_is_an_error() {
        assert!(parse("[]", "{}", "[]").is_err());
        assert!(parse("not json", ACTIVE, "[]").is_err());
    }
}
