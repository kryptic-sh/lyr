use std::collections::HashMap;
use std::os::unix::net::UnixStream;
use std::time::Duration;

use lyr_core::network::{Reading, State};
use tokio::sync::watch;
use zbus::connection::Builder;
use zbus::object_server::InterfaceRef;
use zbus::{Connection, Guid, OwnedObjectPath, OwnedVariant};

use super::NetworkManager;

const ROOT: &str = "/org/freedesktop/NetworkManager";
const ACTIVE: &str = "/org/freedesktop/NetworkManager/ActiveConnection/1";
const DEVICE: &str = "/org/freedesktop/NetworkManager/Devices/3";
const CONFIG: &str = "/org/freedesktop/NetworkManager/IP4Config/5";
const POINT: &str = "/org/freedesktop/NetworkManager/AccessPoint/7";
/// An access point the reading does not use.
const OTHER_POINT: &str = "/org/freedesktop/NetworkManager/AccessPoint/8";

fn path(path: &str) -> OwnedObjectPath {
    OwnedObjectPath::try_from(path).expect("a valid path")
}

/// A stand-in for NetworkManager's root object.
struct FakeManager {
    primary: &'static str,
    activating: &'static str,
    wireless_enabled: bool,
    wireless_hardware_enabled: bool,
}

#[zbus::interface(name = "org.freedesktop.NetworkManager")]
impl FakeManager {
    #[zbus(property)]
    fn primary_connection(&self) -> OwnedObjectPath {
        path(self.primary)
    }
    #[zbus(property)]
    fn activating_connection(&self) -> OwnedObjectPath {
        path(self.activating)
    }
    #[zbus(property)]
    fn wireless_enabled(&self) -> bool {
        self.wireless_enabled
    }
    #[zbus(property)]
    fn wireless_hardware_enabled(&self) -> bool {
        self.wireless_hardware_enabled
    }
}

struct FakeActive;

#[zbus::interface(name = "org.freedesktop.NetworkManager.Connection.Active")]
impl FakeActive {
    #[zbus(property)]
    fn devices(&self) -> Vec<OwnedObjectPath> {
        vec![path(DEVICE)]
    }
}

/// A Wi-Fi device, with an address or still waiting for one.
struct FakeDevice {
    config: &'static str,
}

#[zbus::interface(name = "org.freedesktop.NetworkManager.Device")]
impl FakeDevice {
    #[zbus(property)]
    fn interface(&self) -> String {
        "wlan0".to_owned()
    }
    #[zbus(property)]
    fn device_type(&self) -> u32 {
        2
    }
    #[zbus(property)]
    fn ip4_config(&self) -> OwnedObjectPath {
        path(self.config)
    }
}

struct FakeWireless;

#[zbus::interface(name = "org.freedesktop.NetworkManager.Device.Wireless")]
impl FakeWireless {
    #[zbus(property)]
    fn active_access_point(&self) -> OwnedObjectPath {
        path(POINT)
    }
}

struct FakeConfig {
    gateway: &'static str,
}

#[zbus::interface(name = "org.freedesktop.NetworkManager.IP4Config")]
impl FakeConfig {
    #[zbus(property)]
    fn address_data(&self) -> Vec<HashMap<String, OwnedVariant>> {
        vec![HashMap::from([
            (
                "address".to_owned(),
                OwnedVariant::from(zbus::Str::from("192.168.1.20")),
            ),
            ("prefix".to_owned(), OwnedVariant::from(24_u32)),
        ])]
    }
    #[zbus(property)]
    fn gateway(&self) -> String {
        self.gateway.to_owned()
    }
}

struct FakePoint {
    ssid: &'static str,
    strength: u8,
}

#[zbus::interface(name = "org.freedesktop.NetworkManager.AccessPoint")]
impl FakePoint {
    #[zbus(property)]
    fn ssid(&self) -> Vec<u8> {
        self.ssid.as_bytes().to_vec()
    }
    #[zbus(property)]
    fn strength(&self) -> u8 {
        self.strength
    }
}

/// NetworkManager with `manager` at its root and one Wi-Fi device whose
/// IPv4 configuration is at `config`.
async fn fake(manager: FakeManager, config: &'static str) -> (Connection, Connection) {
    let (server, client) = UnixStream::pair().expect("socket pair");
    let server = Builder::unix_stream(server)
        .server(Guid::generate())
        .p2p()
        .serve_at(ROOT, manager)
        .serve_at(ACTIVE, FakeActive)
        .serve_at(DEVICE, FakeDevice { config })
        .serve_at(DEVICE, FakeWireless)
        .serve_at(
            CONFIG,
            FakeConfig {
                gateway: "192.168.1.1",
            },
        )
        .serve_at(
            POINT,
            FakePoint {
                ssid: "home",
                strength: 71,
            },
        )
        .serve_at(
            OTHER_POINT,
            FakePoint {
                ssid: "next door",
                strength: 30,
            },
        )
        .build();
    let client = Builder::unix_stream(client).p2p().build();
    tokio::try_join!(server, client).expect("connected")
}

fn up() -> FakeManager {
    FakeManager {
        primary: ACTIVE,
        activating: "/",
        wireless_enabled: true,
        wireless_hardware_enabled: true,
    }
}

