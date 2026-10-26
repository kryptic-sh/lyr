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
    /// `xesam:title`, else the name of its desktop entry, else "Media
    /// Player", as swaync 0.12.6's `update_title` gives it.
    pub title: String,
    /// The line under the title ([`subtitle`]); empty for none.
    pub subtitle: String,
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

/// The line under a player's title, from its first artist (`xesam:artist`,
/// else `xesam:albumArtist`) and `xesam:album`: "artist - album" as swaync
/// 0.12.6's `update_sub_title` joins them, or the album alone. Where swaync
/// shows nothing for an artist with no album, the artist alone is shown,
/// since many players send no album.
#[must_use]
pub fn subtitle(artist: &str, album: &str) -> String {
    match (artist.is_empty(), album.is_empty()) {
        (false, false) => format!("{artist} - {album}"),
        (true, _) => album.to_owned(),
        (false, true) => artist.to_owned(),
    }
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
    use super::{Status, subtitle};

    #[test]
    fn the_subtitle_joins_the_artist_and_the_album() {
        assert_eq!(
            subtitle("Miles Davis", "Kind of Blue"),
            "Miles Davis - Kind of Blue"
        );
        assert_eq!(subtitle("", "Kind of Blue"), "Kind of Blue");
        assert_eq!(subtitle("Miles Davis", ""), "Miles Davis");
        assert_eq!(subtitle("", ""), "");
    }

    #[test]
    fn a_status_is_read_by_its_mpris_name() {
        assert_eq!(Status::from_name("Playing"), Status::Playing);
        assert_eq!(Status::from_name("Paused"), Status::Paused);
        assert_eq!(Status::from_name("Stopped"), Status::Stopped);
        assert_eq!(Status::from_name("Buffering"), Status::Stopped);
    }
}
