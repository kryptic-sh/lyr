//! The bluetooth source on Linux: BlueZ over the system bus.
//!
//! BlueZ publishes its adapters and devices through
//! `org.freedesktop.DBus.ObjectManager` at `/`. lyr reads them all once, and
//! again whenever one of them changes (`PropertiesChanged` under
//! `/org/bluez`) or one comes or goes (`InterfacesAdded` and
//! `InterfacesRemoved`). BlueZ has few objects, so a whole read per change
//! costs little and cannot drift from BlueZ's own state.

use futures_util::StreamExt;
use futures_util::stream::select;
use lyr_core::bluetooth::Reading;
use tokio::sync::watch;
use zbus::fdo::{ManagedObjects, ObjectManagerProxy};
use zbus::message::Type;
use zbus::proxy::CacheProperties;
use zbus::{Connection, MatchRule, MessageStream, ObjectPath, OwnedVariant};

use super::{Adapter, Device, DeviceAction, Switch, Tracker};

/// BlueZ's service name.
const SERVICE: &str = "org.bluez";
const ADAPTER: &str = "org.bluez.Adapter1";
const DEVICE: &str = "org.bluez.Device1";
const BATTERY: &str = "org.bluez.Battery1";

/// An adapter's calls and switches (`org.bluez.Adapter1`), as the
/// Bluetooth menu uses them.
#[zbus::proxy(interface = "org.bluez.Adapter1", default_service = "org.bluez")]
trait AdapterControl {
    fn start_discovery(&self) -> zbus::Result<()>;
    fn stop_discovery(&self) -> zbus::Result<()>;
    fn remove_device(&self, device: &ObjectPath<'_>) -> zbus::Result<()>;
    #[zbus(property)]
    fn powered(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn set_powered(&self, on: bool) -> zbus::Result<()>;
    #[zbus(property)]
    fn discoverable(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn set_discoverable(&self, on: bool) -> zbus::Result<()>;
    #[zbus(property)]
    fn pairable(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn set_pairable(&self, on: bool) -> zbus::Result<()>;
}

/// A device's calls and its trust (`org.bluez.Device1`).
#[zbus::proxy(interface = "org.bluez.Device1", default_service = "org.bluez")]
trait DeviceControl {
    fn connect(&self) -> zbus::Result<()>;
    fn disconnect(&self) -> zbus::Result<()>;
    fn pair(&self) -> zbus::Result<()>;
    #[zbus(property)]
    fn trusted(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn set_trusted(&self, on: bool) -> zbus::Result<()>;
}

/// BlueZ on one connection.
#[derive(Clone, Debug)]
pub struct BlueZ {
    connection: Connection,
    objects: ObjectManagerProxy<'static>,
}

impl BlueZ {
    /// BlueZ on the system bus.
    ///
    /// # Errors
    ///
    /// When the system bus cannot be reached.
    pub async fn system() -> zbus::Result<Self> {
        Self::on(&Connection::system().await?).await
    }

    /// BlueZ on `connection`.
    ///
    /// # Errors
    ///
    /// When the proxy cannot be built.
    pub async fn on(connection: &Connection) -> zbus::Result<Self> {
        let objects = ObjectManagerProxy::builder(connection)
            .destination(SERVICE)
            .path("/")
            .build()
            .await?;
        Ok(Self {
            connection: connection.clone(),
            objects,
        })
    }

    async fn adapter(&self, adapter: &str) -> zbus::Result<AdapterControlProxy<'static>> {
        AdapterControlProxy::builder(&self.connection)
            .path(adapter.to_owned())
            .cache_properties(CacheProperties::No)
            .build()
            .await
    }

    /// Turns the `adapter`'s `switch` on or off.
    ///
    /// # Errors
    ///
    /// When BlueZ refuses (an rfkill block refuses power, for one).
    pub async fn set(&self, adapter: &str, switch: Switch, on: bool) -> zbus::Result<()> {
        let proxy = self.adapter(adapter).await?;
        match switch {
            Switch::Powered => proxy.set_powered(on).await,
            Switch::Discoverable => proxy.set_discoverable(on).await,
            Switch::Pairable => proxy.set_pairable(on).await,
        }
    }

    /// Starts or stops the `adapter`'s scan for new devices.
    ///
    /// # Errors
    ///
    /// When BlueZ refuses.
    pub async fn scan(&self, adapter: &str, on: bool) -> zbus::Result<()> {
        let proxy = self.adapter(adapter).await?;
        if on {
            proxy.start_discovery().await
        } else {
            proxy.stop_discovery().await
        }
    }

    /// Does `action` to the device at `device`.
    ///
    /// # Errors
    ///
    /// When BlueZ or the device refuses.
    pub async fn device(&self, device: &str, action: DeviceAction) -> zbus::Result<()> {
        let proxy = DeviceControlProxy::builder(&self.connection)
            .path(device.to_owned())
            .cache_properties(CacheProperties::No)
            .build()
            .await?;
        match action {
            DeviceAction::Connect => proxy.connect().await,
            DeviceAction::Disconnect => proxy.disconnect().await,
            DeviceAction::Pair => proxy.pair().await,
            DeviceAction::Trust => proxy.set_trusted(true).await,
            DeviceAction::Untrust => proxy.set_trusted(false).await,
        }
    }

    /// Forgets the device at `device`: the `adapter` removes it, pairing and
    /// all.
    ///
    /// # Errors
    ///
    /// When BlueZ refuses.
    pub async fn forget(&self, adapter: &str, device: &str) -> zbus::Result<()> {
        let path = ObjectPath::try_from(device)?;
        self.adapter(adapter).await?.remove_device(&path).await
    }

    /// The first adapter and every device, as they are now.
    ///
    /// # Errors
    ///
    /// When BlueZ does not answer.
    pub async fn read(&self) -> zbus::Result<(Option<Adapter>, Vec<Device>)> {
        Ok(parse(&self.objects.get_managed_objects().await?))
    }

    /// Sends the reading to `readings` now and after every change, until
    /// BlueZ goes away or every receiver is dropped.
    ///
    /// # Errors
    ///
    /// When subscribing or reading fails.
    pub async fn watch(&self, readings: &watch::Sender<Option<Reading>>) -> zbus::Result<()> {
        let properties = MatchRule::builder()
            .msg_type(Type::Signal)
            .interface("org.freedesktop.DBus.Properties")
            .member("PropertiesChanged")
            .path_namespace("/org/bluez")
            .build()?;
        let objects = MatchRule::builder()
            .msg_type(Type::Signal)
            .interface("org.freedesktop.DBus.ObjectManager")
            .build()?;
        // Subscribed before the first read, so a change in between is not lost.
        let mut changes = select(
            MessageStream::for_match_rule(properties, &self.connection, None).await?,
            MessageStream::for_match_rule(objects, &self.connection, None).await?,
        );
        let mut tracker = Tracker::default();
        let (adapter, devices) = self.read().await?;
        readings.send_replace(tracker.reading(adapter.as_ref(), &devices));
        while changes.next().await.is_some() {
            let (adapter, devices) = self.read().await?;
            if readings
                .send(tracker.reading(adapter.as_ref(), &devices))
                .is_err()
            {
                break;
            }
        }
        Ok(())
    }
}

/// The first adapter, by object path, and every device, from BlueZ's
/// managed objects.
fn parse(objects: &ManagedObjects) -> (Option<Adapter>, Vec<Device>) {
    let mut paths: Vec<_> = objects.keys().collect();
    paths.sort_by_key(|path| path.as_str());
    let mut adapter = None;
    let mut devices = Vec::new();
    for path in paths {
        let interfaces = &objects[path];
        let property = |interface: &str, name: &str| -> Option<&OwnedVariant> {
            interfaces
                .iter()
                .find(|(key, _)| key.as_str() == interface)
                .and_then(|(_, properties)| properties.get(name))
        };
        let flag = |interface, name| {
            property(interface, name).and_then(|value| bool::try_from(value).ok())
        };
        let text = |interface, name| {
            property(interface, name).and_then(|value| String::try_from(value.clone()).ok())
        };
        if adapter.is_none() && property(ADAPTER, "Powered").is_some() {
            adapter = Some(Adapter {
                path: path.to_string(),
                powered: flag(ADAPTER, "Powered").unwrap_or(false),
                power_state: text(ADAPTER, "PowerState"),
            });
        }
        if property(DEVICE, "Address").is_some() {
            devices.push(Device {
                path: path.to_string(),
                alias: text(DEVICE, "Alias").unwrap_or_default(),
                connected: flag(DEVICE, "Connected").unwrap_or(false),
                battery: property(BATTERY, "Percentage").and_then(|value| u8::try_from(value).ok()),
            });
        }
    }
    (adapter, devices)
}

#[cfg(test)]
mod tests;
