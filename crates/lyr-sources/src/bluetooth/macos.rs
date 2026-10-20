//! The bluetooth module's source on macOS: IOBluetooth's default controller
//! and paired devices, read again when the controller powers on or off
//! (`IOBluetoothHostControllerPoweredOnNotification` and
//! `…PoweredOffNotification`) and when a device connects or disconnects
//! (`registerForConnectNotifications:selector:` and each device's
//! `registerForDisconnectNotification:selector:`), so nothing is polled.
//!
//! They are turned into BlueZ's shape ([`Adapter`], [`Device`]) so
//! [`Tracker`] gives the reading on every platform: a device's address
//! stands for its path, and macOS reports no rfkill state, trust or battery
//! here. IOBluetooth delivers its notifications on the run loop of the
//! thread that registered them, so the watch is made, and lives, on the
//! bar's main thread, whose run loop runs.

use std::cell::RefCell;
use std::ffi::{c_int, c_uint};
use std::ptr::NonNull;

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObject, NSObjectProtocol, ProtocolObject};
use objc2::{AnyThread, DefinedClass, define_class, msg_send, sel};
use objc2_foundation::{NSArray, NSNotification, NSNotificationCenter, NSString};
use objc2_io_bluetooth::{
    BluetoothHCIPowerState, IOBluetoothDevice, IOBluetoothHostController,
    IOBluetoothHostControllerPoweredOffNotification,
    IOBluetoothHostControllerPoweredOnNotification, IOBluetoothUserNotification,
};

use super::{Adapter, Device, Snapshot, Tracker};

/// The adapter's path, which nothing else names on macOS.
const DEFAULT_ADAPTER: &str = "default";

/// The default controller, or `None` on a Mac without Bluetooth.
fn adapter() -> Option<Adapter> {
    // SAFETY: plain Objective-C calls with no preconditions.
    let controller = unsafe { IOBluetoothHostController::defaultController() }?;
    // `objc2-io-bluetooth` 0.3.2 binds `powerState` as returning an unsigned
    // `BluetoothHCIPowerState`, but the runtime declares it `int` (seen on
    // CI's Mac, where objc2's check refused the send), so it is sent here
    // with the type the runtime has.
    // SAFETY: `powerState` takes no arguments and returns an `int`.
    let state: c_int = unsafe { msg_send![&*controller, powerState] };
    // SAFETY: as above.
    let address = unsafe { controller.addressAsString() };
    let powered = c_uint::try_from(state).is_ok_and(|state| state == BluetoothHCIPowerState::ON.0);
    Some(Adapter {
        path: address.map_or_else(|| DEFAULT_ADAPTER.to_owned(), |address| address.to_string()),
        powered,
        discoverable: false,
        pairable: false,
        discovering: false,
        power_state: None,
    })
}

/// The paired devices.
fn paired() -> Vec<Retained<IOBluetoothDevice>> {
    // SAFETY: a plain class method; the array holds `IOBluetoothDevice`s.
    let Some(devices) = (unsafe { IOBluetoothDevice::pairedDevices() }) else {
        return Vec::new();
    };
    // SAFETY: as above.
    let devices: Retained<NSArray<IOBluetoothDevice>> =
        unsafe { Retained::cast_unchecked(devices) };
    devices.to_vec()
}

fn device(device: &IOBluetoothDevice) -> Device {
    // SAFETY: plain Objective-C calls with no preconditions.
    unsafe {
        Device {
            path: device
                .addressString()
                .map(|address| address.to_string())
                .unwrap_or_default(),
            alias: device
                .nameOrAddress()
                .map(|name| name.to_string())
                .unwrap_or_default(),
            connected: device.isConnected(),
            paired: true,
            trusted: false,
            battery: None,
        }
    }
}

/// What IOBluetooth reports now, with `tracker` remembering the order
/// devices connect in.
#[must_use]
pub fn snapshot(tracker: &mut Tracker) -> Snapshot {
    let adapter = adapter();
    let devices: Vec<Device> = paired().iter().map(|paired| device(paired)).collect();
    Snapshot {
        reading: tracker.reading(adapter.as_ref(), &devices),
        adapter,
        devices,
    }
}

/// What the observer keeps: the callback, and each device's disconnect
/// registration, to undo when the watch goes.
struct Ivars {
    on_change: Box<dyn Fn()>,
    disconnects: RefCell<Vec<Retained<IOBluetoothUserNotification>>>,
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements, and `Observer` does
    // not implement `Drop`.
    #[unsafe(super(NSObject))]
    #[name = "LyrBluetoothObserver"]
    #[ivars = Ivars]
    struct Observer;

