//! Folding Hyprland's events into the workspaces the bar shows.

use std::collections::HashMap;

use super::Event;
use crate::{State, Workspace, WorkspaceId};

/// The [`State`] Hyprland's events keep current, with what folding them
/// needs besides.
///
/// Hyprland emits no event for workspaces that existed before lyr subscribed,
/// so a tracker starts from the initial-state query ([`Tracker::seeded`]) and
/// is kept current by events from then on.
#[derive(Clone, Debug, Default)]
pub struct Tracker {
    state: State,
    /// The monitor Hyprland creates new workspaces on.
    focused_monitor: Option<String>,
    /// Window address to workspace, so an `urgent` event, which names only a
    /// window, can mark the workspace it is on.
    windows: HashMap<String, WorkspaceId>,
}

impl Tracker {
    /// A tracker as the initial-state query found it: the workspaces, the one
    /// active on the focused monitor, which window is on which workspace, and
    /// the focused window's class and title.
    /// Special workspaces are left out, as [`Tracker::apply`] leaves them out.
    #[cfg(unix)]
    pub(super) fn seeded(
        workspaces: impl IntoIterator<Item = Workspace>,
        active: Option<(WorkspaceId, String)>,
        windows: impl IntoIterator<Item = (String, WorkspaceId)>,
        window: Option<(String, String)>,
    ) -> Self {
        let (active, focused_monitor) = active.unzip();
        let mut workspaces: Vec<Workspace> = workspaces
            .into_iter()
            .filter(|workspace| !is_special(&workspace.name))
            .collect();
        workspaces.sort_by_key(|workspace| workspace.id);
        Self {
            state: State {
                workspaces,
                active,
                window,
            },
            focused_monitor,
            windows: windows.into_iter().collect(),
        }
    }

    /// What the bar shows.
    #[must_use]
    pub const fn state(&self) -> &State {
        &self.state
    }

    /// Applies one event and returns whether anything the bar draws changed,
    /// so the caller repaints only then.
    pub fn apply(&mut self, event: Event) -> bool {
        match event {
            Event::Workspace { id, name } => {
                let inserted = self.insert(id, name);
                self.show(id) || inserted
            }
            Event::CreateWorkspace { id, name } => self.insert(id, name),
            Event::DestroyWorkspace { id } => {
                self.windows.retain(|_, workspace| *workspace != id);
                let before = self.state.workspaces.len();
                self.state.workspaces.retain(|workspace| workspace.id != id);
                self.state.workspaces.len() != before
            }
            Event::RenameWorkspace { id, name } => match self.state.get_mut(id) {
                Some(workspace) if workspace.name != name => {
                    workspace.name = name;
                    true
                }
                _ => false,
            },
            Event::FocusedMonitor { monitor, workspace } => {
                self.focused_monitor = Some(monitor);
                self.show(workspace)
            }
            Event::MoveWorkspace { id, monitor } => match self.state.get_mut(id) {
                Some(workspace) if workspace.monitor.as_ref() != Some(&monitor) => {
                    workspace.monitor = Some(monitor);
                    true
                }
                _ => false,
            },
            Event::OpenWindow {
                window,
                workspace_name,
            } => {
                let id = self
                    .state
                    .workspaces()
                    .find(|workspace| workspace.name == workspace_name)
                    .map(|workspace| workspace.id);
                if let Some(id) = id {
                    self.windows.insert(window, id);
                }
                false
            }
            Event::MoveWindow { window, workspace } => {
                self.windows.insert(window, workspace);
                false
            }
            Event::CloseWindow { window } => {
                self.windows.remove(&window);
                false
            }
            Event::ActiveWindow { class, title } => {
                let window = (!class.is_empty() || !title.is_empty()).then_some((class, title));
                let changed = window != self.state.window;
                self.state.window = window;
                changed
            }
            Event::Urgent { window } => {
                let Some(&id) = self.windows.get(&window) else {
                    return false;
                };
                if Some(id) == self.state.active {
                    return false;
                }
                match self.state.get_mut(id) {
                    Some(workspace) if !workspace.urgent => {
                        workspace.urgent = true;
                        true
                    }
                    _ => false,
                }
            }
        }
    }

    /// Adds a workspace lyr has not seen, in id order, placed on the focused
    /// monitor, which is where Hyprland creates one. Special workspaces are
    /// not buttons, so they are never added.
    fn insert(&mut self, id: WorkspaceId, name: String) -> bool {
        let workspaces = &mut self.state.workspaces;
        let at = workspaces.partition_point(|workspace| workspace.id < id);
        if is_special(&name)
            || workspaces
                .get(at)
                .is_some_and(|workspace| workspace.id == id)
        {
            return false;
        }
        let monitor = self.focused_monitor.clone();
        workspaces.insert(
            at,
            Workspace {
                id,
                name,
                monitor,
                urgent: false,
            },
        );
        true
    }

