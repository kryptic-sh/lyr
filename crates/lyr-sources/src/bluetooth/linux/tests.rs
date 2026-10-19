use std::os::unix::net::UnixStream;
use std::time::Duration;

use lyr_core::bluetooth::{Reading, State};
use tokio::sync::watch;
use zbus::connection::Builder;
use zbus::fdo::ObjectManager;
use zbus::object_server::InterfaceRef;
use zbus::{Connection, Guid};

use super::BlueZ;
use crate::bluetooth::Snapshot;

const ADAPTER_PATH: &str = "/org/bluez/hci0";
const DEVICE_PATH: &str = "/org/bluez/hci0/dev_AA_BB_CC_DD_EE_FF";

/// A stand-in for BlueZ's adapter.
struct FakeAdapter;

#[zbus::interface(name = "org.bluez.Adapter1")]
impl FakeAdapter {
    #[zbus(property)]
    fn powered(&self) -> bool {
        true
    }
    #[zbus(property)]
    fn discovering(&self) -> bool {
        true
    }
    #[zbus(property)]
    fn pairable(&self) -> bool {
        true
    }
}

/// A stand-in for one paired device.
struct FakeDevice {
    connected: bool,
}

#[zbus::interface(name = "org.bluez.Device1")]
impl FakeDevice {
    #[zbus(property)]
    fn address(&self) -> String {
        "AA:BB:CC:DD:EE:FF".to_owned()
    }
    #[zbus(property)]
    fn alias(&self) -> String {
        "Buds".to_owned()
    }
    #[zbus(property)]
    fn connected(&self) -> bool {
        self.connected
    }
    #[zbus(property)]
    fn paired(&self) -> bool {
        true
    }
}

/// The device's battery.
struct FakeBattery;

#[zbus::interface(name = "org.bluez.Battery1")]
impl FakeBattery {
    #[zbus(property)]
    fn percentage(&self) -> u8 {
        70
    }
}

async fn fake_bluez() -> (Connection, Connection) {
    let (server, client) = UnixStream::pair().expect("socket pair");
    let server = Builder::unix_stream(server)
        .server(Guid::generate())
        .p2p()
        .serve_at("/", ObjectManager)
        .serve_at(ADAPTER_PATH, FakeAdapter)
        .serve_at(DEVICE_PATH, FakeDevice { connected: false })
        .serve_at(DEVICE_PATH, FakeBattery)
        .build();
    let client = Builder::unix_stream(client).p2p().build();
    tokio::try_join!(server, client).expect("connected")
}

async fn next(readings: &mut watch::Receiver<Snapshot>) -> Option<Reading> {
    tokio::time::timeout(Duration::from_secs(5), readings.changed())
        .await
        .expect("a reading within 5 s")
        .expect("the source is running");
    readings.borrow_and_update().reading.clone()
}

#[tokio::test]
async fn a_device_connecting_comes_through_with_its_battery() {
    let (server, client) = fake_bluez().await;
    let bluez = BlueZ::on(&client).await.expect("proxy");
    let (sender, mut readings) = watch::channel(Snapshot::default());
    let source = tokio::spawn(async move { bluez.watch(&sender).await });

    let first = next(&mut readings).await.expect("an adapter");
    assert_eq!(first.state, State::On);
    assert_eq!(first.device, None);

    let device: InterfaceRef<FakeDevice> = server
        .object_server()
        .interface(DEVICE_PATH)
        .await
        .expect("served");
    {
        let mut fake = device.get_mut().await;
        fake.connected = true;
        fake.connected_changed(device.signal_emitter())
            .await
            .expect("signalled");
    }
    let connected = next(&mut readings).await.expect("an adapter");
    assert_eq!(connected.state, State::Connected);
    assert_eq!(connected.connections, 1);
    let shown = connected.device.expect("the device shown");
    assert_eq!(shown.alias, "Buds");
    assert_eq!(shown.battery, Some(70));
    source.abort();
}

type Calls = std::sync::Arc<std::sync::Mutex<Vec<String>>>;

/// An adapter that records what the menu asks of it.
struct RecordingAdapter {
    calls: Calls,
}

