use std::collections::HashMap;
use std::os::unix::net::UnixStream;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use lyr_core::wifi_menu::{Security, Wifi};
use tokio::sync::watch;
use zbus::connection::Builder;
use zbus::object_server::{InterfaceRef, SignalEmitter};
use zbus::{Connection, Guid, ObjectPath, OwnedObjectPath, OwnedVariant};

use super::super::NetworkManager;
use super::since_boot;
use crate::network::{Failed, Join};

const ROOT: &str = "/org/freedesktop/NetworkManager";
const SETTINGS: &str = "/org/freedesktop/NetworkManager/Settings";
const WIRED: &str = "/org/freedesktop/NetworkManager/Devices/1";
const DEVICE: &str = "/org/freedesktop/NetworkManager/Devices/2";
const HOME: &str = "/org/freedesktop/NetworkManager/AccessPoint/1";
const CAFE: &str = "/org/freedesktop/NetworkManager/AccessPoint/2";
const HOME_PROFILE: &str = "/org/freedesktop/NetworkManager/Settings/1";
const HOTSPOT_PROFILE: &str = "/org/freedesktop/NetworkManager/Settings/2";
const WIRED_PROFILE: &str = "/org/freedesktop/NetworkManager/Settings/3";
/// The profile `AddAndActivateConnection` makes.
const NEW_PROFILE: &str = "/org/freedesktop/NetworkManager/Settings/9";
/// A profile and an access point listed but not served: gone before they
/// are read.
const GONE_PROFILE: &str = "/org/freedesktop/NetworkManager/Settings/5";
const GONE_POINT: &str = "/org/freedesktop/NetworkManager/AccessPoint/9";

const ACTIVATED: u32 = 100;
const FAILED: u32 = 120;
const NO_SECRETS: u32 = 7;

fn path(path: &str) -> OwnedObjectPath {
    OwnedObjectPath::try_from(path).expect("a valid path")
}

/// What the fake was asked, shared with the test.
#[derive(Default)]
struct Asked {
    calls: Vec<String>,
    /// The settings `AddAndActivateConnection` was given.
    added: Option<HashMap<String, HashMap<String, OwnedVariant>>>,
    /// The settings `Update` was given.
    updated: Option<HashMap<String, HashMap<String, OwnedVariant>>>,
    /// The saved profiles still there.
    profiles: Vec<&'static str>,
    /// Whether the lists name a profile and an access point that are gone.
    gone: bool,
}

type Shared = Arc<Mutex<Asked>>;

fn asked(shared: &Shared) -> std::sync::MutexGuard<'_, Asked> {
    shared.lock().unwrap_or_else(PoisonError::into_inner)
}

/// NetworkManager's root: a join ends in `outcome`, a state and a reason.
struct FakeManager {
    shared: Shared,
    wireless_enabled: bool,
    outcome: (u32, u32),
}

impl FakeManager {
    async fn finish(&self, connection: &Connection) -> zbus::fdo::Result<()> {
        let emitter = SignalEmitter::new(connection, DEVICE)?;
        let (state, reason) = self.outcome;
        FakeDevice::device_state_changed(&emitter, 40, 30, 0).await?;
        FakeDevice::device_state_changed(&emitter, state, 40, reason).await?;
        Ok(())
    }
}

#[zbus::interface(name = "org.freedesktop.NetworkManager")]
impl FakeManager {
    fn get_devices(&self) -> Vec<OwnedObjectPath> {
        vec![path(WIRED), path(DEVICE)]
    }

    async fn activate_connection(
        &self,
        connection: ObjectPath<'_>,
        device: ObjectPath<'_>,
        specific_object: ObjectPath<'_>,
        #[zbus(connection)] bus: &Connection,
    ) -> zbus::fdo::Result<OwnedObjectPath> {
        asked(&self.shared)
            .calls
            .push(format!("activate {connection} {device} {specific_object}"));
        self.finish(bus).await?;
        Ok(path("/org/freedesktop/NetworkManager/ActiveConnection/1"))
    }

    async fn add_and_activate_connection(
        &self,
        connection: HashMap<String, HashMap<String, OwnedVariant>>,
        device: ObjectPath<'_>,
        specific_object: ObjectPath<'_>,
        #[zbus(connection)] bus: &Connection,
    ) -> zbus::fdo::Result<(OwnedObjectPath, OwnedObjectPath)> {
        {
            let mut asked = asked(&self.shared);
            asked.calls.push(format!("add {device} {specific_object}"));
            asked.added = Some(connection);
        }
        self.finish(bus).await?;
        Ok((
            path(NEW_PROFILE),
            path("/org/freedesktop/NetworkManager/ActiveConnection/1"),
        ))
    }

