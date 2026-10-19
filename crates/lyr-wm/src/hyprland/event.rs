//! Parsing `.socket2.sock` event lines.
//!
//! The formats are the ones Hyprland's wiki lists for each event
//! (`content/ipc/_index.md` in hyprwm/hyprland-wiki). Fields are separated by
//! commas and nothing is escaped, so a name that contains a comma is only
//! recoverable when it is the one field whose position is fixed by the
//! others; each variant says how it splits.

use super::WorkspaceId;

/// An event lyr acts on. Every other event is skipped by [`parse_line`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// `workspacev2>>ID,NAME`: the focused monitor now shows this workspace.
    /// The name is everything after the first comma.
    Workspace { id: WorkspaceId, name: String },
    /// `createworkspacev2>>ID,NAME`, split like [`Event::Workspace`].
    CreateWorkspace { id: WorkspaceId, name: String },
    /// `destroyworkspacev2>>ID,NAME`.
    DestroyWorkspace { id: WorkspaceId },
    /// `renameworkspace>>ID,NEWNAME`, split like [`Event::Workspace`].
    RenameWorkspace { id: WorkspaceId, name: String },
    /// `focusedmonv2>>MONNAME,ID`: focus moved to this monitor, which shows
    /// this workspace. The id is everything after the last comma.
    FocusedMonitor {
        monitor: String,
        workspace: WorkspaceId,
    },
    /// `moveworkspacev2>>ID,NAME,MONNAME`: the workspace moved to this
    /// monitor. The monitor is everything after the last comma.
    MoveWorkspace { id: WorkspaceId, monitor: String },
    /// `openwindow>>ADDRESS,WORKSPACENAME,CLASS,TITLE`. The workspace name is
    /// the second field, so a workspace name containing a comma is cut short
    /// here and the window is not matched to it.
    OpenWindow {
        window: String,
        workspace_name: String,
    },
    /// `movewindowv2>>ADDRESS,ID,NAME`: the window moved to this workspace.
    MoveWindow {
        window: String,
        workspace: WorkspaceId,
    },
    /// `closewindow>>ADDRESS`.
    CloseWindow { window: String },
    /// `urgent>>ADDRESS`: a window asked for attention.
    Urgent { window: String },
    /// `activewindow>>CLASS,TITLE`: the focused window, or its title, changed.
    /// The title is everything after the first comma; both are empty when no
    /// window has focus.
    ActiveWindow { class: String, title: String },
}

/// An event line lyr acts on that does not have the shape its event's
/// documented format says it should.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ParseError {
    #[error("event line has no `>>`: {0:?}")]
    NoSeparator(String),
    #[error("malformed `{event}` event data: {data:?}")]
    Malformed { event: String, data: String },
}

/// Parses one event line, without its trailing newline.
///
/// Returns `Ok(None)` for an event lyr does not act on, so that new events in
/// a future Hyprland are skipped rather than reported as errors.
pub fn parse_line(line: &str) -> Result<Option<Event>, ParseError> {
    let (event, data) = line
        .split_once(">>")
        .ok_or_else(|| ParseError::NoSeparator(line.to_owned()))?;
    let parsed = match event {
        "workspacev2" => id_and_name(data).map(|(id, name)| Event::Workspace { id, name }),
        "createworkspacev2" => {
            id_and_name(data).map(|(id, name)| Event::CreateWorkspace { id, name })
        }
        "destroyworkspacev2" => id_and_name(data).map(|(id, _)| Event::DestroyWorkspace { id }),
        "renameworkspace" => {
            id_and_name(data).map(|(id, name)| Event::RenameWorkspace { id, name })
        }
        "focusedmonv2" => data.rsplit_once(',').and_then(|(monitor, id)| {
            Some(Event::FocusedMonitor {
                monitor: monitor.to_owned(),
                workspace: id.parse().ok()?,
            })
        }),
        "moveworkspacev2" => data.split_once(',').and_then(|(id, rest)| {
            let (_name, monitor) = rest.rsplit_once(',')?;
            Some(Event::MoveWorkspace {
                id: id.parse().ok()?,
                monitor: monitor.to_owned(),
            })
        }),
        "openwindow" => {
            let mut fields = data.splitn(4, ',');
            match (fields.next(), fields.next(), fields.next(), fields.next()) {
                (Some(window), Some(workspace_name), Some(_class), Some(_title)) => {
                    Some(Event::OpenWindow {
                        window: window.to_owned(),
                        workspace_name: workspace_name.to_owned(),
                    })
                }
                _ => None,
            }
        }
        "movewindowv2" => {
            let mut fields = data.splitn(3, ',');
            match (fields.next(), fields.next(), fields.next()) {
                (Some(window), Some(id), Some(_name)) => {
                    id.parse().ok().map(|workspace| Event::MoveWindow {
                        window: window.to_owned(),
                        workspace,
                    })
                }
                _ => None,
            }
        }
        "closewindow" => Some(Event::CloseWindow {
            window: data.to_owned(),
        }),
        "urgent" => Some(Event::Urgent {
            window: data.to_owned(),
        }),
        "activewindow" => data
            .split_once(',')
            .map(|(class, title)| Event::ActiveWindow {
                class: class.to_owned(),
                title: title.to_owned(),
            }),
        _ => return Ok(None),
    };
    parsed.map(Some).ok_or_else(|| ParseError::Malformed {
        event: event.to_owned(),
        data: data.to_owned(),
    })
}

