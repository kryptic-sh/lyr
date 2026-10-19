use lyr_config::{Platform, Theme, from_sources};

use super::{BORDER, INSET, WIDTH, draw, wrap};
use crate::draw::Item;
use crate::notifications::{Incoming, Urgency};

fn theme() -> Theme {
    from_sources(None, Platform::Linux)
        .expect("the defaults")
        .theme
}

/// Every character is 10 px wide.
fn measure(text: &str) -> f32 {
    text.chars().count() as f32 * 10.0
}

#[test]
fn text_wraps_at_spaces_within_the_width() {
    assert_eq!(
        wrap("the quick brown fox", 100.0, measure),
        ["the quick", "brown fox"]
    );
    // "the quick" is 90 px, and "the quick b" would be 110.
    assert_eq!(wrap("the quick b", 100.0, measure), ["the quick", "b"]);
    assert_eq!(wrap("exactly 10", 100.0, measure), ["exactly 10"], "fits");
}

#[test]
fn line_breaks_are_kept_and_a_long_word_runs_over() {
    assert_eq!(wrap("one\ntwo three", 100.0, measure), ["one", "two three"]);
    assert_eq!(
        wrap("a supercalifragilistic b", 100.0, measure),
        ["a", "supercalifragilistic", "b"]
    );
    assert_eq!(
        wrap("supercalifragilistic b", 100.0, measure),
        ["supercalifragilistic", "b"],
        "no empty line before a long first word"
    );
    assert_eq!(
        wrap("first\n\nthird", 100.0, measure),
        ["first", "", "third"]
    );
}

fn texts(items: &[Item]) -> Vec<(&str, f32)> {
    items
        .iter()
        .filter_map(|item| match item {
            Item::Text { text, rect, .. } => Some((text.as_str(), rect.y)),
            _ => None,
        })
        .collect()
}

#[test]
fn a_toast_stacks_its_summary_over_its_body() {
    let theme = theme();
    let content = Incoming {
        summary: "Download done".to_owned(),
        body: "file.zip".to_owned(),
        ..Incoming::default()
    };
    let (list, size) = draw(&content, measure, 20.0, &theme);
    let top = BORDER + INSET;
    assert_eq!((size.width, size.height), (WIDTH, 2.0 * top + 40.0));
    assert_eq!(
        texts(&list.items),
        [("Download done", top), ("file.zip", top + 20.0)]
    );
    let no_body = Incoming {
        summary: "Ping".to_owned(),
        ..Incoming::default()
    };
    let (list, size) = draw(&no_body, measure, 20.0, &theme);
    assert_eq!(texts(&list.items), [("Ping", top)]);
    assert_eq!(size.height, 2.0 * top + 20.0);
}

#[test]
fn the_border_follows_the_urgency() {
    let theme = theme();
    let border = |urgency| {
        let content = Incoming {
            summary: "x".to_owned(),
            urgency,
            ..Incoming::default()
        };
        match draw(&content, measure, 20.0, &theme).0.items[0] {
            Item::Box {
                border: Some((width, colour)),
                fill,
                ..
            } => {
                assert_eq!((width, fill), (BORDER, theme.toast_background));
                colour
            }
            _ => panic!("a box first"),
        }
    };
    assert_eq!(border(Urgency::Low), theme.toast_border_low);
    assert_eq!(border(Urgency::Normal), theme.toast_border);
    assert_eq!(border(Urgency::Critical), theme.toast_border_critical);
}
