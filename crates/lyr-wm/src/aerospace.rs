//! The AeroSpace workspace backend, macOS's (`docs/plan/06-workspaces.md`).
//!
//! AeroSpace is driven through its `aerospace` command: `list-workspaces
//! --all --json` and `list-windows --focused --json` give the state, and
//! `workspace` switches. AeroSpace pushes nothing to a bar, so its config
//! runs `lyr event workspace` from `on-focus-changed` (any focused window or
//! workspace change), which has lyr read the state again. The commands and
//! their JSON are AeroSpace v0.21.3-Beta's (`docs/aerospace-list-workspaces.adoc`
//! and `Sources/AppBundle/command/formatToJson.swift`).
//!
//! [`state`] and [`focus_args`] are what the commands mean; [`read`] and
//! [`focus`] run them, on macOS only.

use serde::Deserialize;

use crate::{State, Workspace, WorkspaceId};

/// The command lyr runs.
pub const PROGRAM: &str = "aerospace";

/// Every workspace, as JSON objects keyed by the format's variables.
pub const LIST_WORKSPACES: [&str; 5] = [
    "list-workspaces",
    "--all",
    "--json",
    "--format",
    "%{workspace} %{workspace-is-focused} %{monitor-name}",
];

/// The focused window, if any, as a JSON array of at most one object.
pub const FOCUSED_WINDOW: [&str; 5] = [
    "list-windows",
    "--focused",
    "--json",
    "--format",
    "%{app-name} %{window-title}",
];

#[derive(Deserialize)]
struct WorkspaceReply {
    workspace: String,
    #[serde(rename = "workspace-is-focused")]
    focused: bool,
    #[serde(rename = "monitor-name")]
    monitor: String,
}

#[derive(Deserialize)]
struct WindowReply {
    #[serde(rename = "app-name")]
    app: String,
    #[serde(rename = "window-title")]
    title: String,
}

/// The state the two replies describe. `window` is `None` when no window
/// has focus (`list-windows --focused` then fails, as AeroSpace reports
/// it).
///
/// AeroSpace names workspaces and numbers none, so each button's id is its
/// place in the list, which keeps AeroSpace's order; a click switches by
/// name, so the id never reaches AeroSpace.
///
/// # Errors
///
/// When a reply is not the JSON AeroSpace writes.
pub fn state(workspaces: &str, window: Option<&str>) -> serde_json::Result<State> {
    let workspaces: Vec<WorkspaceReply> = serde_json::from_str(workspaces)?;
    let window = match window {
        Some(reply) => {
            let windows: Vec<WindowReply> = serde_json::from_str(reply)?;
            windows
                .into_iter()
                .next()
                .map(|window| (window.app, window.title))
        }
        None => None,
    };
    let mut active = None;
    let workspaces = workspaces
        .into_iter()
        .zip(1..)
        .map(|(reply, id): (WorkspaceReply, WorkspaceId)| {
            if reply.focused {
                active = Some(id);
            }
            Workspace {
                id,
                name: reply.workspace,
                monitor: Some(reply.monitor),
                urgent: false,
            }
        })
        .collect();
    Ok(State {
        workspaces,
        active,
        window,
    })
}

/// The arguments that switch to `workspace`. `--` ends the options, so a
/// name starting with `-` is a name; each argument goes to AeroSpace as is,
/// with no shell to quote for.
#[must_use]
pub fn focus_args(workspace: &Workspace) -> [&str; 3] {
    ["workspace", "--", &workspace.name]
}

#[cfg(target_os = "macos")]
pub use run::{Error, focus, read};

#[cfg(target_os = "macos")]
mod run {
    use std::io;
    use std::process::{Command, Output};

    use super::{FOCUSED_WINDOW, LIST_WORKSPACES, PROGRAM, focus_args, state};
    use crate::{State, Workspace};

