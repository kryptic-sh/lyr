//! `org.freedesktop.Notifications` at `/org/freedesktop/Notifications`, as
//! the Desktop Notifications Specification 1.3 defines it.
//!
//! The bar does not serve it yet: until lyr draws toasts, taking the name
//! would swallow every notification on a desktop with no other daemon.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use lyr_core::notifications::{CloseReason, Expiry, Incoming, Store, Urgency};
use tokio::sync::mpsc::UnboundedSender;
use zbus::OwnedVariant;
use zbus::object_server::SignalEmitter;

/// The object path the spec puts the daemon at.
pub const PATH: &str = "/org/freedesktop/Notifications";

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
        let id = self.store().notify(incoming);
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

#[cfg(test)]
mod tests;
