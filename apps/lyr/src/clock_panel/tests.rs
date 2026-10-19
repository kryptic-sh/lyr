use std::time::Instant;

use chrono::NaiveDate;
use crcbl_shell::{
    ButtonState, HeadlessShell, KeyCode, PhysicalPoint, PointerButton, ScrollDelta, Shell,
    ShellEvent,
};
use lyr_config::{Platform, Theme, from_sources};
use lyr_core::calendar::Mode;
use lyr_core::centre::Target;
use lyr_core::notifications::Store;
use lyr_core::pointer::Press;

use super::{ClockPanel, Contents, Outcome};
use crate::tooltips::Look;

/// Every character is 10 px wide.
fn measure(text: &str) -> f32 {
    text.chars().count() as f32 * 10.0
}

struct Setup {
    shell: HeadlessShell,
    theme: Theme,
    panel: ClockPanel,
    /// Present while lyr is the notification daemon.
    store: Option<Store>,
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
        store: None,
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
                .event(
                    &mut self.shell,
                    event,
                    &look,
                    &Contents {
                        today: today(),
                        store: self.store.as_ref(),
                        players: &[],
                        now: Instant::now(),
                    },
                )
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
    setup.lay_out(2000.0);
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

impl Setup {
    /// Lays the panel out as its first resize would, `height` tall.
    fn lay_out(&mut self, height: f32) {
        self.panel.size.height = height;
        let look = Look {
            theme: &self.theme,
            measure: &measure,
            line_height: 20.0,
        };
        self.panel.refresh(
            &look,
            &Contents {
                today: today(),
                store: self.store.as_ref(),
                players: &[],
                now: Instant::now(),
            },
        );
    }
}

#[test]
fn a_click_in_the_centre_is_the_caller_s_and_a_right_click_there_keeps_the_mode() {
    let mut setup = setup();
    setup.store = Some(Store::default());
    setup.lay_out(2000.0);
    let window = setup.panel.window;
    // Clear All sits at the right of the centre's first row.
    let (ox, oy) = setup.panel.centre_origin;
    assert!(oy > setup.panel.calendar_bottom, "below the calendar");
    let width = setup.panel.size.width - 2.0 * ox;
    let at = PhysicalPoint {
        x: f64::from(ox + width - 5.0),
        y: f64::from(oy + 5.0),
    };
    setup
        .shell
        .move_pointer(window, at, (0.0, 0.0))
        .expect("moved");
    setup
        .shell
        .button(window, PointerButton::Left, ButtonState::Pressed, Some(at))
        .expect("clicked");
    assert_eq!(setup.pump(), Outcome::Centre(Target::ClearAll, Press::Left));
    setup
        .shell
        .button(window, PointerButton::Right, ButtonState::Pressed, Some(at))
        .expect("clicked");
    assert_eq!(
        setup.pump(),
        Outcome::Centre(Target::ClearAll, Press::Right)
    );
    assert_eq!(setup.panel.mode, Mode::Year, "not the calendar's click");
}

#[test]
fn without_the_daemon_there_is_no_centre() {
    let mut setup = setup();
    setup.lay_out(2000.0);
    let window = setup.panel.window;
    let at = PhysicalPoint {
        x: 50.0,
        y: f64::from(setup.panel.calendar_bottom + 40.0),
    };
    setup
        .shell
        .move_pointer(window, at, (0.0, 0.0))
        .expect("moved");
    setup
        .shell
        .button(window, PointerButton::Left, ButtonState::Pressed, Some(at))
        .expect("clicked");
    assert_eq!(setup.pump(), Outcome::Kept);
}

#[test]
fn a_tall_history_scrolls_by_three_lines_and_no_further_than_it_goes() {
    let mut setup = setup();
    let mut store = Store::default();
    for summary in 0..20 {
        store.notify(
            lyr_core::notifications::Incoming {
                app_name: format!("app {summary}"),
                summary: summary.to_string(),
                ..lyr_core::notifications::Incoming::default()
            },
            Instant::now(),
        );
    }
    setup.store = Some(store);
    setup.lay_out(1000.0);
    assert!(setup.panel.scroll_max > 0.0, "taller than the panel");
    let window = setup.panel.window;
    let at = PhysicalPoint {
        x: 50.0,
        y: f64::from(setup.panel.centre_view.y + 10.0),
    };
    setup
        .shell
        .move_pointer(window, at, (0.0, 0.0))
        .expect("moved");
    let wheel = |setup: &mut Setup, y: f32| {
        setup
            .shell
            .scroll(window, ScrollDelta::Lines { x: 0.0, y }, Some(at))
            .expect("scrolled");
        setup.pump();
    };
    wheel(&mut setup, -1.0);
    assert_eq!(setup.panel.scroll, 60.0, "three 20 px lines down");
    assert_eq!(
        setup.panel.centre_origin.1,
        setup.panel.centre_view.y - 60.0,
        "clicks follow the scroll"
    );
    wheel(&mut setup, 1.0);
    wheel(&mut setup, 1.0);
    assert_eq!(setup.panel.scroll, 0.0, "no further than the top");
    for _ in 0..1000 {
        wheel(&mut setup, -1.0);
    }
    assert_eq!(setup.panel.scroll, setup.panel.scroll_max, "nor the bottom");
    assert_eq!(setup.panel.mode, Mode::Year, "the calendar did not move");
}
