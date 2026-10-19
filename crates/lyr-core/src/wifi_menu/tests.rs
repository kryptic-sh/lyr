use lyr_config::{Platform, Theme, from_sources};

use super::{
    AccessPoint, Header, Network, Radio, Scan, Security, Shown, Target, draw, networks,
    password_fits,
};
use crate::draw::Item;
use crate::menu::{GAP, PADDING};

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

#[test]
fn security_comes_from_the_key_management_bits() {
    assert_eq!(Security::of(0, 0, 0), Security::Open);
    assert_eq!(Security::of(1, 0, 0), Security::Wep, "privacy alone");
    assert_eq!(Security::of(1, 0x100, 0), Security::Psk);
    assert_eq!(
        Security::of(1, 0, 0x100 | 0x400),
        Security::Psk,
        "WPA3 transition"
    );
    assert_eq!(Security::of(1, 0, 0x400), Security::Sae, "WPA3 alone");
    assert_eq!(Security::of(1, 0, 0x800), Security::Owe);
    assert_eq!(
        Security::of(0, 0, 0x1000),
        Security::Open,
        "OWE transition's open side"
    );
    assert_eq!(
        Security::of(1, 0, 0x1000),
        Security::Open,
        "WEP is privacy with no WPA or RSN bits at all"
    );
    assert_eq!(Security::of(1, 0x200, 0x100), Security::Enterprise);
    assert!(Security::Wep.refusal().is_some());
    assert!(Security::Enterprise.refusal().is_some());
    assert!(Security::Sae.refusal().is_none());
    assert!(Security::Psk.needs_password() && Security::Sae.needs_password());
    assert!(!Security::Owe.needs_password() && !Security::Open.needs_password());
}

#[test]
fn a_password_fits_as_networkmanager_counts_it() {
    assert!(!password_fits(Security::Psk, "1234567"));
    assert!(password_fits(Security::Psk, "12345678"));
    assert!(password_fits(Security::Psk, &"a".repeat(63)));
    assert!(!password_fits(Security::Psk, &"g".repeat(64)));
    assert!(
        password_fits(Security::Psk, &"aF".repeat(32)),
        "64 hex digits"
    );
    assert!(!password_fits(Security::Psk, &"a".repeat(65)));
    // Bytes, not characters: four three-byte characters are twelve bytes.
    assert!(password_fits(Security::Psk, "\u{20AC}\u{20AC}\u{20AC}"));
    assert!(password_fits(Security::Sae, "x"));
    assert!(!password_fits(Security::Sae, ""));
    assert!(password_fits(Security::Open, ""));
}

fn point(path: &str, ssid: &str, strength: u8, security: Security) -> AccessPoint {
    AccessPoint {
        path: path.to_owned(),
        ssid: ssid.to_owned(),
        strength,
        security,
    }
}

#[test]
fn one_row_per_ssid_strongest_first_and_most_demanding() {
    let points = [
        point("/ap/3", "home", 80, Security::Open),
        point("/ap/2", "cafe", 60, Security::Open),
        point("/ap/1", "home", 40, Security::Psk),
        point("/ap/4", "", 99, Security::Open),
        point("/ap/5", "attic", 60, Security::Sae),
    ];
    let rows = networks(&points, Some("/ap/1"), &["home".to_owned()]);
    let names: Vec<&str> = rows.iter().map(|row| row.ssid.as_str()).collect();
    assert_eq!(names, ["home", "attic", "cafe"], "no hidden network");
    let home = &rows[0];
    assert_eq!(home.strength, 80);
    assert_eq!(home.point, "/ap/3", "the strongest access point");
    assert_eq!(home.security, Security::Psk, "an open twin cannot lower it");
    assert!(home.in_use, "in use through its weaker access point");
    assert!(home.known);
    assert!(!rows[1].in_use && !rows[1].known);
}

fn network(ssid: &str, in_use: bool, known: bool) -> Network {
    Network {
        ssid: ssid.to_owned(),
        strength: 50,
        security: Security::Psk,
        point: format!("/ap/{ssid}"),
        in_use,
        known,
    }
}

const ON: Radio = Radio {
    enabled: true,
    hardware_enabled: true,
};

fn shown<'a>(networks: &'a [Network], header: Option<Header<'a>>) -> Shown<'a> {
    Shown {
        radio: ON,
        scan: Scan::Ready,
        networks,
        header,
    }
}

/// The text drawn, a field's included, in order.
fn texts(list: &crate::draw::DrawList) -> Vec<String> {
    fn walk(items: &[Item], texts: &mut Vec<String>) {
        for item in items {
            match item {
                Item::Text { text, .. } => texts.push(text.clone()),
                Item::Clip { items, .. } => walk(items, texts),
                _ => {}
            }
        }
    }
    let mut texts = Vec::new();
    walk(&list.items, &mut texts);
    texts
}

