//! What `lyr <command>` asks a running bar to do over its control socket
//! (`docs/plan/02-architecture.md#control-socket`), and how a request is
//! written on the wire.
//!
//! A request is one line of text, such as `panel wifi`. Text rather than a
//! binary format, so the socket can be driven by hand while debugging. The
//! socket itself is platform code and lives in the binary.

use std::fmt;
use std::str::FromStr;

/// One request to the running bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Request {
    /// Reload the config file.
    Reload,
    /// Hide or show the bars.
    Toggle,
    /// Stop the bar.
    Quit,
    /// Open a panel on the focused output, or close it if it is open.
    Panel(Panel),
    /// Turn do-not-disturb on or off.
    Dnd,
    /// Clear the notification history.
    Clear,
}

/// A panel `lyr panel` can open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Panel {
    Clock,
    Wifi,
    Bluetooth,
    Audio,
}

impl Panel {
    /// Every panel, in the order help text lists them.
    pub const ALL: [Self; 4] = [Self::Clock, Self::Wifi, Self::Bluetooth, Self::Audio];

    /// The panel's name in a request and on the command line.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Clock => "clock",
            Self::Wifi => "wifi",
            Self::Bluetooth => "bluetooth",
            Self::Audio => "audio",
        }
    }
}

impl fmt::Display for Request {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Reload => f.write_str("reload"),
            Self::Toggle => f.write_str("toggle"),
            Self::Quit => f.write_str("quit"),
            Self::Panel(panel) => write!(f, "panel {}", panel.name()),
            Self::Dnd => f.write_str("dnd"),
            Self::Clear => f.write_str("clear"),
        }
    }
}

/// A line that is not a request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnknownRequest(pub String);

impl fmt::Display for UnknownRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown request `{}`", self.0)
    }
}

impl std::error::Error for UnknownRequest {}

impl FromStr for Request {
    type Err = UnknownRequest;

    /// Parses one request line, without its line ending.
    fn from_str(line: &str) -> Result<Self, Self::Err> {
        let unknown = || UnknownRequest(line.to_owned());
        let mut words = line.split(' ');
        let request = match words.next() {
            Some("reload") => Self::Reload,
            Some("toggle") => Self::Toggle,
            Some("quit") => Self::Quit,
            Some("dnd") => Self::Dnd,
            Some("clear") => Self::Clear,
            Some("panel") => {
                let name = words.next().ok_or_else(unknown)?;
                let panel = Panel::ALL
                    .into_iter()
                    .find(|panel| panel.name() == name)
                    .ok_or_else(unknown)?;
                Self::Panel(panel)
            }
            _ => return Err(unknown()),
        };
        match words.next() {
            Some(_) => Err(unknown()),
            None => Ok(request),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Panel, Request};

    #[test]
    fn every_request_survives_the_wire() {
        let mut all = vec![
            Request::Reload,
            Request::Toggle,
            Request::Quit,
            Request::Dnd,
            Request::Clear,
        ];
        all.extend(Panel::ALL.map(Request::Panel));
        for request in all {
            assert_eq!(request.to_string().parse(), Ok(request), "{request}");
        }
    }

    #[test]
    fn a_panel_request_names_the_panel() {
        assert_eq!(Request::Panel(Panel::Wifi).to_string(), "panel wifi");
        assert_eq!("panel audio".parse(), Ok(Request::Panel(Panel::Audio)));
    }

    #[test]
    fn anything_else_is_refused() {
        for line in [
            "",
            "panel",
            "panel tray",
            "reload now",
            "panel wifi x",
            "Reload",
            " quit",
        ] {
            assert!(line.parse::<Request>().is_err(), "{line:?} parsed");
        }
    }
}
