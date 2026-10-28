//! `DrawList` snapshots of the default bar (`docs/plan/13-testing.md`): the
//! Linux defaults in representative states, laid out with every character
//! 10 px wide, each compared with its file under `snapshots/`. Structure,
//! not pixels: a change to what a module shows or where it sits changes the
//! file. `LYR_UPDATE_SNAPSHOTS=1` writes the files anew for a change that is
//! meant, to review in the diff.

use std::fmt::Write as _;
use std::path::PathBuf;

use chrono::DateTime;
use lyr_config::{Colour, Config, Platform, from_sources};
use lyr_core::battery::{Reading, Status, TimeTo};
use lyr_core::draw::{Bar, DrawList, Item, build};
use lyr_core::hit::Rect;

use super::{Data, entries};

/// The bar of `config` showing `data` at 09:05 on 2026-10-19, 1920 px wide.
fn bar(config: &Config, data: &Data) -> DrawList {
    let now = DateTime::parse_from_rfc3339("2026-10-19T09:05:00+08:00").expect("a time");
    let zone = |items| entries(items, config, data, &now, None, None);
    let (left, center, right) = (
        zone(&config.layout.left),
        zone(&config.layout.center),
        zone(&config.layout.right),
    );
    let bar = Bar {
        width: 1920.0,
        height: config.bar.height as f32,
        left: &left,
        center: &center,
        right: &right,
    };
    build(&bar, &config.theme, |text| {
        text.chars().count() as f32 * 10.0
    })
    .0
}

fn colour(colour: Colour) -> String {
    let Colour { r, g, b, a } = colour;
    format!("#{r:02x}{g:02x}{b:02x}{a:02x}")
}

fn rect(rect: Rect) -> String {
    let Rect {
        x,
        y,
        width,
        height,
    } = rect;
    format!("{x},{y} {width}x{height}")
}

/// One line per item, a group's or clip's items indented under it.
fn describe(items: &[Item], depth: usize, out: &mut String) {
    for item in items {
        let indent = "  ".repeat(depth);
        let line = match item {
            Item::Box {
                rect: at,
                radius,
                fill,
                border,
            } => {
                let border = border.map_or_else(
                    || "none".to_owned(),
                    |(width, border)| format!("{width} {}", colour(border)),
                );
                format!(
                    "box {} radius {radius} fill {} border {border}",
                    rect(*at),
                    colour(*fill)
                )
            }
            Item::Text {
                rect: at,
                text,
                colour: ink,
                style,
            } => {
                let flags = [
                    (style.bold, " bold"),
                    (style.italic, " italic"),
                    (style.underline, " underline"),
                    (style.calendar, " calendar"),
                ];
                let style: String = flags
                    .into_iter()
                    .filter_map(|(set, name)| set.then_some(name))
                    .collect();
                format!("text {} {}{style} {text:?}", rect(*at), colour(*ink))
            }
            Item::Image { rect: at, picture } => format!(
                "image {} {}x{}",
                rect(*at),
                picture.width(),
                picture.height()
            ),
            Item::Group { opacity, .. } => format!("group {opacity}"),
            Item::Clip { rect: at, .. } => format!("clip {}", rect(*at)),
        };
        let _ = writeln!(out, "{indent}{line}");
        if let Item::Group { items, .. } | Item::Clip { items, .. } = item {
            describe(items, depth + 1, out);
        }
    }
}

/// Compares `list` with the snapshot `name`, or writes it when asked to.
fn check(name: &str, list: &DrawList) {
    let mut described = String::new();
    describe(&list.items, 0, &mut described);
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src/modules/snapshots")
        .join(format!("{name}.txt"));
    if std::env::var_os("LYR_UPDATE_SNAPSHOTS").is_some() {
        std::fs::write(&path, &described).expect("snapshot written");
        return;
    }
    let stored = std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!(
            "{}: {error}; LYR_UPDATE_SNAPSHOTS=1 writes it",
            path.display()
        )
    });
    assert!(
        stored == described,
        "{name} differs from {}; LYR_UPDATE_SNAPSHOTS=1 writes it anew\n{described}",
        path.display()
    );
}

fn defaults() -> Config {
    from_sources(None, Platform::Linux).expect("the defaults")
}

#[test]
fn the_bar_with_no_source_answering_yet() {
    check("no_sources", &bar(&defaults(), &Data::default()));
}

#[test]
fn the_bar_on_battery_with_wifi_bluetooth_and_audio() {
    let data = Data {
        battery: Some(Reading {
            capacity: 64,
            status: Status::Discharging,
            power: Some(7.5),
            time_to: TimeTo::Empty(3.0 * 3600.0),
        }),
        power_profile: Some(lyr_sources::power_profile::Profiles {
            active: lyr_core::power_profile::Profile {
                name: "balanced".to_owned(),
                driver: "intel_pstate".to_owned(),
            },
            available: ["power-saver", "balanced", "performance"]
                .map(str::to_owned)
                .to_vec(),
        }),
        network: Some(lyr_core::network::Reading {
            link: Some(lyr_core::network::Link {
                ifname: "wlan0".to_owned(),
                ipaddr: "192.168.1.20".to_owned(),
                cidr: "24".to_owned(),
                gwaddr: "192.168.1.1".to_owned(),
                essid: "Home".to_owned(),
                signal_strength: 70,
                wireless: false,
            }),
            radio_blocked: false,
        }),
        bluetooth: Some(lyr_core::bluetooth::Reading {
            state: lyr_core::bluetooth::State::Connected,
            connections: 1,
            device: Some(lyr_core::bluetooth::Device {
                alias: "Buds".to_owned(),
                battery: Some(80),
            }),
        }),
        sink: Some(lyr_core::audio::Sink {
            volume: 40,
            name: "alsa_output.usb-headset".to_owned(),
            port: "analog-output-headphones".to_owned(),
            description: "USB Headset".to_owned(),
            ..lyr_core::audio::Sink::default()
        }),
        source: Some(lyr_core::audio::Source {
            volume: 65,
            muted: true,
            description: "USB Headset Mic".to_owned(),
        }),
        ..Data::default()
    };
    check("on_battery", &bar(&defaults(), &data));
}
