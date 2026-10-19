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

use lyr_core::notifications::{CloseReason, Expiry, Incoming, Store, Urgency};
use tokio::sync::mpsc::UnboundedSender;
use zbus::fdo::{DBusProxy, RequestNameFlags, RequestNameReply};
use zbus::names::BusName;
use zbus::object_server::SignalEmitter;
use zbus::{Connection, OwnedVariant};

/// The object path the spec puts the daemon at.
pub const PATH: &str = "/org/freedesktop/Notifications";
/// The bus name the spec gives the daemon.
pub const NAME: &str = "org.freedesktop.Notifications";

/// What changed in the store, for the bar to redraw.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    Notified(u32),
    Closed(u32, CloseReason),
}

/// The daemon: the store it shares with the bar, and where it reports
/// changes.
#[derive(Debug)]
pub struct Daemon {
    pub store: Arc<Mutex<Store>>,
    pub changes: UnboundedSender<Change>,
}

impl Daemon {
    fn store(&self) -> std::sync::MutexGuard<'_, Store> {
        // A panic while the store was held leaves it as it was; carrying on
        // with it is better than a daemon that never answers again.
        self.store.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn report(&self, change: Change) {
        // The bar has gone only when lyr is exiting.
        let _gone = self.changes.send(change);
    }
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
        let incoming = Incoming {
            app_name,
            replaces_id,
            app_icon,
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
            desktop_entry: text("desktop-entry"),
            transient: flag("transient"),
            resident: flag("resident"),
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

    /// Each capability is advertised only once it works; lyr draws nothing
    /// yet, so it advertises none.
    fn get_capabilities(&self) -> Vec<String> {
        Vec::new()
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
    match connection.request_name_with_flags(NAME, flags).await? {
        RequestNameReply::PrimaryOwner | RequestNameReply::AlreadyOwner => Ok(served),
        RequestNameReply::InQueue | RequestNameReply::Exists => {
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
    }
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
    /// unless it is `resident`, as the spec's hint asks.
    ///
    /// # Errors
    ///
    /// When a signal cannot be sent.
    pub async fn invoke(&self, id: u32, key: &str) -> zbus::Result<()> {
        let resident = self
            .store
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(id)
            .map(|held| held.content.resident);
        let Some(resident) = resident else {
            return Ok(());
        };
        Daemon::action_invoked(&self.emitter()?, id, key.to_owned()).await?;
        if !resident {
            self.close(id, CloseReason::Dismissed).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
