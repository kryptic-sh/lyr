use std::time::Duration;

use lyr_config::{Platform, Theme, from_sources};

use super::{BORDER, INSET, TIME_GAP, WIDTH, draw, relative_time, wrap};
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
    let (list, size, _) = draw(&content, Duration::ZERO, WIDTH, measure, 20.0, &theme);
    let top = BORDER + INSET;
    assert_eq!((size.width, size.height), (WIDTH, 2.0 * top + 40.0));
    assert_eq!(
        texts(&list.items),
        [
            ("Now", top),
            ("Download done", top),
            ("file.zip", top + 20.0)
        ]
    );
    let no_body = Incoming {
        summary: "Ping".to_owned(),
        ..Incoming::default()
    };
    let (list, size, _) = draw(&no_body, Duration::ZERO, WIDTH, measure, 20.0, &theme);
    assert_eq!(texts(&list.items), [("Now", top), ("Ping", top)]);
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
        match draw(&content, Duration::ZERO, WIDTH, measure, 20.0, &theme)
            .0
            .items[0]
        {
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

#[test]
fn the_time_reads_as_swaync_words_it() {
    let at = |secs| relative_time(Duration::from_secs(secs));
    assert_eq!(at(0), "Now");
    assert_eq!(at(59), "Now");
    assert_eq!(at(60), "1 min ago");
    assert_eq!(at(119), "1 min ago");
    assert_eq!(at(120), "2 mins ago");
    assert_eq!(at(3599), "59 mins ago");
    assert_eq!(at(3600), "1 hour ago");
    assert_eq!(at(7200), "2 hours ago");
    assert_eq!(at(86_399), "23 hours ago");
    assert_eq!(at(86_400), "1 day ago");
    assert_eq!(at(3 * 86_400), "3 days ago");
}

#[test]
fn the_time_sits_at_the_right_of_the_summary() {
    let theme = theme();
    let content = Incoming {
        summary: "hello".to_owned(),
        ..Incoming::default()
    };
    let (list, _, _) = draw(
        &content,
        Duration::from_secs(120),
        WIDTH,
        measure,
        20.0,
        &theme,
    );
    let time = list
        .items
        .iter()
        .find_map(|item| match item {
            Item::Text { text, rect, colour } if text == "2 mins ago" => Some((*rect, *colour)),
            _ => None,
        })
        .expect("the time");
    assert_eq!(time.0.x + time.0.width, WIDTH - BORDER - INSET);
    assert_eq!(time.0.y, BORDER + INSET);
    assert_eq!(time.1, theme.toast_time);
}

#[test]
fn the_summary_stops_short_of_the_time() {
    let theme = theme();
    let content = Incoming {
        summary: "a summary long enough to run into the time".to_owned(),
        body: "a body that may use the whole width".to_owned(),
        ..Incoming::default()
    };
    let (list, _, _) = draw(&content, Duration::ZERO, WIDTH, measure, 20.0, &theme);
    let width = |wanted: &str| {
        list.items
            .iter()
            .find_map(|item| match item {
                Item::Text { text, rect, .. } if text.starts_with(wanted) => Some(rect.width),
                _ => None,
            })
            .expect("drawn")
    };
    let text_width = WIDTH - 2.0 * (BORDER + INSET);
    // "Now" is 30 px.
    assert_eq!(width("a summary"), text_width - 30.0 - TIME_GAP);
    assert_eq!(width("a body"), text_width);
}

fn with_actions() -> Incoming {
    Incoming {
        summary: "Meeting".to_owned(),
        actions: [("default", "Open"), ("snooze", "Snooze"), ("join", "Join")]
            .map(|(key, label)| (key.to_owned(), label.to_owned()))
            .to_vec(),
        ..Incoming::default()
    }
}

#[test]
fn each_action_but_the_default_gets_a_button_in_a_row() {
    let theme = theme();
    let (list, size, buttons) = draw(
        &with_actions(),
        Duration::ZERO,
        WIDTH,
        measure,
        20.0,
        &theme,
    );
    let content = WIDTH - 2.0 * (BORDER + INSET);
    // Two buttons share the row, 2 * 4 px apart.
    let width = (content - 8.0) / 2.0;
    let text_bottom = BORDER + INSET + 20.0;
    let y = text_bottom + 4.0;
    let height = 20.0 + 2.0 * (8.0 + BORDER);
    assert_eq!(
        buttons
            .iter()
            .map(|(index, rect)| (*index, rect.x, rect.y, rect.width, rect.height))
            .collect::<Vec<_>>(),
        [
            (1, BORDER + INSET, y, width, height),
            (2, BORDER + INSET + width + 8.0, y, width, height)
        ]
    );
    assert_eq!(size.height, text_bottom + height + 8.0 + BORDER + INSET);
    let labels: Vec<&str> = list
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Text { text, .. } if text == "Snooze" || text == "Join" || text == "Open" => {
                Some(text.as_str())
            }
            _ => None,
        })
        .collect();
    assert_eq!(labels, ["Snooze", "Join"], "no button for the default");
}

#[test]
fn a_notification_with_only_the_default_action_has_no_buttons() {
    let theme = theme();
    let content = Incoming {
        summary: "x".to_owned(),
        actions: vec![("default".to_owned(), "Open".to_owned())],
        ..Incoming::default()
    };
    let (_, size, buttons) = draw(&content, Duration::ZERO, WIDTH, measure, 20.0, &theme);
    assert!(buttons.is_empty());
    assert_eq!(size.height, 2.0 * (BORDER + INSET) + 20.0);
}