#[zbus::interface(name = "org.bluez.Adapter1")]
impl RecordingAdapter {
    fn start_discovery(&self) {
        self.calls
            .lock()
            .expect("calls")
            .push("StartDiscovery".to_owned());
    }
    fn stop_discovery(&self) {
        self.calls
            .lock()
            .expect("calls")
            .push("StopDiscovery".to_owned());
    }
    fn remove_device(&self, device: zbus::ObjectPath<'_>) {
        let mut calls = self.calls.lock().expect("calls");
        calls.push(format!("RemoveDevice({device})"));
    }
    #[zbus(property)]
    fn powered(&self) -> bool {
        true
    }
    #[zbus(property)]
    fn set_powered(&mut self, on: bool) {
        self.calls
            .lock()
            .expect("calls")
            .push(format!("Powered={on}"));
    }
    #[zbus(property)]
    fn discoverable(&self) -> bool {
        false
    }
    #[zbus(property)]
    fn set_discoverable(&mut self, on: bool) {
        let mut calls = self.calls.lock().expect("calls");
        calls.push(format!("Discoverable={on}"));
    }
    #[zbus(property)]
    fn pairable(&self) -> bool {
        false
    }
    #[zbus(property)]
    fn set_pairable(&mut self, on: bool) {
        self.calls
            .lock()
            .expect("calls")
            .push(format!("Pairable={on}"));
    }
}

/// A device that records what the menu asks of it.
struct RecordingDevice {
    calls: Calls,
}

#[zbus::interface(name = "org.bluez.Device1")]
impl RecordingDevice {
    fn connect(&self) {
        self.calls.lock().expect("calls").push("Connect".to_owned());
    }
    fn disconnect(&self) {
        self.calls
            .lock()
            .expect("calls")
            .push("Disconnect".to_owned());
    }
    fn pair(&self) {
        self.calls.lock().expect("calls").push("Pair".to_owned());
    }
    #[zbus(property)]
    fn trusted(&self) -> bool {
        false
    }
    #[zbus(property)]
    fn set_trusted(&mut self, on: bool) {
        self.calls
            .lock()
            .expect("calls")
            .push(format!("Trusted={on}"));
    }
}

#[tokio::test]
async fn the_menu_s_actions_reach_the_adapter_and_the_device() {
    use crate::bluetooth::{DeviceAction, Switch};
    let calls = Calls::default();
    let (server, client) = UnixStream::pair().expect("socket pair");
    let server = Builder::unix_stream(server)
        .server(Guid::generate())
        .p2p()
        .serve_at(
            ADAPTER_PATH,
            RecordingAdapter {
                calls: calls.clone(),
            },
        )
        .serve_at(
            DEVICE_PATH,
            RecordingDevice {
                calls: calls.clone(),
            },
        )
        .build();
    let client = Builder::unix_stream(client).p2p().build();
    let (_server, client) = tokio::try_join!(server, client).expect("connected");
    let bluez = BlueZ::on(&client).await.expect("bluez");
    for (switch, on) in [
        (Switch::Powered, false),
        (Switch::Discoverable, true),
        (Switch::Pairable, true),
    ] {
        bluez.set(ADAPTER_PATH, switch, on).await.expect("set");
    }
    bluez.scan(ADAPTER_PATH, true).await.expect("scanning");
    bluez.scan(ADAPTER_PATH, false).await.expect("not scanning");
    for action in [
        DeviceAction::Connect,
        DeviceAction::Disconnect,
        DeviceAction::Pair,
        DeviceAction::Trust,
        DeviceAction::Untrust,
    ] {
        bluez.device(DEVICE_PATH, action).await.expect("done");
    }
    bluez
        .forget(ADAPTER_PATH, DEVICE_PATH)
        .await
        .expect("forgotten");
    assert_eq!(
        *calls.lock().expect("calls"),
        [
            "Powered=false",
            "Discoverable=true",
            "Pairable=true",
            "StartDiscovery",
            "StopDiscovery",
            "Connect",
            "Disconnect",
            "Pair",
            "Trusted=true",
            "Trusted=false",
            &format!("RemoveDevice({DEVICE_PATH})"),
        ]
    );
}

#[tokio::test]
async fn the_menu_s_state_is_read_with_the_rest() {
    let (_server, client) = fake_bluez().await;
    let bluez = BlueZ::on(&client).await.expect("bluez");
    let (adapter, devices) = bluez.read().await.expect("read");
    let adapter = adapter.expect("an adapter");
    assert_eq!(adapter.path, ADAPTER_PATH);
    assert_eq!(
        (adapter.discoverable, adapter.pairable, adapter.discovering),
        (false, true, true),
        "an unsent property is off"
    );
    let [device] = &devices[..] else {
        panic!("one device: {devices:?}");
    };
    assert_eq!((device.paired, device.trusted), (true, false));
}
