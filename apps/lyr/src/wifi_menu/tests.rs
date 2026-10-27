use std::time::{Duration, Instant};

use crcbl_shell::{HeadlessShell, KeyCode, Shell, ShellEvent, TextPurpose};
use lyr_config::{Platform, Theme, from_sources};
use lyr_core::wifi_menu::{AccessPoint, Profile, Radio, Scan, Security, Wifi};

use super::{Joined, Outcome, SCAN_WAIT, WifiMenu};
use crate::menu_panel::Place;
use crate::tooltips::Look;

/// Every character is 10 px wide.
fn measure(text: &str) -> f32 {
    text.chars().count() as f32 * 10.0
}

fn point(path: &str, ssid: &str, strength: u8, security: Security) -> AccessPoint {
    AccessPoint {
        path: path.to_owned(),
        ssid: ssid.to_owned(),
        strength,
        security,
    }
}

/// A device that sees "cafe" (WPA2, unknown), "home" (WPA2, saved) and
/// "attic" (WEP), strongest in that order, last scanned 100 s after boot.
fn wifi() -> Wifi {
    Wifi {
        device: Some("/dev/wifi".to_owned()),
        radio: Radio {
            enabled: true,
            hardware_enabled: true,
        },
        available: true,
        last_scan: Some(Duration::from_secs(100)),
        points: vec![
            point("/ap/1", "cafe", 90, Security::Psk),
            point("/ap/2", "home", 80, Security::Psk),
            point("/ap/3", "attic", 70, Security::Wep),
        ],
        active: None,
        profiles: vec![Profile {
            path: "/settings/1".to_owned(),
            ssid: "home".to_owned(),
        }],
    }
}

struct Setup {
    shell: HeadlessShell,
    theme: Theme,
    menu: WifiMenu,
    opened: Instant,
    /// What NetworkManager reports, which every redraw shows.
    wifi: Wifi,
}

impl Setup {
    /// Presses `key` and feeds the menu its queued input events, sizing
    /// ones left out (they open a GPU surface), returning the last outcome.
    fn key(&mut self, key: KeyCode) -> Outcome {
        let window = self.menu.panel.window();
        self.shell.key_press(window, key).expect("a key");
        self.pump()
    }

    /// As the bar does, the menu is drawn again after whatever it did.
    fn pump(&mut self) -> Outcome {
        let outcome = self.events();
        if !matches!(outcome, Outcome::Close | Outcome::Closed) {
            let wifi = self.wifi.clone();
            self.refresh(&wifi, self.opened);
        }
        outcome
    }

    fn events(&mut self) -> Outcome {
        let mut events = Vec::new();
        self.shell.pump(&mut |event| events.push(event));
        let mut outcome = Outcome::Kept;
        for event in events.iter().filter(|event| {
            !matches!(
                event,
                ShellEvent::Resized { .. } | ShellEvent::ScaleFactorChanged { .. }
            )
        }) {
            outcome = self.menu.event(&mut self.shell, event).expect("applied");
        }
        outcome
    }

    /// Moves the outline to the `nth` target, in the order drawn, and
    /// presses Enter.
    fn choose(&mut self, nth: usize) -> Outcome {
        // Up past the first target, wherever the outline is.
        for _ in 0..16 {
            self.key(KeyCode::ArrowUp);
        }
        for _ in 0..nth {
            self.key(KeyCode::ArrowDown);
        }
        self.key(KeyCode::Enter)
    }

    fn refresh(&mut self, wifi: &Wifi, now: Instant) {
        let theme = self.theme.clone();
        let look = look(&theme);
        self.menu
            .refresh(&mut self.shell, &look, wifi, now)
            .expect("refreshed");
    }

    fn typing(&self) -> Option<TextPurpose> {
        self.shell
            .text_input(self.menu.panel.window())
            .expect("a window")
    }
}

fn look(theme: &Theme) -> Look<'_> {
    Look {
        theme,
        measure: &measure,
        measure_in: &|text, _| measure(text),
        lay_out: &|_, _| Vec::new(),
        line_height: 20.0,
        calendar_line_height: 20.0,
    }
}