    /// What went wrong running AeroSpace's command.
    #[derive(Debug, thiserror::Error)]
    pub enum Error {
        #[error("`aerospace` did not run (is AeroSpace installed?): {0}")]
        NotRunning(io::Error),
        #[error("`aerospace {args}` failed: {stderr}")]
        Failed { args: String, stderr: String },
        #[error("`aerospace {args}` wrote what lyr does not read: {source}")]
        Json {
            args: String,
            source: serde_json::Error,
        },
    }

    fn run(args: &[&str]) -> Result<Output, Error> {
        Command::new(PROGRAM)
            .args(args)
            .output()
            .map_err(Error::NotRunning)
    }

    /// The output of `args`, or why it failed.
    fn stdout(args: &[&str]) -> Result<String, Error> {
        let output = run(args)?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).into_owned())
        } else {
            Err(Error::Failed {
                args: args.join(" "),
                stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
            })
        }
    }

    /// The workspaces and the focused window now.
    ///
    /// # Errors
    ///
    /// When `aerospace` cannot run, fails to list the workspaces, or writes
    /// JSON lyr does not read.
    pub fn read() -> Result<State, Error> {
        let workspaces = stdout(&LIST_WORKSPACES)?;
        // With no window focused the command fails, which means no window.
        let window = stdout(&FOCUSED_WINDOW).ok();
        state(&workspaces, window.as_deref()).map_err(|source| Error::Json {
            args: LIST_WORKSPACES.join(" "),
            source,
        })
    }

    /// Switches to `workspace`.
    ///
    /// # Errors
    ///
    /// When `aerospace` cannot run or refuses.
    pub fn focus(workspace: &Workspace) -> Result<(), Error> {
        stdout(&focus_args(workspace)).map(drop)
    }
}

#[cfg(test)]
mod tests {
    use super::{focus_args, state};
    use crate::Workspace;

    /// Two monitors, workspace 2 focused, as `list-workspaces --all --json`
    /// writes them for the format lyr asks for.
    const WORKSPACES: &str = r#"[
        {"workspace": "1", "workspace-is-focused": false, "monitor-name": "Built-in Retina Display"},
        {"workspace": "2", "workspace-is-focused": true, "monitor-name": "Built-in Retina Display"},
        {"workspace": "M", "workspace-is-focused": false, "monitor-name": "DELL U2720Q"}
    ]"#;

    #[test]
    fn the_replies_give_the_buttons_in_aerospaces_order() {
        let window = r#"[{"app-name": "kitty", "window-title": "vim"}]"#;
        let state = state(WORKSPACES, Some(window)).expect("valid replies");
        assert_eq!(
            state.workspaces().cloned().collect::<Vec<_>>(),
            [
                Workspace {
                    id: 1,
                    name: "1".into(),
                    monitor: Some("Built-in Retina Display".into()),
                    urgent: false
                },
                Workspace {
                    id: 2,
                    name: "2".into(),
                    monitor: Some("Built-in Retina Display".into()),
                    urgent: false
                },
                Workspace {
                    id: 3,
                    name: "M".into(),
                    monitor: Some("DELL U2720Q".into()),
                    urgent: false
                },
            ]
        );
        assert_eq!(state.active(), Some(2));
        assert_eq!(state.active_window(), Some(("kitty", "vim")));
    }

    #[test]
    fn no_focused_window_is_no_window() {
        assert_eq!(
            state(WORKSPACES, None).expect("valid").active_window(),
            None
        );
        assert_eq!(
            state(WORKSPACES, Some("[]"))
                .expect("valid")
                .active_window(),
            None
        );
    }

    #[test]
    fn a_reply_of_the_wrong_shape_is_an_error() {
        assert!(state("{}", None).is_err());
        assert!(state(WORKSPACES, Some("{}")).is_err());
    }

    #[test]
    fn a_switch_ends_the_options_before_the_name() {
        let named = Workspace {
            id: 1,
            name: "-web".into(),
            monitor: None,
            urgent: false,
        };
        assert_eq!(focus_args(&named), ["workspace", "--", "-web"]);
    }
}
