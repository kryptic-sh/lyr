//! Folding niri's events into the workspaces the bar shows.
//!
//! The events and their fields are `niri-ipc`'s `Event`, `Workspace` and
//! `Window` (read at niri v26.04); only the fields lyr reads are declared,
//! and events lyr does not act on, newer ones included, are skipped. The
//! folding follows `niri_ipc::state`.

use std::collections::HashMap;

use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::{State, Workspace, WorkspaceId};

/// A workspace as niri reports it.
#[derive(Clone, Debug, Deserialize)]
struct NiriWorkspace {
    /// niri's id, fixed while the workspace moves. Read as lyr's id type, so
    /// one out of its range is an error rather than a wrapped number.
    id: WorkspaceId,
    /// Its position on its output, from 1.
    idx: u8,
    name: Option<String>,
    output: Option<String>,
    /// Missing before niri reported workspace urgency.
    #[serde(default)]
    is_urgent: bool,
    is_active: bool,
    is_focused: bool,
}

/// A window as niri reports it.
#[derive(Clone, Debug, Deserialize)]
struct NiriWindow {
    id: u64,
    title: Option<String>,
    app_id: Option<String>,
    is_focused: bool,
}

/// What niri's event stream has said so far.
#[derive(Clone, Debug, Default)]
pub struct Tracker {
    workspaces: HashMap<WorkspaceId, NiriWorkspace>,
    windows: HashMap<u64, NiriWindow>,
    /// Whether the stream's first workspace and window lists have come.
    seen_workspaces: bool,
    seen_windows: bool,
}

impl Tracker {
    /// Applies one event line.
    ///
    /// # Errors
    ///
    /// When the line is not an event object, or an event lyr acts on does
    /// not have the fields niri sends.
    pub fn apply(&mut self, line: &str) -> serde_json::Result<()> {
        let event: serde_json::Map<String, serde_json::Value> = serde_json::from_str(line)?;
        for (kind, body) in event {
            self.apply_event(&kind, body)?;
        }
        Ok(())
    }

