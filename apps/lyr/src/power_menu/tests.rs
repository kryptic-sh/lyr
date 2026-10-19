use crcbl_shell::{
    ButtonState, HeadlessShell, KeyCode, PhysicalPoint, PointerButton, Shell, ShellEvent,
    WindowDesc, WindowId,
};
use lyr_config::{Platform, Position, Theme, from_sources};
use lyr_core::hit::Rect;
use lyr_core::power_menu::{Entry, PADDING, entries};

use super::{Outcome, PowerMenu, configured};
use crate::tooltips::Look;

/// Every character is 10 px wide.
fn measure(text: &str) -> f32 {
    text.chars().count() as f32 * 10.0
}

struct Setup {
    shell: HeadlessShell,
    theme: Theme,
    menu: PowerMenu,
}

fn setup() -> Setup {
    let mut shell = HeadlessShell::new();
    let bar = shell
        .create_window(&WindowDesc {
            title: "lyr",
            app_id: "lyr",
            ..WindowDesc::default()
        })
        .expect("a bar");
    let theme = from_sources(None, Platform::Linux)
        .expect("the defaults")
        .theme;
    let anchor = Rect {
        x: 10.0,
        y: 10.0,
        width: 50.0,
        height: 38.0,
    };
    let menu = PowerMenu::open(
        &mut shell,
        bar,
        anchor,
        Position::Top,
        entries(Platform::Linux),
        &Look {
            theme: &theme,
            measure: &measure,
            measure_in: &|text, _| measure(text),
            line_height: 20.0,
            calendar_line_height: 20.0,
        },
    )
    .expect("opened")
    .expect("a menu");
    assert!(menu.opened_from(bar) && !menu.owns(bar));
    Setup { shell, theme, menu }
}

impl Setup {
    fn window(&self) -> WindowId {
        self.menu.window
    }

    /// Feeds the menu every input and close event queued for it, returning
    /// the last outcome. Sizing events are left out: they open a GPU surface.
    fn pump(&mut self) -> Outcome {
        let mut events = Vec::new();
        self.shell.pump(&mut |event| events.push(event));
        let look = Look {
            theme: &self.theme,
            measure: &measure,
            measure_in: &|text, _| measure(text),
            line_height: 20.0,
            calendar_line_height: 20.0,
        };
        let mut outcome = Outcome::Kept;
        for event in events.iter().filter(|event| {
            !matches!(
                event,
                ShellEvent::Resized { .. } | ShellEvent::ScaleFactorChanged { .. }
            )
        }) {
            outcome = self
                .menu
                .event(&mut self.shell, event, &look)
                .expect("applied");
        }
        outcome
    }

    fn key(&mut self, key: KeyCode) -> Outcome {
        let window = self.window();
        self.shell.key_press(window, key).expect("a key");
        self.pump()
    }
}

/// The middle of the row at `index`, in the menu's pixels.
fn row(index: usize) -> PhysicalPoint {
    // Rows are 20 + 2 * 6 px tall, inside the menu's padding.
    PhysicalPoint {
        x: f64::from(PADDING) + 5.0,
        y: f64::from(PADDING) + 32.0 * index as f64 + 16.0,
    }
}

#[test]
fn the_arrow_keys_move_and_enter_chooses() {
    let mut setup = setup();
    assert_eq!(setup.key(KeyCode::ArrowDown), Outcome::Kept);
    assert!(setup.menu.dirty(), "the selection is redrawn");
    setup.key(KeyCode::ArrowDown);
    setup.key(KeyCode::ArrowUp);
    assert_eq!(setup.key(KeyCode::Enter), Outcome::Chosen(Entry::Sleep));
    assert_eq!(
        setup.key(KeyCode::Escape),
        Outcome::Dismissed,
        "crcbl's grab closes on Escape"
    );
}

#[test]
fn the_pointer_selects_and_a_click_chooses() {
    let mut setup = setup();
    let window = setup.window();
    setup
        .shell
        .move_pointer(window, row(3), (0.0, 0.0))
        .expect("moved");
    assert_eq!(setup.pump(), Outcome::Kept);
    assert_eq!(setup.key(KeyCode::Enter), Outcome::Chosen(Entry::Shutdown));
    setup
        .shell
        .move_pointer(window, row(5), (0.0, 0.0))
        .expect("moved");
    assert_eq!(setup.pump(), Outcome::Kept);
    setup
        .shell
        .button(
            window,
            PointerButton::Left,
            ButtonState::Pressed,
            Some(row(5)),
        )
        .expect("clicked");
    assert_eq!(setup.pump(), Outcome::Chosen(Entry::Reboot));
}

#[test]
fn a_close_request_dismisses_it_and_closes_its_window() {
    let mut setup = setup();
    let window = setup.window();
    setup.shell.request_close(window).expect("asked");
    assert_eq!(setup.pump(), Outcome::Dismissed);
    let Setup {
        mut shell, menu, ..
    } = setup;
    menu.close(&mut shell, true).expect("closed");
    let mut destroyed = false;
    shell.pump(&mut |event| {
        destroyed |=
            matches!(event, ShellEvent::WindowDestroyed { window: gone } if gone == window);
    });
    assert!(destroyed, "the reply closed it");
}

#[test]
fn closing_destroys_its_window() {
    let Setup {
        mut shell, menu, ..
    } = setup();
    let window = menu.window;
    menu.close(&mut shell, false).expect("closed");
    assert!(shell.title(window).is_err());
}

#[test]
fn each_entry_reads_its_own_command() {
    let exec = |text: &str| {
        Some(lyr_config::Exec {
            exec: text.to_owned(),
        })
    };
    let menu = lyr_config::PowerMenu {
        lock: exec("lock"),
        sleep: exec("sleep"),
        hibernate: exec("hibernate"),
        shutdown: exec("shutdown"),
        logout: exec("logout"),
        reboot: exec("reboot"),
    };
    for entry in entries(Platform::Linux) {
        assert_eq!(
            configured(&menu, entry).map(|command| command.exec.as_str()),
            Some(entry.label().to_lowercase().as_str())
        );
    }
    assert_eq!(
        configured(&lyr_config::PowerMenu::default(), Entry::Lock),
        None
    );
}
