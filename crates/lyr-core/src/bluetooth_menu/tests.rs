use lyr_config::{Platform, Theme, from_sources};

use super::{
    Abilities, Adapter, Ask, BUSY, CONNECT, DISCONNECT, Device, DeviceAction, FORGET, GAP, Header,
    PADDING, PAIR, Question, RENAME, SETTINGS, Shown, Switch, TRUST, Target, UNTRUST, order,
};
use crate::draw::{DrawList, Item, Size};
use crate::hit::Regions;

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

/// The menu for `adapter` and `devices` with nothing busy.
fn draw(
    adapter: &Adapter,
    devices: &[Device],
    header: Option<Header<'_>>,
    focus: Option<Target>,
    measure: &dyn Fn(&str) -> f32,
    line_height: f32,
    theme: &Theme,
) -> (DrawList, Size, Regions<Target>) {
    let shown = Shown {
        adapter,
        devices,
        busy: &[],
        header,
    };
    super::draw(&shown, focus, measure, line_height, theme)
}

fn adapter() -> Adapter {
    Adapter {
        path: "/org/bluez/hci0".to_owned(),
        powered: true,
        discoverable: false,
        pairable: true,
        discovering: false,
        power_state: None,
        abilities: Abilities::BLUEZ,
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
    let (list, _, regions) = draw(&adapter(), &devices, None, None, &measure, LINE, &theme);
    // Four switch rows, then the devices after a gap.
    let first = PADDING + 4.0 * (ROW + GAP) + GAP;
    let second = first + ROW + GAP;
    let names = ["Buds", "Phone"];
    assert_eq!(
        buttons_at(&list.items, first + 2.0, &names),
        [DISCONNECT, UNTRUST, RENAME, FORGET],
        "Rename once paired"
    );
    assert_eq!(
        buttons_at(&list.items, second + 2.0, &names),
        [CONNECT, PAIR, TRUST, FORGET]
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
fn windows_draws_only_what_it_can_do() {
    let theme = theme();
    let windows = Adapter {
        abilities: Abilities::WINDOWS,
        ..adapter()
    };
    let devices = [
        device("Buds", true, true, true),
        device("Phone", false, false, false),
    ];
    let (list, _, regions) = draw(&windows, &devices, None, None, &measure, LINE, &theme);
    // Power and Scan only, then the devices after a gap.
    let first = PADDING + 2.0 * (ROW + GAP) + GAP;
    let second = first + ROW + GAP;
    let names = ["Buds", "Phone"];
    assert_eq!(
        buttons_at(&list.items, first + 2.0, &names),
        [SETTINGS, FORGET],
        "a paired device connects in Settings"
    );
    assert_eq!(
        buttons_at(&list.items, second + 2.0, &names),
        [PAIR],
        "a found device can only pair"
    );
    let labels: Vec<&str> = list
        .items
        .iter()
        .filter_map(|item| match item {
            crate::draw::Item::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert!(labels.contains(&"Bluetooth") && labels.contains(&"Scan"));
    assert!(!labels.contains(&"Discoverable") && !labels.contains(&"Pairable"));
    // Buttons start after the widest label ("Bluetooth", 90 px).
    let x = PADDING + 90.0 + GAP + 1.0;
    assert_eq!(regions.hit(x, first + 1.0), Some(&Target::Settings));
    assert_eq!(
        regions.hit(x, second + 1.0),
        Some(&Target::Device(1, DeviceAction::Pair))
    );
}

#[test]
fn off_it_shows_only_its_switch_and_says_so() {
    let theme = theme();
    let off = Adapter {
        powered: false,
        ..adapter()
    };
    let devices = [device("Buds", true, true, true)];
    let (list, size, regions) = draw(&off, &devices, None, None, &measure, LINE, &theme);
    let texts: Vec<&str> = list
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Text { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(texts, ["Bluetooth", "Bluetooth is off"]);
    let x = size.width - PADDING - 1.0;
    assert_eq!(
        regions.hit(x, PADDING + 1.0),
        Some(&Target::Switch(Switch::Powered)),
        "it can be turned back on"
    );
    let below = PADDING + ROW + GAP;
    let clickable = (0..(size.height as usize))
        .filter(|y| (*y as f32) > below)
        .any(|y| (0..(size.width as usize)).any(|x| regions.hit(x as f32, y as f32).is_some()));
    assert!(!clickable, "nothing else takes a click");
}

#[test]
fn the_switches_show_the_adapter_and_take_clicks() {
    let theme = theme();
    let (list, size, regions) = draw(&adapter(), &[], None, None, &measure, LINE, &theme);
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

#[test]
fn the_agent_s_question_comes_first_with_yes_and_no() {
    let theme = theme();
    let ask = Ask {
        device: "Buds",
        question: Question::Confirm(42),
        typed: "",
    };
    assert_eq!(ask.text(), "Pair with Buds? Passkey 000042");
    let header = Some(Header::Ask(ask));
    let (list, size, regions) = draw(&adapter(), &[], header, None, &measure, LINE, &theme);
    let x = size.width - PADDING - 1.0;
    assert_eq!(
        regions.hit(x, PADDING + 1.0),
        Some(&Target::Answer(false)),
        "No last"
    );
    // "No" is 20 px with 8 px each side; Yes sits a gap before it.
    let yes = size.width - PADDING - 36.0 - GAP - 1.0;
    assert_eq!(regions.hit(yes, PADDING + 1.0), Some(&Target::Answer(true)));
    // The switches move down below it.
    let switches = PADDING + ROW + 2.0 * GAP;
    assert_eq!(
        regions.hit(x, switches + 1.0),
        Some(&Target::Switch(Switch::Powered))
    );
    assert!(list.items.iter().any(
        |item| matches!(item, Item::Text { text, colour, .. } if text == "Pair with Buds? Passkey 000042" && *colour == theme.workspace_active)
    ));
    let asks = [
        (Question::Authorize, "Let Buds pair?"),
        (Question::AuthorizeService, "Let Buds connect?"),
    ];
    for (question, words) in asks {
        assert_eq!(
            Ask {
                device: "Buds",
                question,
                typed: "",
            }
            .text(),
            words
        );
    }
}

#[test]
fn a_pin_is_typed_into_a_field_with_ok_and_cancel() {
    let theme = theme();
    let ask = Ask {
        device: "Kbd",
        question: Question::Pin,
        typed: "0042",
    };
    assert_eq!(ask.text(), "PIN for Kbd");
    let header = Some(Header::Ask(ask));
    let (list, size, regions) = draw(&adapter(), &[], header, None, &measure, LINE, &theme);
    // The question, then a field as wide as the longest PIN with its
    // padding, then OK and Cancel.
    let field = PADDING + 110.0 + GAP;
    let field_width = 16.0 * 10.0 + 16.0;
    assert_eq!(
        size.width,
        2.0 * PADDING + 110.0 + GAP + field_width + GAP + 36.0 + GAP + 76.0
    );
    assert!(list.items.iter().any(|item| matches!(
        item,
        Item::Box { rect, .. } if rect.x == field && rect.width == field_width
    )));
    assert_eq!(
        typed_in(&list.items),
        Some(("0042".to_owned(), field + 8.0))
    );
    let labels: Vec<&str> = list
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Text { text, rect, .. } if rect.y < PADDING + ROW && rect.x > field + 8.0 => {
                Some(text.as_str())
            }
            _ => None,
        })
        .collect();
    assert_eq!(labels, ["Cancel", "OK"], "drawn right to left");
    let x = size.width - PADDING - 1.0;
    assert_eq!(regions.hit(x, PADDING + 1.0), Some(&Target::Answer(false)));
}

/// The text typed into the header's field, and where it starts.
fn typed_in(items: &[Item]) -> Option<(String, f32)> {
    items.iter().find_map(|item| match item {
        Item::Clip { items, .. } => match items.as_slice() {
            [Item::Text { text, rect, .. }] => Some((text.clone(), rect.x)),
            _ => None,
        },
        _ => None,
    })
}

#[test]
fn a_name_longer_than_its_field_scrolls_to_its_end() {
    let theme = theme();
    fn header(typed: &str) -> Option<Header<'_>> {
        Some(Header::Rename {
            device: "Kbd",
            typed,
        })
    }
    assert_eq!(
        header("").map(|header| header.text()).as_deref(),
        Some("Name for Kbd")
    );
    let (list, _, regions) = draw(
        &adapter(),
        &[],
        header("Desk"),
        None,
        &measure,
        LINE,
        &theme,
    );
    // "Name for Kbd" is 120 px.
    let inside = PADDING + 120.0 + GAP + 8.0;
    assert_eq!(typed_in(&list.items), Some(("Desk".to_owned(), inside)));
    let Some(Item::Clip { rect, .. }) = list
        .items
        .iter()
        .find(|item| matches!(item, Item::Clip { .. }))
    else {
        panic!("a clip");
    };
    assert_eq!(
        (rect.x, rect.width),
        (inside, 160.0),
        "the field less its padding"
    );
    assert!(
        regions
            .targets()
            .any(|target| *target == Target::Answer(true))
    );
    // 20 characters in a field 16 wide: 40 px out of sight on the left.
    let long = "A".repeat(20);
    let (list, _, _) = draw(&adapter(), &[], header(&long), None, &measure, LINE, &theme);
    assert_eq!(typed_in(&list.items), Some((long.clone(), inside - 40.0)));
}

#[test]
fn the_focused_target_is_outlined_in_the_active_colour() {
    let theme = theme();
    let (_, _, plain) = draw(&adapter(), &[], None, None, &measure, LINE, &theme);
    let targets: Vec<Target> = plain.targets().copied().collect();
    assert_eq!(
        targets,
        [
            Target::Switch(Switch::Powered),
            Target::Switch(Switch::Discoverable),
            Target::Switch(Switch::Pairable),
            Target::Scan
        ],
        "in the order drawn"
    );
    let (list, _, regions) = draw(
        &adapter(),
        &[],
        None,
        Some(Target::Scan),
        &measure,
        LINE,
        &theme,
    );
    let Some(Item::Box {
        rect, fill, border, ..
    }) = list.items.last()
    else {
        panic!("an outline last");
    };
    assert_eq!(Some(*rect), regions.rect(&Target::Scan));
    assert_eq!(*fill, lyr_config::Colour::TRANSPARENT);
    assert_eq!(
        border.map(|(_, colour)| colour),
        Some(theme.workspace_active)
    );
}

#[test]
fn a_busy_device_says_so_and_waits() {
    let theme = theme();
    let devices = [
        device("Buds", true, false, true),
        device("Mouse", true, false, true),
    ];
    let busy = [("/org/bluez/hci0/Buds".to_owned(), DeviceAction::Connect)];
    let shown = Shown {
        adapter: &adapter(),
        devices: &devices,
        busy: &busy,
        header: None,
    };
    let (list, _, regions) = super::draw(&shown, None, &measure, LINE, &theme);
    let first = PADDING + 4.0 * (ROW + GAP) + GAP;
    let second = first + ROW + GAP;
    let names = ["Buds", "Mouse"];
    assert_eq!(
        buttons_at(&list.items, first + 2.0, &names),
        [BUSY, UNTRUST, RENAME, FORGET]
    );
    let x = PADDING + 120.0 + GAP + 1.0;
    assert_eq!(regions.hit(x, first + 1.0), None, "Buds waits");
    assert!(
        !regions.targets().any(|target| matches!(
            target,
            Target::Device(0, _) | Target::Rename(0) | Target::Forget(0)
        )),
        "none of its buttons takes a click"
    );
    assert_eq!(
        regions.hit(x, second + 1.0),
        Some(&Target::Device(1, DeviceAction::Connect)),
        "the other device does not wait"
    );

    // The spinner takes the icon's square: nothing moves.
    let (_, idle, _) = draw(&adapter(), &devices, None, None, &measure, LINE, &theme);
    let (_, waiting, _) = super::draw(&shown, None, &measure, LINE, &theme);
    assert_eq!(waiting, idle);
}

#[test]
fn a_failure_shows_in_red_until_ok() {
    let theme = theme();
    let message = DeviceAction::Connect.failed("Buds", "Host is down");
    assert_eq!(message, "Connecting Buds failed: Host is down");
    let header = Header::Failed(&message);
    assert_eq!(header.typed(), None);
    let (list, _, regions) = draw(&adapter(), &[], Some(header), None, &measure, LINE, &theme);
    assert!(list.items.iter().any(|item| matches!(
        item,
        Item::Text { text, colour, .. } if *text == message && *colour == theme.battery_critical
    )));
    let answers: Vec<Target> = regions
        .targets()
        .copied()
        .filter(|target| matches!(target, Target::Answer(_)))
        .collect();
    assert_eq!(answers, [Target::Answer(true)], "OK alone");
}

#[test]
fn an_icon_button_s_tooltip_is_its_word() {
    assert_eq!(
        Target::Device(0, DeviceAction::Disconnect).tooltip(),
        Some("Disconnect")
    );
    assert_eq!(Target::Forget(2).tooltip(), Some("Forget"));
    assert_eq!(Target::Scan.tooltip(), None, "labelled already");
    assert_eq!(Target::Answer(true).tooltip(), None, "labelled already");
}
