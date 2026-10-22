//! What Windows reports about Bluetooth, turned into BlueZ's shapes
//! ([`Adapter`], [`Device`]) so [`super::Tracker`] gives the reading on every
//! platform's tests; [`super::windows`] reads it through WinRT.

use super::{Adapter, Device};

/// `Windows.Devices.Radios.RadioState`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Radio {
    On,
    /// Switched off in Windows.
    Off,
    /// Turned off by a hardware switch or airplane mode, which is what
    /// BlueZ's `off-blocked` says on Linux.
    Disabled,
    Unknown,
}

/// The adapter at `address`, whose radio is in `radio`; `None` when the
/// radio could not be read, which shows the adapter as off.
#[must_use]
pub fn adapter(address: u64, radio: Option<Radio>) -> Adapter {
    Adapter {
        path: format_address(address),
        powered: radio == Some(Radio::On),
        // Windows answers these in Settings, not to programs.
        discoverable: false,
        pairable: false,
        discovering: false,
        power_state: (radio == Some(Radio::Disabled)).then(|| "off-blocked".to_owned()),
    }
}

/// A Bluetooth address, the low 48 bits of `address`, as BlueZ writes one:
/// six upper-case hex bytes, most significant first.
#[must_use]
pub fn format_address(address: u64) -> String {
    (0..6)
        .rev()
        .map(|byte| format!("{:02X}", (address >> (8 * byte)) & 0xff))
        .collect::<Vec<_>>()
        .join(":")
}

/// One paired association endpoint, as a `DeviceWatcher` or `FindAllAsync`
/// lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Endpoint {
    /// `DeviceInformation.Id`.
    pub id: String,
    pub name: String,
    /// `System.Devices.Aep.DeviceAddress`, such as `aa:bb:cc:dd:ee:ff`.
    pub address: Option<String>,
    /// `System.Devices.Aep.IsConnected`.
    pub connected: bool,
}

/// The paired devices `endpoints` stand for. A device paired over both
/// classic Bluetooth and LE has an endpoint for each, so endpoints with the
/// same address are one device, connected when either is; it is named by
/// the first, and its address stands for its path, as on macOS. An
/// endpoint with no address keeps its id.
#[must_use]
pub fn devices(endpoints: &[Endpoint]) -> Vec<Device> {
    let mut devices: Vec<Device> = Vec::new();
    for endpoint in endpoints {
        let path = endpoint
            .address
            .as_deref()
            .map_or_else(|| endpoint.id.clone(), str::to_uppercase);
        if let Some(device) = devices.iter_mut().find(|device| device.path == path) {
            device.connected |= endpoint.connected;
            continue;
        }
        devices.push(Device {
            path,
            alias: endpoint.name.clone(),
            connected: endpoint.connected,
            paired: true,
            // Windows has no trust, and keeps a device's battery to itself.
            trusted: false,
            battery: None,
        });
    }
    devices
}

#[cfg(test)]
mod tests {
    use super::{Endpoint, Radio, adapter, devices, format_address};

    #[test]
    fn an_address_is_six_bytes_most_significant_first() {
        assert_eq!(format_address(0x0000_a1b2_c3d4_e5f6), "A1:B2:C3:D4:E5:F6");
        assert_eq!(format_address(0x0102), "00:00:00:00:01:02");
        assert_eq!(
            format_address(0xffff_0000_0000_0001),
            "00:00:00:00:00:01",
            "only the low 48 bits"
        );
    }

    #[test]
    fn the_radio_decides_power_and_the_block() {
        let on = adapter(1, Some(Radio::On));
        assert!(on.powered);
        assert_eq!(on.power_state, None);
        assert_eq!(on.path, "00:00:00:00:00:01");
        let off = adapter(1, Some(Radio::Off));
        assert!(!off.powered);
        assert_eq!(off.power_state, None);
        let disabled = adapter(1, Some(Radio::Disabled));
        assert!(!disabled.powered);
        assert_eq!(disabled.power_state.as_deref(), Some("off-blocked"));
        assert!(!adapter(1, Some(Radio::Unknown)).powered);
        assert!(!adapter(1, None).powered, "an unreadable radio");
    }

    fn endpoint(id: &str, name: &str, address: Option<&str>, connected: bool) -> Endpoint {
        Endpoint {
            id: id.to_owned(),
            name: name.to_owned(),
            address: address.map(str::to_owned),
            connected,
        }
    }

    #[test]
    fn a_device_on_classic_and_le_is_one_device() {
        let listed = devices(&[
            endpoint("classic", "Buds", Some("aa:bb:cc:dd:ee:ff"), false),
            endpoint("le", "Buds LE", Some("AA:BB:CC:DD:EE:FF"), true),
            endpoint("mouse", "Mouse", Some("11:22:33:44:55:66"), false),
        ]);
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].path, "AA:BB:CC:DD:EE:FF");
        assert_eq!(listed[0].alias, "Buds", "named by the first endpoint");
        assert!(listed[0].connected, "connected through either");
        assert!(listed[0].paired);
        assert!(!listed[1].connected);
        let listed = devices(&[
            endpoint("classic", "Buds", Some("aa:bb:cc:dd:ee:ff"), true),
            endpoint("le", "Buds LE", Some("aa:bb:cc:dd:ee:ff"), false),
        ]);
        assert!(
            listed[0].connected,
            "an idle second endpoint does not undo it"
        );
    }

    #[test]
    fn an_endpoint_without_an_address_keeps_its_id() {
        let listed = devices(&[
            endpoint("one", "One", None, true),
            endpoint("two", "Two", None, false),
        ]);
        assert_eq!(listed.len(), 2, "no address is not the same address");
        assert_eq!(listed[0].path, "one");
        assert!(listed[0].connected);
    }
}