    fn apply_event(&mut self, kind: &str, body: serde_json::Value) -> serde_json::Result<()> {
        match kind {
            "WorkspacesChanged" => {
                #[derive(Deserialize)]
                struct Changed {
                    workspaces: Vec<NiriWorkspace>,
                }
                let changed: Changed = field(body)?;
                self.workspaces = changed
                    .workspaces
                    .into_iter()
                    .map(|workspace| (workspace.id, workspace))
                    .collect();
                self.seen_workspaces = true;
            }
            "WorkspaceActivated" => {
                #[derive(Deserialize)]
                struct Activated {
                    id: WorkspaceId,
                    focused: bool,
                }
                let Activated { id, focused } = field(body)?;
                let Some(output) = self.workspaces.get(&id).map(|w| w.output.clone()) else {
                    return Ok(());
                };
                for workspace in self.workspaces.values_mut() {
                    let activated = workspace.id == id;
                    if workspace.output == output {
                        workspace.is_active = activated;
                    }
                    if focused {
                        workspace.is_focused = activated;
                    }
                }
            }
            "WorkspaceUrgencyChanged" => {
                #[derive(Deserialize)]
                struct Urgency {
                    id: WorkspaceId,
                    urgent: bool,
                }
                let Urgency { id, urgent } = field(body)?;
                if let Some(workspace) = self.workspaces.get_mut(&id) {
                    workspace.is_urgent = urgent;
                }
            }
            "WindowsChanged" => {
                #[derive(Deserialize)]
                struct Changed {
                    windows: Vec<NiriWindow>,
                }
                let changed: Changed = field(body)?;
                self.windows = changed
                    .windows
                    .into_iter()
                    .map(|window| (window.id, window))
                    .collect();
                self.seen_windows = true;
            }
            "WindowOpenedOrChanged" => {
                #[derive(Deserialize)]
                struct Opened {
                    window: NiriWindow,
                }
                let Opened { window } = field(body)?;
                if window.is_focused {
                    for other in self.windows.values_mut() {
                        other.is_focused = false;
                    }
                }
                self.windows.insert(window.id, window);
            }
            "WindowClosed" => {
                #[derive(Deserialize)]
                struct Closed {
                    id: u64,
                }
                let Closed { id } = field(body)?;
                self.windows.remove(&id);
            }
            "WindowFocusChanged" => {
                #[derive(Deserialize)]
                struct Focus {
                    id: Option<u64>,
                }
                let Focus { id } = field(body)?;
                for window in self.windows.values_mut() {
                    window.is_focused = Some(window.id) == id;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Whether the stream's first workspace and window lists have both come,
    /// so the state is whole.
    #[must_use]
    pub const fn is_whole(&self) -> bool {
        self.seen_workspaces && self.seen_windows
    }

    /// What the bar shows: the workspaces by output, then by position on it,
    /// labelled by name or else by position; the focused one active; and the
    /// focused window's app id and title.
    #[must_use]
    pub fn state(&self) -> State {
        let mut workspaces: Vec<&NiriWorkspace> = self.workspaces.values().collect();
        workspaces.sort_by(|a, b| (&a.output, a.idx).cmp(&(&b.output, b.idx)));
        let window = self
            .windows
            .values()
            .find(|window| window.is_focused)
            .and_then(|window| {
                let class = window.app_id.clone().unwrap_or_default();
                let title = window.title.clone().unwrap_or_default();
                (!class.is_empty() || !title.is_empty()).then_some((class, title))
            });
        State {
            active: workspaces
                .iter()
                .find(|workspace| workspace.is_focused)
                .map(|workspace| workspace.id),
            workspaces: workspaces
                .into_iter()
                .map(|workspace| Workspace {
                    id: workspace.id,
                    name: workspace
                        .name
                        .clone()
                        .unwrap_or_else(|| workspace.idx.to_string()),
                    monitor: workspace.output.clone(),
                    urgent: workspace.is_urgent,
                })
                .collect(),
            window,
        }
    }
}

/// An event's body as `T`.
fn field<T: DeserializeOwned>(body: serde_json::Value) -> serde_json::Result<T> {
    serde_json::from_value(body)
}

#[cfg(test)]
mod tests {
    use super::Tracker;
    use crate::Workspace;

    /// Two outputs, workspace 2 on DP-1 focused, listed in no particular
    /// order, with ids that do not follow the outputs' order. Fields lyr
    /// does not read are kept.
    const WORKSPACES: &str = r#"{"WorkspacesChanged":{"workspaces":[
        {"id":3,"idx":1,"name":null,"output":"HDMI-A-1","is_urgent":false,
         "is_active":true,"is_focused":false,"active_window_id":null},
        {"id":2,"idx":2,"name":null,"output":"DP-1","is_urgent":false,
         "is_active":true,"is_focused":true,"active_window_id":8},
        {"id":5,"idx":1,"name":"web","output":"DP-1","is_urgent":false,
         "is_active":false,"is_focused":false,"active_window_id":null}
    ]}}"#;

    /// One window, focused, with the fields niri sends.
    const WINDOWS: &str = r#"{"WindowsChanged":{"windows":[
        {"id":8,"title":"vim","app_id":"kitty","pid":42,"workspace_id":2,
         "is_focused":true,"is_floating":false,"is_urgent":false,
         "layout":{"pos_in_scrolling_layout":[1,1],"tile_size":[960.0,1080.0],
                   "window_size":[956,1076],"tile_pos_in_workspace_view":null,
                   "window_offset_in_tile":[2.0,2.0]},
         "focus_timestamp":{"secs":10,"nanos":0}}
    ]}}"#;

    fn tracker(lines: &[&str]) -> Tracker {
        let mut tracker = Tracker::default();
        for line in lines {
            tracker.apply(line).expect("a valid event");
        }
        tracker
    }

    fn names(tracker: &Tracker) -> Vec<String> {
        tracker
            .state()
            .workspaces()
            .map(|workspace| workspace.name.clone())
            .collect()
    }

    #[test]
    fn the_first_lists_give_the_buttons_by_output_and_position() {
        let tracker = tracker(&[WORKSPACES, WINDOWS]);
        assert!(tracker.is_whole());
        let state = tracker.state();
        assert_eq!(
            state.workspaces().cloned().collect::<Vec<_>>(),
            [
                Workspace {
                    id: 5,
                    name: "web".into(),
                    monitor: Some("DP-1".into()),
                    urgent: false
                },
                Workspace {
                    id: 2,
                    name: "2".into(),
                    monitor: Some("DP-1".into()),
                    urgent: false
                },
                Workspace {
                    id: 3,
                    name: "1".into(),
                    monitor: Some("HDMI-A-1".into()),
                    urgent: false
                },
            ]
        );
        assert_eq!(state.active(), Some(2));
        assert_eq!(state.active_window(), Some(("kitty", "vim")));
    }

    #[test]
    fn the_state_is_whole_once_both_lists_have_come() {
        assert!(!tracker(&[]).is_whole());
        assert!(!tracker(&[WORKSPACES]).is_whole());
        assert!(!tracker(&[WINDOWS]).is_whole());
    }

    #[test]
    fn activating_moves_focus_and_activity_on_that_output_only() {
        let tracker = tracker(&[
            WORKSPACES,
            WINDOWS,
            r#"{"WorkspaceActivated":{"id":5,"focused":true}}"#,
        ]);
        assert_eq!(tracker.state().active(), Some(5));
        let active = |id| tracker.workspaces[&id].is_active;
        assert!(active(5));
        assert!(!active(2), "DP-1 shows 1 now");
        assert!(active(3), "HDMI-A-1 is untouched");

        let mut unfocused = tracker.clone();
        unfocused
            .apply(r#"{"WorkspaceActivated":{"id":3,"focused":false}}"#)
            .expect("valid");
        assert_eq!(
            unfocused.state().active(),
            Some(5),
            "activated without focus keeps the focused workspace"
        );
    }

    #[test]
    fn urgency_marks_a_workspace() {
        let tracker = tracker(&[
            WORKSPACES,
            r#"{"WorkspaceUrgencyChanged":{"id":3,"urgent":true}}"#,
        ]);
        let urgent: Vec<i64> = tracker
            .state()
            .workspaces()
            .filter(|workspace| workspace.urgent)
            .map(|workspace| workspace.id)
            .collect();
        assert_eq!(urgent, [3]);
    }

    #[test]
    fn the_focused_window_follows_its_events() {
        let opened = r#"{"WindowOpenedOrChanged":{"window":
            {"id":9,"title":"lyr","app_id":"foot","is_focused":true}}}"#;
        let mut tracker = tracker(&[WORKSPACES, WINDOWS, opened]);
        assert_eq!(tracker.state().active_window(), Some(("foot", "lyr")));

        tracker
            .apply(r#"{"WindowFocusChanged":{"id":8}}"#)
            .expect("valid");
        assert_eq!(tracker.state().active_window(), Some(("kitty", "vim")));

        tracker
            .apply(r#"{"WindowClosed":{"id":8}}"#)
            .expect("valid");
        assert_eq!(tracker.state().active_window(), None, "closed");

        tracker
            .apply(r#"{"WindowFocusChanged":{"id":null}}"#)
            .expect("valid");
        assert_eq!(tracker.state().active_window(), None);

        let blank = r#"{"WindowOpenedOrChanged":{"window":
            {"id":10,"title":null,"app_id":null,"is_focused":true}}}"#;
        tracker.apply(blank).expect("valid");
        assert_eq!(
            tracker.state().active_window(),
            None,
            "a window with neither app id nor title shows nothing"
        );
    }

    #[test]
    fn events_lyr_does_not_act_on_are_skipped() {
        let before = tracker(&[WORKSPACES, WINDOWS]).state();
        let after = tracker(&[
            WORKSPACES,
            WINDOWS,
            r#"{"KeyboardLayoutSwitched":{"idx":1}}"#,
            r#"{"SomethingNewerThanV2604":{"what":[1,2,3]}}"#,
        ])
        .state();
        assert_eq!(before, after);
    }

    #[test]
    fn a_line_of_the_wrong_shape_is_an_error() {
        let mut tracker = Tracker::default();
        assert!(tracker.apply("[]").is_err());
        assert!(tracker.apply(r#"{"WorkspacesChanged":{}}"#).is_err());
        assert!(tracker.apply(r#"{"WindowClosed":{"id":"eight"}}"#).is_err());
    }

    #[test]
    fn a_workspace_with_no_name_is_labelled_by_position() {
        assert_eq!(names(&tracker(&[WORKSPACES])), ["web", "2", "1"]);
    }
}
