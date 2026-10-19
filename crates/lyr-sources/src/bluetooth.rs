//! The bluetooth module's source (`docs/plan/07-modules.md#bluetooth`).
//!
//! [`Tracker`] turns BlueZ's adapter and devices into the view's reading, on
//! every platform's tests; [`linux`] reads them over D-Bus. Windows and macOS
//! come with milestones M9 and M10.

use lyr_core::bluetooth::{Device as Shown, Reading, State};
pub use lyr_core::bluetooth_menu::{Adapter, Device, DeviceAction, Switch};

#[cfg(target_os = "linux")]
pub mod linux;

/// The connected devices in the order they connected, so the module shows
/// the latest, as waybar's `connected_devices_.back()` does.
#[derive(Clone, Debug, Default)]
pub struct Tracker {
    connected: Vec<String>,
}

impl Tracker {
    /// The reading for `adapter` and `devices`, remembering the order devices
    /// connect in. `None` without an adapter, which hides the module.
    pub fn reading(&mut self, adapter: Option<&Adapter>, devices: &[Device]) -> Option<Reading> {
        let adapter = adapter?;
        let connected = |path: &String| {
            devices
                .iter()
                .any(|device| device.connected && device.path == *path)
        };
        self.connected.retain(connected);
        for device in devices.iter().filter(|device| device.connected) {
            if !self.connected.contains(&device.path) {
                self.connected.push(device.path.clone());
            }
        }
        let state = if adapter.power_state.as_deref() == Some("off-blocked") {
            State::Disabled
        } else if !adapter.powered {
            State::Off
        } else if self.connected.is_empty() {
            State::On
        } else {
            State::Connected
        };
        let device = self
            .connected
            .last()
            .and_then(|path| devices.iter().find(|device| device.path == *path))
            .map(|device| Shown {
                alias: device.alias.clone(),
                battery: device.battery,
            });
        Some(Reading {
            state,
            connections: self.connected.len(),
            device,
        })
    }
}

#[cfg(test)]
mod tests {
    use lyr_core::bluetooth::State;

    use super::{Adapter, Device, Tracker};

    fn adapter(powered: bool) -> Adapter {
        Adapter {
            path: "/org/bluez/hci0".to_owned(),
            powered,
            discoverable: false,
            pairable: false,
            discovering: false,
            power_state: None,
        }
    }

    fn device(path: &str, connected: bool) -> Device {
        Device {
            path: path.to_owned(),
            alias: path.trim_start_matches("/org/bluez/hci0/").to_owned(),
            connected,
            paired: true,
            trusted: false,
            battery: None,
        }
    }

    #[test]
    fn no_adapter_hides_the_module() {
        assert!(Tracker::default().reading(None, &[]).is_none());
    }

    #[test]
    fn the_state_follows_power_rfkill_and_connections() {
        let state = |adapter: Adapter, devices: &[Device]| {
            Tracker::default()
                .reading(Some(&adapter), devices)
                .map(|reading| reading.state)
        };
        assert_eq!(state(adapter(false), &[]), Some(State::Off));
        assert_eq!(
            state(adapter(true), &[device("/a", false)]),
            Some(State::On)
        );
        assert_eq!(
            state(adapter(true), &[device("/a", true)]),
            Some(State::Connected)
        );
        let blocked = Adapter {
            path: "/org/bluez/hci0".to_owned(),
            powered: false,
            discoverable: false,
            pairable: false,
            discovering: false,
            power_state: Some("off-blocked".to_owned()),
        };
        assert_eq!(state(blocked, &[]), Some(State::Disabled));
    }

    #[test]
    fn the_device_shown_is_the_last_to_connect() {
        let mut tracker = Tracker::default();
        let on = adapter(true);
        let shown = |tracker: &mut Tracker, devices: &[Device]| {
            tracker
                .reading(Some(&on), devices)
                .and_then(|reading| reading.device)
                .map(|device| device.alias)
        };
        // Buds connect first, then Mouse: Mouse is shown.
        assert_eq!(
            shown(
                &mut tracker,
                &[device("buds", true), device("mouse", false)]
            ),
            Some("buds".to_owned())
        );
        assert_eq!(
            shown(&mut tracker, &[device("buds", true), device("mouse", true)]),
            Some("mouse".to_owned())
        );
        // Listed in the other order, Mouse is still the latest.
        assert_eq!(
            shown(&mut tracker, &[device("mouse", true), device("buds", true)]),
            Some("mouse".to_owned())
        );
        // Mouse goes: back to Buds.
        let reading = tracker
            .reading(Some(&on), &[device("buds", true), device("mouse", false)])
            .expect("an adapter");
        assert_eq!(reading.connections, 1);
        assert_eq!(
            reading.device.map(|device| device.alias).as_deref(),
            Some("buds")
        );
    }
}
