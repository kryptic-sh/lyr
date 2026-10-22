//! The bluetooth module's source on Windows, through WinRT: the default
//! adapter (`BluetoothAdapter::GetDefaultAsync`) and its radio's state
//! (`Windows.Devices.Radios`), and the paired devices, classic and LE, as
//! association endpoints with whether each is connected
//! (`System.Devices.Aep.IsConnected`). [`Watch`] runs a `DeviceWatcher` on
//! the adapters and on each kind of paired device, and follows the radio's
//! `StateChanged`, so nothing is polled.
//!
//! Every call here runs in COM's multithreaded apartment: the caller's
//! thread joins it with [`init`]. Checked on a machine with no working
//! Bluetooth adapter only (`docs/backlog.md`).

use std::io;
use std::sync::Arc;

use windows::Devices::Bluetooth::{BluetoothAdapter, BluetoothDevice, BluetoothLEDevice};
use windows::Devices::Enumeration::{DeviceInformation, DeviceInformationKind, DeviceWatcher};
use windows::Devices::Radios::{Radio as WinRadio, RadioState};
use windows::Foundation::{IReference, TypedEventHandler};
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use windows_collections::IIterable;
use windows_core::{HSTRING, IInspectable, Interface};

use super::win32::{Endpoint, Radio, adapter as shaped, devices};
use super::{Adapter, Snapshot, Tracker};

/// Whether an endpoint is connected, which a watcher asked for it reports
/// changes of.
const IS_CONNECTED: &str = "System.Devices.Aep.IsConnected";
/// An endpoint's Bluetooth address, which joins a device's classic and LE
/// endpoints.
const DEVICE_ADDRESS: &str = "System.Devices.Aep.DeviceAddress";

/// Joins the calling thread to COM's multithreaded apartment, which WinRT
/// needs.
///
/// # Errors
///
/// When the thread is already in a single-threaded apartment.
pub fn init() -> io::Result<()> {
    // SAFETY: no reserved argument; any thread may join the MTA.
    unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }
        .ok()
        .map_err(io::Error::other)
}

/// What Windows reports now, with `tracker` remembering the order devices
/// connect in. No adapter, or one Windows will not describe, hides the
/// module.
#[must_use]
pub fn snapshot(tracker: &mut Tracker) -> Snapshot {
    let adapter = default_adapter();
    let devices = adapter.as_ref().map_or_else(Vec::new, |_| {
        let mut endpoints = paired(&BluetoothDevice::GetDeviceSelectorFromPairingState(true));
        endpoints.extend(paired(
            &BluetoothLEDevice::GetDeviceSelectorFromPairingState(true),
        ));
        devices(&endpoints)
    });
    Snapshot {
        reading: tracker.reading(adapter.as_ref(), &devices),
        adapter,
        devices,
    }
}

/// The default adapter, with its radio's state.
fn default_adapter() -> Option<Adapter> {
    let adapter = BluetoothAdapter::GetDefaultAsync().ok()?.join().ok()?;
    let address = adapter.BluetoothAddress().ok()?;
    let radio = radio(&adapter).and_then(|radio| radio.State().ok());
    Some(shaped(address, radio.map(state)))
}

fn radio(adapter: &BluetoothAdapter) -> Option<WinRadio> {
    adapter.GetRadioAsync().ok()?.join().ok()
}

fn state(state: RadioState) -> Radio {
    match state {
        RadioState::On => Radio::On,
        RadioState::Off => Radio::Off,
        RadioState::Disabled => Radio::Disabled,
        _ => Radio::Unknown,
    }
}

/// The properties asked of each endpoint.
fn properties() -> IIterable<HSTRING> {
    IIterable::from(vec![
        HSTRING::from(IS_CONNECTED),
        HSTRING::from(DEVICE_ADDRESS),
    ])
}

/// The paired endpoints `selector` (a pairing-state selector) finds; none
/// when the selector or the query fails.
fn paired(selector: &windows_core::Result<HSTRING>) -> Vec<Endpoint> {
    let Ok(selector) = selector else {
        return Vec::new();
    };
    let Ok(found) = DeviceInformation::FindAllAsyncWithKindAqsFilterAndAdditionalProperties(
        selector,
        &properties(),
        DeviceInformationKind::AssociationEndpoint,
    )
    .and_then(|query| query.join()) else {
        return Vec::new();
    };
    (0..found.Size().unwrap_or(0))
        .filter_map(|at| found.GetAt(at).ok())
        .filter_map(|info| endpoint(&info))
        .collect()
}

fn endpoint(info: &DeviceInformation) -> Option<Endpoint> {
    let property = |name: &str| info.Properties().ok()?.Lookup(&HSTRING::from(name)).ok();
    Some(Endpoint {
        id: info.Id().ok()?.to_string(),
        name: info.Name().map(|name| name.to_string()).unwrap_or_default(),
        address: property(DEVICE_ADDRESS)
            .and_then(|value| reference::<HSTRING>(&value))
            .map(|address| address.to_string()),
        connected: property(IS_CONNECTED)
            .and_then(|value| reference::<bool>(&value))
            .unwrap_or(false),
    })
}