    /// Makes `id` the active workspace. Showing a workspace clears its urgent
    /// mark, as waybar does.
    fn show(&mut self, id: WorkspaceId) -> bool {
        let moved = self.state.active != Some(id);
        self.state.active = Some(id);
        let cleared = self
            .state
            .get_mut(id)
            .is_some_and(|workspace| std::mem::take(&mut workspace.urgent));
        moved || cleared
    }
}

/// Special workspaces (scratchpads) are named `special:NAME` and are not
/// buttons on the bar.
fn is_special(name: &str) -> bool {
    name.starts_with("special:")
}

#[cfg(test)]
mod tests {
    use super::Tracker;
    use crate::hyprland::parse_line;

    /// Applies event lines in order and returns what each `apply` reported.
    fn feed(state: &mut Tracker, lines: &[&str]) -> Vec<bool> {
        lines
            .iter()
            .map(|line| {
                let event = parse_line(line)
                    .expect("valid line")
                    .expect("an event lyr acts on");
                state.apply(event)
            })
            .collect()
    }

    fn names(state: &Tracker) -> Vec<&str> {
        state
            .state()
            .workspaces()
            .map(|workspace| workspace.name.as_str())
            .collect()
    }

    #[test]
    fn workspaces_appear_in_id_order_on_the_focused_monitor() {
        let mut state = Tracker::default();
        feed(
            &mut state,
            &[
                "focusedmonv2>>DP-1,1",
                "createworkspacev2>>3,3",
                "createworkspacev2>>1,1",
                "createworkspacev2>>-98,special:scratch",
            ],
        );
        assert_eq!(names(&state), ["1", "3"]);
        assert!(
            state
                .state()
                .workspaces()
                .all(|workspace| workspace.monitor.as_deref() == Some("DP-1"))
        );
    }

    #[test]
    fn switching_moves_the_active_workspace_and_reports_a_change() {
        let mut state = Tracker::default();
        let changed = feed(
            &mut state,
            &[
                "createworkspacev2>>1,1",
                "workspacev2>>1,1",
                "workspacev2>>1,1",
                "createworkspacev2>>2,2",
                "workspacev2>>2,2",
            ],
        );
        assert_eq!(changed, [true, true, false, true, true]);
        assert_eq!(state.state().active(), Some(2));
    }

    #[test]
    fn rename_move_and_destroy_change_the_buttons() {
        let mut state = Tracker::default();
        let changed = feed(
            &mut state,
            &[
                "createworkspacev2>>2,2",
                "renameworkspace>>2,web",
                "renameworkspace>>2,web",
                "moveworkspacev2>>2,web,HDMI-A-1",
                "destroyworkspacev2>>2,web",
                "destroyworkspacev2>>2,web",
            ],
        );
        assert_eq!(changed, [true, true, false, true, true, false]);
        assert!(names(&state).is_empty());
    }

    #[test]
    fn an_urgent_window_marks_its_workspace_until_it_is_shown() {
        let mut state = Tracker::default();
        feed(
            &mut state,
            &[
                "createworkspacev2>>1,1",
                "createworkspacev2>>2,2",
                "workspacev2>>1,1",
                "openwindow>>aa,2,kitty,shell",
                "movewindowv2>>bb,1,1",
            ],
        );
        let urgent = |state: &Tracker| -> Vec<i64> {
            state
                .state()
                .workspaces()
                .filter(|workspace| workspace.urgent)
                .map(|workspace| workspace.id)
                .collect()
        };

        assert_eq!(feed(&mut state, &["urgent>>bb"]), [false], "already shown");
        assert_eq!(feed(&mut state, &["urgent>>zz"]), [false], "unknown window");
        assert_eq!(feed(&mut state, &["urgent>>aa"]), [true]);
        assert_eq!(urgent(&state), [2]);

        assert_eq!(feed(&mut state, &["focusedmonv2>>DP-2,2"]), [true]);
        assert!(urgent(&state).is_empty());
    }

    #[test]
    fn a_closed_window_is_forgotten() {
        let mut state = Tracker::default();
        feed(
            &mut state,
            &[
                "createworkspacev2>>2,2",
                "movewindowv2>>aa,2,2",
                "closewindow>>aa",
            ],
        );
        assert_eq!(feed(&mut state, &["urgent>>aa"]), [false]);
    }

    #[test]
    fn the_focused_window_follows_its_events() {
        let mut state = Tracker::default();
        assert_eq!(state.state().active_window(), None);
        assert_eq!(
            feed(
                &mut state,
                &[
                    "activewindow>>kitty,vim",
                    "activewindow>>kitty,vim",
                    "activewindow>>kitty,vim: lyr",
                ]
            ),
            [true, false, true],
            "a title change redraws, the same one does not"
        );
        assert_eq!(state.state().active_window(), Some(("kitty", "vim: lyr")));
        assert_eq!(feed(&mut state, &["activewindow>>,"]), [true]);
        assert_eq!(state.state().active_window(), None, "no window focused");
    }
}
