//! The system time zone on Linux: systemd's `org.freedesktop.timedate1` on
//! the system bus, its `Timezone` property, followed on
//! `PropertiesChanged`. timedated is started on demand and exits when idle;
//! a change made through it starts it, so the signal still comes.

use tokio::sync::watch;

use crate::bus::Follow;
use zbus::Connection;
use zbus::fdo::{PropertiesChanged, PropertiesProxy};
use zbus::names::InterfaceName;
use zbus::proxy::CacheProperties;

const INTERFACE: &str = "org.freedesktop.timedate1";

#[zbus::proxy(
    interface = "org.freedesktop.timedate1",
    default_service = "org.freedesktop.timedate1",
    default_path = "/org/freedesktop/timedate1"
)]
trait Daemon {
    #[zbus(property)]
    fn timezone(&self) -> zbus::Result<String>;
}

/// timedate1 on one connection.
#[derive(Clone, Debug)]
pub struct TimeDate {
    daemon: DaemonProxy<'static>,
    properties: PropertiesProxy<'static>,
}

impl TimeDate {
    /// timedate1 on the system bus.
    ///
    /// # Errors
    ///
    /// When the system bus cannot be reached.
    pub async fn system() -> zbus::Result<Self> {
        Self::on(&Connection::system().await?).await
    }

    /// timedate1 on `connection`.
    ///
    /// # Errors
    ///
    /// When the proxies cannot be built.
    pub async fn on(connection: &Connection) -> zbus::Result<Self> {
        let daemon = DaemonProxy::builder(connection)
            .cache_properties(CacheProperties::No)
            .build()
            .await?;
        let properties = PropertiesProxy::builder(connection)
            .destination(INTERFACE)
            .path("/org/freedesktop/timedate1")
            .build()
            .await?;
        Ok(Self { daemon, properties })
    }

    /// The zone's name, such as `Asia/Manila`, or `None` when none is set.
    ///
    /// # Errors
    ///
    /// When timedated does not answer.
    pub async fn zone(&self) -> zbus::Result<Option<String>> {
        let zone = self.daemon.timezone().await?;
        Ok((!zone.is_empty()).then_some(zone))
    }

    /// Sends the zone to `zones` now and after every change, until every
    /// receiver is dropped.
    ///
    /// # Errors
    ///
    /// When subscribing or reading fails.
    pub async fn watch(&self, zones: &watch::Sender<Option<String>>) -> zbus::Result<()> {
        let interface = InterfaceName::try_from(INTERFACE)?;
        // Subscribe before the first read, so a change in between is not lost.
        let mut changes = Follow::new(self.properties.receive_properties_changed().await?);
        let wanted = |change: PropertiesChanged| -> zbus::Result<bool> {
            Ok(change.args()?.interface_name() == &interface)
        };
        zones.send_replace(changes.during(self.zone(), wanted).await??);
        while changes.changed(wanted).await? {
            if !crate::bus::publish(zones, changes.during(self.zone(), wanted).await??) {
                break;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