/// A boxed property value, `None` when it holds another type.
fn reference<T: windows_core::RuntimeType + 'static>(value: &IInspectable) -> Option<T> {
    value.cast::<IReference<T>>().ok()?.Value().ok()
}

/// What a change calls.
type OnChange = Arc<dyn Fn() + Send + Sync>;

/// The watchers and the radio's registration calling `on_change` while it
/// lives.
pub struct Watch {
    watchers: Vec<DeviceWatcher>,
    radio: Option<(WinRadio, i64)>,
}

impl std::fmt::Debug for Watch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Watch")
            .field("watchers", &self.watchers.len())
            .field("radio", &self.radio.is_some())
            .finish()
    }
}

impl Watch {
    /// Calls `on_change` when an adapter comes or goes, a paired device is
    /// added, removed or connects or disconnects, or the default adapter's
    /// radio switches. The radio is followed only when an adapter is there
    /// at the start; one plugged in later still shows, through the
    /// adapters' watcher, but its switching is seen at the next other
    /// change.
    ///
    /// # Errors
    ///
    /// When a watcher cannot be made or started.
    pub fn new(on_change: impl Fn() + Send + Sync + 'static) -> io::Result<Self> {
        let on_change: OnChange = Arc::new(on_change);
        let mut watch = Self {
            watchers: Vec::new(),
            radio: None,
        };
        let empty: IIterable<HSTRING> = IIterable::from(Vec::<HSTRING>::new());
        let watchers = [
            (
                BluetoothAdapter::GetDeviceSelector(),
                empty,
                DeviceInformationKind::DeviceInterface,
            ),
            (
                BluetoothDevice::GetDeviceSelectorFromPairingState(true),
                properties(),
                DeviceInformationKind::AssociationEndpoint,
            ),
            (
                BluetoothLEDevice::GetDeviceSelectorFromPairingState(true),
                properties(),
                DeviceInformationKind::AssociationEndpoint,
            ),
        ];
        for (selector, asked, kind) in watchers {
            let watcher = DeviceInformation::CreateWatcherWithKindAqsFilterAndAdditionalProperties(
                &selector.map_err(io::Error::other)?,
                &asked,
                kind,
            )
            .map_err(io::Error::other)?;
            watch_all(&watcher, &on_change).map_err(io::Error::other)?;
            watcher.Start().map_err(io::Error::other)?;
            watch.watchers.push(watcher);
        }
        let radio = BluetoothAdapter::GetDefaultAsync()
            .and_then(|adapter| adapter.join())
            .ok()
            .and_then(|adapter| radio(&adapter));
        if let Some(radio) = radio {
            let changed = Arc::clone(&on_change);
            let token = radio
                .StateChanged(&TypedEventHandler::new(move |_, _| {
                    changed();
                    Ok(())
                }))
                .map_err(io::Error::other)?;
            watch.radio = Some((radio, token));
        }
        Ok(watch)
    }
}

/// Has `watcher` call `on_change` for every event it raises. A watcher
/// raises updates only once it has handlers for both `Added` and `Updated`.
fn watch_all(watcher: &DeviceWatcher, on_change: &OnChange) -> windows_core::Result<()> {
    let call = |on_change: &OnChange| {
        let on_change = Arc::clone(on_change);
        move || on_change()
    };
    let added = call(on_change);
    watcher.Added(&TypedEventHandler::new(move |_, _| {
        added();
        Ok(())
    }))?;
    let updated = call(on_change);
    watcher.Updated(&TypedEventHandler::new(move |_, _| {
        updated();
        Ok(())
    }))?;
    let removed = call(on_change);
    watcher.Removed(&TypedEventHandler::new(move |_, _| {
        removed();
        Ok(())
    }))?;
    let completed = call(on_change);
    watcher.EnumerationCompleted(&TypedEventHandler::new(move |_, _| {
        completed();
        Ok(())
    }))?;
    Ok(())
}

impl Drop for Watch {
    fn drop(&mut self) {
        for watcher in &self.watchers {
            // A watcher that already stopped, or never finished starting,
            // has nothing left to stop.
            let _stopped = watcher.Stop();
        }
        if let Some((radio, token)) = &self.radio {
            // The radio went with its adapter: nothing left to undo.
            let _removed = radio.RemoveStateChanged(*token);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Tracker, Watch, init, snapshot};

    #[test]
    fn this_machine_s_bluetooth_reads_and_a_watch_starts() {
        init().expect("in the multithreaded apartment");
        let snapshot = snapshot(&mut Tracker::default());
        // A machine may have no adapter; whatever it has is consistent.
        assert_eq!(snapshot.reading.is_some(), snapshot.adapter.is_some());
        if snapshot.adapter.is_none() {
            assert!(snapshot.devices.is_empty());
        }
        let watch = Watch::new(|| {}).expect("the watchers start");
        assert!(!format!("{watch:?}").is_empty());
    }
}