    #[zbus(property)]
    fn wireless_enabled(&self) -> bool {
        self.wireless_enabled
    }

    /// zbus signals the change itself.
    #[zbus(property)]
    fn set_wireless_enabled(&mut self, on: bool) {
        self.wireless_enabled = on;
    }

    #[zbus(property)]
    fn wireless_hardware_enabled(&self) -> bool {
        true
    }
}

/// A device of `kind` (`NM_DEVICE_TYPE_*`) in `state`.
struct FakeDevice {
    shared: Shared,
    kind: u32,
    state: u32,
}

#[zbus::interface(name = "org.freedesktop.NetworkManager.Device")]
impl FakeDevice {
    #[zbus(property)]
    fn device_type(&self) -> u32 {
        self.kind
    }
    #[zbus(property)]
    fn state(&self) -> u32 {
        self.state
    }
    fn disconnect(&self) {
        asked(&self.shared)
            .calls
            .push(format!("disconnect {}", self.kind));
    }

    #[zbus(signal, name = "StateChanged")]
    async fn device_state_changed(
        emitter: &SignalEmitter<'_>,
        new_state: u32,
        old_state: u32,
        reason: u32,
    ) -> zbus::Result<()>;
}

struct FakeWireless {
    shared: Shared,
}

#[zbus::interface(name = "org.freedesktop.NetworkManager.Device.Wireless")]
impl FakeWireless {
    fn get_all_access_points(&self) -> Vec<OwnedObjectPath> {
        let mut points = vec![path(HOME), path(CAFE)];
        if asked(&self.shared).gone {
            points.push(path(GONE_POINT));
        }
        points
    }
    fn request_scan(&self, _options: HashMap<String, OwnedVariant>) {
        asked(&self.shared).calls.push("scan".to_owned());
    }
    #[zbus(property)]
    fn last_scan(&self) -> i64 {
        12_345
    }
    #[zbus(property)]
    fn active_access_point(&self) -> OwnedObjectPath {
        path(HOME)
    }
}

struct FakePoint {
    ssid: &'static str,
    strength: u8,
    rsn_flags: u32,
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
    #[zbus(property)]
    fn flags(&self) -> u32 {
        u32::from(self.rsn_flags != 0)
    }
    #[zbus(property)]
    fn wpa_flags(&self) -> u32 {
        0
    }
    #[zbus(property)]
    fn rsn_flags(&self) -> u32 {
        self.rsn_flags
    }
}

struct FakeSettings {
    shared: Shared,
}

#[zbus::interface(name = "org.freedesktop.NetworkManager.Settings")]
impl FakeSettings {
    fn list_connections(&self) -> Vec<OwnedObjectPath> {
        let asked = asked(&self.shared);
        let mut profiles: Vec<_> = asked.profiles.iter().map(|profile| path(profile)).collect();
        if asked.gone {
            profiles.push(path(GONE_PROFILE));
        }
        profiles
    }

    #[zbus(signal)]
    async fn new_connection(
        emitter: &SignalEmitter<'_>,
        connection: ObjectPath<'_>,
    ) -> zbus::Result<()>;
}

/// A saved profile of `kind`, joining `ssid` in `mode` when it is Wi-Fi.
struct FakeProfile {
    shared: Shared,
    at: &'static str,
    kind: &'static str,
    ssid: &'static str,
    mode: &'static str,
}

fn variant(value: impl Into<zbus::Variant<'static>>) -> OwnedVariant {
    OwnedVariant::try_from(value.into()).expect("an owned variant")
}

#[zbus::interface(name = "org.freedesktop.NetworkManager.Settings.Connection")]
impl FakeProfile {
    fn get_settings(&self) -> HashMap<String, HashMap<String, OwnedVariant>> {
        let mut settings = HashMap::from([(
            "connection".to_owned(),
            HashMap::from([
                ("type".to_owned(), variant(self.kind)),
                ("id".to_owned(), variant(self.ssid)),
            ]),
        )]);
        if self.kind == "802-11-wireless" {
            settings.insert(
                "802-11-wireless".to_owned(),
                HashMap::from([
                    ("ssid".to_owned(), variant(self.ssid.as_bytes().to_vec())),
                    ("mode".to_owned(), variant(self.mode)),
                ]),
            );
        }
        settings
    }

