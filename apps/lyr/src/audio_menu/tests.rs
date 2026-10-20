use crcbl_shell::{
    ButtonState, HeadlessShell, KeyCode, PhysicalPoint, PointerButton, Shell, ShellEvent,
};
use lyr_config::{Platform, Theme, from_sources};
use lyr_core::audio_menu::{self, Node, Nodes, Step, Tab, Target, level_at};

use super::{Action, AudioMenu, Outcome};
use crate::menu_panel::Place;
use crate::tooltips::Look;

/// Every character is 10 px wide.
fn measure(text: &str) -> f32 {
    text.chars().count() as f32 * 10.0
}

const LINE: f32 = 20.0;

fn node(name: &str, default: bool) -> Node {
    Node {
        id: name.to_lowercase(),
        name: name.to_owned(),
        volume: 50,
        muted: false,
        default,
        device: None,
    }
}

fn nodes() -> Nodes {
    Nodes {
        playback: vec![node("Firefox", false)],
        recording: Vec::new(),
        outputs: vec![node("Speakers", true), node("Headphones", false)],
        inputs: vec![node("Microphone", true)],
        cards: None,
    }
}

struct Setup {
    shell: HeadlessShell,
    theme: Theme,
    menu: AudioMenu,
    /// What the menu was last given to show.
    nodes: Nodes,
}

impl Setup {
    /// Clicks the middle of `target` as the menu draws `tab`, and returns
    /// the last outcome of the events that queued, sizing ones left out
    /// (they open a GPU surface).
    fn click(&mut self, tab: Tab, target: Target) -> Outcome {
        let (_, _, regions) = audio_menu::draw(&self.nodes, tab, None, &measure, LINE, &self.theme);
        let rect = regions.rect(&target).expect("drawn");
        let at = PhysicalPoint {
            x: f64::from(rect.x + rect.width / 2.0),
            y: f64::from(rect.y + rect.height / 2.0),
        };
        let window = self.menu.panel.window();
        self.shell
            .move_pointer(window, at, (0.0, 0.0))
            .expect("moved");
        self.shell
            .button(window, PointerButton::Left, ButtonState::Pressed, Some(at))
            .expect("clicked");
        self.pump()
    }

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

    fn refresh(&mut self) {
        let theme = self.theme.clone();
        let look = Look {
            theme: &theme,
            measure: &measure,
            measure_in: &|text, _| measure(text),
            line_height: LINE,
            calendar_line_height: LINE,
        };
        self.menu
            .refresh(&mut self.shell, &look, &self.nodes)
            .expect("refreshed");
    }
}

fn setup(tab: Tab) -> Setup {
    let mut shell = HeadlessShell::new();
    let theme = from_sources(None, Platform::Windows)
        .expect("the defaults")
        .theme;
    let look = Look {
        theme: &theme,
        measure: &measure,
        measure_in: &|text, _| measure(text),
        line_height: LINE,
        calendar_line_height: LINE,
    };
    let menu = AudioMenu::open(&mut shell, Place::EDGE, &look, &nodes(), tab).expect("opened");
    Setup {
        shell,
        theme,
        menu,
        nodes: nodes(),
    }
}

#[test]
fn a_row_s_button_is_the_caller_s_by_its_id_and_escape_closes() {
    let mut setup = setup(Tab::Output);
    assert_eq!(
        setup.click(Tab::Output, Target::Default(1)),
        Outcome::Act(Tab::Output, "headphones".to_owned(), Action::Default)
    );
    assert_eq!(
        setup.click(Tab::Output, Target::Step(0, Step::Up)),
        Outcome::Act(Tab::Output, "speakers".to_owned(), Action::Step(Step::Up))
    );
    let window = setup.menu.panel.window();
    setup
        .shell
        .key_press(window, KeyCode::Escape)
        .expect("a key");
    assert_eq!(setup.pump(), Outcome::Close);
}

