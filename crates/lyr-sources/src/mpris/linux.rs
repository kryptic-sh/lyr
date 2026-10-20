//! MPRIS players over the session bus (the MediaPlayer2 specification 2.2):
//! every `org.mpris.MediaPlayer2.*` name, its `Player` interface at
//! `/org/mpris/MediaPlayer2`, re-read when a player comes or goes
//! (`NameOwnerChanged`) or changes (`PropertiesChanged`). Nothing is polled.

use std::collections::HashMap;

use futures_util::StreamExt;
use futures_util::stream;
use tokio::sync::watch;
use zbus::fdo::DBusProxy;
use zbus::message::Type;
use zbus::proxy::CacheProperties;
use zbus::{Connection, MatchRule, MessageStream, OwnedVariant};

use lyr_core::media::{Control, Player, Status};

use crate::bus::Follow;

/// The bus names MPRIS players take.
pub const PREFIX: &str = "org.mpris.MediaPlayer2.";
/// Where every player serves its interfaces.
pub const PATH: &str = "/org/mpris/MediaPlayer2";

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

/// What the player at `bus_name` plays now.
///
/// # Errors
///
/// When it does not answer.
pub async fn read(connection: &Connection, bus_name: &str) -> zbus::Result<Player> {
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
    Ok(Player {
        bus_name: bus_name.to_owned(),
        title: text("xesam:title"),
        artist,
        status: Status::from_name(&player.playback_status().await?),
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
            if let Ok(player) = read(&self.connection, &name).await {
                players.push(player);
            }
        }
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
        let wanted = |()| Ok(true);
        players.send_replace(changes.during(self.all(), wanted).await??);
        while changes.changed(wanted).await? {
            if players
                .send(changes.during(self.all(), wanted).await??)
                .is_err()
            {
                break;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
