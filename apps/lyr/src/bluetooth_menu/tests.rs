use crcbl_shell::{
    ButtonState, HeadlessShell, KeyCode, PhysicalPoint, PointerButton, Shell, ShellEvent,
    TextPurpose,
};
use lyr_config::{Platform, Theme, from_sources};
use lyr_core::bluetooth_menu::{
    Adapter, Ask, Device, PADDING, PIN_LENGTH, Question, Reply, Switch, Target,
};

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
    let menu =
        BluetoothMenu::open(&mut shell, 16.0, &look, &adapter(), devices, None).expect("opened");
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
        .refresh(&mut setup.shell, &look, &adapter(), &renamed, None)
        .expect("refreshed");
    assert_eq!(setup.menu.window, first, "the same size");
    setup
        .menu
        .refresh(
            &mut setup.shell,
            &look,
            &adapter(),
            &[device("Buds"), device("Phone")],
            None,
        )
        .expect("refreshed");
    let second = setup.menu.window;
    assert_ne!(second, first, "taller: recreated");
    assert!(setup.shell.title(first).is_err() && setup.shell.title(second).is_ok());
}

#[test]
fn the_arrow_keys_move_through_the_targets_and_enter_chooses() {
    let mut setup = setup(&[]);
    let window = setup.menu.window;
    let key = |setup: &mut Setup, key| {
        setup.shell.key_press(window, key).expect("a key");
        setup.pump()
    };
    assert_eq!(
        key(&mut setup, KeyCode::Enter),
        Outcome::Kept,
        "nothing yet"
    );
    assert_eq!(key(&mut setup, KeyCode::ArrowDown), Outcome::Redraw);
    assert_eq!(
        key(&mut setup, KeyCode::Enter),
        Outcome::Act(Target::Switch(Switch::Powered)),
        "the first"
    );
    key(&mut setup, KeyCode::ArrowDown);
    assert_eq!(
        key(&mut setup, KeyCode::Enter),
        Outcome::Act(Target::Switch(Switch::Discoverable))
    );
    for _ in 0..10 {
        key(&mut setup, KeyCode::ArrowDown);
    }
    assert_eq!(
        key(&mut setup, KeyCode::Enter),
        Outcome::Act(Target::Scan),
        "stops at the last"
    );
    for _ in 0..10 {
        key(&mut setup, KeyCode::ArrowUp);
    }
    assert_eq!(
        key(&mut setup, KeyCode::Enter),
        Outcome::Act(Target::Switch(Switch::Powered)),
        "and the first"
    );
}

#[test]
fn a_pin_is_typed_with_the_text_input_on_and_sent_with_enter() {
    let mut setup = setup(&[]);
    let window = setup.menu.window;
    assert_eq!(setup.shell.text_input(window).expect("a window"), None);
    let theme = setup.theme.clone();
    let look = Look {
        theme: &theme,
        measure: &measure,
        line_height: 20.0,
    };
    let ask = Ask {
        device: "Kbd",
        question: Question::Pin,
        typed: "",
    };
    setup
        .menu
        .refresh(&mut setup.shell, &look, &adapter(), &[], Some(ask))
        .expect("refreshed");
    let window = setup.menu.window;
    assert_eq!(
        setup.shell.text_input(window).expect("a window"),
        Some(TextPurpose::Pin)
    );
    setup
        .shell
        .key_press(window, KeyCode::Enter)
        .expect("a key");
    assert_eq!(setup.pump(), Outcome::Kept, "nothing typed: no answer");
    setup.shell.commit_text(window, "12\u{8}3").expect("typed");
    assert_eq!(setup.pump(), Outcome::Redraw);
    setup
        .shell
        .key_press(window, KeyCode::Backspace)
        .expect("a key");
    assert_eq!(setup.pump(), Outcome::Redraw);
    setup
        .shell
        .key_press(window, KeyCode::Enter)
        .expect("a key");
    assert_eq!(
        setup.pump(),
        Outcome::Answer(Reply::Pin("12".to_owned())),
        "no control characters, and Backspace took the 3"
    );
    setup
        .shell
        .commit_text(window, &"9".repeat(PIN_LENGTH + 4))
        .expect("typed");
    setup.pump();
    assert_eq!(setup.menu.typed.chars().count(), PIN_LENGTH, "at most");
    setup
        .menu
        .refresh(&mut setup.shell, &look, &adapter(), &[], None)
        .expect("refreshed");
    let window = setup.menu.window;
    assert_eq!(setup.shell.text_input(window).expect("a window"), None);
    assert!(setup.menu.typed.is_empty(), "the question went");
}

