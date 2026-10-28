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

use super::{CENTRE_WIDTH, ClockPanel, Contents, Outcome};
use crate::menu_panel::{GAP, Place};
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
        Place::EDGE,
        &Look {
            theme: &theme,
            measure: &measure,
            measure_in: &|text, _| measure(text),
            lay_out: &|_, _| Vec::new(),
            line_height: 20.0,
            calendar_line_height: 20.0,
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
    /// Right-clicks the calendar, which switches its mode.
    fn right_click_calendar(&mut self) {
        let window = self.panel.window;
        let at = PhysicalPoint { x: 50.0, y: 50.0 };
        self.shell
            .move_pointer(window, at, (0.0, 0.0))
            .expect("moved");
        self.shell
            .button(window, PointerButton::Right, ButtonState::Pressed, Some(at))
            .expect("clicked");
        self.pump();
    }

    /// Asks for the size the panel needs, as each draw does first.
    fn draw(&mut self) {
        self.panel.ask_size(&mut self.shell).expect("resized");
    }

    /// Feeds the panel its queued events, sizing ones left out (they open a
    /// GPU surface), and returns the last outcome.
    fn pump(&mut self) -> Outcome {
        let mut events = Vec::new();
        self.shell.pump(&mut |event| events.push(event));
        let look = Look {
            theme: &self.theme,
            measure: &measure,
            measure_in: &|text, _| measure(text),
            lay_out: &|_, _| Vec::new(),
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
fn it_opens_as_big_as_the_year() {
    let setup = setup();
    // Three 240 px month blocks and two 30 px gaps.
    assert_eq!(setup.panel.size.width, 780.0 + 2.0 * (16.0 + 2.0));
    assert_eq!((setup.panel.mode, setup.panel.shown), (Mode::Year, today()));
    assert!(setup.shell.title(setup.panel.window).is_ok());
}

#[test]
fn it_takes_the_size_of_the_view_it_shows() {
    let mut setup = setup();
    setup.lay_out(2000.0);
    let year = setup.panel.size;
    assert_eq!(setup.panel.resize_to, None, "opened at the year's size");
    let window = setup.panel.window;
    setup.right_click_calendar();
    assert_eq!(setup.panel.mode, Mode::Month);
    let month = setup.panel.size;
    // One 240 px month block.
    assert_eq!(month.width, 240.0 + 2.0 * (16.0 + 2.0));
    assert!(month.height < year.height, "a month is shorter than a year");
    assert_eq!(setup.panel.resize_to, Some(month), "asked for in place");
    setup.draw();
    assert_eq!(setup.panel.resize_to, None, "asked once");
    let mut events = Vec::new();
    setup.shell.pump(&mut |event| events.push(event));
    let resized = events.iter().find_map(|event| match event {
        ShellEvent::Resized {
            window: at, size, ..
        } if *at == window => Some(*size),
        _ => None,
    });
    let resized = resized.expect("the window was resized in place");
    assert_eq!(f64::from(resized.width), f64::from(month.width));
}

#[test]
fn the_centre_widens_it_and_the_output_holds_it() {
    let mut setup = setup();
    setup.store = Some(Store::default());
    setup.lay_out(2000.0);
    setup.right_click_calendar();
    assert_eq!(setup.panel.size.width, CENTRE_WIDTH, "the centre's width");
    let tall = setup.panel.size.height;
    assert!(tall > setup.panel.calendar_bottom, "the centre below");
    setup.lay_out(200.0);
    assert_eq!(setup.panel.size.height, 200.0, "held short of the far edge");
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
    assert!(setup.panel.ready(), "redrawn");
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
    /// Lays the panel out with `height` to fill at most.
    fn lay_out(&mut self, height: f32) {
        self.panel.place.room = f64::from(height) + 2.0 * GAP;
        let look = Look {
            theme: &self.theme,
            measure: &measure,
            measure_in: &|text, _| measure(text),
            lay_out: &|_, _| Vec::new(),
            line_height: 20.0,
            calendar_line_height: 20.0,
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
        // As the bar's next draw does: the window takes the new size.
        self.panel.ask_size(&mut self.shell).expect("resized");
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

#[test]
fn the_entry_under_the_pointer_shows_its_close_button() {
    let mut setup = setup();
    let mut store = Store::default();
    let id = store.notify(
        lyr_core::notifications::Incoming {
            summary: "hello".to_owned(),
            ..lyr_core::notifications::Incoming::default()
        },
        Instant::now(),
    );
    setup.store = Some(store);
    setup.lay_out(2000.0);
    let window = setup.panel.window;
    let (ox, oy) = setup.panel.centre_origin;
    let entry = setup
        .panel
        .centre
        .rect(&Target::Notification(id))
        .expect("an entry");
    let point = |x: f32, y: f32| PhysicalPoint {
        x: f64::from(ox + x),
        y: f64::from(oy + y),
    };
    let on_entry = point(entry.x + 5.0, entry.y + entry.height - 5.0);
    setup
        .shell
        .move_pointer(window, on_entry, (0.0, 0.0))
        .expect("moved");
    assert_eq!(setup.pump(), Outcome::Kept);
    assert_eq!(setup.panel.hovered, Some(id));
    let close = setup
        .panel
        .centre
        .rect(&Target::Close(id))
        .expect("a close button");
    let at = point(close.x + 2.0, close.y + 2.0);
    setup
        .shell
        .move_pointer(window, at, (0.0, 0.0))
        .expect("moved");
    setup
        .shell
        .button(window, PointerButton::Left, ButtonState::Pressed, Some(at))
        .expect("clicked");
    assert_eq!(
        setup.pump(),
        Outcome::Centre(Target::Close(id), Press::Left)
    );
    // Over the calendar, no entry is.
    setup
        .shell
        .move_pointer(window, PhysicalPoint { x: 30.0, y: 30.0 }, (0.0, 0.0))
        .expect("moved");
    setup.pump();
    assert_eq!(setup.panel.hovered, None);
    assert_eq!(
        setup.panel.centre.rect(&Target::Close(id)),
        None,
        "drawn without it"
    );
}

#[test]
fn an_entry_scrolled_up_under_the_calendar_is_not_hovered() {
    let mut setup = setup();
    let mut store = Store::default();
    for summary in 0..20 {
        store.notify(
            lyr_core::notifications::Incoming {
                summary: summary.to_string(),
                ..lyr_core::notifications::Incoming::default()
            },
            Instant::now(),
        );
    }
    setup.store = Some(store);
    setup.lay_out(1000.0);
    setup.panel.scroll = setup.panel.scroll_max;
    setup.lay_out(1000.0);
    let window = setup.panel.window;
    let (ox, oy) = setup.panel.centre_origin;
    let (x, y) = (ox + 5.0, setup.panel.centre_view.y - 5.0);
    assert!(
        setup
            .panel
            .centre
            .hit(x - ox, y - oy)
            .is_some_and(|target| target.entry().is_some()),
        "an entry lies there, out of view"
    );
    setup
        .shell
        .move_pointer(
            window,
            PhysicalPoint {
                x: f64::from(x),
                y: f64::from(y),
            },
            (0.0, 0.0),
        )
        .expect("moved");
    setup.pump();
    assert_eq!(setup.panel.hovered, None);
}

#[test]
fn the_calendar_is_laid_out_in_its_own_line_height() {
    let mut setup = setup();
    let bottom = |setup: &mut Setup, calendar_line_height| {
        let look = Look {
            theme: &setup.theme,
            measure: &measure,
            measure_in: &|text, _| measure(text),
            lay_out: &|_, _| Vec::new(),
            line_height: 20.0,
            calendar_line_height,
        };
        setup.panel.refresh(
            &look,
            &Contents {
                today: today(),
                store: None,
                players: &[],
                now: Instant::now(),
            },
        );
        setup.panel.calendar_bottom
    };
    let tall = bottom(&mut setup, 20.0);
    let short = bottom(&mut setup, 10.0);
    assert!(short < tall, "{short} < {tall}");
}

#[test]
fn a_press_outside_the_panel_closes_it() {
    let mut setup = setup();
    setup.draw();
    // The configure, which gives the window its size.
    setup.pump();
    let window = setup.panel.window;
    // Where Hyprland put a click on the bar above it.
    let outside = PhysicalPoint { x: 569.0, y: -41.0 };
    setup
        .shell
        .button(
            window,
            PointerButton::Left,
            ButtonState::Pressed,
            Some(outside),
        )
        .expect("clicked");
    assert_eq!(setup.pump(), Outcome::Close);
}

#[test]
fn the_keys_outline_the_centre_s_controls_enter_acts_and_delete_dismisses() {
    let mut setup = setup();
    let mut store = Store::default();
    let id = store.notify(
        lyr_core::notifications::Incoming {
            summary: "hello".to_owned(),
            ..lyr_core::notifications::Incoming::default()
        },
        Instant::now(),
    );
    setup.store = Some(store);
    setup.lay_out(2000.0);
    let window = setup.panel.window;
    let targets: Vec<Target> = setup.panel.centre.targets().copied().collect();
    let entry = targets
        .iter()
        .position(|target| *target == Target::Notification(id))
        .expect("the entry is a target");
    let key = |setup: &mut Setup, key| {
        setup.shell.key_press(window, key).expect("a key");
        setup.pump()
    };
    let outlined = |setup: &Setup| {
        let border = Some((
            setup.theme.border_width as f32,
            setup.theme.workspace_active,
        ));
        setup.panel.list.items.iter().any(|item| match item {
            lyr_core::draw::Item::Clip { items, .. } => items.iter().any(
                |item| matches!(item, lyr_core::draw::Item::Box { border: b, .. } if *b == border),
            ),
            _ => false,
        })
    };
    assert_eq!(
        key(&mut setup, KeyCode::Enter),
        Outcome::Kept,
        "nothing outlined"
    );
    assert!(!outlined(&setup));
    assert_eq!(key(&mut setup, KeyCode::ArrowDown), Outcome::Kept);
    assert!(outlined(&setup), "the first control is outlined");
    assert_eq!(
        key(&mut setup, KeyCode::Enter),
        Outcome::Centre(targets[0], Press::Left)
    );
    assert_eq!(
        key(&mut setup, KeyCode::Delete),
        Outcome::Kept,
        "{:?} is no notification's",
        targets[0]
    );
    for _ in 0..entry {
        key(&mut setup, KeyCode::ArrowDown);
    }
    assert_eq!(
        key(&mut setup, KeyCode::Enter),
        Outcome::Centre(Target::Notification(id), Press::Left)
    );
    assert_eq!(
        key(&mut setup, KeyCode::Delete),
        Outcome::Centre(Target::Close(id), Press::Left)
    );
}

#[test]
fn the_outline_is_scrolled_into_view() {
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
    let count = setup.panel.centre.targets().count();
    let press = |setup: &mut Setup, key| {
        setup.shell.key_press(window, key).expect("a key");
        setup.pump();
        let panel = &setup.panel;
        let rect = panel
            .focus
            .and_then(|at| panel.centre.rect_at(at))
            .expect("an outline");
        assert!(
            rect.y >= panel.scroll
                && rect.y + rect.height <= panel.scroll + panel.centre_view.height,
            "{rect:?} out of view at {}",
            panel.scroll
        );
    };
    for _ in 0..count {
        press(&mut setup, KeyCode::ArrowDown);
    }
    assert_eq!(setup.panel.scroll, setup.panel.scroll_max, "at the last");
    for _ in 0..count {
        press(&mut setup, KeyCode::ArrowUp);
    }
    assert_eq!(setup.panel.scroll, 0.0, "back at the first");
}
