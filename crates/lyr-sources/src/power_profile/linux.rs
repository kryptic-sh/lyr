//! The power-profile source on Linux: power-profiles-daemon over the system
//! bus.
//!
//! The daemon is `org.freedesktop.UPower.PowerProfiles` at
//! `/org/freedesktop/UPower/PowerProfiles`, and before that
//! `net.hadess.PowerProfiles` at `/net/hadess/PowerProfiles`, the name waybar
//! still uses; the interface is the same under both
//! (`src/org.freedesktop.UPower.PowerProfiles.xml.in` in the daemon's
//! repository, read 2026-10-19). lyr tries the new name first.

use std::collections::HashMap;

use tokio::sync::watch;

use crate::bus::Follow;
use zbus::Connection;
use zbus::OwnedVariant;
use zbus::fdo::{PropertiesChanged, PropertiesProxy};
use zbus::names::InterfaceName;
use zbus::proxy::CacheProperties;

use super::{Entry, Profiles};

/// Where the daemon is: its bus name, which is also its interface, and its
/// object path, newest first.
const NAMES: [(&str, &str); 2] = [
    (
        "org.freedesktop.UPower.PowerProfiles",
        "/org/freedesktop/UPower/PowerProfiles",
    ),
    ("net.hadess.PowerProfiles", "/net/hadess/PowerProfiles"),
];

/// The daemon's properties lyr uses. The interface name and path are set per
/// connection, from [`NAMES`].
#[zbus::proxy(
    interface = "org.freedesktop.UPower.PowerProfiles",
    default_service = "org.freedesktop.UPower.PowerProfiles",
    default_path = "/org/freedesktop/UPower/PowerProfiles"
)]
trait Daemon {
    #[zbus(property)]
    fn active_profile(&self) -> zbus::Result<String>;
    #[zbus(property)]
    fn set_active_profile(&self, profile: &str) -> zbus::Result<()>;
    #[zbus(property)]
    fn profiles(&self) -> zbus::Result<Vec<HashMap<String, OwnedVariant>>>;
}

/// power-profiles-daemon on one connection.
#[derive(Clone, Debug)]
pub struct PowerProfiles {
    daemon: DaemonProxy<'static>,
    properties: PropertiesProxy<'static>,
    interface: &'static str,
}

impl PowerProfiles {
    /// The daemon on the system bus.
    ///
    /// # Errors
    ///
    /// When the system bus cannot be reached, or neither name answers.
    pub async fn system() -> zbus::Result<Self> {
        Self::on(&Connection::system().await?).await
    }

    /// The daemon on `connection`, under whichever of its names answers.
    ///
    /// # Errors
    ///
    /// When neither name answers.
    pub async fn on(connection: &Connection) -> zbus::Result<Self> {
        let mut last = None;
        for (name, path) in NAMES {
            let daemon = DaemonProxy::builder(connection)
                .destination(name)
                .path(path)
                .interface(name)
                .cache_properties(CacheProperties::No)
                .build()
                .await?;
            match daemon.active_profile().await {
                Ok(_) => {
                    let properties = PropertiesProxy::builder(connection)
                        .destination(name)
                        .path(path)
                        .build()
                        .await?;
                    return Ok(Self {
                        daemon,
                        properties,
                        interface: name,
                    });
                }
                Err(error) => last = Some(error),
            }
        }
        Err(last.unwrap_or_else(|| zbus::Error::Failure("no daemon name to try".to_owned())))
    }

    /// The daemon's state now, or `None` when its active profile is not
    /// among those it lists.
    ///
    /// # Errors
    ///
    /// When the daemon does not answer, or answers with the wrong types.
    pub async fn profiles(&self) -> zbus::Result<Option<Profiles>> {
        let active = self.daemon.active_profile().await?;
        let entries: Vec<Entry> = self
            .daemon
            .profiles()
            .await?
            .iter()
            .filter_map(|entry| {
                let text = |key: &str| {
                    entry
                        .get(key)
                        .and_then(|value| String::try_from(value.clone()).ok())
                };
                Some(Entry {
                    profile: text("Profile")?,
                    driver: text("Driver").unwrap_or_default(),
                })
            })
            .collect();
        Ok(Profiles::new(&active, &entries))
    }

    /// Switches to `profile`, as a click does.
    ///
    /// # Errors
    ///
    /// When the daemon refuses.
    pub async fn set(&self, profile: &str) -> zbus::Result<()> {
        self.daemon.set_active_profile(profile).await
    }

    /// Sends the daemon's state to `states` now and after every change,
    /// until the daemon goes away or every receiver is dropped.
    ///
    /// # Errors
    ///
    /// When subscribing or reading fails.
    pub async fn watch(&self, states: &watch::Sender<Option<Profiles>>) -> zbus::Result<()> {
        let interface = InterfaceName::try_from(self.interface)?;
        // Subscribe before the first read, so a change in between is not lost.
        let mut changes = Follow::new(self.properties.receive_properties_changed().await?);
        let wanted = |change: PropertiesChanged| Ok(change.args()?.interface_name() == &interface);
        states.send_replace(changes.during(self.profiles(), wanted).await??);
        while changes.changed(wanted).await? {
            if states
                .send(changes.during(self.profiles(), wanted).await??)
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
