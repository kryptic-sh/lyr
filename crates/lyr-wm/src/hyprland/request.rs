//! Building `.socket.sock` requests.
//!
//! `hyprctl` frames a request as `FLAGS/COMMAND ARGS` and writes it raw, with
//! no newline, then reads until Hyprland closes the connection
//! (`hyprctl/src/main.cpp` in hyprwm/Hyprland). With no flags the request
//! starts with the bare `/`. Building the same bytes here is what lets a click
//! skip spawning `hyprctl`.

use super::Workspace;

/// Which dispatcher syntax the running Hyprland config accepts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dialect {
    /// A Lua config (Hyprland 0.55+): `dispatch` takes a Lua expression, and
    /// the classic `dispatch workspace N` is a parse error.
    Lua,
    /// A hyprlang config: `dispatch workspace N`.
    Hyprlang,
}

/// The request that switches to `workspace`, or `None` when lyr has no safe
/// way to name that workspace in this dialect.
///
/// Numbered workspaces are named by id in both dialects. A named workspace
/// (one with a non-positive id) is `name:NAME` in hyprlang, since a negative
/// number there means "that many workspaces back". In Lua it is `None`: the
/// form the dotfiles use takes a number, and splicing a name into a Lua
/// expression without a quoting rule would let the name run as code.
#[must_use]
pub fn focus_request(workspace: &Workspace, dialect: Dialect) -> Option<String> {
    let id = workspace.id;
    match dialect {
        Dialect::Lua if id > 0 => Some(format!("/dispatch hl.dsp.focus({{ workspace = {id} }})")),
        Dialect::Lua => None,
        Dialect::Hyprlang if id > 0 => Some(format!("/dispatch workspace {id}")),
        Dialect::Hyprlang => Some(format!("/dispatch workspace name:{}", workspace.name)),
    }
}

#[cfg(test)]
mod tests {
    use super::{Dialect, focus_request};
    use crate::hyprland::Workspace;

    fn workspace(id: i64, name: &str) -> Workspace {
        Workspace {
            id,
            name: name.into(),
            monitor: None,
            urgent: false,
        }
    }

    #[test]
    fn numbered_workspaces_are_named_by_id() {
        let three = workspace(3, "3");
        assert_eq!(
            focus_request(&three, Dialect::Lua).as_deref(),
            Some("/dispatch hl.dsp.focus({ workspace = 3 })")
        );
        assert_eq!(
            focus_request(&three, Dialect::Hyprlang).as_deref(),
            Some("/dispatch workspace 3")
        );
    }

    #[test]
    fn named_workspaces_are_never_sent_as_a_negative_number() {
        let mail = workspace(-1337, "mail");
        assert_eq!(
            focus_request(&mail, Dialect::Hyprlang).as_deref(),
            Some("/dispatch workspace name:mail")
        );
        assert_eq!(focus_request(&mail, Dialect::Lua), None);
    }
}
