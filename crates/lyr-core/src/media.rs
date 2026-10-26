//! A media player, as the clock panel's media widget shows it
//! (`docs/plan/09-notifications.md#media-players-the-mpris-widget`): what it
//! plays and its controls. The platforms' sources fill it in
//! (`lyr_sources::mpris`).

use crate::draw::Picture;

/// Whether a player is playing: MPRIS's `PlaybackStatus`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Status {
    Playing,
    Paused,
    #[default]
    Stopped,
}

impl Status {
    /// From `PlaybackStatus`; anything the spec does not name is stopped.
    #[must_use]
    pub fn from_name(name: &str) -> Self {
        match name {
            "Playing" => Self::Playing,
            "Paused" => Self::Paused,
            _ => Self::Stopped,
        }
    }
}

/// One player and what it plays.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Player {
    /// Its bus name, `org.mpris.MediaPlayer2.<name>`, by which it is
    /// controlled.
    pub bus_name: String,
    /// `xesam:title`, empty when it gives none.
    pub title: String,
    /// `xesam:artist`, joined with ", ".
    pub artist: String,
    pub status: Status,
    /// What it shows for art: the art `mpris:artUrl` names, else an icon
    /// standing in for it.
    pub art: Option<Artwork>,
}

/// A player's art, or the icon shown in its place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Artwork {
    /// Drawn as it is.
    Picture(Picture),
    /// A symbolic icon, drawn in the text's colour as GTK draws one.
    Symbolic(Picture),
}

/// A control button.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Control {
    Previous,
    PlayPause,
    Next,
}

#[cfg(test)]
mod tests {
    use super::Status;

    #[test]
    fn a_status_is_read_by_its_mpris_name() {
        assert_eq!(Status::from_name("Playing"), Status::Playing);
        assert_eq!(Status::from_name("Paused"), Status::Paused);
        assert_eq!(Status::from_name("Stopped"), Status::Stopped);
        assert_eq!(Status::from_name("Buffering"), Status::Stopped);
    }
}
