//! The bar's state from sway's JSON replies, and which window events can
//! change it.
//!
//! Only the fields lyr reads are declared; serde skips the rest. The shapes
//! are `sway/ipc-json.c`'s, read at sway 1.12.

use serde::Deserialize;

use crate::{State, Workspace, WorkspaceId};

/// One entry of the `GET_WORKSPACES` reply.
#[derive(Deserialize)]
struct WorkspaceReply {
    id: WorkspaceId,
    name: String,
    output: Option<String>,
    /// Whether the default seat's focus is on it.
    focused: bool,
    urgent: bool,
}

/// One node of the `GET_TREE` reply, reduced to what finding the focused
/// window needs.
#[derive(Deserialize)]
struct Node {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    focused: bool,
    name: Option<String>,
    /// The Wayland app id; `None` under Xwayland.
    app_id: Option<String>,
    /// An Xwayland window's properties, its class among them.
    window_properties: Option<WindowProperties>,
    #[serde(default)]
    nodes: Vec<Self>,
    #[serde(default)]
    floating_nodes: Vec<Self>,
}

#[derive(Deserialize)]
struct WindowProperties {
    class: Option<String>,
}

impl Node {
    /// The focused node at or under this one.
    fn focused(&self) -> Option<&Self> {
        if self.focused {
            return Some(self);
        }
        self.nodes
            .iter()
            .chain(&self.floating_nodes)
            .find_map(Self::focused)
    }
}

/// The node types that are windows; a focused `workspace` is an empty one.
const WINDOW_TYPES: [&str; 2] = ["con", "floating_con"];

/// Builds the state from the replies to `GET_WORKSPACES` and `GET_TREE`.
///
/// The buttons keep the reply's order, which is sway's own: by output, and
/// on each output numbered workspaces by number, then named ones in the
/// order they were made (`output_sort_workspaces`). The active workspace is
/// the focused one.
///
/// # Errors
///
/// When either reply is not the JSON sway sends.
pub fn state(workspaces: &str, tree: &str) -> serde_json::Result<State> {
    let workspaces: Vec<WorkspaceReply> = serde_json::from_str(workspaces)?;
    let tree: Node = serde_json::from_str(tree)?;
    let window = tree
        .focused()
        .filter(|node| WINDOW_TYPES.contains(&node.kind.as_str()))
        .and_then(|node| {
            let class = node
                .app_id
                .clone()
                .or_else(|| node.window_properties.as_ref()?.class.clone())
                .unwrap_or_default();
            let title = node.name.clone().unwrap_or_default();
            (!class.is_empty() || !title.is_empty()).then_some((class, title))
        });
    Ok(State {
        active: workspaces
            .iter()
            .find(|workspace| workspace.focused)
            .map(|workspace| workspace.id),
        workspaces: workspaces
            .into_iter()
            .map(|workspace| Workspace {
                id: workspace.id,
                name: workspace.name,
                monitor: workspace.output,
                urgent: workspace.urgent,
            })
            .collect(),
        window,
    })
}

/// Whether a `window` event can change which window has focus or what its
/// title is, so the tree is worth reading again. Moves, floating, marks and
/// fullscreen changes cannot.
///
/// # Errors
///
/// When the event is not the JSON sway sends.
pub fn window_event_moves_focus(event: &str) -> serde_json::Result<bool> {
    #[derive(Deserialize)]
    struct WindowEvent {
        change: String,
    }
    let event: WindowEvent = serde_json::from_str(event)?;
    Ok(matches!(
        event.change.as_str(),
        "new" | "close" | "focus" | "title"
    ))
}

#[cfg(test)]
mod tests {
    use super::{state, window_event_moves_focus};
    use crate::Workspace;