    fn update(&self, properties: HashMap<String, HashMap<String, OwnedVariant>>) {
        let mut asked = asked(&self.shared);
        asked.calls.push(format!("update {}", self.at));
        asked.updated = Some(properties);
    }

    fn delete(&self) {
        let mut asked = asked(&self.shared);
        asked.calls.push(format!("delete {}", self.at));
        asked.profiles.retain(|profile| *profile != self.at);
    }
}

/// NetworkManager with a wired device and a Wi-Fi device in `state` that
/// sees "home" (WPA2, in use) and "cafe" (open), and a saved profile for
/// "home", a hotspot and a wired one. A join ends in `outcome`.
async fn fake(state: u32, outcome: (u32, u32)) -> (Connection, Connection, Shared) {
    let shared = Shared::default();
    asked(&shared).profiles = vec![HOME_PROFILE, HOTSPOT_PROFILE, WIRED_PROFILE];
    let profile = |at, kind, ssid, mode| FakeProfile {
        shared: shared.clone(),
        at,
        kind,
        ssid,
        mode,
    };
    let (server, client) = UnixStream::pair().expect("socket pair");
    let server = Builder::unix_stream(server)
        .server(Guid::generate())
        .p2p()
        .serve_at(
            ROOT,
            FakeManager {
                shared: shared.clone(),
                wireless_enabled: true,
                outcome,
            },
        )
        .serve_at(
            WIRED,
            FakeDevice {
                shared: shared.clone(),
                kind: 1,
                state: 100,
            },
        )
        .serve_at(
            DEVICE,
            FakeDevice {
                shared: shared.clone(),
                kind: 2,
                state,
            },
        )
        .serve_at(
            DEVICE,
            FakeWireless {
                shared: shared.clone(),
            },
        )
        .serve_at(
            HOME,
            FakePoint {
                ssid: "home",
                strength: 70,
                rsn_flags: 0x100,
            },
        )
        .serve_at(
            CAFE,
            FakePoint {
                ssid: "cafe",
                strength: 40,
                rsn_flags: 0,
            },
        )
        .serve_at(
            SETTINGS,
            FakeSettings {
                shared: shared.clone(),
            },
        )
        .serve_at(
            HOME_PROFILE,
            profile(HOME_PROFILE, "802-11-wireless", "home", "infrastructure"),
        )
        .serve_at(
            HOTSPOT_PROFILE,
            profile(HOTSPOT_PROFILE, "802-11-wireless", "lyr hotspot", "ap"),
        )
        .serve_at(
            WIRED_PROFILE,
            profile(WIRED_PROFILE, "802-3-ethernet", "wired", ""),
        )
        .build();
    let client = Builder::unix_stream(client).p2p().build();
    let (server, client) = tokio::try_join!(server, client).expect("connected");
    (server, client, shared)
}

/// A join's end, which must come within 5 s.
async fn joining(
    join: impl Future<Output = zbus::Result<Result<(), Failed>>>,
) -> Result<(), Failed> {
    tokio::time::timeout(Duration::from_secs(5), join)
        .await
        .expect("the join ends within 5 s")
        .expect("asked")
}

async fn network(client: &Connection) -> NetworkManager {
    NetworkManager::on(client).await.expect("proxy")
}

#[tokio::test]
async fn the_wifi_device_its_networks_and_profiles_are_read() {
    let (_server, client, _) = fake(ACTIVATED, (ACTIVATED, 0)).await;
    let wifi = network(&client).await.wifi().await.expect("read");
    assert_eq!(wifi.device.as_deref(), Some(DEVICE), "not the wired one");
    assert!(wifi.radio.enabled && wifi.radio.hardware_enabled);
    assert!(wifi.available);
    assert_eq!(wifi.last_scan, Some(Duration::from_millis(12_345)));
    assert_eq!(wifi.active.as_deref(), Some(HOME));
    let paths: Vec<&str> = wifi.profiles.iter().map(|p| p.path.as_str()).collect();
    assert_eq!(paths, [HOME_PROFILE], "client Wi-Fi profiles only");
    let rows = wifi.networks();
    assert_eq!(rows[0].ssid, "home");
    assert_eq!(rows[0].security, Security::Psk);
    assert!(rows[0].in_use && rows[0].known);
    assert_eq!(rows[1].ssid, "cafe");
    assert_eq!(rows[1].security, Security::Open);
}

