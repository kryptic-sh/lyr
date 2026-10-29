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

/// The request that switches to `workspace`.
///
/// Numbered workspaces are named by id in both dialects. A named workspace
/// (one with a non-positive id) is `name:NAME`, since a negative number means
/// "that many workspaces back". In Lua, `hl.focus`'s `workspace` takes that
/// string and hands it to the same selector parser hyprlang's `workspace`
/// does (`hlFocus` and `dsp_changeWorkspace` in Hyprland's
/// `src/config/lua/bindings/LuaBindingsDispatchers.cpp`, main `12f2425`,
/// read 2026-10-20); the name goes in a long string ([`lua_long_string`]), so
/// it cannot run as code.
#[must_use]
pub fn focus_request(workspace: &Workspace, dialect: Dialect) -> String {
    let id = workspace.id;
    match dialect {
        Dialect::Lua if id > 0 => format!("/dispatch hl.dsp.focus({{ workspace = {id} }})"),
        Dialect::Lua => format!(
            "/dispatch hl.dsp.focus({{ workspace = {} }})",
            lua_long_string(&format!("name:{}", workspace.name))
        ),
        Dialect::Hyprlang if id > 0 => format!("/dispatch workspace {id}"),
        Dialect::Hyprlang => format!("/dispatch workspace name:{}", workspace.name),
    }
}

/// `text` as a Lua long string, `[[…]]` with as many `=` between the brackets
/// as it takes: Lua reads its content as it is, with no escapes (the Lua 5.4
/// manual, section 3.1), up to the first closing bracket of its level. The
/// level is the least whose closing bracket first appears where it is put,
/// so neither a `]]` in the text nor a `]` at its end can close it early.
/// A long string skips a line break straight after its opening bracket, so
/// `text` must not start with one; lyr's always starts with `name:` or
/// `address:`.
fn lua_long_string(text: &str) -> String {
    let mut equals = String::new();
    loop {
        let close = format!("]{equals}]");
        if format!("{text}{close}").find(&close) == Some(text.len()) {
            return format!("[{equals}[{text}{close}");
        }
        equals.push('=');
    }
}

/// The request that focuses the window at `address`, as Hyprland names it in
/// `j/clients`, switching to the workspace it is on. Lua's `hl.dsp.focus`
/// takes the same `address:` selector hyprlang's `focuswindow` does (tried on
/// Hyprland 0.56.2, 2026-10-29); the selector goes in a long string, as a
/// workspace's name does.
#[must_use]
pub fn focus_window_request(address: &str, dialect: Dialect) -> String {
    let window = format!("address:{address}");
    match dialect {
        Dialect::Lua => format!(
            "/dispatch hl.dsp.focus({{ window = {} }})",
            lua_long_string(&window)
        ),
        Dialect::Hyprlang => format!("/dispatch focuswindow {window}"),
    }
}

/// The request that ends the Hyprland session, as the power menu's Log out
/// does. In Lua it is what the dotfiles' `.menu-power` sends today.
#[must_use]
pub const fn exit_request(dialect: Dialect) -> &'static str {
    match dialect {
        Dialect::Lua => "/dispatch hl.dsp.exit()",
        Dialect::Hyprlang => "/dispatch exit",
    }
}

#[cfg(test)]
mod tests {
    use super::{Dialect, exit_request, focus_request, focus_window_request, lua_long_string};

    #[test]
    fn exit_is_spelled_for_each_dialect() {
        assert_eq!(exit_request(Dialect::Lua), "/dispatch hl.dsp.exit()");
        assert_eq!(exit_request(Dialect::Hyprlang), "/dispatch exit");
    }
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
            focus_request(&three, Dialect::Lua),
            "/dispatch hl.dsp.focus({ workspace = 3 })"
        );
        assert_eq!(
            focus_request(&three, Dialect::Hyprlang),
            "/dispatch workspace 3"
        );
    }

    #[test]
    fn named_workspaces_are_never_sent_as_a_negative_number() {
        let mail = workspace(-1337, "mail");
        assert_eq!(
            focus_request(&mail, Dialect::Hyprlang),
            "/dispatch workspace name:mail"
        );
        assert_eq!(
            focus_request(&mail, Dialect::Lua),
            "/dispatch hl.dsp.focus({ workspace = [[name:mail]] })"
        );
    }

    #[test]
    fn a_lua_long_string_cannot_be_closed_early() {
        assert_eq!(lua_long_string("name:a"), "[[name:a]]");
        assert_eq!(
            lua_long_string("name:a]] os.exit() --"),
            "[=[name:a]] os.exit() --]=]",
            "a closing bracket in the name"
        );
        assert_eq!(
            lua_long_string("name:x]"),
            "[=[name:x]]=]",
            "a bracket at the end would join the close"
        );
        assert_eq!(lua_long_string("name:]=]]"), "[==[name:]=]]]==]");
    }

    #[test]
    fn a_window_is_focused_by_its_address_in_either_dialect() {
        assert_eq!(
            focus_window_request("0x56044ba5f780", Dialect::Lua),
            "/dispatch hl.dsp.focus({ window = [[address:0x56044ba5f780]] })"
        );
        assert_eq!(
            focus_window_request("0x56044ba5f780", Dialect::Hyprlang),
            "/dispatch focuswindow address:0x56044ba5f780"
        );
    }
}
