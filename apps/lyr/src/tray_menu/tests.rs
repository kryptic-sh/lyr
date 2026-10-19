use crcbl_shell::{HeadlessShell, KeyCode, Shell, ShellEvent, WindowDesc, WindowId};
use lyr_config::{Platform, Position, Theme, from_sources};
use lyr_core::hit::Rect;
use lyr_core::tray::menu::Entry;

use super::{Outcome, Read, TrayMenu};
use crate::tooltips::Look;

/// Every character is 10 px wide.
fn measure(text: &str) -> f32 {
    text.chars().count() as f32 * 10.0
}

fn look(theme: &Theme) -> Look<'_> {
    Look {
        theme,
        measure: &measure,
        measure_in: &|text, _| measure(text),
        line_height: 20.0,
        calendar_line_height: 20.0,
    }
}

/// "Open" (1), then "More" (4) holding "Sync" (5).
fn root() -> Entry {
    let mut more = Entry {
        label: "More".to_owned(),
        ..Entry::new(4)
    };
    more.children.push(Entry {
        label: "Sync".to_owned(),
        ..Entry::new(5)
    });
    let mut root = Entry::new(0);
    root.children = vec![
        Entry {
            label: "Open".to_owned(),
            ..Entry::new(1)
        },
        more,
    ];
    root
}

struct Setup {
    shell: HeadlessShell,
    theme: Theme,
    menu: TrayMenu,
    bar: WindowId,
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
    let read = Read {
        bus_name: ":1.7".to_owned(),
        menu: "/MenuBar".to_owned(),
        root: root(),
        bar,
        anchor: Rect {
            x: 400.0,
            y: 6.0,
            width: 24.0,
            height: 38.0,
        },
    };
    let menu = TrayMenu::open(&mut shell, read, Position::Top, &look(&theme))
        .expect("opened")
        .expect("a menu");
    Setup {
        shell,
        theme,
        menu,
        bar,
    }
}

impl Setup {
    fn key(&mut self, key: KeyCode) -> Outcome {
        let window = self.menu.popup.window();
        self.shell.key_press(window, key).expect("a key");
        let mut events = Vec::new();
        self.shell.pump(&mut |event| events.push(event));
        let theme = self.theme.clone();
        let mut outcome = Outcome::Kept;
        for event in events.iter().filter(|event| {
            !matches!(
                event,
                ShellEvent::Resized { .. } | ShellEvent::ScaleFactorChanged { .. }
            )
        }) {
            if event.window() == Some(self.menu.popup.window()) {
                outcome = self
                    .menu
                    .event(&mut self.shell, event, &look(&theme))
                    .expect("applied");
            }
        }
        outcome
    }
}

#[test]
fn an_entry_is_clicked_with_enter() {
    let mut setup = setup();
    assert_eq!(setup.menu.item(), (":1.7", "/MenuBar"));
    assert!(setup.menu.opened_from(setup.bar));
    assert_eq!(setup.key(KeyCode::Enter), Outcome::Clicked(1));
}

#[test]
fn a_submenu_opens_in_place_and_back_leaves_it() {
    let mut setup = setup();
    let first = setup.menu.popup.window();
    setup.key(KeyCode::ArrowDown);
    assert_eq!(setup.key(KeyCode::Enter), Outcome::Kept, "into More");
    let second = setup.menu.popup.window();
    assert_ne!(second, first, "opened again");
    assert!(setup.shell.title(first).is_err(), "the first popup is gone");
    // Back, then Sync.
    setup.key(KeyCode::ArrowDown);
    assert_eq!(setup.key(KeyCode::Enter), Outcome::Clicked(5));
    setup.key(KeyCode::ArrowUp);
    assert_eq!(setup.key(KeyCode::Enter), Outcome::Kept, "Back");
    assert_eq!(
        setup.key(KeyCode::Enter),
        Outcome::Clicked(1),
        "the top again"
    );
}

#[test]
fn escape_closes_it() {
    let mut setup = setup();
    assert_eq!(setup.key(KeyCode::Escape), Outcome::Closed(true));
}