#[tokio::test]
async fn a_profile_or_access_point_gone_before_it_is_read_is_left_out() {
    let (_server, client, shared) = fake(ACTIVATED, (ACTIVATED, 0)).await;
    // Listed, then gone before its read: lyr deleted a failed join's
    // profile, or an access point dropped out of a scan. That read failing
    // ended the menu's watch, which kept showing a deleted profile.
    asked(&shared).gone = true;
    let wifi = network(&client).await.wifi().await.expect("read");
    let paths: Vec<&str> = wifi.profiles.iter().map(|p| p.path.as_str()).collect();
    assert_eq!(paths, [HOME_PROFILE]);
    let points: Vec<&str> = wifi.points.iter().map(|p| p.path.as_str()).collect();
    assert_eq!(points, [HOME, CAFE]);
}

#[tokio::test]
async fn disconnecting_asks_the_wifi_device() {
    let (_server, client, shared) = fake(ACTIVATED, (ACTIVATED, 0)).await;
    network(&client)
        .await
        .disconnect(DEVICE)
        .await
        .expect("disconnected");
    assert_eq!(asked(&shared).calls, ["disconnect 2"], "the Wi-Fi one");
}

#[tokio::test]
async fn an_unavailable_device_cannot_scan() {
    let (_server, client, _) = fake(20, (ACTIVATED, 0)).await;
    let wifi = network(&client).await.wifi().await.expect("read");
    assert!(!wifi.available);
    let (_server, client, _) = fake(10, (ACTIVATED, 0)).await;
    assert!(!network(&client).await.wifi().await.expect("read").available);
}

#[tokio::test]
async fn the_radio_switches_and_a_scan_is_asked_for() {
    let (_server, client, shared) = fake(ACTIVATED, (ACTIVATED, 0)).await;
    let network = network(&client).await;
    network.set_radio(false).await.expect("switched");
    assert!(!network.wifi().await.expect("read").radio.enabled);
    network.scan(DEVICE).await.expect("asked");
    assert_eq!(asked(&shared).calls, ["scan"]);
}

#[tokio::test]
async fn a_saved_profile_joins_through_the_access_point() {
    let (_server, client, shared) = fake(ACTIVATED, (ACTIVATED, 0)).await;
    let joined = joining(network(&client).await.join(
        DEVICE,
        &Join::Saved {
            profile: HOME_PROFILE.to_owned(),
            point: HOME.to_owned(),
        },
    ))
    .await;
    assert_eq!(joined, Ok(()));
    assert_eq!(
        asked(&shared).calls,
        [format!("activate {HOME_PROFILE} {DEVICE} {HOME}")]
    );
}

/// The string at `section`.`key` of `settings`.
fn text(
    settings: &HashMap<String, HashMap<String, OwnedVariant>>,
    section: &str,
    key: &str,
) -> String {
    String::try_from(settings[section][key].clone()).expect("a string")
}

#[tokio::test]
async fn a_new_profile_carries_the_scan_s_security_and_its_failure() {
    let (_server, client, shared) = fake(ACTIVATED, (FAILED, NO_SECRETS)).await;
    let joined = joining(network(&client).await.join(
        DEVICE,
        &Join::New {
            ssid: "home".to_owned(),
            security: Security::Psk,
            password: Some(" spaced out ".to_owned()),
            point: HOME.to_owned(),
        },
    ))
    .await;
    assert_eq!(
        joined,
        Err(Failed {
            reason: NO_SECRETS,
            profile: Some(NEW_PROFILE.to_owned()),
        })
    );
    let asked = asked(&shared);
    assert_eq!(asked.calls, [format!("add {DEVICE} {HOME}")]);
    let added = asked.added.as_ref().expect("settings");
    assert_eq!(
        text(added, "802-11-wireless-security", "key-mgmt"),
        "wpa-psk"
    );
    assert_eq!(
        text(added, "802-11-wireless-security", "psk"),
        " spaced out ",
        "spaces kept"
    );
    assert_eq!(text(added, "802-11-wireless", "mode"), "infrastructure");
    assert_eq!(text(added, "connection", "type"), "802-11-wireless");
    let ssid = Vec::<u8>::try_from(added["802-11-wireless"]["ssid"].clone()).expect("bytes");
    assert_eq!(ssid, b"home");
}

#[tokio::test]
async fn an_open_network_gets_no_security_settings() {
    let (_server, client, shared) = fake(ACTIVATED, (ACTIVATED, 0)).await;
    let joined = joining(network(&client).await.join(
        DEVICE,
        &Join::New {
            ssid: "cafe".to_owned(),
            security: Security::Open,
            password: None,
            point: CAFE.to_owned(),
        },
    ))
    .await;
    assert_eq!(joined, Ok(()));
    let asked = asked(&shared);
    let added = asked.added.as_ref().expect("settings");
    assert!(!added.contains_key("802-11-wireless-security"));
}