/// `ID,NAME`, where the name is everything after the first comma.
fn id_and_name(data: &str) -> Option<(WorkspaceId, String)> {
    let (id, name) = data.split_once(',')?;
    Some((id.parse().ok()?, name.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::{Event, ParseError, parse_line};

    fn parsed(line: &str) -> Event {
        match parse_line(line) {
            Ok(Some(event)) => event,
            other => panic!("{line:?} parsed to {other:?}"),
        }
    }

    #[test]
    fn workspace_events_take_the_name_after_the_first_comma() {
        assert_eq!(
            parsed("workspacev2>>3,3"),
            Event::Workspace {
                id: 3,
                name: "3".into()
            }
        );
        assert_eq!(
            parsed("createworkspacev2>>-1337,mail, chat"),
            Event::CreateWorkspace {
                id: -1337,
                name: "mail, chat".into()
            }
        );
        assert_eq!(
            parsed("renameworkspace>>2,web"),
            Event::RenameWorkspace {
                id: 2,
                name: "web".into()
            }
        );
        assert_eq!(
            parsed("destroyworkspacev2>>4,4"),
            Event::DestroyWorkspace { id: 4 }
        );
    }

    #[test]
    fn monitor_events_take_the_monitor_from_its_fixed_end() {
        assert_eq!(
            parsed("focusedmonv2>>DP-1,2"),
            Event::FocusedMonitor {
                monitor: "DP-1".into(),
                workspace: 2
            }
        );
        assert_eq!(
            parsed("moveworkspacev2>>5,a,b,HDMI-A-1"),
            Event::MoveWorkspace {
                id: 5,
                monitor: "HDMI-A-1".into()
            }
        );
    }

    #[test]
    fn window_events_keep_the_address() {
        assert_eq!(
            parsed("openwindow>>5f1e2a,3,kitty,~/src, again"),
            Event::OpenWindow {
                window: "5f1e2a".into(),
                workspace_name: "3".into()
            }
        );
        assert_eq!(
            parsed("movewindowv2>>5f1e2a,4,4"),
            Event::MoveWindow {
                window: "5f1e2a".into(),
                workspace: 4
            }
        );
        assert_eq!(
            parsed("closewindow>>5f1e2a"),
            Event::CloseWindow {
                window: "5f1e2a".into()
            }
        );
        assert_eq!(
            parsed("urgent>>5f1e2a"),
            Event::Urgent {
                window: "5f1e2a".into()
            }
        );
    }

    #[test]
    fn events_lyr_does_not_act_on_are_skipped() {
        for line in [
            "workspace>>3",
            "activewindowv2>>5f1e2a",
            "activespecialv2>>,,DP-1",
            "some_future_event>>x",
        ] {
            assert_eq!(parse_line(line), Ok(None), "{line}");
        }
    }

    #[test]
    fn the_active_window_s_title_is_after_the_first_comma() {
        assert_eq!(
            parse_line("activewindow>>kitty,~/src: vim, a,b"),
            Ok(Some(Event::ActiveWindow {
                class: "kitty".into(),
                title: "~/src: vim, a,b".into(),
            }))
        );
        assert_eq!(
            parse_line("activewindow>>,"),
            Ok(Some(Event::ActiveWindow {
                class: String::new(),
                title: String::new(),
            })),
            "no window"
        );
    }

    #[test]
    fn a_known_event_with_bad_data_is_an_error() {
        for line in [
            "activewindow>>kitty",
            "workspacev2>>three,3",
            "workspacev2>>3",
            "focusedmonv2>>DP-1",
            "moveworkspacev2>>5,HDMI-A-1",
            "openwindow>>5f1e2a,3,kitty",
            "movewindowv2>>5f1e2a,x,4",
        ] {
            assert!(
                matches!(parse_line(line), Err(ParseError::Malformed { .. })),
                "{line}"
            );
        }
        assert_eq!(
            parse_line("workspacev2 3"),
            Err(ParseError::NoSeparator("workspacev2 3".into()))
        );
    }
}
