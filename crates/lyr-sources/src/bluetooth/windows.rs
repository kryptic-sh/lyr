//! The bluetooth module's and menu's source on Windows, through WinRT: the
//! default adapter (`BluetoothAdapter::GetDefaultAsync`) and its radio's
//! state (`Windows.Devices.Radios`), and the paired devices, classic and
//! LE, as association endpoints with whether each is connected
//! (`System.Devices.Aep.IsConnected`). [`Watch`] runs a `DeviceWatcher` on
//! the adapters and on each kind of paired device, and follows the radio's
//! `StateChanged`, so nothing is polled. The menu's actions: [`set_power`],
//! a [`Scan`] for devices not paired, [`pair`] (asking the menu what a
//! pairing needs) and [`unpair`]; connecting a paired device has no public
//! API, so the menu sends that to Settings.
//!
//! Every call here runs in COM's multithreaded apartment: the caller's
//! thread joins it with [`init`]. Checked on a machine with no working
//! Bluetooth adapter only (`docs/backlog.md`).

use std::io;
use std::sync::{Arc, Mutex, PoisonError};

use lyr_core::bluetooth_menu::{Question, Reply};
use windows::Devices::Bluetooth::{BluetoothAdapter, BluetoothDevice, BluetoothLEDevice};
use windows::Devices::Enumeration::{
    DeviceInformation, DeviceInformationKind, DeviceInformationUpdate, DevicePairingKinds,
    DevicePairingRequestedEventArgs, DevicePairingResultStatus, DeviceUnpairingResultStatus,
    DeviceWatcher,
};
use windows::Devices::Radios::{Radio as WinRadio, RadioAccessStatus, RadioState};
use windows::Foundation::{IReference, TypedEventHandler};
use windows::Win32::System::Com::{COINIT_MULTITHREADED, CoInitializeEx};
use windows_collections::IIterable;
use windows_core::{HSTRING, IInspectable, Interface, Ref};

use super::win32::{Endpoint, Kind, Radio, accept, adapter as shaped, devices, question};
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
/// connect in, and the devices `scan` has found; and the endpoints behind
/// the devices, which [`pair`] and [`unpair`] go by. No adapter, or one
/// Windows will not describe, hides the module.
#[must_use]
pub fn snapshot(tracker: &mut Tracker, scan: Option<&Scan>) -> (Snapshot, Vec<Endpoint>) {
    let adapter = default_adapter().map(|adapter| Adapter {
        discovering: scan.is_some(),
        ..adapter
    });
    let mut endpoints = Vec::new();
    if adapter.is_some() {
        endpoints.extend(paired(&BluetoothDevice::GetDeviceSelectorFromPairingState(
            true,
        )));
        endpoints.extend(paired(
            &BluetoothLEDevice::GetDeviceSelectorFromPairingState(true),
        ));
        endpoints.extend(scan.map(Scan::found).unwrap_or_default());
    }
    let devices = devices(&endpoints);
    (
        Snapshot {
            reading: tracker.reading(adapter.as_ref(), &devices),
            adapter,
            devices,
        },
        endpoints,
    )
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
        .filter_map(|info| endpoint(&info, true))
        .collect()
}

fn endpoint(info: &DeviceInformation, paired: bool) -> Option<Endpoint> {
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
        paired,
    })
}

/// A boxed property value, `None` when it holds another type.
fn reference<T: windows_core::RuntimeType + 'static>(value: &IInspectable) -> Option<T> {
    value.cast::<IReference<T>>().ok()?.Value().ok()
}

/// `status` as success, or as an error naming what was refused.
fn allowed(status: RadioAccessStatus) -> io::Result<()> {
    if status == RadioAccessStatus::Allowed {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "Windows refused the radio (status {})",
            status.0
        )))
    }
}

/// Switches the default adapter's radio on or off.
///
/// # Errors
///
/// When there is no adapter, or Windows refuses this program the radio.
pub fn set_power(on: bool) -> io::Result<()> {
    allowed(
        WinRadio::RequestAccessAsync()
            .and_then(|access| access.join())
            .map_err(io::Error::other)?,
    )?;
    let adapter = BluetoothAdapter::GetDefaultAsync()
        .and_then(|adapter| adapter.join())
        .map_err(io::Error::other)?;
    let radio = radio(&adapter).ok_or_else(|| io::Error::other("the adapter has no radio"))?;
    allowed(
        radio
            .SetStateAsync(if on { RadioState::On } else { RadioState::Off })
            .and_then(|set| set.join())
            .map_err(io::Error::other)?,
    )
}

/// Pairs the endpoint `id`, asking `ask` what the pairing needs: a yes, or
/// a PIN to confirm or type. `ask` answering `None` refuses. Returns once
/// the pairing has ended.
///
/// # Errors
///
/// When the endpoint cannot be found or the pairing does not end paired.
pub fn pair(
    id: &str,
    ask: impl Fn(Question) -> Option<Reply> + Send + Sync + 'static,
) -> io::Result<()> {
    let custom = DeviceInformation::CreateFromIdAsync(&HSTRING::from(id))
        .and_then(|info| info.join())
        .and_then(|info| info.Pairing())
        .and_then(|pairing| pairing.Custom())
        .map_err(io::Error::other)?;
    custom
        .PairingRequested(&TypedEventHandler::new(
            move |_, args: Ref<DevicePairingRequestedEventArgs>| {
                let args = args.ok()?;
                let kind = match args.PairingKind()? {
                    DevicePairingKinds::ConfirmOnly => Kind::ConfirmOnly,
                    DevicePairingKinds::DisplayPin => Kind::DisplayPin,
                    DevicePairingKinds::ProvidePin => Kind::ProvidePin,
                    DevicePairingKinds::ConfirmPinMatch => Kind::ConfirmPinMatch,
                    // A kind not offered below: left unanswered, it fails.
                    _ => return Ok(()),
                };
                let pin = args.Pin().map(|pin| pin.to_string()).unwrap_or_default();
                let Some(reply) = question(kind, &pin).and_then(&ask) else {
                    return Ok(());
                };
                match accept(&reply) {
                    None => Ok(()),
                    Some(None) => args.Accept(),
                    Some(Some(pin)) => args.AcceptWithPin(&HSTRING::from(pin)),
                }
            },
        ))
        .map_err(io::Error::other)?;
    let kinds = DevicePairingKinds::ConfirmOnly
        | DevicePairingKinds::DisplayPin
        | DevicePairingKinds::ProvidePin
        | DevicePairingKinds::ConfirmPinMatch;
    let status = custom
        .PairAsync(kinds)
        .and_then(|pairing| pairing.join())
        .and_then(|result| result.Status())
        .map_err(io::Error::other)?;
    if status == DevicePairingResultStatus::Paired
        || status == DevicePairingResultStatus::AlreadyPaired
    {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "pairing ended with status {}",
            status.0
        )))
    }
}