    /// Two outputs, as `GET_WORKSPACES` lists them: by output, numbered
    /// before named. Fields lyr does not read are kept, as sway sends them.
    const WORKSPACES: &str = r#"[
        {"id": 4, "type": "workspace", "name": "1", "num": 1, "output": "eDP-1",
         "visible": true, "focused": false, "urgent": false,
         "rect": {"x": 0, "y": 0, "width": 1920, "height": 1080}},
        {"id": 9, "type": "workspace", "name": "mail", "num": -1, "output": "eDP-1",
         "visible": false, "focused": false, "urgent": true,
         "rect": {"x": 0, "y": 0, "width": 1920, "height": 1080}},
        {"id": 6, "type": "workspace", "name": "2:web", "num": 2, "output": "HDMI-A-1",
         "visible": true, "focused": true, "urgent": false,
         "rect": {"x": 1920, "y": 0, "width": 2560, "height": 1440}}
    ]"#;

    /// A tree whose focus is on `focused`, a window on workspace 2:web.
    fn tree(focused: &str) -> String {
        format!(
            r#"{{"id": 1, "type": "root", "name": "root", "focused": false, "nodes": [
                {{"id": 3, "type": "output", "name": "HDMI-A-1", "focused": false, "nodes": [
                    {{"id": 6, "type": "workspace", "name": "2:web", "focused": false,
                      "nodes": [], "floating_nodes": [{focused}]}}
                ]}}
            ], "floating_nodes": []}}"#
        )
    }

    #[test]
    fn the_replies_give_the_buttons_in_sways_order_and_the_focused_one() {
        let state = state(
            WORKSPACES,
            &tree(r#"{"id": 12, "type": "floating_con", "focused": true, "name": "lyr — vim", "app_id": "kitty", "nodes": []}"#),
        )
        .expect("valid replies");
        assert_eq!(
            state.workspaces().cloned().collect::<Vec<_>>(),
            [
                Workspace {
                    id: 4,
                    name: "1".into(),
                    monitor: Some("eDP-1".into()),
                    urgent: false
                },
                Workspace {
                    id: 9,
                    name: "mail".into(),
                    monitor: Some("eDP-1".into()),
                    urgent: true
                },
                Workspace {
                    id: 6,
                    name: "2:web".into(),
                    monitor: Some("HDMI-A-1".into()),
                    urgent: false
                },
            ]
        );
        assert_eq!(state.active(), Some(6));
        assert_eq!(state.active_window(), Some(("kitty", "lyr — vim")));
    }

    #[test]
    fn an_xwayland_window_is_named_by_its_class() {
        let state = state(
            WORKSPACES,
            &tree(r#"{"id": 12, "type": "con", "focused": true, "name": "Steam", "app_id": null, "window_properties": {"class": "steam", "instance": "steamwebhelper"}}"#),
        )
        .expect("valid replies");
        assert_eq!(state.active_window(), Some(("steam", "Steam")));
    }

    #[test]
    fn a_focused_empty_workspace_has_no_window() {
        let tree = r#"{"id": 1, "type": "root", "focused": false, "nodes": [
            {"id": 6, "type": "workspace", "name": "2:web", "focused": true, "nodes": []}
        ]}"#;
        let state = state(WORKSPACES, tree).expect("valid replies");
        assert_eq!(state.active_window(), None);
        assert_eq!(state.active(), Some(6));
    }

    #[test]
    fn a_reply_of_the_wrong_shape_is_an_error() {
        assert!(state("{}", &tree("")).is_err());
        assert!(state(WORKSPACES, "[]").is_err());
    }

    #[test]
    fn only_window_changes_that_can_move_focus_reread_the_tree() {
        for change in ["new", "close", "focus", "title"] {
            let event = format!(r#"{{"change": "{change}", "container": {{"id": 12}}}}"#);
            assert_eq!(
                window_event_moves_focus(&event).ok(),
                Some(true),
                "{change}"
            );
        }
        for change in ["move", "floating", "fullscreen_mode", "urgent", "mark"] {
            let event = format!(r#"{{"change": "{change}", "container": {{"id": 12}}}}"#);
            assert_eq!(
                window_event_moves_focus(&event).ok(),
                Some(false),
                "{change}"
            );
        }
        assert!(window_event_moves_focus("[]").is_err());
    }
}
