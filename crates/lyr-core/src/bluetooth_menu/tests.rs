use lyr_config::{Platform, Theme, from_sources};

use super::{Adapter, Device, DeviceAction, GAP, PADDING, Switch, Target, draw, order};
use crate::draw::Item;

fn theme() -> Theme {
    from_sources(None, Platform::Linux)
        .expect("the defaults")
        .theme
}

/// Every character is 10 px wide.
fn measure(text: &str) -> f32 {
    text.chars().count() as f32 * 10.0
}

const LINE: f32 = 20.0;
/// A row: a line and 2 px above and below.
const ROW: f32 = LINE + 4.0;

fn adapter() -> Adapter {
    Adapter {
        path: "/org/bluez/hci0".to_owned(),
        powered: true,
        discoverable: false,
        pairable: true,
        discovering: false,
        power_state: None,
    }
}

fn device(alias: &str, paired: bool, connected: bool, trusted: bool) -> Device {
    Device {
        path: format!("/org/bluez/hci0/{alias}"),
        alias: alias.to_owned(),
        connected,
        paired,
        trusted,
        battery: None,
    }
}

#[test]
fn paired_devices_come_first_then_by_name() {
    let devices = [
        device("Zeta", false, false, false),
        device("Buds", true, false, false),
        device("Alpha", false, false, false),
        device("Mouse", true, true, true),
    ];
    let names: Vec<&str> = order(&devices)
        .into_iter()
        .map(|device| device.alias.as_str())
        .collect();
    assert_eq!(names, ["Buds", "Mouse", "Alpha", "Zeta"]);
}

/// The labels of the buttons drawn in the row at `y`.
fn buttons_at(items: &[Item], y: f32, names: &[&str]) -> Vec<String> {
    items
        .iter()
        .filter_map(|item| match item {
            Item::Text { text, rect, .. }
                if (rect.y - y).abs() < 1.0 && !names.contains(&text.as_str()) =>
            {
                Some(text.clone())
            }
            _ => None,
        })
        .collect()
}

#[test]
fn each_device_has_the_buttons_its_state_calls_for() {
    let theme = theme();
    let devices = [
        device("Buds", true, true, true),
        device("Phone", false, false, false),
    ];
    let (list, _, regions) = draw(&adapter(), &devices, &measure, LINE, &theme);
    // Four switch rows, then the devices after a gap.
    let first = PADDING + 4.0 * (ROW + GAP) + GAP;
    let second = first + ROW + GAP;
    let names = ["Buds", "Phone"];
    assert_eq!(
        buttons_at(&list.items, first + 2.0, &names),
        ["Disconnect", "Untrust", "Forget"]
    );
    assert_eq!(
        buttons_at(&list.items, second + 2.0, &names),
        ["Connect", "Pair", "Trust", "Forget"]
    );
    // Buttons start after the widest name ("Discoverable", 120 px).
    let x = PADDING + 120.0 + GAP + 1.0;
    assert_eq!(
        regions.hit(x, first + 1.0),
        Some(&Target::Device(0, DeviceAction::Disconnect))
    );
    assert_eq!(
        regions.hit(x, second + 1.0),
        Some(&Target::Device(1, DeviceAction::Connect))
    );
}

#[test]
fn the_switches_show_the_adapter_and_take_clicks() {
    let theme = theme();
    let (list, size, regions) = draw(&adapter(), &[], &measure, LINE, &theme);
    let x = size.width - PADDING - 1.0;
    let targets: Vec<_> = (0..4)
        .map(|row| regions.hit(x, PADDING + row as f32 * (ROW + GAP) + 1.0))
        .collect();
    assert_eq!(
        targets,
        [
            Some(&Target::Switch(Switch::Powered)),
            Some(&Target::Switch(Switch::Discoverable)),
            Some(&Target::Switch(Switch::Pairable)),
            Some(&Target::Scan),
        ]
    );
    let fills: Vec<_> = list
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Box { rect, fill, .. } if rect.height == ROW && rect.width == 2.0 * ROW => {
                Some(*fill)
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        fills,
        [
            theme.switch_on,
            theme.toast_background,
            theme.switch_on,
            theme.toast_background
        ],
        "power and pairable on, discoverable and scan off"
    );
    assert_eq!(size.height, PADDING + 4.0 * ROW + 3.0 * GAP + PADDING);
}
