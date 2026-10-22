//! lyr's Linux sources read through the machine's real services: UPower,
//! power-profiles-daemon, NetworkManager, BlueZ, timedate1 and the sound
//! server. Every other test of them runs against a fake written from the
//! service's documentation; these say what the installed services answer.
//! They only read, except three that change something and put it back:
//! one steps the default sink's volume down and up and then sets it to the
//! whole percent it was, one starts and stops a Bluetooth scan, and one
//! switches the power profile and back.
//!
//! Ignored by default, since CI's runners have none of the services. On a
//! desktop that runs them: `cargo test -p lyr-sources --test system_services
//! -- --ignored --nocapture`, which prints each reading.
#![cfg(target_os = "linux")]

use std::time::Duration;

use lyr_sources::battery::linux::UPower;
use lyr_sources::bluetooth::Snapshot;
use lyr_sources::bluetooth::linux::BlueZ;
use lyr_sources::clock::linux::TimeDate;
use lyr_sources::network::linux::NetworkManager;
use lyr_sources::power_profile::linux::PowerProfiles;
use tokio::sync::watch;

/// How long a service gets to send its first reading.
const FIRST_READING: Duration = Duration::from_secs(10);

#[tokio::test]
#[ignore = "reads this machine's UPower; run with --ignored"]
async fn upower_reads_the_battery_the_kernel_reports() {
    let device = UPower::system()
        .await
        .expect("UPower answers")
        .device()
        .await
        .expect("the display device reads");
    let reading = device.reading();
    eprintln!("UPower: {device:?}\nreading: {reading:?}");
    // The kernel's own figure for the first battery, where there is one.
    let Ok(kernel) = std::fs::read_to_string("/sys/class/power_supply/BAT0/capacity") else {
        assert_eq!(reading, None, "no battery, no reading");
        return;
    };
    let kernel: i16 = kernel.trim().parse().expect("a percentage");
    let reading = reading.expect("a battery is present");
    assert!(
        (i16::from(reading.capacity) - kernel).abs() <= 1,
        "UPower's {}% against the kernel's {kernel}%",
        reading.capacity
    );
}

#[tokio::test]
#[ignore = "reads this machine's power-profiles-daemon; run with --ignored"]
async fn power_profiles_daemon_reads_its_profiles() {
    let profiles = PowerProfiles::system()
        .await
        .expect("the daemon answers")
        .profiles()
        .await
        .expect("its state reads")
        .expect("a profile is active");
    eprintln!("power profiles: {profiles:?}");
    assert!(
        profiles.available.contains(&profiles.active.name),
        "the active profile is one it offers"
    );
}

#[tokio::test]
#[ignore = "reads this machine's NetworkManager; run with --ignored"]
async fn network_manager_reads_the_primary_connection() {
    let (reading, objects) = NetworkManager::system()
        .await
        .expect("NetworkManager answers")
        .read()
        .await
        .expect("the primary connection reads");
    eprintln!("network: {reading:?}\nfollowing {} objects", objects.len());
    assert!(!objects.is_empty(), "it follows what it read");
}

#[tokio::test]
#[ignore = "reads this machine's BlueZ; run with --ignored"]
async fn bluez_reads_the_adapter() {
    let bluez = BlueZ::system().await.expect("BlueZ answers");
    let (snapshots, mut first) = watch::channel(Snapshot::default());
    let read = async {
        first.changed().await.expect("a reading");
        first.borrow().clone()
    };
    let snapshot = tokio::select! {
        snapshot = read => snapshot,
        ended = bluez.watch(&snapshots) => panic!("the watch ended first: {ended:?}"),
        () = tokio::time::sleep(FIRST_READING) => panic!("no reading in time"),
    };
    eprintln!("bluetooth: {snapshot:?}");
    assert!(snapshot.adapter.is_some(), "an adapter");
}

#[tokio::test]
#[ignore = "reads this machine's timedate1; run with --ignored"]
async fn timedate1_reads_the_zone() {
    let zone = TimeDate::system()
        .await
        .expect("timedated answers")
        .zone()
        .await
        .expect("the zone reads");
    eprintln!("zone: {zone:?}");
    let zone = zone.expect("a zone is set");
    assert!(
        std::path::Path::new("/usr/share/zoneinfo")
            .join(&zone)
            .exists(),
        "{zone} is a zone the system knows"
    );
}

#[tokio::test]
#[ignore = "reads this machine's sound server; run with --ignored"]
async fn the_mixer_reads_the_defaults_and_the_menu() {
    let audio = lyr_sources::audio::mixer::Audio::open()
        .await
        .expect("the mixer opens");
    let ((sink, source), nodes) = audio.read();
    eprintln!("sink: {sink:?}\nsource: {source:?}\nmenu: {nodes:?}");
    assert!(sink.is_some(), "a default sink");
    assert!(
        nodes.outputs.iter().filter(|node| node.default).count() == 1,
        "one output is the default"
    );
}

