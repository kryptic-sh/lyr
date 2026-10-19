use std::os::unix::net::UnixStream;
use std::time::Duration;

use lyr_core::bluetooth::{Reading, State};
use tokio::sync::watch;
use zbus::connection::Builder;
use zbus::fdo::ObjectManager;
use zbus::object_server::InterfaceRef;
use zbus::{Connection, Guid};

use super::BlueZ;

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

async fn next(readings: &mut watch::Receiver<Option<Reading>>) -> Option<Reading> {
    tokio::time::timeout(Duration::from_secs(5), readings.changed())
        .await
        .expect("a reading within 5 s")
        .expect("the source is running");
    readings.borrow_and_update().clone()
}

#[tokio::test]
async fn a_device_connecting_comes_through_with_its_battery() {
    let (server, client) = fake_bluez().await;
    let bluez = BlueZ::on(&client).await.expect("proxy");
    let (sender, mut readings) = watch::channel(None);
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
