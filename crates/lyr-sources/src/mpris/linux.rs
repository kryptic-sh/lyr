//! MPRIS players over the session bus (the MediaPlayer2 specification 2.2):
//! every `org.mpris.MediaPlayer2.*` name, its `Player` interface at
//! `/org/mpris/MediaPlayer2`, re-read when a player comes or goes
//! (`NameOwnerChanged`) or changes (`PropertiesChanged`). Nothing is polled.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};

use futures_util::StreamExt;
use futures_util::stream;
use tokio::sync::watch;
use zbus::fdo::DBusProxy;
use zbus::message::Type;
use zbus::proxy::CacheProperties;
use zbus::{Connection, MatchRule, MessageStream, OwnedVariant};

use crcbl_image::theme::FileVersion;
use lyr_core::draw::Picture;
use lyr_core::media::{Artwork, Control, Player, Status};

use crate::bus::Follow;
use crate::desktop_entry;
use crate::icons::{self, Icons};

/// The bus names MPRIS players take.
pub const PREFIX: &str = "org.mpris.MediaPlayer2.";
/// Where every player serves its interfaces.
pub const PATH: &str = "/org/mpris/MediaPlayer2";

/// What stands for a player's art when it has none and its desktop entry
/// gives no icon: swaync 0.12.6's default in `update_album_art`.
const NO_ART_ICON: &str = "audio-x-generic-symbolic";

#[zbus::proxy(
    interface = "org.mpris.MediaPlayer2",
    default_path = "/org/mpris/MediaPlayer2"
)]
trait MediaPlayer2 {
    #[zbus(property)]
    fn desktop_entry(&self) -> zbus::Result<String>;
}

#[zbus::proxy(
    interface = "org.mpris.MediaPlayer2.Player",
    default_path = "/org/mpris/MediaPlayer2"
)]
trait MediaPlayer {
    fn previous(&self) -> zbus::Result<()>;
    fn play_pause(&self) -> zbus::Result<()>;
    fn next(&self) -> zbus::Result<()>;
    #[zbus(property)]
    fn metadata(&self) -> zbus::Result<HashMap<String, OwnedVariant>>;
    #[zbus(property)]
    fn playback_status(&self) -> zbus::Result<String>;
}

/// The player at `bus_name` on `connection`.
async fn proxy(connection: &Connection, bus_name: &str) -> zbus::Result<MediaPlayerProxy<'static>> {
    MediaPlayerProxy::builder(connection)
        .destination(bus_name.to_owned())
        .cache_properties(CacheProperties::No)
        .build()
        .await
}

/// The side an SVG's art is drawn at, in pixels; other formats keep their
/// own size.
const ART_SIZE: u32 = 256;

/// Players' art read so far, by `mpris:artUrl`, each file read again only
/// once it changed, and the icons that stand in for art a player lacks.
#[derive(Debug)]
pub struct Art {
    read: HashMap<String, (FileVersion, Option<Picture>)>,
    /// The URLs asked for since the last [`Self::sweep`].
    asked: HashSet<String>,
    icons: Icons,
    /// Where players' desktop entries are looked for.
    entry_dirs: Vec<PathBuf>,
}

impl Default for Art {
    fn default() -> Self {
        Self::new(Icons::default(), desktop_entry::dirs())
    }
}

impl Art {
    /// Art looked up in `icons`, desktop entries under `entry_dirs`.
    #[must_use]
    pub fn new(icons: Icons, entry_dirs: Vec<PathBuf>) -> Self {
        Self {
            read: HashMap::new(),
            asked: HashSet::new(),
            icons,
            entry_dirs,
        }
    }

    /// What a player shows for art, as swaync 0.12.6's `update_album_art`
    /// picks it: the art `url` names, else the icon of the player's desktop
    /// entry `entry`, else [`NO_ART_ICON`]. An icon named `…-symbolic` is
    /// drawn in the text's colour, as GTK draws it.
    fn shown(&mut self, url: &str, entry: &str) -> Option<Artwork> {
        let icon = |name: &str, picture: Picture| {
            if name.ends_with("-symbolic") {
                Artwork::Symbolic(picture)
            } else {
                Artwork::Picture(picture)
            }
        };
        self.picture(url)
            .map(Artwork::Picture)
            .or_else(|| {
                let name = desktop_entry::icon_in(&self.entry_dirs, [entry])?;
                Some(icon(&name, self.icons.uri_or_name(&name, ART_SIZE)?))
            })
            .or_else(|| {
                Some(icon(
                    NO_ART_ICON,
                    self.icons.named(NO_ART_ICON, "", ART_SIZE)?,
                ))
            })
    }

