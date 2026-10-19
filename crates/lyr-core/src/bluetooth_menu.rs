//! The Bluetooth controller as the module's source and the Bluetooth menu
//! (`docs/plan/10-popups-and-menus.md#bluetooth-menu`) see it: the adapter,
//! its devices, and what the menu does to them. The platforms' sources fill
//! it in (`lyr_sources::bluetooth`).

/// The adapter's properties the module uses (`org.bluez.Adapter1`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Adapter {
    /// Its object path, which identifies it.
    pub path: String,
    pub powered: bool,
    /// The menu's switches, and whether a scan is running.
    pub discoverable: bool,
    pub pairable: bool,
    pub discovering: bool,
    /// `PowerState`, which BlueZ marks experimental and may not send;
    /// `off-blocked` is an rfkill block.
    pub power_state: Option<String>,
}

/// A device's properties the module uses (`org.bluez.Device1`, and
/// `org.bluez.Battery1` when the device reports its battery).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Device {
    /// Its object path, which identifies it.
    pub path: String,
    pub alias: String,
    pub connected: bool,
    /// Paired with this machine; a device a scan found is not.
    pub paired: bool,
    pub trusted: bool,
    pub battery: Option<u8>,
}

/// One of the adapter's switches in the Bluetooth menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Switch {
    Powered,
    Discoverable,
    Pairable,
}

/// What the Bluetooth menu does to a device.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeviceAction {
    Connect,
    Disconnect,
    Pair,
    Trust,
    Untrust,
}