#[test]
fn a_tab_shows_its_rows() {
    let mut setup = setup(Tab::Output);
    assert_eq!(
        setup.click(Tab::Output, Target::Tab(Tab::Input)),
        Outcome::Redraw
    );
    setup.refresh();
    assert_eq!(
        setup.click(Tab::Input, Target::Mute(0)),
        Outcome::Act(Tab::Input, "microphone".to_owned(), Action::Mute)
    );
}

#[test]
fn a_click_on_a_slider_sets_where_it_lands_and_enter_there_does_nothing() {
    let mut setup = setup(Tab::Output);
    assert_eq!(
        setup.click(Tab::Output, Target::Level(1)),
        Outcome::Act(
            Tab::Output,
            "headphones".to_owned(),
            Action::Set(level_at(0.5))
        ),
        "the middle"
    );
    let (_, _, regions) =
        audio_menu::draw(&nodes(), Tab::Output, None, &measure, LINE, &setup.theme);
    let at = regions
        .targets()
        .position(|target| *target == Target::Level(0))
        .expect("drawn");
    let window = setup.menu.panel.window();
    for _ in 0..=at {
        setup
            .shell
            .key_press(window, KeyCode::ArrowDown)
            .expect("a key");
    }
    setup.pump();
    assert_eq!(setup.menu.panel.focused(), Some(Target::Level(0)));
    setup
        .shell
        .key_press(window, KeyCode::Enter)
        .expect("a key");
    assert_eq!(setup.pump(), Outcome::Kept);
}

#[test]
fn a_drag_on_a_slider_sets_as_it_moves_until_let_go() {
    let mut setup = setup(Tab::Output);
    let (_, _, regions) =
        audio_menu::draw(&nodes(), Tab::Output, None, &measure, LINE, &setup.theme);
    let slider = regions.rect(&Target::Level(1)).expect("drawn");
    let at = |share: f32| PhysicalPoint {
        x: f64::from(slider.x + slider.width * share),
        y: f64::from(slider.y + slider.height / 2.0),
    };
    let window = setup.menu.panel.window();
    let set = |percent| Outcome::Act(Tab::Output, "headphones".to_owned(), Action::Set(percent));
    setup
        .shell
        .move_pointer(window, at(0.25), (0.0, 0.0))
        .expect("moved");
    setup
        .shell
        .button(
            window,
            PointerButton::Left,
            ButtonState::Pressed,
            Some(at(0.25)),
        )
        .expect("pressed");
    assert_eq!(setup.pump(), set(level_at(0.25)));
    setup
        .shell
        .move_pointer(window, at(0.75), (0.0, 0.0))
        .expect("moved");
    assert_eq!(setup.pump(), set(level_at(0.75)), "followed the drag");
    setup
        .shell
        .button(
            window,
            PointerButton::Left,
            ButtonState::Released,
            Some(at(0.75)),
        )
        .expect("let go");
    setup.pump();
    setup
        .shell
        .move_pointer(window, at(0.5), (0.0, 0.0))
        .expect("moved");
    assert_eq!(setup.pump(), Outcome::Kept, "let go: a move sets nothing");
}

#[test]
fn a_drag_from_a_button_does_not_press_it_again() {
    let mut setup = setup(Tab::Output);
    assert_eq!(
        setup.click(Tab::Output, Target::Mute(0)),
        Outcome::Act(Tab::Output, "speakers".to_owned(), Action::Mute)
    );
    let window = setup.menu.panel.window();
    setup
        .shell
        .move_pointer(window, PhysicalPoint { x: 20.0, y: 20.0 }, (0.0, 0.0))
        .expect("moved");
    assert_eq!(setup.pump(), Outcome::Kept);
}

#[test]
fn a_click_on_a_stream_s_device_routes_it() {
    let mut setup = setup(Tab::Playback);
    let mut moving = nodes();
    moving.playback[0].device = Some("speakers".to_owned());
    setup.nodes = moving;
    setup.refresh();
    assert_eq!(
        setup.click(Tab::Playback, Target::Route(0)),
        Outcome::Act(Tab::Playback, "firefox".to_owned(), Action::Route)
    );
}
