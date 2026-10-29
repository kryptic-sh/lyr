//! The sway workspace backend.
//!
//! sway speaks i3's IPC over the Unix socket `$SWAYSOCK` names
//! (`sway-ipc(7)`, read at sway 1.12): each message is the magic `i3-ipc`, a
//! payload length and a type, both 32-bit in native byte order, then a JSON
//! payload. Replies are framed the same, and so are events, with the type's
//! high bit set.
//!
//! [`frame`] and [`Header`] are that framing, and [`focus_command`] writes
//! the command a click sends; those are platform-free. `state` builds the
//! bar's [`crate::State`] from the replies to `GET_WORKSPACES` and
//! `GET_TREE`, and `Sway` does the socket I/O. Those exist only on Unix,
//! where sway runs, and where lyr has its JSON parser.

#[cfg(unix)]
mod ipc;
#[cfg(unix)]
mod reply;

#[cfg(unix)]
pub use ipc::{Error, Events, Sway};
#[cfg(unix)]
pub use reply::{app_window, state, window_event_moves_focus};

use crate::Workspace;

/// The magic every message starts with, kept from i3.
pub const MAGIC: &[u8; 6] = b"i3-ipc";

/// The length of a header: the magic, the payload length and the type.
pub const HEADER_LENGTH: usize = MAGIC.len() + 8;

/// Runs the payload as sway commands.
pub const RUN_COMMAND: u32 = 0;
/// Lists the workspaces.
pub const GET_WORKSPACES: u32 = 1;
/// Subscribes the connection to the events named in the payload.
pub const SUBSCRIBE: u32 = 2;
/// The node tree, which says which window has focus.
pub const GET_TREE: u32 = 4;

/// The bit every event's type has set.
pub const EVENT: u32 = 1 << 31;
/// A workspace was created, emptied, focused, moved, renamed or marked urgent.
pub const WORKSPACE_EVENT: u32 = EVENT;
/// The outputs changed.
pub const OUTPUT_EVENT: u32 = EVENT | 1;
/// A window was created, closed, focused, retitled or otherwise changed.
pub const WINDOW_EVENT: u32 = EVENT | 3;
/// sway is exiting.
pub const SHUTDOWN_EVENT: u32 = EVENT | 6;

/// The events lyr subscribes to, as `SUBSCRIBE`'s payload names them.
pub const SUBSCRIPTION: &str = r#"["workspace","window","output","shutdown"]"#;

/// One message: its header, then `payload`.
#[must_use]
pub fn frame(kind: u32, payload: &str) -> Vec<u8> {
    let length = u32::try_from(payload.len()).expect("a command lyr writes fits in 32 bits");
    let mut message = Vec::with_capacity(HEADER_LENGTH + payload.len());
    message.extend_from_slice(MAGIC);
    message.extend_from_slice(&length.to_ne_bytes());
    message.extend_from_slice(&kind.to_ne_bytes());
    message.extend_from_slice(payload.as_bytes());
    message
}

/// A reply's or an event's header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Header {
    /// How many payload bytes follow.
    pub length: u32,
    /// The message type, with [`EVENT`] set for an event.
    pub kind: u32,
}

impl Header {
    /// Reads a header, or `None` when it does not start with [`MAGIC`].
    #[must_use]
    pub fn parse(bytes: &[u8; HEADER_LENGTH]) -> Option<Self> {
        let (magic, rest) = bytes.split_at(MAGIC.len());
        if magic != MAGIC {
            return None;
        }
        let (length, kind) = rest.split_at(4);
        Some(Self {
            length: u32::from_ne_bytes(length.try_into().ok()?),
            kind: u32::from_ne_bytes(kind.try_into().ok()?),
        })
    }
}

/// Why lyr will not ask sway to switch to a workspace.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Unquotable {
    /// sway keeps a backslash inside quotes, ends the quote at `"`, and
    /// replaces a `$variable`, so the name would arrive changed.
    #[error(
        "the workspace name {0:?} has a `\"`, `\\` or `$`, which sway would not read as written"
    )]
    Character(String),
    /// sway reads the name as one of `workspace`'s own keywords.
    #[error("the workspace name {0:?} is a keyword of sway's `workspace` command")]
    Keyword(String),
}