async fn read(manager: FakeManager, config: &'static str) -> Reading {
    let (_server, client) = fake(manager, config).await;
    NetworkManager::on(&client)
        .await
        .expect("proxy")
        .read()
        .await
        .expect("read")
        .0
}

#[tokio::test]
async fn the_primary_connection_s_wifi_device_is_read() {
    let reading = read(up(), CONFIG).await;
    assert_eq!(reading.state(), State::Wifi);
    let link = reading.link.expect("a link");
    assert_eq!(link.ifname, "wlan0");
    assert_eq!(link.ipaddr, "192.168.1.20");
    assert_eq!(link.cidr, "24");
    assert_eq!(link.gwaddr, "192.168.1.1");
    assert_eq!(link.essid, "home");
    assert_eq!(link.signal_strength, 71);
}

#[tokio::test]
async fn a_connection_still_activating_is_linked() {
    let manager = FakeManager {
        primary: "/",
        activating: ACTIVE,
        ..up()
    };
    let reading = read(manager, "/").await;
    assert_eq!(reading.state(), State::Linked);
    assert_eq!(reading.link.expect("a link").ifname, "wlan0");
}

#[tokio::test]
async fn no_connection_is_disconnected_or_disabled() {
    let down = || FakeManager {
        primary: "/",
        activating: "/",
        ..up()
    };
    assert_eq!(read(down(), CONFIG).await.state(), State::Disconnected);
    let blocked = FakeManager {
        wireless_enabled: false,
        ..down()
    };
    assert_eq!(read(blocked, CONFIG).await.state(), State::Disabled);
    let switched_off = FakeManager {
        wireless_hardware_enabled: false,
        ..down()
    };
    assert_eq!(read(switched_off, CONFIG).await.state(), State::Disabled);
}

async fn next(readings: &mut watch::Receiver<Option<Reading>>) -> Option<Reading> {
    tokio::time::timeout(Duration::from_secs(5), readings.changed())
        .await
        .expect("a reading within 5 s")
        .expect("the source is running");
    readings.borrow_and_update().clone()
}

#[tokio::test]
async fn the_signal_follows_its_access_point_only() {
    let (server, client) = fake(up(), CONFIG).await;
    let network = NetworkManager::on(&client).await.expect("proxy");
    let (sender, mut readings) = watch::channel(None);
    let source = tokio::spawn(async move { network.watch(&sender).await });
    let strength =
        |reading: Option<Reading>| reading.and_then(|r| r.link).map(|l| l.signal_strength);
    assert_eq!(strength(next(&mut readings).await), Some(71));

    let point = async |at: &str| -> InterfaceRef<FakePoint> {
        server.object_server().interface(at).await.expect("served")
    };
    let used = point(POINT).await;
    let other = point(OTHER_POINT).await;
    // A change it does not follow reads nothing, so a quiet change to the
    // used access point stays unseen.
    used.get_mut().await.strength = 50;
    {
        let mut fake = other.get_mut().await;
        fake.strength = 31;
        fake.strength_changed(other.signal_emitter())
            .await
            .expect("signalled");
    }
    assert!(
        tokio::time::timeout(Duration::from_millis(300), readings.changed())
            .await
            .is_err(),
        "another access point's signal is ignored"
    );

    {
        let fake = used.get().await;
        fake.strength_changed(used.signal_emitter())
            .await
            .expect("signalled");
    }
    assert_eq!(strength(next(&mut readings).await), Some(50));
    source.abort();
}

#[tokio::test]
async fn a_new_connection_s_objects_are_followed() {
    let manager = FakeManager {
        primary: "/",
        ..up()
    };
    let (server, client) = fake(manager, CONFIG).await;
    let network = NetworkManager::on(&client).await.expect("proxy");
    let (sender, mut readings) = watch::channel(None);
    let source = tokio::spawn(async move { network.watch(&sender).await });
    let strength =
        |reading: Option<Reading>| reading.and_then(|r| r.link).map(|l| l.signal_strength);
    assert_eq!(strength(next(&mut readings).await), None);

    let root: InterfaceRef<FakeManager> = server
        .object_server()
        .interface(ROOT)
        .await
        .expect("served");
    {
        let mut fake = root.get_mut().await;
        fake.primary = ACTIVE;
        fake.primary_connection_changed(root.signal_emitter())
            .await
            .expect("signalled");
    }
    assert_eq!(strength(next(&mut readings).await), Some(71));

    let point: InterfaceRef<FakePoint> = server
        .object_server()
        .interface(POINT)
        .await
        .expect("served");
    {
        let mut fake = point.get_mut().await;
        fake.strength = 40;
        fake.strength_changed(point.signal_emitter())
            .await
            .expect("signalled");
    }
    assert_eq!(strength(next(&mut readings).await), Some(40));

    let config: InterfaceRef<FakeConfig> = server
        .object_server()
        .interface(CONFIG)
        .await
        .expect("served");
    {
        let mut fake = config.get_mut().await;
        fake.gateway = "192.168.1.254";
        fake.gateway_changed(config.signal_emitter())
            .await
            .expect("signalled");
    }
    let gateway = next(&mut readings)
        .await
        .and_then(|r| r.link)
        .map(|l| l.gwaddr);
    assert_eq!(gateway.as_deref(), Some("192.168.1.254"));
    source.abort();
}
