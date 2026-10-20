use lyr_config::{Platform, Theme, from_sources};

use super::{GAP, MAX_PERCENT, Node, Nodes, PADDING, Step, Tab, Target, draw, level_at};
use crate::draw::Item;

fn theme() -> Theme {
    from_sources(None, Platform::Windows)
        .expect("the defaults")
        .theme
}

/// Every character is 10 px wide.
fn measure(text: &str) -> f32 {
    text.chars().count() as f32 * 10.0
}

const LINE: f32 = 20.0;
/// A row: a line and 2 px above and below.
const ROW: f32 = LINE + 4.0;

fn node(name: &str, volume: u32, muted: bool, default: bool) -> Node {
    Node {
        id: name.to_lowercase(),
        name: name.to_owned(),
        volume,
        muted,
        default,
        device: None,
    }
}

fn nodes() -> Nodes {
    Nodes {
        playback: vec![node("Firefox", 80, false, false)],
        recording: Vec::new(),
        outputs: vec![
            node("Speakers", 40, false, true),
            node("Headphones", 100, true, false),
        ],
        inputs: vec![node("Microphone", 100, false, true)],
    }
}

/// The texts drawn in the row at `y`, left to right.
fn texts_at(items: &[Item], y: f32) -> Vec<String> {
    let mut texts: Vec<(f32, String)> = items
        .iter()
        .filter_map(|item| match item {
            Item::Text { text, rect, .. } if (rect.y - y).abs() < 3.0 => {
                Some((rect.x, text.clone()))
            }
            _ => None,
        })
        .collect();
    texts.sort_by(|a, b| a.0.total_cmp(&b.0));
    texts.into_iter().map(|(_, text)| text).collect()
}

/// The first row below the tabs.
const FIRST: f32 = PADDING + ROW + 2.0 * GAP;

#[test]
fn the_shown_tab_is_text_and_the_others_are_buttons() {
    let (list, _, regions) = draw(&nodes(), Tab::Output, None, &measure, LINE, &theme());
    let tabs: Vec<Target> = regions
        .targets()
        .copied()
        .filter(|target| matches!(target, Target::Tab(_)))
        .collect();
    assert_eq!(
        tabs,
        [
            Target::Tab(Tab::Playback),
            Target::Tab(Tab::Recording),
            Target::Tab(Tab::Input)
        ]
    );
    assert_eq!(
        texts_at(&list.items, PADDING),
        ["Playback", "Recording", "Output devices", "Input devices"]
    );
}

#[test]
fn a_device_row_has_mute_steps_its_level_and_default_unless_it_is_default() {
    let (list, size, regions) = draw(&nodes(), Tab::Output, None, &measure, LINE, &theme());
    assert_eq!(
        texts_at(&list.items, FIRST),
        ["Speakers", "Mute", "-", "40%", "+"]
    );
    assert_eq!(
        texts_at(&list.items, FIRST + ROW + GAP),
        ["Headphones", "Unmute", "-", "100%", "+", "Default"]
    );
    for target in [
        Target::Mute(0),
        Target::Step(0, Step::Down),
        Target::Step(0, Step::Up),
        Target::Mute(1),
        Target::Default(1),
    ] {
        assert!(regions.rect(&target).is_some(), "{target:?}");
    }
    assert_eq!(
        regions.rect(&Target::Default(0)),
        None,
        "already the default"
    );
    // The tallest is the two rows under the tabs.
    assert!((size.height - (FIRST + 2.0 * ROW + GAP + PADDING)).abs() < 0.01);
}

#[test]
fn the_rows_line_up_and_a_level_is_right_aligned() {
    let (_, _, regions) = draw(&nodes(), Tab::Output, None, &measure, LINE, &theme());
    let x = |target| regions.rect(&target).expect("drawn").x;
    assert!((x(Target::Mute(0)) - x(Target::Mute(1))).abs() < 0.01);
    assert!((x(Target::Step(0, Step::Up)) - x(Target::Step(1, Step::Up))).abs() < 0.01);
    let (list, _, _) = draw(&nodes(), Tab::Output, None, &measure, LINE, &theme());
    let level_end = |level: &str| {
        list.items
            .iter()
            .find_map(|item| match item {
                Item::Text { text, rect, .. } if text == level => Some(rect.x + rect.width),
                _ => None,
            })
            .expect("drawn")
    };
    assert!((level_end("40%") - level_end("100%")).abs() < 0.01);
}