/// The words sway's `workspace` command reads as keywords rather than
/// names, compared without case (`sway/commands/workspace.c`).
const KEYWORDS: [&str; 8] = [
    "next",
    "prev",
    "next_on_output",
    "prev_on_output",
    "current",
    "back_and_forth",
    "number",
    "--no-auto-back-and-forth",
];

/// The command that switches to `workspace`, by name.
///
/// `--no-auto-back-and-forth` keeps a click on the shown workspace from
/// bouncing back where `workspace_auto_back_and_forth` is on.
///
/// # Errors
///
/// When sway would read the name as something else ([`Unquotable`]).
pub fn focus_command(workspace: &Workspace) -> Result<String, Unquotable> {
    let name = &workspace.name;
    if name.contains(['"', '\\', '$']) {
        return Err(Unquotable::Character(name.clone()));
    }
    if KEYWORDS
        .iter()
        .any(|keyword| keyword.eq_ignore_ascii_case(name))
    {
        return Err(Unquotable::Keyword(name.clone()));
    }
    Ok(format!(r#"workspace --no-auto-back-and-forth "{name}""#))
}

/// The command that focuses the window sway gave `id`, switching to the
/// workspace it is on.
#[must_use]
pub fn focus_window_command(id: i64) -> String {
    format!("[con_id={id}] focus")
}

/// The command that ends the session: the power menu's Log out.
pub const EXIT_COMMAND: &str = "exit";

#[cfg(test)]
mod tests {
    use super::{
        EVENT, GET_WORKSPACES, HEADER_LENGTH, Header, Unquotable, WINDOW_EVENT, focus_command,
        frame,
    };
    use crate::Workspace;

    fn named(name: &str) -> Workspace {
        Workspace {
            id: 7,
            name: name.to_owned(),
            monitor: None,
            urgent: false,
        }
    }

    #[test]
    fn a_message_is_the_magic_length_type_and_payload() {
        let message = frame(0, "exit");
        // The hexdump `sway-ipc(7)` gives for `exit`, on a little-endian
        // machine; the fields are native-endian, so read them back instead
        // on a big-endian one.
        if cfg!(target_endian = "little") {
            assert_eq!(
                message,
                b"i3-ipc\x04\x00\x00\x00\x00\x00\x00\x00exit".as_slice()
            );
        }
        let header: [u8; HEADER_LENGTH] = message[..HEADER_LENGTH].try_into().expect("a header");
        assert_eq!(Header::parse(&header), Some(Header { length: 4, kind: 0 }));
    }

    #[test]
    fn a_header_round_trips_and_needs_the_magic() {
        for kind in [GET_WORKSPACES, WINDOW_EVENT] {
            let message = frame(kind, "[]");
            let header: [u8; HEADER_LENGTH] =
                message[..HEADER_LENGTH].try_into().expect("a header");
            assert_eq!(Header::parse(&header), Some(Header { length: 2, kind }));
        }
        let mut wrong = [0; HEADER_LENGTH];
        wrong[..6].copy_from_slice(b"i3-ipx");
        assert_eq!(Header::parse(&wrong), None);
        assert_eq!(WINDOW_EVENT & EVENT, EVENT);
    }

    #[test]
    fn a_name_is_quoted_and_auto_back_and_forth_is_off() {
        assert_eq!(
            focus_command(&named("1:web")).as_deref(),
            Ok(r#"workspace --no-auto-back-and-forth "1:web""#)
        );
        assert_eq!(
            focus_command(&named("mail; exit")).as_deref(),
            Ok(r#"workspace --no-auto-back-and-forth "mail; exit""#),
            "inside quotes sway does not split commands at `;`"
        );
    }

    #[test]
    fn a_name_sway_would_read_differently_is_refused() {
        for name in [r#"say "hi""#, r"back\slash", "$term"] {
            assert_eq!(
                focus_command(&named(name)),
                Err(Unquotable::Character(name.to_owned()))
            );
        }
        for name in ["next", "Back_And_Forth", "number"] {
            assert_eq!(
                focus_command(&named(name)),
                Err(Unquotable::Keyword(name.to_owned()))
            );
        }
        assert!(
            focus_command(&named("next door")).is_ok(),
            "only whole words"
        );
    }
}