    impl Observer {
        /// A device connected: follow its disconnect, and read again.
        #[unsafe(method(connected:fromDevice:))]
        fn connected(&self, _notification: &IOBluetoothUserNotification, device: &IOBluetoothDevice) {
            self.follow(device);
            (self.ivars().on_change)();
        }

        /// A device disconnected: read again.
        #[unsafe(method(disconnected:fromDevice:))]
        fn disconnected(
            &self,
            _notification: &IOBluetoothUserNotification,
            _device: &IOBluetoothDevice,
        ) {
            (self.ivars().on_change)();
        }
    }
);

impl Observer {
    fn new(on_change: Box<dyn Fn()>) -> Retained<Self> {
        let this = Self::alloc().set_ivars(Ivars {
            on_change,
            disconnects: RefCell::new(Vec::new()),
        });
        // SAFETY: NSObject's `init`, on an object just allocated.
        unsafe { msg_send![super(this), init] }
    }

    /// Calls `disconnected:fromDevice:` when `device` disconnects.
    fn follow(&self, device: &IOBluetoothDevice) {
        let observer: &AnyObject = self.as_ref();
        // SAFETY: the selector is this class's own, taking the notification
        // and the device as IOBluetooth passes them.
        let registration = unsafe {
            device.registerForDisconnectNotification_selector(
                Some(observer),
                Some(sel!(disconnected:fromDevice:)),
            )
        };
        if let Some(registration) = registration {
            self.ivars().disconnects.borrow_mut().push(registration);
        }
    }
}

/// The registrations calling a callback, on the thread that made them.
pub struct Watch {
    observer: Retained<Observer>,
    connects: Option<Retained<IOBluetoothUserNotification>>,
    power: Vec<Retained<ProtocolObject<dyn NSObjectProtocol>>>,
}

impl std::fmt::Debug for Watch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Watch").finish_non_exhaustive()
    }
}

impl Watch {
    /// Calls `on_change` when the controller powers on or off or a device
    /// connects or disconnects, on this thread's run loop, until dropped.
    pub fn new(on_change: impl Fn() + 'static) -> Self {
        let on_change: std::rc::Rc<dyn Fn()> = std::rc::Rc::new(on_change);
        let observer = Observer::new(Box::new({
            let on_change = on_change.clone();
            move || on_change()
        }));
        let target: &AnyObject = observer.as_ref();
        // SAFETY: the selector is `Observer`'s own, taking the notification
        // and the device as IOBluetooth passes them.
        let connects = unsafe {
            IOBluetoothDevice::registerForConnectNotifications_selector(
                Some(target),
                Some(sel!(connected:fromDevice:)),
            )
        };
        for device in paired() {
            // SAFETY: a plain Objective-C call.
            if unsafe { device.isConnected() } {
                observer.follow(&device);
            }
        }
        // SAFETY: the names are IOBluetooth's statics.
        let names = unsafe {
            [
                IOBluetoothHostControllerPoweredOnNotification,
                IOBluetoothHostControllerPoweredOffNotification,
            ]
        };
        let center = NSNotificationCenter::defaultCenter();
        let power = names
            .into_iter()
            .flatten()
            .map(|name: &NSString| {
                let on_change = on_change.clone();
                let block = RcBlock::new(move |_: NonNull<NSNotification>| on_change());
                // SAFETY: no object and no queue, so the block runs where
                // the notification is posted: the thread whose run loop
                // IOBluetooth delivers on, which made this watch.
                unsafe {
                    center.addObserverForName_object_queue_usingBlock(
                        Some(name),
                        None,
                        None,
                        &block,
                    )
                }
            })
            .collect();
        Self {
            observer,
            connects,
            power,
        }
    }
}

impl Drop for Watch {
    fn drop(&mut self) {
        // SAFETY: each registration is undone once, on the thread that made
        // it.
        unsafe {
            if let Some(connects) = &self.connects {
                connects.unregister();
            }
            for disconnect in self.observer.ivars().disconnects.borrow_mut().drain(..) {
                disconnect.unregister();
            }
            let center = NSNotificationCenter::defaultCenter();
            for observer in &self.power {
                let observer: &AnyObject = (**observer).as_ref();
                center.removeObserver(observer);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Tracker, Watch, snapshot};

    #[test]
    fn the_controller_reads_and_a_watch_starts_and_stops() {
        // CI's Macs have no Bluetooth controller; a laptop has one.
        let snapshot = snapshot(&mut Tracker::default());
        if snapshot.adapter.is_none() {
            assert_eq!(snapshot.reading, None, "no controller hides the module");
        }
        drop(Watch::new(|| {}));
    }
}