#[tokio::test]
async fn forgetting_deletes_and_a_new_password_keeps_the_rest() {
    let (_server, client, shared) = fake(ACTIVATED, (ACTIVATED, 0)).await;
    let network = network(&client).await;
    network
        .replace_password(HOME_PROFILE, "new secret")
        .await
        .expect("updated");
    {
        let asked = asked(&shared);
        let updated = asked.updated.as_ref().expect("settings");
        assert_eq!(
            text(updated, "802-11-wireless-security", "psk"),
            "new secret"
        );
        assert_eq!(text(updated, "connection", "id"), "home", "the rest kept");
    }
    network
        .forget(&[HOME_PROFILE.to_owned()])
        .await
        .expect("deleted");
    assert_eq!(
        asked(&shared).calls,
        [
            format!("update {HOME_PROFILE}"),
            format!("delete {HOME_PROFILE}")
        ]
    );
    assert!(network.wifi().await.expect("read").profiles.is_empty());
}

async fn next(states: &mut watch::Receiver<Wifi>) -> Wifi {
    tokio::time::timeout(Duration::from_secs(5), states.changed())
        .await
        .expect("a state within 5 s")
        .expect("the source is running");
    states.borrow_and_update().clone()
}

#[tokio::test]
async fn the_menu_follows_the_radio_and_the_profiles() {
    let (server, client, shared) = fake(ACTIVATED, (ACTIVATED, 0)).await;
    let watcher = network(&client).await;
    let (sender, mut states) = watch::channel(Wifi::default());
    let source = tokio::spawn(async move { watcher.watch_wifi(&sender).await });
    assert!(next(&mut states).await.radio.enabled);

    network(&client)
        .await
        .set_radio(false)
        .await
        .expect("switched");
    assert!(!next(&mut states).await.radio.enabled);

    // A profile saved elsewhere.
    asked(&shared).profiles.clear();
    let settings: InterfaceRef<FakeSettings> = server
        .object_server()
        .interface(SETTINGS)
        .await
        .expect("served");
    FakeSettings::new_connection(
        settings.signal_emitter(),
        ObjectPath::try_from(HOME_PROFILE).expect("a path"),
    )
    .await
    .expect("signalled");
    assert!(next(&mut states).await.profiles.is_empty());

    // An access point's own change waits.
    let point: InterfaceRef<FakePoint> = server
        .object_server()
        .interface(CAFE)
        .await
        .expect("served");
    {
        let mut fake = point.get_mut().await;
        fake.strength = 41;
        fake.strength_changed(point.signal_emitter())
            .await
            .expect("signalled");
    }
    assert!(
        tokio::time::timeout(Duration::from_millis(300), states.changed())
            .await
            .is_err(),
        "an access point's signal is not followed"
    );
    source.abort();
}

#[test]
fn the_time_since_boot_is_read() {
    let first = since_boot().expect("/proc/uptime");
    assert!(first > Duration::ZERO);
    assert!(since_boot().expect("/proc/uptime") >= first);
}

#[test]
fn an_access_point_is_read_from_its_properties_whole() {
    use zbus::Variant;

    let path =
        OwnedObjectPath::try_from("/org/freedesktop/NetworkManager/AccessPoint/7").expect("a path");
    let value = |value: Variant<'_>| OwnedVariant::try_from(value).expect("owned");
    let mut properties: HashMap<String, OwnedVariant> = HashMap::from([
        ("Ssid".to_owned(), value(Variant::from(b"home".to_vec()))),
        ("Strength".to_owned(), value(Variant::from(64_u8))),
        ("Flags".to_owned(), value(Variant::from(1_u32))),
        ("WpaFlags".to_owned(), value(Variant::from(0_u32))),
        ("RsnFlags".to_owned(), value(Variant::from(0x188_u32))),
    ]);
    let point = super::access_point(&path, &properties).expect("read");
    assert_eq!(point.ssid, "home");
    assert_eq!(point.strength, 64);
    assert_eq!(point.security, Security::of(1, 0, 0x188));
    assert_eq!(point.path, path.as_str());

    properties.insert("Strength".to_owned(), value(Variant::from("strong")));
    assert!(
        super::access_point(&path, &properties).is_err(),
        "wrong type"
    );
    properties.remove("Strength");
    assert!(super::access_point(&path, &properties).is_err(), "missing");
}
