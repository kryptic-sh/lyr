use std::time::{Duration, Instant};

use crcbl_shell::{HeadlessShell, Shell, WindowDesc, WindowId};
use lyr_config::{Module, Platform, Position, Theme, from_sources};
use lyr_core::draw::Target;
use lyr_core::hit::{Rect, Regions};
use lyr_core::tooltip::HOVER_DELAY;

use super::{Anchor, Look, Tooltips};

fn theme() -> Theme {
    from_sources(None, Platform::Linux)
        .expect("the defaults")
        .theme
}

/// Every character is 10 px wide.
fn measure(text: &str) -> f32 {
    text.chars().count() as f32 * 10.0
}

struct Setup {
    shell: HeadlessShell,
    bar: WindowId,
    regions: Regions<Target<Module>>,
    theme: Theme,
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
    let mut regions = Regions::default();
    regions.push(
        Rect {
            x: 900.0,
            y: 10.0,
            width: 150.0,
            height: 38.0,
        },
        Target::Module(Module::Clock),
    );
    Setup {
        shell,
        bar,
        regions,
        theme: theme(),
    }
}

impl Setup {
    fn sync(
        &mut self,
        tooltips: &mut Tooltips,
        text: Option<&str>,
        now: Instant,
    ) -> Option<WindowId> {
        tooltips
            .sync(
                &mut self.shell,
                &Anchor {
                    bar: self.bar,
                    regions: &self.regions,
                    position: Position::Top,
                },
                &Look {
                    theme: &self.theme,
                    measure: &measure,
                    measure_in: &|text, _| measure(text),
                    line_height: 20.0,
                    calendar_line_height: 20.0,
                },
                |_| text.map(str::to_owned),
                now,
            )
            .expect("synced");
        tooltips.popup.as_ref().map(|popup| popup.window)
    }

    fn open(&self, window: WindowId) -> bool {
        self.shell.title(window).is_ok()
    }
}

#[test]
fn a_tooltip_opens_after_the_delay_and_closes_when_the_pointer_leaves() {
    let mut setup = setup();
    let mut tooltips = Tooltips::default();
    let start = Instant::now();
    tooltips.hover(Some(&Target::Module(Module::Clock)), start);
    assert_eq!(setup.sync(&mut tooltips, Some("UTC+08:00"), start), None);
    let popup = setup
        .sync(&mut tooltips, Some("UTC+08:00"), start + HOVER_DELAY)
        .expect("a popup");
    assert!(setup.open(popup));
    assert!(tooltips.owns(popup) && !tooltips.owns(setup.bar));
    assert_eq!(
        setup.sync(&mut tooltips, Some("UTC+08:00"), start + HOVER_DELAY),
        Some(popup),
        "kept while nothing changed"
    );
    let left = start + HOVER_DELAY + Duration::from_millis(1);
    tooltips.hover(None, left);
    assert_eq!(setup.sync(&mut tooltips, Some("UTC+08:00"), left), None);
    assert!(!setup.open(popup), "its window is gone");
}

#[test]
fn new_text_replaces_the_popup() {
    let mut setup = setup();
    let mut tooltips = Tooltips::default();
    let start = Instant::now();
    tooltips.hover(Some(&Target::Module(Module::Clock)), start);
    let shown = start + HOVER_DELAY;
    let first = setup
        .sync(&mut tooltips, Some("UTC+08:00"), shown)
        .expect("a popup");
    let second = setup
        .sync(&mut tooltips, Some("Asia/Manila, UTC+08:00"), shown)
        .expect("a popup");
    assert_ne!(first, second);
    assert!(!setup.open(first) && setup.open(second));
}

#[test]
fn nothing_opens_without_text_a_box_or_a_module() {
    let mut setup = setup();
    let mut tooltips = Tooltips::default();
    let start = Instant::now();
    let shown = start + HOVER_DELAY;
    tooltips.hover(Some(&Target::Module(Module::Clock)), start);
    assert_eq!(setup.sync(&mut tooltips, None, shown), None, "no text");
    let mut tooltips = Tooltips::default();
    tooltips.hover(Some(&Target::Module(Module::Battery)), start);
    assert_eq!(
        setup.sync(&mut tooltips, Some("80%"), shown),
        None,
        "no box"
    );
    let mut tooltips = Tooltips::default();
    tooltips.hover(Some(&Target::Button(Module::Workspaces, 0)), start);
    assert_eq!(tooltips.next_wake(), None, "a workspace button has none");
}

#[test]
fn closing_destroys_the_popup() {
    let mut setup = setup();
    let mut tooltips = Tooltips::default();
    let start = Instant::now();
    tooltips.hover(Some(&Target::Module(Module::Clock)), start);
    let popup = setup
        .sync(&mut tooltips, Some("UTC+08:00"), start + HOVER_DELAY)
        .expect("a popup");
    tooltips.close(&mut setup.shell).expect("closed");
    assert!(!setup.open(popup) && !tooltips.owns(popup));
}

/// Moves the clock's box `by` pixels across.
fn move_clock(setup: &mut Setup, by: f32) {
    let mut regions = Regions::default();
    regions.push(
        Rect {
            x: 900.0 + by,
            y: 10.0,
            width: 150.0,
            height: 38.0,
        },
        Target::Module(Module::Clock),
    );
    setup.regions = regions;
}

#[test]
fn a_tooltip_follows_its_module_when_it_moves() {
    let mut setup = setup();
    let mut tooltips = Tooltips::default();
    let start = Instant::now();
    tooltips.hover(Some(&Target::Module(Module::Clock)), start);
    let shown = start + HOVER_DELAY;
    let popup = setup
        .sync(&mut tooltips, Some("UTC+08:00"), shown)
        .expect("a popup");
    move_clock(&mut setup, -40.0);
    assert_eq!(
        setup.sync(&mut tooltips, Some("UTC+08:00"), shown),
        Some(popup),
        "repositioned, not reopened"
    );
    assert_eq!(
        tooltips.popup.as_ref().map(|popup| popup.anchor.x),
        Some(860.0)
    );
}

#[test]
fn without_repositioning_a_moved_tooltip_is_opened_again() {
    let mut setup = setup();
    let caps = HeadlessShell::new()
        .caps()
        .difference(crcbl_shell::ShellCaps::POPUP_REPOSITION);
    setup.shell = HeadlessShell::new().with_caps(caps);
    setup.bar = setup
        .shell
        .create_window(&WindowDesc {
            title: "lyr",
            app_id: "lyr",
            ..WindowDesc::default()
        })
        .expect("a bar");
    let mut tooltips = Tooltips::default();
    let start = Instant::now();
    tooltips.hover(Some(&Target::Module(Module::Clock)), start);
    let shown = start + HOVER_DELAY;
    let popup = setup
        .sync(&mut tooltips, Some("UTC+08:00"), shown)
        .expect("a popup");
    assert_eq!(
        setup.sync(&mut tooltips, Some("UTC+08:00"), shown),
        Some(popup),
        "unmoved, kept"
    );
    move_clock(&mut setup, -40.0);
    let again = setup
        .sync(&mut tooltips, Some("UTC+08:00"), shown)
        .expect("a popup");
    assert_ne!(again, popup);
    assert!(!setup.open(popup) && setup.open(again));
}
