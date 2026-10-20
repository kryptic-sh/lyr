//! The battery source on Linux: UPower's display device over the system bus.
//!
//! Nothing is polled. lyr reads the device once, then again each time UPower
//! signals `PropertiesChanged` on it, and sends the new reading on.

use lyr_core::battery::Reading;
use tokio::sync::watch;

use crate::bus::Follow;
use zbus::Connection;
use zbus::fdo::{PropertiesChanged, PropertiesProxy};
use zbus::names::InterfaceName;
use zbus::proxy::CacheProperties;

use super::upower::Device;

/// UPower's service name and its composite battery.
const SERVICE: &str = "org.freedesktop.UPower";
const DISPLAY_DEVICE: &str = "/org/freedesktop/UPower/devices/DisplayDevice";
const INTERFACE: &str = "org.freedesktop.UPower.Device";

/// The display device's properties lyr reads. Uncached, so a read straight
/// after a change signal sees the new values.
#[zbus::proxy(
    interface = "org.freedesktop.UPower.Device",
    default_service = "org.freedesktop.UPower",
    default_path = "/org/freedesktop/UPower/devices/DisplayDevice"
)]
trait DisplayDevice {
    #[zbus(property, name = "Type")]
    fn kind(&self) -> zbus::Result<u32>;
    #[zbus(property)]
    fn is_present(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn state(&self) -> zbus::Result<u32>;
    #[zbus(property)]
    fn percentage(&self) -> zbus::Result<f64>;
    #[zbus(property)]
    fn energy_rate(&self) -> zbus::Result<f64>;
    #[zbus(property)]
    fn time_to_empty(&self) -> zbus::Result<i64>;
    #[zbus(property)]
    fn time_to_full(&self) -> zbus::Result<i64>;
}

/// UPower's display device on one connection.
#[derive(Clone, Debug)]
pub struct UPower {
    device: DisplayDeviceProxy<'static>,
    properties: PropertiesProxy<'static>,
}

impl UPower {
    /// UPower on the system bus.
    ///
    /// # Errors
    ///
    /// When the system bus cannot be reached.
    pub async fn system() -> zbus::Result<Self> {
        Self::on(&Connection::system().await?).await
    }

    /// UPower on `connection`.
    ///
    /// # Errors
    ///
    /// When the proxies cannot be built.
    pub async fn on(connection: &Connection) -> zbus::Result<Self> {
        let device = DisplayDeviceProxy::builder(connection)
            .cache_properties(CacheProperties::No)
            .build()
            .await?;
        let properties = PropertiesProxy::builder(connection)
            .destination(SERVICE)
            .path(DISPLAY_DEVICE)
            .build()
            .await?;
        Ok(Self { device, properties })
    }

    /// The display device as it is now.
    ///
    /// # Errors
    ///
    /// When UPower does not answer, or answers with the wrong types.
    pub async fn device(&self) -> zbus::Result<Device> {
        Ok(Device {
            kind: self.device.kind().await?,
            present: self.device.is_present().await?,
            state: self.device.state().await?,
            percentage: self.device.percentage().await?,
            energy_rate: self.device.energy_rate().await?,
            time_to_empty: self.device.time_to_empty().await?,
            time_to_full: self.device.time_to_full().await?,
        })
    }

    /// Sends the battery reading to `readings` now and after every change,
    /// until UPower goes away or every receiver is dropped.
    ///
    /// # Errors
    ///
    /// When subscribing or reading fails.
    pub async fn watch(&self, readings: &watch::Sender<Option<Reading>>) -> zbus::Result<()> {
        let interface = InterfaceName::from_static_str(INTERFACE)?;
        // Subscribe before the first read, so a change in between is not lost.
        let mut changes = Follow::new(self.properties.receive_properties_changed().await?);
        let wanted = |change: PropertiesChanged| -> zbus::Result<bool> {
            Ok(change.args()?.interface_name() == &interface)
        };
        readings.send_replace(changes.during(self.device(), wanted).await??.reading());
        while changes.changed(wanted).await? {
            if !crate::bus::publish(
                readings,
                changes.during(self.device(), wanted).await??.reading(),
            ) {
                break;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
