use std::os::unix::net::UnixStream;
use std::time::Duration;

use lyr_core::battery::{Reading, Status, TimeTo};
use tokio::sync::watch;
use zbus::connection::Builder;
use zbus::object_server::InterfaceRef;
use zbus::{Connection, Guid};

use super::{DISPLAY_DEVICE, UPower};

/// A stand-in for UPower's display device, with the properties lyr reads.
struct FakeDevice {
    state: u32,
    percentage: f64,
}

#[zbus::interface(name = "org.freedesktop.UPower.Device")]
impl FakeDevice {
    #[zbus(property, name = "Type")]
    fn kind(&self) -> u32 {
        2
    }
    #[zbus(property)]
    fn is_present(&self) -> bool {
        true
    }
    #[zbus(property)]
    fn state(&self) -> u32 {
        self.state
    }
    #[zbus(property)]
    fn percentage(&self) -> f64 {
        self.percentage
    }
    #[zbus(property)]
    fn energy_rate(&self) -> f64 {
        9.0
    }
    #[zbus(property)]
    fn time_to_empty(&self) -> i64 {
        7200
    }
    #[zbus(property)]
    fn time_to_full(&self) -> i64 {
        0
    }
}

/// A connected pair: the fake UPower's side and lyr's side.
async fn fake_upower(device: FakeDevice) -> (Connection, Connection) {
    let (server, client) = UnixStream::pair().expect("socket pair");
    let guid = Guid::generate();
    let server = Builder::unix_stream(server)
        .server(guid)
        .p2p()
        .serve_at(DISPLAY_DEVICE, device)
        .build();
    let client = Builder::unix_stream(client).p2p().build();
    let (server, client) = tokio::try_join!(server, client).expect("connected");
    (server, client)
}

/// Waits for the next reading, failing the test rather than hanging.
async fn next(readings: &mut watch::Receiver<Option<Reading>>) -> Option<Reading> {
    tokio::time::timeout(Duration::from_secs(5), readings.changed())
        .await
        .expect("a reading within 5 s")
        .expect("the source is running");
    *readings.borrow_and_update()
}

#[tokio::test]
async fn the_reading_follows_upowers_changes() {
    let (server, client) = fake_upower(FakeDevice {
        state: 2,
        percentage: 64.0,
    })
    .await;
    let upower = UPower::on(&client).await.expect("proxies");
    let (sender, mut readings) = watch::channel(None);
    let source = tokio::spawn(async move { upower.watch(&sender).await });

    let first = next(&mut readings).await.expect("a battery");
    assert_eq!(first.capacity, 64);
    assert_eq!(first.status, Status::Discharging);
    assert_eq!(first.time_to, TimeTo::Empty(2.0));
    assert_eq!(first.power, Some(9.0));

    let device: InterfaceRef<FakeDevice> = server
        .object_server()
        .interface(DISPLAY_DEVICE)
        .await
        .expect("served");
    {
        let mut fake = device.get_mut().await;
        fake.state = 1;
        fake.percentage = 65.0;
        fake.percentage_changed(device.signal_emitter())
            .await
            .expect("signalled");
    }
    let charging = next(&mut readings).await.expect("a battery");
    assert_eq!(charging.capacity, 65);
    assert_eq!(charging.status, Status::Charging);

    source.abort();
}
