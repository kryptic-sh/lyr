//! lyr's Linux sources read through the machine's real services: UPower,
//! power-profiles-daemon, NetworkManager, BlueZ, timedate1 and the sound
//! server. Every other test of them runs against a fake written from the
//! service's documentation; these say what the installed services answer.
//! They only read, except one that steps the default sink's volume down and
//! up and then sets it back to the whole percent it was.
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
#[ignore = "reads this machine's sound server through pactl; run with --ignored"]
async fn pactl_reads_the_defaults_and_the_menu() {
    let ((sink, source), nodes) = lyr_sources::audio::pactl::read()
        .await
        .expect("pactl reads");
    eprintln!("sink: {sink:?}\nsource: {source:?}\nmenu: {nodes:?}");
    assert!(sink.is_some(), "a default sink");
    assert!(
        nodes.outputs.iter().filter(|node| node.default).count() == 1,
        "one output is the default"
    );
}

#[tokio::test]
#[ignore = "steps this machine's default sink down and back; run with --ignored"]
async fn pactl_steps_the_default_sink_both_ways() {
    use lyr_sources::audio::pactl::{Flow, Target, read, set, step};

    let volume = || async {
        read()
            .await
            .expect("pactl reads")
            .0
            .0
            .expect("a default sink")
            .volume
    };
    let sink = Target::Default(Flow::Output);
    let before = volume().await;
    let down = step(&sink, -5).await;
    let lowered = volume().await;
    let up = step(&sink, 5).await;
    let raised = volume().await;
    // Back where it was, whatever the steps did.
    set(&sink, before).await.expect("restored");
    eprintln!("{before}% -> {lowered}% -> {raised}%");
    down.expect("a step down");
    up.expect("a step up");
    assert!(
        lowered < before || before == 0,
        "{before}% stepped down to {lowered}%"
    );
    assert!(raised > lowered, "{lowered}% stepped up to {raised}%");
}
