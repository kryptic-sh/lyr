use chrono::NaiveDate;
use crcbl_shell::{
    ButtonState, HeadlessShell, KeyCode, PhysicalPoint, PointerButton, ScrollDelta, Shell,
    ShellEvent,
};
use lyr_config::{Platform, Theme, from_sources};
use lyr_core::calendar::Mode;

use super::{ClockPanel, Outcome};
use crate::tooltips::Look;

/// Every character is 10 px wide.
fn measure(text: &str) -> f32 {
    text.chars().count() as f32 * 10.0
}

struct Setup {
    shell: HeadlessShell,
    theme: Theme,
    panel: ClockPanel,
}

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 10, 19).expect("a date")
}

fn setup() -> Setup {
    let mut shell = HeadlessShell::new();
    let theme = from_sources(None, Platform::Linux)
        .expect("the defaults")
        .theme;
    let panel = ClockPanel::open(
        &mut shell,
        &Look {
            theme: &theme,
            measure: &measure,
            line_height: 20.0,
        },
        today(),
    )
    .expect("opened");
    Setup {
        shell,
        theme,
        panel,
    }
}

impl Setup {
    /// Feeds the panel its queued events, sizing ones left out (they open a
    /// GPU surface), and returns the last outcome.
    fn pump(&mut self) -> Outcome {
        let mut events = Vec::new();
        self.shell.pump(&mut |event| events.push(event));
        let look = Look {
            theme: &self.theme,
            measure: &measure,
            line_height: 20.0,
        };
        let mut outcome = Outcome::Kept;
        for event in events.iter().filter(|event| {
            !matches!(
                event,
                ShellEvent::Resized { .. } | ShellEvent::ScaleFactorChanged { .. }
            )
        }) {
            outcome = self
                .panel
                .event(&mut self.shell, event, &look, today())
                .expect("applied");
        }
        outcome
    }
}

#[test]
fn it_opens_as_wide_as_swaync_s_centre_or_the_year() {
    let setup = setup();
    // Three 240 px month blocks and two 30 px gaps are 780 px, more than 500.
    assert_eq!(setup.panel.size.width, 780.0 + 2.0 * (16.0 + 2.0));
    assert_eq!((setup.panel.mode, setup.panel.shown), (Mode::Year, today()));
    assert!(setup.shell.title(setup.panel.window).is_ok());
}

#[test]
fn escape_and_losing_focus_close_it() {
    let mut setup = setup();
    let window = setup.panel.window;
    setup
        .shell
        .key_press(window, KeyCode::ArrowDown)
        .expect("a key");
    assert_eq!(setup.pump(), Outcome::Kept);
    setup
        .shell
        .key_press(window, KeyCode::Escape)
        .expect("a key");
    assert_eq!(setup.pump(), Outcome::Close);
    setup.shell.set_focus(window, true).expect("focused");
    assert_eq!(setup.pump(), Outcome::Kept);
    setup.shell.set_focus(window, false).expect("unfocused");
    assert_eq!(setup.pump(), Outcome::Close);
}

#[test]
fn a_close_request_closes_its_window() {
    let mut setup = setup();
    let window = setup.panel.window;
    setup.shell.request_close(window).expect("asked");
    assert_eq!(setup.pump(), Outcome::Closed);
}

#[test]
fn a_right_click_switches_the_mode_and_scrolling_moves_by_it() {
    let mut setup = setup();
    let window = setup.panel.window;
    let at = PhysicalPoint { x: 50.0, y: 50.0 };
    setup
        .shell
        .move_pointer(window, at, (0.0, 0.0))
        .expect("moved");
    setup
        .shell
        .scroll(window, ScrollDelta::Lines { x: 0.0, y: 1.0 }, Some(at))
        .expect("scrolled");
    setup.pump();
    assert_eq!(
        setup.panel.shown,
        NaiveDate::from_ymd_opt(2027, 10, 1).expect("a date"),
        "a year in year mode"
    );
    setup
        .shell
        .button(window, PointerButton::Right, ButtonState::Pressed, Some(at))
        .expect("clicked");
    setup.pump();
    assert_eq!(setup.panel.mode, Mode::Month);
    setup
        .shell
        .scroll(window, ScrollDelta::Lines { x: 0.0, y: -1.0 }, Some(at))
        .expect("scrolled");
    setup.pump();
    assert_eq!(
        setup.panel.shown,
        NaiveDate::from_ymd_opt(2027, 9, 1).expect("a date"),
        "a month in month mode"
    );
    assert!(setup.panel.dirty(), "redrawn");
}

#[test]
fn closing_destroys_its_window() {
    let Setup {
        mut shell, panel, ..
    } = setup();
    let window = panel.window;
    panel.close(&mut shell, false).expect("closed");
    assert!(shell.title(window).is_err());
}
