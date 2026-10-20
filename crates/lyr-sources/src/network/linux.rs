//! The network source on Linux: NetworkManager over the system bus.
//!
//! The interface shown is the one waybar shows, the one with the default
//! route, which is the device of NetworkManager's `PrimaryConnection`, or of
//! its `ActivatingConnection` while nothing is up yet. Names are from
//! NetworkManager's introspection XML (`introspection/` on its main branch,
//! read 2026-10-20).
//!
//! The Wi-Fi menu's reading and actions are in [`wifi`].
//!
//! lyr reads the objects that make up the reading once, and again whenever
//! one of them sends `PropertiesChanged`. Signals from other objects, such
//! as the access points a scan updates, are ignored.

use std::collections::HashMap;

use lyr_core::network::{Link, Reading};
use tokio::sync::watch;

use crate::bus::Follow;
use zbus::message::Type;
use zbus::proxy::CacheProperties;
use zbus::{
    Connection, MatchRule, MessageStream, ObjectPath, OwnedObjectPath, OwnedVariant, Variant,
};

/// NetworkManager's root object.
const ROOT: &str = "/org/freedesktop/NetworkManager";
/// The path NetworkManager gives for "no object".
const NONE: &str = "/";
/// `NM_DEVICE_TYPE_WIFI`.
const DEVICE_TYPE_WIFI: u32 = 2;

#[zbus::proxy(
    interface = "org.freedesktop.NetworkManager",
    default_service = "org.freedesktop.NetworkManager",
    default_path = "/org/freedesktop/NetworkManager"
)]
trait Manager {
    fn get_devices(&self) -> zbus::Result<Vec<OwnedObjectPath>>;
    fn activate_connection(
        &self,
        connection: &ObjectPath<'_>,
        device: &ObjectPath<'_>,
        specific_object: &ObjectPath<'_>,
    ) -> zbus::Result<OwnedObjectPath>;
    fn add_and_activate_connection(
        &self,
        connection: HashMap<&str, HashMap<&str, Variant<'_>>>,
        device: &ObjectPath<'_>,
        specific_object: &ObjectPath<'_>,
    ) -> zbus::Result<(OwnedObjectPath, OwnedObjectPath)>;
    #[zbus(property)]
    fn primary_connection(&self) -> zbus::Result<OwnedObjectPath>;
    #[zbus(property)]
    fn activating_connection(&self) -> zbus::Result<OwnedObjectPath>;
    #[zbus(property)]
    fn wireless_enabled(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn set_wireless_enabled(&self, on: bool) -> zbus::Result<()>;
    #[zbus(property)]
    fn wireless_hardware_enabled(&self) -> zbus::Result<bool>;
}

#[zbus::proxy(
    interface = "org.freedesktop.NetworkManager.Connection.Active",
    default_service = "org.freedesktop.NetworkManager"
)]
trait Active {
    #[zbus(property)]
    fn devices(&self) -> zbus::Result<Vec<OwnedObjectPath>>;
}

#[zbus::proxy(
    interface = "org.freedesktop.NetworkManager.Device",
    default_service = "org.freedesktop.NetworkManager"
)]
trait Device {
    #[zbus(property)]
    fn interface(&self) -> zbus::Result<String>;
    #[zbus(property)]
    fn device_type(&self) -> zbus::Result<u32>;
    #[zbus(property)]
    fn ip4_config(&self) -> zbus::Result<OwnedObjectPath>;
    #[zbus(property)]
    fn state(&self) -> zbus::Result<u32>;
    /// `StateChanged`, named apart from `State`'s change stream.
    #[zbus(signal, name = "StateChanged")]
    fn device_state_changed(&self, new_state: u32, old_state: u32, reason: u32)
    -> zbus::Result<()>;
}

#[zbus::proxy(
    interface = "org.freedesktop.NetworkManager.Device.Wireless",
    default_service = "org.freedesktop.NetworkManager"
)]
trait Wireless {
    fn get_all_access_points(&self) -> zbus::Result<Vec<OwnedObjectPath>>;
    fn request_scan(&self, options: HashMap<&str, Variant<'_>>) -> zbus::Result<()>;
    #[zbus(property)]
    fn last_scan(&self) -> zbus::Result<i64>;
    #[zbus(property)]
    fn active_access_point(&self) -> zbus::Result<OwnedObjectPath>;
}

#[zbus::proxy(
    interface = "org.freedesktop.NetworkManager.IP4Config",
    default_service = "org.freedesktop.NetworkManager"
)]
trait Ip4Config {
    #[zbus(property)]
    fn address_data(&self) -> zbus::Result<Vec<HashMap<String, OwnedVariant>>>;
    #[zbus(property)]
    fn gateway(&self) -> zbus::Result<String>;
}

#[zbus::proxy(
    interface = "org.freedesktop.NetworkManager.AccessPoint",
    default_service = "org.freedesktop.NetworkManager"
)]
trait AccessPoint {
    #[zbus(property)]
    fn ssid(&self) -> zbus::Result<Vec<u8>>;
    #[zbus(property)]
    fn strength(&self) -> zbus::Result<u8>;
    #[zbus(property)]
    fn flags(&self) -> zbus::Result<u32>;
    #[zbus(property)]
    fn wpa_flags(&self) -> zbus::Result<u32>;
    #[zbus(property)]
    fn rsn_flags(&self) -> zbus::Result<u32>;
}

