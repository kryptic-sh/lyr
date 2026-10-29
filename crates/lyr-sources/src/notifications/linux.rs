//! `org.freedesktop.Notifications` at `/org/freedesktop/Notifications`, as
//! the Desktop Notifications Specification 1.3 defines it.
//!
//! [`serve`] takes the name only when no other daemon has it, unless told to
//! replace it (`docs/plan/09-notifications.md#when-another-daemon-owns-the-name`):
//! switching from swaync to lyr is then a dotfiles change, not a race at
//! start-up.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Instant;

use crcbl_image::raw::{self, ImageData};
use lyr_core::draw::Picture;
use lyr_core::notifications::{CloseReason, Expiry, Incoming, Store, Urgency};
use tokio::sync::mpsc::UnboundedSender;
use zbus::fdo::{DBusProxy, RequestNameFlags};
use zbus::names::BusName;
use zbus::object_server::SignalEmitter;
use zbus::{Connection, OwnedVariant};

use crate::desktop_entry;
use crate::icons::{self, Icons};

/// The object path the spec puts the daemon at.
pub const PATH: &str = "/org/freedesktop/Notifications";
/// The bus name the spec gives the daemon.
pub const NAME: &str = "org.freedesktop.Notifications";
/// What `GetCapabilities` answers.
pub const CAPABILITIES: [&str; 9] = [
    "actions",
    "body",
    "body-hyperlinks",
    "body-markup",
    "icon-static",
    "persistence",
    "synchronous",
    "private-synchronous",
    "x-canonical-private-synchronous",
];

/// The size an icon from the theme is found and drawn at: more than a
/// toast's [`lyr_core::toast::PICTURE_SIZE`] takes on a 3x output. Each
/// window resizes a picture to the pixels it covers, as the tray's do.
const PICTURE_SIZE: u32 = 64;

/// What changed in the store, for the bar to redraw.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    Notified(u32),
    Closed(u32, CloseReason),
}

/// The daemon: the store it shares with the bar, where it reports
/// changes, and the icons it reads for the notifications.
#[derive(Debug)]
pub struct Daemon {
    pub store: Arc<Mutex<Store>>,
    pub changes: UnboundedSender<Change>,
    icons: Mutex<Icons>,
}

impl Daemon {
    /// The daemon over `store`, reporting to `changes`, finding icons in
    /// the user's icon theme.
    #[must_use]
    pub fn new(store: Arc<Mutex<Store>>, changes: UnboundedSender<Change>) -> Self {
        Self {
            store,
            changes,
            icons: Mutex::new(Icons::default()),
        }
    }

    fn store(&self) -> std::sync::MutexGuard<'_, Store> {
        // A panic while the store was held leaves it as it was; carrying on
        // with it is better than a daemon that never answers again.
        self.store.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn report(&self, change: Change) {
        // The bar has gone only when lyr is exiting.
        let _gone = self.changes.send(change);
    }

    /// The notification's image: `image-data`, else `image-path`, else
    /// `icon_data`, each under its deprecated name too, as the spec orders
    /// them. One that cannot be read is left out.
    fn image(&self, hints: &HashMap<String, OwnedVariant>) -> Option<Picture> {
        let hint = |names: &[&str]| names.iter().find_map(|name| hints.get(*name));
        hint(&["image-data", "image_data"])
            .and_then(image_data)
            .or_else(|| {
                let path = String::try_from(hint(&["image-path", "image_path"])?.clone()).ok()?;
                self.icons().uri_or_name(&path, PICTURE_SIZE)
            })
            .or_else(|| hint(&["icon_data"]).and_then(image_data))
    }

    /// The app's icon: `app_icon`, else the icon of its desktop entry
    /// ([`entry_ids`]).
    fn icon(&self, app_icon: &str, desktop_entry: Option<&str>, app_name: &str) -> Option<Picture> {
        if let Some(icon) = self.icons().uri_or_name(app_icon, PICTURE_SIZE) {
            return Some(icon);
        }
        let ids = entry_ids(desktop_entry, app_name);
        let icon = desktop_entry::icon(ids.iter().map(String::as_str))?;
        self.icons().uri_or_name(&icon, PICTURE_SIZE)
    }

