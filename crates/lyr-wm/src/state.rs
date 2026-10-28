//! What the workspace module shows, whichever backend reports it.

/// A workspace id, as the backend that reports it numbers workspaces.
/// Hyprland gives numbered workspaces positive ids and named and special
/// ones negative ids, so the sign says nothing about whether a workspace is
/// shown.
pub type WorkspaceId = i64;

/// One workspace button.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Workspace {
    pub id: WorkspaceId,
    pub name: String,
    /// The output it is on, by the compositor's name for it. `None` until
    /// the backend says where it is.
    pub monitor: Option<String>,
    /// A window on it asked for attention since it was last shown.
    pub urgent: bool,
}

/// Which way a scroll over the workspace module moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// To the next workspace button.
    Next,
    /// To the previous workspace button.
    Previous,
}

/// What the workspace module shows: every workspace in the order its
/// buttons are drawn, which one is active, and the focused window.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct State {
    /// The buttons in display order; each backend decides that order.
    pub(crate) workspaces: Vec<Workspace>,
    pub(crate) active: Option<WorkspaceId>,
    /// The focused window's class and title.
    pub(crate) window: Option<(String, String)>,
}

impl State {
    /// The focused window's class and title, while a window has focus.
    #[must_use]
    pub fn active_window(&self) -> Option<(&str, &str)> {
        self.window
            .as_ref()
            .map(|(class, title)| (class.as_str(), title.as_str()))
    }

    /// The workspaces in the order their buttons are drawn.
    pub fn workspaces(&self) -> impl Iterator<Item = &Workspace> {
        self.workspaces.iter()
    }

    /// The workspace shown on the focused output.
    #[must_use]
    pub const fn active(&self) -> Option<WorkspaceId> {
        self.active
    }

    /// The focused output, by the compositor's name for it: the active
    /// workspace's, once the backend has said where that is.
    #[must_use]
    pub fn focused_output(&self) -> Option<&str> {
        let active = self.active?;
        self.workspaces
            .iter()
            .find(|workspace| workspace.id == active)?
            .monitor
            .as_deref()
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

    /// The workspace with `id`, to change in place.
    pub(crate) fn get_mut(&mut self, id: WorkspaceId) -> Option<&mut Workspace> {
        self.workspaces
            .iter_mut()
            .find(|workspace| workspace.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::{State, Step, Workspace, WorkspaceId};

    fn state(ids: &[(WorkspaceId, &str)], active: Option<WorkspaceId>) -> State {
        State {
            workspaces: ids
                .iter()
                .map(|&(id, monitor)| Workspace {
                    id,
                    name: id.to_string(),
                    monitor: Some(monitor.to_owned()),
                    urgent: false,
                })
                .collect(),
            active,
            window: None,
        }
    }

    fn id(workspace: Option<&Workspace>) -> Option<WorkspaceId> {
        workspace.map(|workspace| workspace.id)
    }

    #[test]
    fn the_focused_output_is_the_active_workspace_s() {
        let ids = [(1, "DP-1"), (2, "HDMI-A-1")];
        assert_eq!(state(&ids, Some(2)).focused_output(), Some("HDMI-A-1"));
        assert_eq!(state(&ids, Some(1)).focused_output(), Some("DP-1"));
        assert_eq!(state(&ids, None).focused_output(), None);
        assert_eq!(state(&ids, Some(3)).focused_output(), None, "not listed");
    }

    #[test]
    fn scrolling_steps_through_the_buttons_in_their_order_and_wraps() {
        // Display order, not id order: a backend may order by output.
        let ids = [(5, "DP-1"), (1, "DP-1"), (2, "DP-1")];
        let at = |active| state(&ids, Some(active));
        assert_eq!(id(at(1).step(Step::Next, None)), Some(2));
        assert_eq!(id(at(1).step(Step::Previous, None)), Some(5));
        assert_eq!(id(at(2).step(Step::Next, None)), Some(5), "wraps forward");
        assert_eq!(id(at(5).step(Step::Previous, None)), Some(2), "wraps back");
    }

    #[test]
    fn a_per_output_bar_steps_only_through_its_own_workspaces() {
        let state = state(&[(1, "DP-1"), (2, "HDMI-A-1"), (3, "DP-1")], Some(1));
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
        assert_eq!(state(&[(1, "DP-1")], None).step(Step::Next, None), None);
    }
}