#[test]
fn a_stream_has_no_default_and_an_empty_tab_says_so() {
    let (list, _, regions) = draw(&nodes(), Tab::Playback, None, &measure, LINE, &theme());
    assert_eq!(
        texts_at(&list.items, FIRST),
        ["Firefox", "Mute", "-", "80%", "+"]
    );
    assert!(
        !regions
            .targets()
            .any(|target| matches!(target, Target::Default(_)))
    );
    let (list, _, regions) = draw(&nodes(), Tab::Recording, None, &measure, LINE, &theme());
    assert_eq!(texts_at(&list.items, FIRST), ["Nothing is recording"]);
    assert!(
        regions
            .targets()
            .all(|target| matches!(target, Target::Tab(_)))
    );
}

#[test]
fn the_focus_is_outlined() {
    let theme = theme();
    let (list, _, regions) = draw(
        &nodes(),
        Tab::Output,
        Some(Target::Default(1)),
        &measure,
        LINE,
        &theme,
    );
    let rect = regions.rect(&Target::Default(1)).expect("drawn");
    assert!(matches!(
        list.items.last(),
        Some(Item::Box { rect: outlined, border: Some((_, colour)), .. })
            if *outlined == rect && *colour == theme.workspace_active
    ));
}

/// The width of the level drawn in `slider`: the box inside it with no
/// border, its track being the one with one.
fn level_width(items: &[Item], slider: crate::hit::Rect) -> f32 {
    items
        .iter()
        .find_map(|item| match item {
            Item::Box {
                rect, border: None, ..
            } if (rect.x - slider.x).abs() < 0.01
                && rect.y > slider.y
                && rect.y < slider.y + slider.height =>
            {
                Some(rect.width)
            }
            _ => None,
        })
        .expect("a level drawn")
}

#[test]
fn a_slider_fills_to_the_level_and_sits_between_the_steps() {
    let mut nodes = nodes();
    nodes.outputs[1].volume = MAX_PERCENT + 50;
    let (list, _, regions) = draw(&nodes, Tab::Output, None, &measure, LINE, &theme());
    let slider = regions.rect(&Target::Level(0)).expect("drawn");
    let down = regions.rect(&Target::Step(0, Step::Down)).expect("drawn");
    let up = regions.rect(&Target::Step(0, Step::Up)).expect("drawn");
    assert!(down.x + down.width <= slider.x && slider.x + slider.width <= up.x);
    assert!(
        (level_width(&list.items, slider) - slider.width * 40.0 / MAX_PERCENT as f32).abs() < 0.01,
        "40 % of the scale"
    );
    let boosted = regions.rect(&Target::Level(1)).expect("drawn");
    assert!(
        (level_width(&list.items, boosted) - boosted.width).abs() < 0.01,
        "past the scale fills it, no further"
    );
}

#[test]
fn a_click_across_a_slider_is_a_percent() {
    assert_eq!(level_at(0.0), 0);
    assert_eq!(level_at(0.404), (0.404 * MAX_PERCENT as f32).round() as u32);
    assert_eq!(level_at(1.0), MAX_PERCENT);
    assert_eq!(level_at(1.5), MAX_PERCENT);
    assert_eq!(level_at(-0.5), 0);
}

#[test]
fn a_stream_that_can_move_shows_its_device_by_name() {
    let mut nodes = nodes();
    nodes.playback[0].device = Some("headphones".to_owned());
    let (list, _, regions) = draw(&nodes, Tab::Playback, None, &measure, LINE, &theme());
    let route = regions.rect(&Target::Route(0)).expect("a route button");
    let up = regions.rect(&Target::Step(0, Step::Up)).expect("drawn");
    assert!(route.x >= up.x + up.width, "after the steps");
    assert_eq!(
        texts_at(&list.items, FIRST).last().map(String::as_str),
        Some("Headphones"),
        "the device's name, not its id"
    );
    // A stream that cannot move, and a device, have none.
    let (_, _, regions) = draw(
        &self::nodes(),
        Tab::Playback,
        None,
        &measure,
        LINE,
        &theme(),
    );
    assert_eq!(regions.rect(&Target::Route(0)), None);
    let (_, _, regions) = draw(&self::nodes(), Tab::Output, None, &measure, LINE, &theme());
    assert_eq!(regions.rect(&Target::Route(0)), None);
}