#[test]
fn a_new_question_of_the_same_size_still_turns_the_text_input_on() {
    // A long name holds the width, so the menu keeps its size and window.
    let devices = [device(&"A".repeat(60))];
    let mut setup = setup(&devices);
    let theme = setup.theme.clone();
    let look = Look {
        theme: &theme,
        measure: &measure,
        line_height: 20.0,
    };
    let ask = |question| Ask {
        device: "Kbd",
        question,
        typed: "",
    };
    setup
        .menu
        .refresh(
            &mut setup.shell,
            &look,
            &adapter(),
            &devices,
            Some(ask(Question::Authorize)),
        )
        .expect("refreshed");
    let window = setup.menu.window;
    assert_eq!(setup.shell.text_input(window).expect("a window"), None);
    setup
        .menu
        .refresh(
            &mut setup.shell,
            &look,
            &adapter(),
            &devices,
            Some(ask(Question::Pin)),
        )
        .expect("refreshed");
    assert_eq!(setup.menu.window, window, "the same size");
    assert_eq!(
        setup.shell.text_input(window).expect("a window"),
        Some(TextPurpose::Pin)
    );
}

#[test]
fn a_paired_device_is_renamed_through_a_field_and_escape_cancels_it() {
    let devices = [device("Buds")];
    let mut setup = setup(&devices);
    let theme = setup.theme.clone();
    let look = Look {
        theme: &theme,
        measure: &measure,
        line_height: 20.0,
    };
    let refresh = |setup: &mut Setup| {
        setup
            .menu
            .refresh(&mut setup.shell, &look, &adapter(), &devices, None)
            .expect("refreshed");
        let window = setup.menu.window;
        setup.shell.text_input(window).expect("a window")
    };
    assert_eq!(setup.menu.act(Target::Rename(0)), Outcome::Redraw);
    assert_eq!(refresh(&mut setup), Some(TextPurpose::Normal));
    let window = setup.menu.window;
    setup
        .shell
        .commit_text(window, "Desk speakers")
        .expect("typed");
    assert_eq!(setup.pump(), Outcome::Redraw);
    assert_eq!(
        setup.menu.typed.chars().count(),
        13,
        "a name is not cut at a PIN's length"
    );
    setup
        .shell
        .key_press(window, KeyCode::Enter)
        .expect("a key");
    assert_eq!(
        setup.pump(),
        Outcome::Rename(devices[0].path.clone(), "Desk speakers".to_owned())
    );
    assert_eq!(refresh(&mut setup), None, "sent: the field goes");
    setup.menu.act(Target::Rename(0));
    refresh(&mut setup);
    let window = setup.menu.window;
    setup
        .shell
        .key_press(window, KeyCode::Escape)
        .expect("a key");
    assert_eq!(setup.pump(), Outcome::Redraw, "the rename, not the menu");
    assert_eq!(refresh(&mut setup), None);
    setup.menu.act(Target::Rename(0));
    refresh(&mut setup);
    assert_eq!(
        setup.menu.act(Target::Answer(false)),
        Outcome::Redraw,
        "Cancel"
    );
    assert_eq!(refresh(&mut setup), None);
    assert_eq!(
        setup.menu.act(Target::Rename(5)),
        Outcome::Kept,
        "no such device"
    );
    setup.menu.act(Target::Rename(0));
    refresh(&mut setup);
    setup
        .menu
        .refresh(&mut setup.shell, &look, &adapter(), &[], None)
        .expect("refreshed");
    let window = setup.menu.window;
    assert_eq!(
        (
            setup.menu.renaming.as_ref(),
            setup.shell.text_input(window).expect("a window")
        ),
        (None, None),
        "the device went, and its rename with it"
    );
}

#[test]
fn the_agent_s_question_takes_the_header_from_a_rename() {
    let devices = [device("Buds")];
    let mut setup = setup(&devices);
    let theme = setup.theme.clone();
    let look = Look {
        theme: &theme,
        measure: &measure,
        line_height: 20.0,
    };
    setup.menu.act(Target::Rename(0));
    let ask = Ask {
        device: "Kbd",
        question: Question::Authorize,
        typed: "",
    };
    setup
        .menu
        .refresh(&mut setup.shell, &look, &adapter(), &devices, Some(ask))
        .expect("refreshed");
    assert_eq!(setup.menu.renaming, None);
    let window = setup.menu.window;
    assert_eq!(setup.shell.text_input(window).expect("a window"), None);
    assert_eq!(
        setup.menu.act(Target::Answer(true)),
        Outcome::Answer(Reply::Yes)
    );
}