/// NetworkManager on one connection.
#[derive(Clone, Debug)]
pub struct NetworkManager {
    connection: Connection,
    manager: ManagerProxy<'static>,
}

impl NetworkManager {
    /// NetworkManager on the system bus.
    ///
    /// # Errors
    ///
    /// When the system bus cannot be reached.
    pub async fn system() -> zbus::Result<Self> {
        Self::on(&Connection::system().await?).await
    }

    /// NetworkManager on `connection`.
    ///
    /// # Errors
    ///
    /// When the proxy cannot be built.
    pub async fn on(connection: &Connection) -> zbus::Result<Self> {
        let manager = ManagerProxy::builder(connection)
            .cache_properties(CacheProperties::No)
            .build()
            .await?;
        Ok(Self {
            connection: connection.clone(),
            manager,
        })
    }

    /// A proxy `P` for NetworkManager's object at `path`.
    async fn proxy<P>(&self, path: &OwnedObjectPath) -> zbus::Result<P>
    where
        P: From<zbus::Proxy<'static>> + zbus::proxy::Defaults,
    {
        zbus::proxy::Builder::<P>::new(&self.connection)
            .path(path.clone())
            .cache_properties(CacheProperties::No)
            .build()
            .await
    }

    /// The reading now, and the paths of the objects it was read from.
    ///
    /// # Errors
    ///
    /// When NetworkManager does not answer, or answers with the wrong types.
    pub async fn read(&self) -> zbus::Result<(Reading, Vec<OwnedObjectPath>)> {
        let mut paths = vec![OwnedObjectPath::try_from(ROOT)?];
        let radio_blocked = !self.manager.wireless_enabled().await?
            || !self.manager.wireless_hardware_enabled().await?;
        let mut active = self.manager.primary_connection().await?;
        if active.as_str() == NONE {
            active = self.manager.activating_connection().await?;
        }
        if active.as_str() == NONE {
            return Ok((
                Reading {
                    link: None,
                    radio_blocked,
                },
                paths,
            ));
        }
        let devices = self.proxy::<ActiveProxy>(&active).await?.devices().await?;
        paths.push(active);
        let Some(device_path) = devices.into_iter().next() else {
            return Ok((
                Reading {
                    link: None,
                    radio_blocked,
                },
                paths,
            ));
        };
        let device = self.proxy::<DeviceProxy>(&device_path).await?;
        let mut link = Link {
            ifname: device.interface().await?,
            ..Link::default()
        };
        let config = device.ip4_config().await?;
        let wifi = device.device_type().await? == DEVICE_TYPE_WIFI;
        if config.as_str() != NONE {
            let config_proxy = self.proxy::<Ip4ConfigProxy>(&config).await?;
            if let Some(address) = config_proxy.address_data().await?.first() {
                let field = |key: &str| address.get(key).cloned();
                link.ipaddr = field("address")
                    .and_then(|value| String::try_from(value).ok())
                    .unwrap_or_default();
                link.cidr = field("prefix")
                    .and_then(|value| u32::try_from(value).ok())
                    .map(|prefix| prefix.to_string())
                    .unwrap_or_default();
            }
            link.gwaddr = config_proxy.gateway().await?;
            paths.push(config);
        }
        if wifi {
            let point = self
                .proxy::<WirelessProxy>(&device_path)
                .await?
                .active_access_point()
                .await?;
            if point.as_str() != NONE {
                let point_proxy = self.proxy::<AccessPointProxy>(&point).await?;
                link.essid = String::from_utf8_lossy(&point_proxy.ssid().await?).into_owned();
                link.signal_strength = point_proxy.strength().await?;
                paths.push(point);
            }
        }
        paths.push(device_path);
        Ok((
            Reading {
                link: Some(link),
                radio_blocked,
            },
            paths,
        ))
    }

    /// Sends the reading to `readings` now and after every change, until
    /// NetworkManager goes away or every receiver is dropped.
    ///
    /// # Errors
    ///
    /// When subscribing or reading fails.
    pub async fn watch(&self, readings: &watch::Sender<Option<Reading>>) -> zbus::Result<()> {
        let rule = MatchRule::builder()
            .msg_type(Type::Signal)
            .interface("org.freedesktop.DBus.Properties")
            .member("PropertiesChanged")
            .path_namespace(ROOT)
            .build()?;
        // Subscribed before the first read, so a change in between is not lost.
        let mut changes =
            Follow::new(MessageStream::for_match_rule(rule, &self.connection, None).await?);
        // Any change during the first read; then those from the objects
        // the last reading came from.
        let (reading, mut paths) = changes
            .during(self.read(), |_| Ok::<_, zbus::Error>(true))
            .await??;
        readings.send_replace(Some(reading));
        while changes.changed(|change| from(&paths, change)).await? {
            let (reading, read_from) = changes
                .during(self.read(), |change| from(&paths, change))
                .await??;
            paths = read_from;
            if !crate::bus::publish(readings, Some(reading)) {
                break;
            }
        }
        Ok(())
    }
}

/// Whether `change` came from one of `paths`.
fn from(paths: &[OwnedObjectPath], change: zbus::Result<zbus::Message>) -> zbus::Result<bool> {
    let change = change?;
    let header = change.header();
    Ok(header.path().is_some_and(|path| {
        paths
            .iter()
            .any(|followed| followed.as_str() == path.as_str())
    }))
}

pub mod wifi;

#[cfg(test)]
mod tests;
