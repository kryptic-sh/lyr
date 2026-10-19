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
use zbus::{Connection, MatchRule, MessageStream, OwnedVariant};

use super::{Adapter, Device, Tracker};

/// BlueZ's service name.
const SERVICE: &str = "org.bluez";
const ADAPTER: &str = "org.bluez.Adapter1";
const DEVICE: &str = "org.bluez.Device1";
const BATTERY: &str = "org.bluez.Battery1";

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