#[tokio::test]
#[ignore = "steps this machine's default sink down and back; run with --ignored"]
async fn the_mixer_steps_the_default_sink_both_ways() {
    use lyr_sources::audio::mixer::{Audio, Flow, Target};

    let audio = Audio::open().await.expect("the mixer opens");
    let (states, mut changed) = tokio::sync::watch::channel(Default::default());
    let volume = || audio.read().0.0.expect("a default sink").volume;
    let sink = Target::Default(Flow::Output);
    let steps = async {
        // A change is the server's, so it shows once the watch hears it.
        let mut settle = async || {
            tokio::time::timeout(std::time::Duration::from_secs(2), changed.changed())
                .await
                .ok();
            changed.borrow_and_update();
        };
        settle().await;
        let before = volume();
        let down = audio.step(&sink, -5);
        settle().await;
        let lowered = volume();
        let up = audio.step(&sink, 5);
        settle().await;
        let raised = volume();
        // Back where it was, whatever the steps did.
        audio.set(&sink, before).expect("restored");
        settle().await;
        (before, lowered, raised, down, up)
    };
    let (before, lowered, raised, down, up) = tokio::select! {
        steps = steps => steps,
        () = audio.watch(&states) => unreachable!("the watch ends only when the test drops it"),
    };
    eprintln!("{before}% -> {lowered}% -> {raised}%");
    down.expect("a step down");
    up.expect("a step up");
    assert!(
        lowered < before || before == 0,
        "{before}% stepped down to {lowered}%"
    );
    assert!(raised > lowered, "{lowered}% stepped up to {raised}%");
    assert_eq!(volume(), before, "restored");
}

#[tokio::test]
#[ignore = "starts and stops this machine's Bluetooth scan; run with --ignored"]
async fn bluez_scans_and_stops() {
    let bluez = BlueZ::system().await.expect("BlueZ answers");
    let (snapshots, mut seen) = watch::channel(Snapshot::default());
    let discovering = |snapshot: &Snapshot| snapshot.adapter.as_ref().map(|a| a.discovering);
    let steps = async {
        let adapter = seen
            .wait_for(|snapshot| snapshot.adapter.is_some())
            .await
            .expect("a reading")
            .adapter
            .clone()
            .expect("an adapter");
        assert!(adapter.powered, "a scan needs the adapter on");
        bluez
            .scan(&adapter.path, true)
            .await
            .expect("a scan starts");
        seen.wait_for(|snapshot| discovering(snapshot) == Some(true))
            .await
            .expect("the scan seen");
        bluez
            .scan(&adapter.path, false)
            .await
            .expect("the scan stops");
        seen.wait_for(|snapshot| discovering(snapshot) == Some(false))
            .await
            .expect("the stop seen");
    };
    tokio::select! {
        () = steps => {}
        ended = bluez.watch(&snapshots) => panic!("the watch ended first: {ended:?}"),
        () = tokio::time::sleep(4 * FIRST_READING) => panic!("not seen in time"),
    }
}

#[tokio::test]
#[ignore = "switches this machine's power profile and back; run with --ignored"]
async fn power_profiles_daemon_switches_and_back() {
    let daemon = PowerProfiles::system().await.expect("the daemon answers");
    let read = || async {
        daemon
            .profiles()
            .await
            .expect("its state reads")
            .expect("a profile is active")
    };
    let before = read().await;
    let other = before
        .available
        .iter()
        .find(|name| **name != before.active.name)
        .expect("a second profile")
        .clone();
    let switched = daemon.set(&other).await;
    let during = read().await.active.name;
    // Back where it was, whatever the switch did.
    daemon.set(&before.active.name).await.expect("restored");
    let after = read().await.active.name;
    eprintln!("{} -> {during} -> {after}", before.active.name);
    switched.expect("the switch");
    assert_eq!(during, other);
    assert_eq!(after, before.active.name);
}

#[tokio::test]
#[ignore = "adds and deletes a Wi-Fi profile in this machine's NetworkManager; run with --ignored"]
async fn network_manager_follows_a_profile_coming_and_going() {
    use std::process::Command;

    use lyr_core::wifi_menu::Wifi;

    /// A profile no network has, never activated.
    const SSID: &str = "lyr-test-profile";
    let nmcli = |args: &[&str]| {
        let status = Command::new("nmcli")
            .args(args)
            .status()
            .expect("nmcli runs");
        assert!(status.success(), "nmcli {args:?}");
    };
    let manager = NetworkManager::system()
        .await
        .expect("NetworkManager answers");
    let (states, mut seen) = watch::channel(Wifi::default());
    let known = |wifi: &Wifi| !wifi.profiles_for(SSID).is_empty();
    let steps = async {
        seen.wait_for(|wifi| wifi.device.is_some())
            .await
            .expect("a reading");
        nmcli(&[
            "connection",
            "add",
            "type",
            "wifi",
            "con-name",
            SSID,
            "ssid",
            SSID,
            "autoconnect",
            "no",
        ]);
        let profile = seen
            .wait_for(known)
            .await
            .expect("the new profile seen")
            .profiles_for(SSID)[0]
            .path
            .clone();
        nmcli(&["connection", "delete", SSID]);
        seen.wait_for(|wifi| !known(wifi))
            .await
            .expect("its removal seen");
        // How NetworkManager answers a path it no longer serves, which the
        // Wi-Fi reads skip rather than fail on.
        let gone = manager.forget(&[profile]).await.expect_err("already gone");
        assert!(
            matches!(&gone, zbus::Error::FDO(error)
                if matches!(**error, zbus::fdo::Error::UnknownMethod(_))),
            "{gone:?}"
        );
    };
    let followed = tokio::select! {
        () = steps => Ok(()),
        ended = manager.watch_wifi(&states) => Err(format!("the watch ended first: {ended:?}")),
        () = tokio::time::sleep(4 * FIRST_READING) => Err("not seen in time".to_owned()),
    };
    // Gone whatever happened above; a failure here is only a profile
    // already deleted.
    let _ = Command::new("nmcli")
        .args(["connection", "delete", SSID])
        .output();
    followed.expect("followed");
}