/// The menu opened on `wifi` 110 s after boot: its last scan, at 100 s, is
/// fresh.
fn setup(wifi: &Wifi) -> Setup {
    let mut shell = HeadlessShell::new();
    let theme = from_sources(None, Platform::Linux)
        .expect("the defaults")
        .theme;
    let opened = Instant::now();
    let menu = WifiMenu::open(
        &mut shell,
        Place::EDGE,
        &look(&theme),
        wifi,
        opened,
        Some(Duration::from_secs(110)),
    )
    .expect("opened");
    Setup {
        shell,
        theme,
        menu,
        opened,
        wifi: wifi.clone(),
    }
}

// The targets in the order drawn: the radio, then cafe's Connect, home's
// Connect and Forget, and attic's Connect.
const RADIO: usize = 0;
const CAFE: usize = 1;
const HOME: usize = 2;
const FORGET_HOME: usize = 3;
const ATTIC: usize = 4;

#[test]
fn a_new_secured_network_asks_for_its_password() {
    let mut setup = setup(&wifi());
    assert_eq!(setup.choose(CAFE), Outcome::Redraw);
    assert_eq!(setup.typing(), Some(TextPurpose::Password));
    let window = setup.menu.panel.window();
    setup.shell.commit_text(window, " short").expect("typed");
    setup.pump();
    assert_eq!(setup.key(KeyCode::Enter), Outcome::Kept, "under 8 bytes");
    setup.shell.commit_text(window, "er ").expect("typed");
    setup.pump();
    let Outcome::Join(network, password) = setup.key(KeyCode::Enter) else {
        panic!("joined");
    };
    assert_eq!(network.ssid, "cafe");
    assert_eq!(password.as_deref(), Some(" shorter "), "spaces kept");
    setup.refresh(&wifi(), setup.opened);
    assert_eq!(setup.typing(), None, "no field while joining");
}

#[test]
fn a_saved_network_joins_at_once_and_a_failure_shows_until_dismissed() {
    let mut setup = setup(&wifi());
    let Outcome::Join(network, None) = setup.choose(HOME) else {
        panic!("joined with its profile");
    };
    assert_eq!(network.ssid, "home");
    setup
        .menu
        .joined(Joined::Failed("home did not answer".to_owned()));
    setup.refresh(&wifi(), setup.opened);
    assert_eq!(setup.key(KeyCode::Escape), Outcome::Redraw, "dismissed");
    assert_eq!(setup.key(KeyCode::Escape), Outcome::Close);
}

#[test]
fn a_failed_saved_password_is_offered_again() {
    let mut setup = setup(&wifi());
    setup.choose(HOME);
    setup.menu.joined(Joined::NewPassword);
    setup.refresh(&wifi(), setup.opened);
    assert_eq!(setup.typing(), Some(TextPurpose::Password));
    let window = setup.menu.panel.window();
    setup
        .shell
        .commit_text(window, "new secret")
        .expect("typed");
    setup.pump();
    let Outcome::NewPassword(network, password) = setup.key(KeyCode::Enter) else {
        panic!("a new password");
    };
    assert_eq!(
        (network.ssid.as_str(), password.as_str()),
        ("home", "new secret")
    );
}

#[test]
fn a_refused_network_says_why() {
    let mut setup = setup(&wifi());
    assert_eq!(setup.choose(ATTIC), Outcome::Redraw);
    assert_eq!(setup.typing(), None);
    // The error's OK is now the first target.
    assert_eq!(setup.choose(0), Outcome::Redraw, "dismissed");
}

#[test]
fn forget_is_confirmed_and_the_radio_is_the_caller_s() {
    let mut setup = setup(&wifi());
    assert_eq!(setup.choose(FORGET_HOME), Outcome::Redraw);
    setup.refresh(&wifi(), setup.opened);
    // No, then Yes, drawn from the right, come before the radio now.
    assert_eq!(setup.choose(0), Outcome::Redraw, "No");
    assert_eq!(setup.choose(FORGET_HOME), Outcome::Redraw);
    assert_eq!(setup.choose(1), Outcome::Forget("home".to_owned()));
    setup.refresh(&wifi(), setup.opened);
    assert_eq!(setup.choose(RADIO), Outcome::ToggleRadio);
}