    fn icons(&self) -> std::sync::MutexGuard<'_, Icons> {
        self.icons.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// The desktop entries a notification's app may have, in the order swaync
/// 0.12.6 tries them (`NotifyParams`): the `desktop-entry` hint, then the
/// app's name, each as sent and lowercased, without a `.desktop` ending.
fn entry_ids(desktop_entry: Option<&str>, app_name: &str) -> Vec<String> {
    desktop_entry
        .into_iter()
        .chain([app_name])
        .map(|id| id.strip_suffix(".desktop").unwrap_or(id))
        .flat_map(|id| [id.to_owned(), id.to_lowercase()])
        .collect()
}

/// An `image-data` hint, `(iiibiiay)`, as a picture.
fn image_data(value: &OwnedVariant) -> Option<Picture> {
    let (width, height, rowstride, has_alpha, bits_per_sample, channels, data): (
        i32,
        i32,
        i32,
        bool,
        i32,
        i32,
        Vec<u8>,
    ) = value.clone().try_into().ok()?;
    let image = raw::from_image_data(
        &ImageData {
            width,
            height,
            rowstride,
            has_alpha,
            bits_per_sample,
            channels,
            data: &data,
        },
        &icons::LIMITS,
    )
    .ok()?;
    icons::picture(image)
}

#[zbus::interface(name = "org.freedesktop.Notifications")]
impl Daemon {
    #[allow(clippy::too_many_arguments)] // The spec's signature.
    fn notify(
        &self,
        app_name: String,
        replaces_id: u32,
        app_icon: String,
        summary: String,
        body: String,
        actions: Vec<String>,
        hints: HashMap<String, OwnedVariant>,
        expire_timeout: i32,
    ) -> u32 {
        let text = |key: &str| {
            hints
                .get(key)
                .and_then(|value| String::try_from(value.clone()).ok())
        };
        let flag = |key: &str| {
            hints
                .get(key)
                .and_then(|value| bool::try_from(value).ok())
                .unwrap_or(false)
        };
        let desktop_entry = text("desktop-entry");
        let icon = self.icon(&app_icon, desktop_entry.as_deref(), &app_name);
        let image = self.image(&hints);
        let incoming = Incoming {
            app_name,
            replaces_id,
            app_icon,
            icon,
            image,
            summary,
            body,
            // A flat list of id and label pairs; an odd one out has no label.
            actions: actions
                .as_chunks::<2>()
                .0
                .iter()
                .map(|[id, label]| (id.clone(), label.clone()))
                .collect(),
            urgency: hints
                .get("urgency")
                .and_then(|value| u8::try_from(value).ok())
                .map_or(Urgency::Normal, Urgency::from_byte),
            category: text("category"),
            desktop_entry,
            transient: flag("transient"),
            resident: flag("resident"),
            // swaync clamps it, and shows the bar for any `int32`.
            // Each name swaync reads it under.
            synchronous: text("synchronous")
                .or_else(|| text("private-synchronous"))
                .or_else(|| text("x-canonical-private-synchronous")),
            progress: hints
                .get("value")
                .and_then(|value| i32::try_from(value).ok())
                .and_then(|value| u8::try_from(value.clamp(0, 100)).ok()),
            expiry: Some(Expiry::from_millis(expire_timeout)),
        };
        let id = self.store().notify(incoming, Instant::now());
        self.report(Change::Notified(id));
        id
    }