/// Unpairs the endpoint `id`.
///
/// # Errors
///
/// When the endpoint cannot be found or stays paired.
pub fn unpair(id: &str) -> io::Result<()> {
    let status = DeviceInformation::CreateFromIdAsync(&HSTRING::from(id))
        .and_then(|info| info.join())
        .and_then(|info| info.Pairing())
        .and_then(|pairing| pairing.UnpairAsync())
        .and_then(|unpairing| unpairing.join())
        .and_then(|result| result.Status())
        .map_err(io::Error::other)?;
    if status == DeviceUnpairingResultStatus::Unpaired
        || status == DeviceUnpairingResultStatus::AlreadyUnpaired
    {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "unpairing ended with status {}",
            status.0
        )))
    }
}

/// A scan for devices not paired: a `DeviceWatcher` on each kind, keeping
/// what they find and calling `on_change` as it changes; stopped when
/// dropped.
pub struct Scan {
    watchers: Vec<DeviceWatcher>,
    found: Arc<Mutex<Vec<Endpoint>>>,
}

impl std::fmt::Debug for Scan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Scan")
            .field("found", &self.found().len())
            .finish_non_exhaustive()
    }
}

impl Scan {
    /// Starts a scan.
    ///
    /// # Errors
    ///
    /// When a watcher cannot be made or started.
    pub fn start(on_change: impl Fn() + Send + Sync + 'static) -> io::Result<Self> {
        let on_change: OnChange = Arc::new(on_change);
        let found: Arc<Mutex<Vec<Endpoint>>> = Arc::default();
        let mut watchers = Vec::new();
        for selector in [
            BluetoothDevice::GetDeviceSelectorFromPairingState(false),
            BluetoothLEDevice::GetDeviceSelectorFromPairingState(false),
        ] {
            let watcher = DeviceInformation::CreateWatcherWithKindAqsFilterAndAdditionalProperties(
                &selector.map_err(io::Error::other)?,
                &properties(),
                DeviceInformationKind::AssociationEndpoint,
            )
            .map_err(io::Error::other)?;
            let (kept, changed) = (Arc::clone(&found), Arc::clone(&on_change));
            watcher
                .Added(&TypedEventHandler::new(
                    move |_, info: Ref<DeviceInformation>| {
                        if let Some(endpoint) = endpoint(info.ok()?, false) {
                            let mut kept = kept.lock().unwrap_or_else(PoisonError::into_inner);
                            kept.retain(|known| known.id != endpoint.id);
                            kept.push(endpoint);
                        }
                        changed();
                        Ok(())
                    },
                ))
                .map_err(io::Error::other)?;
            let changed = Arc::clone(&on_change);
            watcher
                .Updated(&TypedEventHandler::new(move |_, _| {
                    changed();
                    Ok(())
                }))
                .map_err(io::Error::other)?;
            let (kept, changed) = (Arc::clone(&found), Arc::clone(&on_change));
            watcher
                .Removed(&TypedEventHandler::new(
                    move |_, update: Ref<DeviceInformationUpdate>| {
                        let id = update.ok()?.Id()?.to_string();
                        kept.lock()
                            .unwrap_or_else(PoisonError::into_inner)
                            .retain(|known| known.id != id);
                        changed();
                        Ok(())
                    },
                ))
                .map_err(io::Error::other)?;
            watcher.Start().map_err(io::Error::other)?;
            watchers.push(watcher);
        }
        Ok(Self { watchers, found })
    }

    /// What it has found so far.
    #[must_use]
    pub fn found(&self) -> Vec<Endpoint> {
        self.found
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl Drop for Scan {
    fn drop(&mut self) {
        for watcher in &self.watchers {
            // A watcher that already stopped has nothing left to stop.
            let _stopped = watcher.Stop();
        }
    }
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
    use super::{Scan, Tracker, Watch, init, snapshot};

    #[test]
    fn this_machine_s_bluetooth_reads_and_a_watch_starts() {
        init().expect("in the multithreaded apartment");
        let (snapshot, endpoints) = snapshot(&mut Tracker::default(), None);
        // A machine may have no adapter; whatever it has is consistent.
        assert_eq!(snapshot.reading.is_some(), snapshot.adapter.is_some());
        if snapshot.adapter.is_none() {
            assert!(snapshot.devices.is_empty());
            assert!(endpoints.is_empty());
        }
        let scan = Scan::start(|| {}).expect("the scan starts");
        assert!(format!("{scan:?}").contains("found"));
        drop(scan);
        let watch = Watch::new(|| {}).expect("the watchers start");
        assert!(!format!("{watch:?}").is_empty());
    }
}