#[test]
fn each_network_has_its_row_and_buttons() {
    let rows = [network("home", true, true), network("cafe", false, false)];
    let (list, size, regions) = draw(&shown(&rows, None), None, &measure, LINE, &theme());
    let texts = texts(&list);
    assert!(texts.contains(&"\u{F00C} home 50%".to_owned()));
    assert!(texts.contains(&"cafe 50%".to_owned()));
    let targets: Vec<Target> = regions.targets().copied().collect();
    assert_eq!(
        targets,
        [Target::Radio, Target::Forget(0), Target::Join(1)],
        "no Connect on the network in use, no Forget on an unknown one"
    );
    // The switch, then a gap before the rows, each a row and a gap apart.
    let first = PADDING + ROW + 2.0 * GAP;
    let join = regions.rect(&Target::Join(1)).expect("drawn");
    assert_eq!(join.y, first + ROW + GAP);
    assert_eq!(size.height, join.y + ROW + PADDING);
}

#[test]
fn the_list_waits_for_the_radio_and_the_scan() {
    let rows = [network("home", false, false)];
    let status = |radio, scan| {
        let (list, _, regions) = draw(
            &Shown {
                radio,
                scan,
                ..shown(&rows, None)
            },
            None,
            &measure,
            LINE,
            &theme(),
        );
        let joinable = regions.targets().any(|target| *target == Target::Join(0));
        // What follows the switch's label: the status or the first row.
        let texts = texts(&list);
        let after = texts
            .iter()
            .position(|text| text == "Wi-Fi")
            .expect("the switch")
            + 1;
        (texts.get(after).cloned().unwrap_or_default(), joinable)
    };
    assert_eq!(status(ON, Scan::Ready), ("home 50%".to_owned(), true));
    assert_eq!(
        status(ON, Scan::Scanning),
        ("Scanning\u{2026}".to_owned(), false)
    );
    assert!(status(ON, Scan::Unavailable).0.contains("unavailable"));
    let off = Radio {
        enabled: false,
        ..ON
    };
    assert_eq!(status(off, Scan::Ready), ("Wi-Fi is off".to_owned(), false));
    let blocked = Radio {
        hardware_enabled: false,
        ..ON
    };
    assert!(status(blocked, Scan::Ready).0.contains("hardware switch"));
}

#[test]
fn a_hardware_block_leaves_the_switch_alone() {
    let blocked = Shown {
        radio: Radio {
            enabled: true,
            hardware_enabled: false,
        },
        ..shown(&[], None)
    };
    let (_, _, regions) = draw(&blocked, None, &measure, LINE, &theme());
    assert_eq!(regions.targets().count(), 0);
}

#[test]
fn the_password_is_masked_and_answered() {
    let rows = [network("cafe", false, false)];
    let header = Header::Password {
        ssid: "cafe",
        typed: "secret 1",
        again: false,
    };
    let (list, _, regions) = draw(&shown(&rows, Some(header)), None, &measure, LINE, &theme());
    let texts = texts(&list);
    assert!(texts.contains(&"Password for cafe".to_owned()));
    assert!(texts.contains(&"\u{2022}".repeat(8)), "{texts:?}");
    assert!(!texts.iter().any(|text| text.contains("secret")));
    let ok = regions.rect(&Target::Answer(true)).expect("OK");
    let cancel = regions.rect(&Target::Answer(false)).expect("Cancel");
    assert!(ok.x < cancel.x);
    let again = Header::Password {
        ssid: "cafe",
        typed: "",
        again: true,
    };
    assert_eq!(again.text(), "Joining cafe failed. New password");
}

#[test]
fn rows_take_no_clicks_while_joining() {
    let rows = [network("cafe", false, true)];
    let (list, _, regions) = draw(
        &shown(&rows, Some(Header::Joining { ssid: "cafe" })),
        None,
        &measure,
        LINE,
        &theme(),
    );
    assert!(texts(&list).contains(&"Joining cafe\u{2026}".to_owned()));
    let targets: Vec<Target> = regions.targets().copied().collect();
    assert_eq!(targets, [Target::Radio]);
}

#[test]
fn an_error_is_shown_in_red_until_ok() {
    let theme = theme();
    let (list, _, regions) = draw(
        &shown(&[], Some(Header::Error("no luck"))),
        None,
        &measure,
        LINE,
        &theme,
    );
    let colour = list.items.iter().find_map(|item| match item {
        Item::Text { text, colour, .. } if text == "no luck" => Some(*colour),
        _ => None,
    });
    assert_eq!(colour, Some(theme.battery_critical));
    let targets: Vec<Target> = regions.targets().copied().collect();
    assert_eq!(targets, [Target::Answer(true), Target::Radio]);
    let forget = Header::Forget { ssid: "home" };
    assert_eq!(forget.text(), "Forget home?");
}

#[test]
fn the_focus_is_outlined() {
    let theme = theme();
    let rows = [network("cafe", false, false)];
    let (list, _, regions) = draw(
        &shown(&rows, None),
        Some(Target::Join(0)),
        &measure,
        LINE,
        &theme,
    );
    let join = regions.rect(&Target::Join(0)).expect("drawn");
    assert!(matches!(
        list.items.last(),
        Some(Item::Box { rect, border: Some((_, colour)), .. })
            if *rect == join && *colour == theme.workspace_active
    ));
}