    async fn close_notification(
        &self,
        id: u32,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> zbus::fdo::Result<()> {
        if self.store().close(id).is_none() {
            // The spec's "empty D-Bus error" for an id that no longer exists.
            return Err(zbus::fdo::Error::Failed(String::new()));
        }
        self.report(Change::Closed(id, CloseReason::Closed));
        Self::notification_closed(&emitter, id, CloseReason::Closed as u32).await?;
        Ok(())
    }

    /// Each capability is advertised only once it works: the body is drawn
    /// with its markup, actions get buttons and are invoked, the image and
    /// the app's icon are drawn, a link opens (`http` and `https` only),
    /// and the history keeps a notification until it is dismissed or
    /// cleared.
    fn get_capabilities(&self) -> Vec<String> {
        CAPABILITIES.iter().map(|&name| name.to_owned()).collect()
    }

    fn get_server_information(&self) -> (String, String, String, String) {
        (
            "lyr".to_owned(),
            "kryptic-sh".to_owned(),
            env!("CARGO_PKG_VERSION").to_owned(),
            "1.3".to_owned(),
        )
    }

    #[zbus(signal)]
    pub async fn notification_closed(
        emitter: &SignalEmitter<'_>,
        id: u32,
        reason: u32,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    pub async fn action_invoked(
        emitter: &SignalEmitter<'_>,
        id: u32,
        action_key: String,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    pub async fn activation_token(
        emitter: &SignalEmitter<'_>,
        id: u32,
        activation_token: String,
    ) -> zbus::Result<()>;
}

/// Why lyr is not serving notifications.
#[derive(Debug, thiserror::Error)]
pub enum ServeError {
    #[error("{owner} (pid {}) owns {NAME}, so lyr leaves notifications to it", .pid.map_or_else(|| "unknown".to_owned(), |pid| pid.to_string()))]
    Owned { owner: String, pid: Option<u32> },
    #[error(transparent)]
    Bus(#[from] zbus::Error),
}

/// Serves the daemon on the session bus `connection` and takes [`NAME`]:
/// only when no other daemon has it, or from whoever has it with
/// `replace`. lyr lets another daemon replace it in turn.
///
/// # Errors
///
/// [`ServeError::Owned`] when another daemon keeps the name; the object is
/// taken down again.
pub async fn serve(
    connection: &Connection,
    daemon: Daemon,
    replace: bool,
) -> Result<Served, ServeError> {
    let served = Served::at(connection, daemon).await?;
    let mut flags = RequestNameFlags::DO_NOT_QUEUE | RequestNameFlags::ALLOW_REPLACEMENT;
    if replace {
        flags |= RequestNameFlags::REPLACE_EXISTING;
    }
    if crate::bus::claim(connection, NAME, flags).await? {
        return Ok(served);
    }
    connection.object_server().remove::<Daemon, _>(PATH).await?;
    let dbus = DBusProxy::new(connection).await?;
    let name = BusName::try_from(NAME)?;
    let owner = dbus.get_name_owner(name).await.map_err(zbus::Error::from)?;
    let pid = dbus
        .get_connection_unix_process_id(owner.clone().into())
        .await
        .ok();
    Err(ServeError::Owned {
        owner: owner.to_string(),
        pid,
    })
}

/// [`serve`] on the session bus.
///
/// # Errors
///
/// As [`serve`], and when the session bus cannot be reached.
pub async fn serve_session(daemon: Daemon, replace: bool) -> Result<Served, ServeError> {
    serve(&Connection::session().await?, daemon, replace).await
}

/// The daemon as served: what lyr's side does to a notification, with the
/// signals the spec asks for.
#[derive(Clone, Debug)]
pub struct Served {
    connection: Connection,
    store: Arc<Mutex<Store>>,
    changes: UnboundedSender<Change>,
}

impl Served {
    /// Serves `daemon` at [`PATH`] on `connection`, without taking a name: a
    /// peer-to-peer connection has none to take.
    ///
    /// # Errors
    ///
    /// When the object cannot be served.
    pub async fn at(connection: &Connection, daemon: Daemon) -> zbus::Result<Self> {
        let store = daemon.store.clone();
        let changes = daemon.changes.clone();
        connection.object_server().at(PATH, daemon).await?;
        Ok(Self {
            connection: connection.clone(),
            store,
            changes,
        })
    }

    fn emitter(&self) -> zbus::Result<SignalEmitter<'_>> {
        SignalEmitter::new(&self.connection, PATH)
    }

    /// Closes `id` for `reason`, signals it, and reports the change. Returns
    /// whether lyr held it.
    ///
    /// # Errors
    ///
    /// When the signal cannot be sent.
    pub async fn close(&self, id: u32, reason: CloseReason) -> zbus::Result<bool> {
        let held = self
            .store
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .close(id)
            .is_some();
        if held {
            // The bar has gone only when lyr is exiting.
            let _gone = self.changes.send(Change::Closed(id, reason));
            Daemon::notification_closed(&self.emitter()?, id, reason as u32).await?;
        }
        Ok(held)
    }

    /// Closes every notification, as Clear All does, signalling each as
    /// dismissed. Returns their ids.
    ///
    /// # Errors
    ///
    /// When a signal cannot be sent.
    pub async fn clear(&self) -> zbus::Result<Vec<u32>> {
        let ids = self
            .store
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clear();
        for &id in &ids {
            let _gone = self
                .changes
                .send(Change::Closed(id, CloseReason::Dismissed));
            Daemon::notification_closed(&self.emitter()?, id, CloseReason::Dismissed as u32)
                .await?;
        }
        Ok(ids)
    }

    /// Invokes the action `key` of `id`, as a click does, then closes it
    /// unless it is `resident`, as the spec's hint asks. `token`, the
    /// xdg-activation token made from the click, is sent just before, so
    /// the application can raise its window.
    ///
    /// # Errors
    ///
    /// When a signal cannot be sent.
    pub async fn invoke(&self, id: u32, key: &str, token: Option<&str>) -> zbus::Result<()> {
        let resident = self
            .store
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(id)
            .map(|held| held.content.resident);
        let Some(resident) = resident else {
            return Ok(());
        };
        let emitter = self.emitter()?;
        if let Some(token) = token {
            Daemon::activation_token(&emitter, id, token.to_owned()).await?;
        }
        Daemon::action_invoked(&emitter, id, key.to_owned()).await?;
        if !resident {
            self.close(id, CloseReason::Dismissed).await?;
        }
        Ok(())
    }
}

/// How an application sees the daemon, as lyr does to send its own
/// notices ([`notify`]).
#[zbus::proxy(
    interface = "org.freedesktop.Notifications",
    default_service = "org.freedesktop.Notifications",
    default_path = "/org/freedesktop/Notifications"
)]
trait Notifications {
    #[allow(clippy::too_many_arguments)]
    fn notify(
        &self,
        app_name: &str,
        replaces_id: u32,
        app_icon: &str,
        summary: &str,
        body: &str,
        actions: &[&str],
        hints: HashMap<&str, OwnedVariant>,
        expire_timeout: i32,
    ) -> zbus::Result<u32>;
    fn close_notification(&self, id: u32) -> zbus::Result<()>;
    fn get_capabilities(&self) -> zbus::Result<Vec<String>>;
    fn get_server_information(&self) -> zbus::Result<(String, String, String, String)>;
    #[zbus(signal)]
    fn notification_closed(&self, id: u32, reason: u32) -> zbus::Result<()>;
    #[zbus(signal)]
    fn action_invoked(&self, id: u32, action_key: String) -> zbus::Result<()>;
    #[zbus(signal)]
    fn activation_token(&self, id: u32, activation_token: String) -> zbus::Result<()>;
}

/// Sends a notice from lyr itself to whichever daemon holds the name on
/// `connection`, lyr's own included: `summary`, and `body` in the spec's
/// markup, so text in it from elsewhere is to be escaped
/// ([`lyr_core::markup::escape`]).
///
/// # Errors
///
/// When no daemon answers.
pub async fn notify(connection: &Connection, summary: &str, body: &str) -> zbus::Result<u32> {
    NotificationsProxy::new(connection)
        .await?
        .notify("lyr", 0, "", summary, body, &[], HashMap::new(), -1)
        .await
}

/// [`notify`] on the session bus, through a connection of its own: lyr's
/// notices are rare.
///
/// # Errors
///
/// When the session bus cannot be reached or no daemon answers.
pub async fn notify_session(summary: &str, body: &str) -> zbus::Result<u32> {
    notify(&Connection::session().await?, summary, body).await
}

#[cfg(test)]
mod tests;
