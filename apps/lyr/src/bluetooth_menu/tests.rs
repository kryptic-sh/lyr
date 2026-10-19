use crcbl_shell::{
    ButtonState, HeadlessShell, KeyCode, PhysicalPoint, PointerButton, Shell, ShellEvent,
};
use lyr_config::{Platform, Theme, from_sources};
use lyr_core::bluetooth_menu::{Adapter, Device, PADDING, Switch, Target};

use super::{BluetoothMenu, Outcome};
use crate::tooltips::Look;

/// Every character is 10 px wide.
fn measure(text: &str) -> f32 {
    text.chars().count() as f32 * 10.0
}

fn adapter() -> Adapter {
    Adapter {
        path: "/org/bluez/hci0".to_owned(),
        powered: true,
        discoverable: false,
        pairable: false,
        discovering: false,
        power_state: None,
    }
}

fn device(alias: &str) -> Device {
    Device {
        path: format!("/org/bluez/hci0/{alias}"),
        alias: alias.to_owned(),
        connected: false,
        paired: true,
        trusted: false,
        battery: None,
    }
}

struct Setup {
    shell: HeadlessShell,
    theme: Theme,
    menu: BluetoothMenu,
}

impl Setup {
    /// Feeds the menu its queued input events, sizing ones left out (they
    /// open a GPU surface), and returns the last outcome.
    fn pump(&mut self) -> Outcome {
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
}

fn setup(devices: &[Device]) -> Setup {
    let mut shell = HeadlessShell::new();
    let theme = from_sources(None, Platform::Linux)
        .expect("the defaults")
        .theme;
    let look = Look {
        theme: &theme,
        measure: &measure,
        line_height: 20.0,
    };
    let menu = BluetoothMenu::open(&mut shell, 16.0, &look, &adapter(), devices).expect("opened");
    Setup { shell, theme, menu }
}

#[test]
fn a_click_on_a_switch_is_the_caller_s_and_escape_closes() {
    let mut setup = setup(&[]);
    let window = setup.menu.window;
    // The power switch, at the right of the first row.
    let at = PhysicalPoint {
        x: f64::from(setup.menu.size.width - PADDING - 2.0),
        y: f64::from(PADDING + 2.0),
    };
    setup
        .shell
        .move_pointer(window, at, (0.0, 0.0))
        .expect("moved");
    setup
        .shell
        .button(window, PointerButton::Left, ButtonState::Pressed, Some(at))
        .expect("clicked");
    assert_eq!(setup.pump(), Outcome::Act(Target::Switch(Switch::Powered)));
    setup
        .shell
        .key_press(window, KeyCode::Escape)
        .expect("a key");
    assert_eq!(setup.pump(), Outcome::Close);
}

#[test]
fn a_refresh_keeps_the_window_unless_the_size_changes() {
    let mut setup = setup(&[device("Buds")]);
    let first = setup.menu.window;
    let renamed = [Device {
        alias: "Bud2".to_owned(),
        ..device("Buds")
    }];
    let theme = setup.theme.clone();
    let look = Look {
        theme: &theme,
        measure: &measure,
        line_height: 20.0,
    };
    setup
        .menu
        .refresh(&mut setup.shell, &look, &adapter(), &renamed)
        .expect("refreshed");
    assert_eq!(setup.menu.window, first, "the same size");
    setup
        .menu
        .refresh(
            &mut setup.shell,
            &look,
            &adapter(),
            &[device("Buds"), device("Phone")],
        )
        .expect("refreshed");
    let second = setup.menu.window;
    assert_ne!(second, first, "taller: recreated");
    assert!(setup.shell.title(first).is_err() && setup.shell.title(second).is_ok());
}