    /// The picture `url` names. Only a `file://` URI is read: lyr fetches
    /// nothing over the network (plan 09).
    fn picture(&mut self, url: &str) -> Option<Picture> {
        let path = icons::file_uri(url)?;
        let version = FileVersion::of(&path)?;
        self.asked.insert(url.to_owned());
        if let Some((read, picture)) = self.read.get(url)
            && *read == version
        {
            return picture.clone();
        }
        let picture = icons::read(&path, ART_SIZE);
        self.read.insert(url.to_owned(), (version, picture.clone()));
        picture
    }

    /// Forgets the art no player asked for since the last sweep, so a
    /// player moving through an album keeps only its current track's.
    fn sweep(&mut self) {
        let asked = std::mem::take(&mut self.asked);
        self.read.retain(|url, _| asked.contains(url));
    }
}

/// What the player at `bus_name` plays now, its art read through `art`
/// ([`Art::shown`]).
///
/// # Errors
///
/// When it does not answer.
pub async fn read(
    connection: &Connection,
    bus_name: &str,
    art: &Mutex<Art>,
) -> zbus::Result<Player> {
    let player = proxy(connection, bus_name).await?;
    let metadata = player.metadata().await?;
    let text = |key: &str| {
        metadata
            .get(key)
            .and_then(|value| String::try_from(value.clone()).ok())
            .unwrap_or_default()
    };
    let artist = metadata
        .get("xesam:artist")
        .and_then(|value| Vec::<String>::try_from(value.clone()).ok())
        .map(|artists| artists.join(", "))
        .unwrap_or_default();
    let status = Status::from_name(&player.playback_status().await?);
    // A player that does not say has none, as swaync reads it.
    let entry = MediaPlayer2Proxy::builder(connection)
        .destination(bus_name.to_owned())
        .cache_properties(CacheProperties::No)
        .build()
        .await?
        .desktop_entry()
        .await
        .unwrap_or_default();
    let art = art
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .shown(&text("mpris:artUrl"), &entry);
    Ok(Player {
        bus_name: bus_name.to_owned(),
        title: text("xesam:title"),
        artist,
        status,
        art,
    })
}

/// Presses `control` on the player at `bus_name`.
///
/// # Errors
///
/// When the player refuses.
pub async fn control(
    connection: &Connection,
    bus_name: &str,
    control: Control,
) -> zbus::Result<()> {
    let player = proxy(connection, bus_name).await?;
    match control {
        Control::Previous => player.previous().await,
        Control::PlayPause => player.play_pause().await,
        Control::Next => player.next().await,
    }
}

/// The players on the session bus.
#[derive(Clone, Debug)]
pub struct Players {
    connection: Connection,
    art: Arc<Mutex<Art>>,
}

impl Players {
    /// The players on the session bus.
    ///
    /// # Errors
    ///
    /// When the session bus cannot be reached.
    pub async fn session() -> zbus::Result<Self> {
        Ok(Self {
            connection: Connection::session().await?,
            art: Arc::default(),
        })
    }

    /// Every player now, in bus-name order. One that does not answer is left
    /// out.
    ///
    /// # Errors
    ///
    /// When the bus cannot list its names.
    pub async fn all(&self) -> zbus::Result<Vec<Player>> {
        let dbus = DBusProxy::new(&self.connection).await?;
        let mut names: Vec<String> = dbus
            .list_names()
            .await?
            .into_iter()
            .map(|name| name.to_string())
            .filter(|name| name.starts_with(PREFIX))
            .collect();
        names.sort();
        let mut players = Vec::new();
        for name in names {
            if let Ok(player) = read(&self.connection, &name, &self.art).await {
                players.push(player);
            }
        }
        self.art
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .sweep();
        Ok(players)
    }

    /// Presses `control` on the player at `bus_name`.
    ///
    /// # Errors
    ///
    /// When the player refuses.
    pub async fn control(&self, bus_name: &str, pressed: Control) -> zbus::Result<()> {
        control(&self.connection, bus_name, pressed).await
    }

    /// Sends every player to `players` now and after every change, until
    /// every receiver is dropped.
    ///
    /// # Errors
    ///
    /// When subscribing or listing fails.
    pub async fn watch(&self, players: &watch::Sender<Vec<Player>>) -> zbus::Result<()> {
        let dbus = DBusProxy::new(&self.connection).await?;
        // Any player's change, whichever player sent it.
        let changed = MatchRule::builder()
            .msg_type(Type::Signal)
            .interface("org.freedesktop.DBus.Properties")
            .member("PropertiesChanged")
            .path(PATH)
            .build()?;
        // Subscribed before the first read, so a change in between is not lost.
        let owners = dbus.receive_name_owner_changed().await?.map(|_| ());
        let properties = MessageStream::for_match_rule(changed, &self.connection, None)
            .await?
            .map(|_| ());
        let mut changes = Follow::new(stream::select(owners, properties));
        let wanted = |()| Ok::<_, zbus::Error>(true);
        players.send_replace(changes.during(self.all(), wanted).await??);
        while changes.changed(wanted).await? {
            if !crate::bus::publish(players, changes.during(self.all(), wanted).await??) {
                break;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