#[test]
fn the_network_in_use_disconnects() {
    let mut on_home = wifi();
    on_home.active = Some("/ap/2".to_owned());
    let mut setup = setup(&on_home);
    // The radio, cafe's Connect, then home's Disconnect and Forget.
    assert_eq!(setup.choose(2), Outcome::Disconnect);
    assert_eq!(setup.choose(3), Outcome::Redraw, "Forget, to confirm");
}

#[test]
fn a_join_under_way_takes_no_clicks() {
    let mut setup = setup(&wifi());
    setup.choose(HOME);
    setup.refresh(&wifi(), setup.opened);
    // Only the radio takes the keyboard.
    assert_eq!(setup.choose(CAFE), Outcome::ToggleRadio);
    setup.menu.joined(Joined::Up);
    setup.refresh(&wifi(), setup.opened);
    assert!(matches!(setup.choose(CAFE), Outcome::Redraw));
}

#[test]
fn a_stale_list_waits_for_a_fresh_scan_or_the_cap() {
    let stale = wifi();
    let mut shell = HeadlessShell::new();
    let theme = from_sources(None, Platform::Linux)
        .expect("the defaults")
        .theme;
    let opened = Instant::now();
    let menu = WifiMenu::open(
        &mut shell,
        Place::EDGE,
        &look(&theme),
        &stale,
        opened,
        Some(Duration::from_secs(200)),
    )
    .expect("opened");
    assert_eq!(menu.scan(&stale, opened), Scan::Scanning);
    assert_eq!(menu.next_wake(), Some(opened + SCAN_WAIT));
    let scanned = Wifi {
        last_scan: Some(Duration::from_secs(205)),
        ..wifi()
    };
    assert_eq!(menu.scan(&scanned, opened), Scan::Ready);
    assert_eq!(menu.scan(&stale, opened + SCAN_WAIT), Scan::Ready, "capped");
    let gone = Wifi {
        available: false,
        ..wifi()
    };
    assert_eq!(menu.scan(&gone, opened), Scan::Unavailable);
    // A fresh scan at opening needs no wait.
    let fresh = setup(&wifi());
    assert_eq!(fresh.menu.scan(&stale, fresh.opened), Scan::Ready);
    assert_eq!(menu.opened(), opened);
}

#[test]
fn the_network_joined_stays_while_the_scan_moves_on() {
    let mut setup = setup(&wifi());
    setup.choose(HOME);
    // The scan no longer sees it, as can happen mid-join.
    let moved_on = Wifi {
        points: wifi()
            .points
            .into_iter()
            .filter(|point| point.ssid != "home")
            .collect(),
        ..wifi()
    };
    setup.refresh(&moved_on, setup.opened);
    setup.menu.joined(Joined::NewPassword);
    setup.refresh(&moved_on, setup.opened);
    assert_eq!(setup.typing(), Some(TextPurpose::Password), "still asked");
}

#[test]
fn the_wait_stays_due_until_the_list_is_drawn() {
    let stale = wifi();
    let mut setup = setup(&stale);
    // Opened 110 s after boot on a scan from 100 s: fresh, no wait.
    assert_eq!(setup.menu.next_wake(), None);

    let mut setup = Setup {
        menu: WifiMenu::open(
            &mut setup.shell,
            Place::EDGE,
            &look(&setup.theme),
            &stale,
            setup.opened,
            Some(Duration::from_secs(200)),
        )
        .expect("opened"),
        ..setup
    };
    let deadline = setup.opened + SCAN_WAIT;
    assert_eq!(setup.menu.next_wake(), Some(deadline));
    // The loop wakes after the deadline: it is still due, so it redraws.
    assert_eq!(setup.menu.next_wake(), Some(deadline), "past the cap");
    setup.refresh(&stale, deadline);
    assert_eq!(setup.menu.next_wake(), None, "the list is drawn");

    // A scan landing before the cap ends the wait the same way.
    let mut setup = Setup {
        menu: WifiMenu::open(
            &mut setup.shell,
            Place::EDGE,
            &look(&setup.theme),
            &stale,
            setup.opened,
            Some(Duration::from_secs(200)),
        )
        .expect("opened"),
        ..setup
    };
    let scanned = Wifi {
        last_scan: Some(Duration::from_secs(205)),
        ..wifi()
    };
    setup.refresh(&scanned, setup.opened);
    assert_eq!(setup.menu.next_wake(), None);
}
