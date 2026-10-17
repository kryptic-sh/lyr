//! Folding events into the workspaces the bar shows.

use std::collections::{BTreeMap, HashMap};

use super::{Event, WorkspaceId};

/// One workspace button.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Workspace {
    pub id: WorkspaceId,
    pub name: String,
    /// The monitor it is on, by Hyprland's monitor name. `None` until an event
    /// says where it is.
    pub monitor: Option<String>,
    /// A window on it asked for attention since it was last shown.
    pub urgent: bool,
}

/// Which way a scroll over the workspace module moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// To the next workspace in id order.
    Next,
    /// To the previous workspace in id order.
    Previous,
}

/// What the workspace module shows: every workspace in id order, which one is
/// active, and which are urgent.
///
/// Hyprland emits no event for workspaces that existed before lyr subscribed,
/// so a state starts from the initial-state query ([`State::seeded`]) and is
/// kept current by events from then on.
#[derive(Clone, Debug, Default)]
pub struct State {
    workspaces: BTreeMap<WorkspaceId, Workspace>,
    active: Option<WorkspaceId>,
    focused_monitor: Option<String>,
    /// Window address to workspace, so an `urgent` event, which names only a
    /// window, can mark the workspace it is on.
    windows: HashMap<String, WorkspaceId>,
}

impl State {
    /// A state as the initial-state query found it: the workspaces, the one
    /// active on the focused monitor, and which window is on which workspace.
    /// Special workspaces are left out, as [`State::apply`] leaves them out.
    #[cfg(unix)]
    pub(super) fn seeded(
        workspaces: impl IntoIterator<Item = Workspace>,
        active: Option<(WorkspaceId, String)>,
        windows: impl IntoIterator<Item = (String, WorkspaceId)>,
    ) -> Self {
        let (active, focused_monitor) = active.unzip();
        Self {
            workspaces: workspaces
                .into_iter()
                .filter(|workspace| !is_special(&workspace.name))
                .map(|workspace| (workspace.id, workspace))
                .collect(),
            active,
            focused_monitor,
            windows: windows.into_iter().collect(),
        }
    }

    /// The workspaces in id order, special workspaces left out.
    pub fn workspaces(&self) -> impl Iterator<Item = &Workspace> {
        self.workspaces.values()
    }

    /// The workspace shown on the focused monitor.
    #[must_use]
    pub const fn active(&self) -> Option<WorkspaceId> {
        self.active
    }

    /// The workspace a scroll over the module moves to: the button after (or
    /// before) the active one, wrapping at the ends.
    ///
    /// `monitor` is the bar's output when it shows only that output's
    /// workspaces, and `None` when every bar shows every workspace. `None`
    /// is returned when the active workspace is not among those buttons
    /// (it is on another output), since there is no button to step from.
    #[must_use]
    pub fn step(&self, step: Step, monitor: Option<&str>) -> Option<&Workspace> {
        let buttons: Vec<&Workspace> = self
            .workspaces()
            .filter(|workspace| monitor.is_none_or(|m| workspace.monitor.as_deref() == Some(m)))
            .collect();
        let active = self.active?;
        let at = buttons
            .iter()
            .position(|workspace| workspace.id == active)?;
        let len = buttons.len();
        let to = match step {
            Step::Next => (at + 1) % len,
            Step::Previous => (at + len - 1) % len,
        };
        Some(buttons[to])
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
                self.workspaces.remove(&id).is_some()
            }
            Event::RenameWorkspace { id, name } => match self.workspaces.get_mut(&id) {
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
            Event::MoveWorkspace { id, monitor } => match self.workspaces.get_mut(&id) {
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
                    .workspaces
                    .values()
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
            Event::Urgent { window } => {
                let Some(&id) = self.windows.get(&window) else {
                    return false;
                };
                if Some(id) == self.active {
                    return false;
                }
                match self.workspaces.get_mut(&id) {
                    Some(workspace) if !workspace.urgent => {
                        workspace.urgent = true;
                        true
                    }
                    _ => false,
                }
            }
        }
    }

    /// Adds a workspace lyr has not seen, placed on the focused monitor,
    /// which is where Hyprland creates one. Special workspaces are not
    /// buttons, so they are never added.
    fn insert(&mut self, id: WorkspaceId, name: String) -> bool {
        if is_special(&name) || self.workspaces.contains_key(&id) {
            return false;
        }
        let monitor = self.focused_monitor.clone();
        self.workspaces.insert(
            id,
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
        let moved = self.active != Some(id);
        self.active = Some(id);
        let cleared = self
            .workspaces
            .get_mut(&id)
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
    use super::{State, Step, Workspace};
    use crate::hyprland::parse_line;

    /// Applies event lines in order and returns what each `apply` reported.
    fn feed(state: &mut State, lines: &[&str]) -> Vec<bool> {
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

    fn names(state: &State) -> Vec<&str> {
        state
            .workspaces()
            .map(|workspace| workspace.name.as_str())
            .collect()
    }

    #[test]
    fn workspaces_appear_in_id_order_on_the_focused_monitor() {
        let mut state = State::default();
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
                .workspaces()
                .all(|workspace| workspace.monitor.as_deref() == Some("DP-1"))
        );
    }

    #[test]
    fn switching_moves_the_active_workspace_and_reports_a_change() {
        let mut state = State::default();
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
        assert_eq!(state.active(), Some(2));
    }

    #[test]
    fn rename_move_and_destroy_change_the_buttons() {
        let mut state = State::default();
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
        let mut state = State::default();
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
        let urgent = |state: &State| -> Vec<i64> {
            state
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
    fn scrolling_steps_through_the_buttons_and_wraps() {
        let mut state = State::default();
        feed(
            &mut state,
            &[
                "focusedmonv2>>DP-1,1",
                "createworkspacev2>>1,1",
                "createworkspacev2>>2,2",
                "createworkspacev2>>5,5",
                "workspacev2>>2,2",
            ],
        );
        let id = |workspace: Option<&Workspace>| workspace.map(|workspace| workspace.id);

        assert_eq!(id(state.step(Step::Next, None)), Some(5));
        assert_eq!(id(state.step(Step::Previous, None)), Some(1));

        feed(&mut state, &["workspacev2>>5,5"]);
        assert_eq!(id(state.step(Step::Next, None)), Some(1), "wraps forward");
        feed(&mut state, &["workspacev2>>1,1"]);
        assert_eq!(id(state.step(Step::Previous, None)), Some(5), "wraps back");
    }

    #[test]
    fn a_per_output_bar_steps_only_through_its_own_workspaces() {
        let mut state = State::default();
        feed(
            &mut state,
            &[
                "focusedmonv2>>DP-1,1",
                "createworkspacev2>>1,1",
                "createworkspacev2>>3,3",
                "focusedmonv2>>HDMI-A-1,2",
                "createworkspacev2>>2,2",
                "focusedmonv2>>DP-1,1",
            ],
        );
        let id = |workspace: Option<&Workspace>| workspace.map(|workspace| workspace.id);

        assert_eq!(id(state.step(Step::Next, Some("DP-1"))), Some(3));
        assert_eq!(id(state.step(Step::Next, None)), Some(2));
        assert_eq!(
            id(state.step(Step::Next, Some("HDMI-A-1"))),
            None,
            "the active workspace is on another output"
        );
    }

    #[test]
    fn nothing_to_step_from_before_a_workspace_is_active() {
        let mut state = State::default();
        feed(&mut state, &["createworkspacev2>>1,1"]);
        assert_eq!(state.step(Step::Next, None), None);
    }

    #[test]
    fn a_closed_window_is_forgotten() {
        let mut state = State::default();
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
}
