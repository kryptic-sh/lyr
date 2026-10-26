use std::time::Duration;

use lyr_config::{Platform, Theme, from_sources};

use super::{
    BORDER, INSET, PICTURE_MARGIN, PICTURE_PADDING, PICTURE_SIZE, TIME_GAP, WIDTH, draw,
    relative_time, wrap,
};
use crate::draw::{Item, Picture, TextStyle};
use crate::hit::Rect;
use crate::markup::{Span, Style, parse, plain};
use crate::notifications::{Incoming, Urgency};

fn theme() -> Theme {
    from_sources(None, Platform::Linux)
        .expect("the defaults")
        .theme
}

/// Every character is 10 px wide, or 15 in bold.
fn measure(text: &str, style: TextStyle) -> f32 {
    let each = if style.bold { 15.0 } else { 10.0 };
    text.chars().count() as f32 * each
}

/// `text`'s lines in a body 100 px wide, without their styles.
fn lines(text: &str) -> Vec<String> {
    wrap(&parse(text), 100.0, measure)
        .iter()
        .map(|line| plain(line))
        .collect()
}

#[test]
fn text_wraps_at_spaces_within_the_width() {
    assert_eq!(lines("the quick brown fox"), ["the quick", "brown fox"]);
    // "the quick" is 90 px, and "the quick b" would be 110.
    assert_eq!(lines("the quick b"), ["the quick", "b"]);
    assert_eq!(lines("exactly 10"), ["exactly 10"], "fits");
}

#[test]
fn line_breaks_are_kept_and_a_long_word_runs_over() {
    assert_eq!(
        lines(
            "one
two three"
        ),
        ["one", "two three"]
    );
    assert_eq!(
        lines("a supercalifragilistic b"),
        ["a", "supercalifragilistic", "b"]
    );
    assert_eq!(
        lines("supercalifragilistic b"),
        ["supercalifragilistic", "b"],
        "no empty line before a long first word"
    );
    assert_eq!(
        lines(
            "first

third"
        ),
        ["first", "", "third"]
    );
    assert_eq!(
        lines(
            "one
two
"
        ),
        ["one", "two"],
        "as str::lines"
    );
    assert!(lines("").is_empty());
    assert_eq!(lines("  a   b "), ["a b"], "spaces collapse");
}

#[test]
fn a_line_may_also_end_after_a_hyphen_or_between_ideographs() {
    // "a well-known" is 120 px; UAX #14 allows a break after the hyphen.
    assert_eq!(lines("a well-known"), ["a well-", "known"]);
    // Twelve ideographs, 120 px, with a break allowed between any two.
    assert_eq!(
        lines(
            "\u{4e00}\u{4e8c}\u{4e09}\u{56db}\u{4e94}\u{516d}\u{4e03}\u{516b}\u{4e5d}\u{5341}\u{767e}\u{5343}"
        ),
        [
            "\u{4e00}\u{4e8c}\u{4e09}\u{56db}\u{4e94}\u{516d}\u{4e03}\u{516b}\u{4e5d}\u{5341}",
            "\u{767e}\u{5343}"
        ]
    );
    // A line separator (U+2028) is a hard break, as a line feed is.
    assert_eq!(lines("one\u{2028}two"), ["one", "two"]);
}

#[test]
fn bold_is_measured_bold_and_the_space_starts_the_next_span() {
    // "bold words" fits 100 px in the regular face, but "bold" is 60 px.
    assert_eq!(lines("<b>bold</b> words"), ["bold", "words"]);
    let bold = Style {
        bold: true,
        ..Style::default()
    };
    assert_eq!(
        wrap(&parse("a <b>b c</b>"), 1000.0, measure),
        [vec![
            Span {
                text: "a".to_owned(),
                style: Style::default()
            },
            Span {
                text: " b c".to_owned(),
                style: bold
            },
        ]],
        "no span ends in a space"
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
            Item::Text {
                text, rect, colour, ..
            } if text == "2 mins ago" => Some((*rect, *colour)),
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
    assert_eq!(
        width("a body"),
        measure("a body that may use the whole width", TextStyle::PLAIN),
        "the body is not cut for the time"
    );
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

#[test]
fn the_body_s_spans_follow_one_another_and_a_link_is_blue_and_underlined() {
    let theme = theme();
    let content = Incoming {
        summary: "s".to_owned(),
        body: "a <b>b</b> <a href=\"https://x\">link</a>".to_owned(),
        ..Incoming::default()
    };
    let (list, _, _) = draw(&content, Duration::ZERO, WIDTH, measure, 20.0, &theme);
    let body: Vec<(&str, f32, f32, TextStyle, bool)> = list
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Text {
                text,
                rect,
                colour,
                style,
            } if rect.y > BORDER + INSET => Some((
                text.as_str(),
                rect.x,
                rect.width,
                *style,
                *colour == theme.toast_link,
            )),
            _ => None,
        })
        .collect();
    let left = BORDER + INSET;
    let bold = TextStyle {
        bold: true,
        ..TextStyle::PLAIN
    };
    let underlined = TextStyle {
        underline: true,
        ..TextStyle::PLAIN
    };
    assert_eq!(
        body,
        [
            ("a", left, 10.0, TextStyle::PLAIN, false),
            (" b", left + 10.0, 30.0, bold, false),
            (" link", left + 40.0, 50.0, underlined, true),
        ]
    );
}

#[test]
fn a_span_is_cut_at_the_text_s_right_edge() {
    let theme = theme();
    let long = "x".repeat(60);
    let content = Incoming {
        summary: "s".to_owned(),
        body: long.clone(),
        ..Incoming::default()
    };
    let (list, _, _) = draw(&content, Duration::ZERO, WIDTH, measure, 20.0, &theme);
    let width = list.items.iter().find_map(|item| match item {
        Item::Text { text, rect, .. } if *text == long => Some(rect.width),
        _ => None,
    });
    assert_eq!(width, Some(WIDTH - 2.0 * (BORDER + INSET)), "not 600 px");
}

/// A picture `width` by `height` pixels.
fn picture(width: u32, height: u32) -> Picture {
    Picture::new(width, height, vec![255; (width * height * 4) as usize]).expect("a picture")
}

/// Each picture drawn, and where.
fn images(items: &[Item]) -> Vec<(Rect, &Picture)> {
    items
        .iter()
        .filter_map(|item| match item {
            Item::Image { rect, picture } => Some((*rect, picture)),
            _ => None,
        })
        .collect()
}

/// Where each text starts.
fn text_xs(items: &[Item]) -> Vec<(&str, f32)> {
    items
        .iter()
        .filter_map(|item| match item {
            Item::Text { text, rect, .. } => Some((text.as_str(), rect.x)),
            _ => None,
        })
        .collect()
}

#[test]
fn an_image_stands_left_of_the_text_with_the_app_icon_over_it() {
    let theme = theme();
    let (image, icon) = (picture(2, 1), picture(1, 1));
    let content = Incoming {
        summary: "Shot".to_owned(),
        body: "saved".to_owned(),
        image: Some(image.clone()),
        icon: Some(icon.clone()),
        ..with_actions()
    };
    let (list, size, buttons) = draw(&content, Duration::ZERO, WIDTH, measure, 20.0, &theme);
    let top = BORDER + INSET;
    let left = top + 2.0 * PICTURE_MARGIN + PICTURE_SIZE + PICTURE_PADDING;
    // The text's two lines are 40 px, the picture's square 16 in its 4 px
    // margins: the square is centred on the text.
    let y = top + (40.0 - PICTURE_SIZE) / 2.0;
    let x = top + PICTURE_MARGIN;
    assert_eq!(
        images(&list.items),
        [
            (
                Rect {
                    x,
                    y: y + 4.0,
                    width: 16.0,
                    height: 8.0
                },
                &image
            ),
            (
                Rect {
                    x: x + PICTURE_PADDING,
                    y,
                    width: 16.0,
                    height: 16.0
                },
                &icon
            ),
        ],
        "the image fitted to the square, the icon at its right end"
    );
    let xs = text_xs(&list.items);
    assert!(xs.contains(&("Shot", left)) && xs.contains(&("saved", left)));
    assert_eq!(
        (buttons[0].1.x, buttons[0].1.width),
        (top, (WIDTH - 2.0 * top - 8.0) / 2.0),
        "the buttons run under the picture as well"
    );
    assert_eq!(
        size.height,
        top + 40.0 + buttons[0].1.height + 8.0 + top,
        "no taller than the text needs"
    );
}

#[test]
fn the_app_icon_alone_takes_the_image_s_place() {
    let theme = theme();
    let icon = picture(1, 1);
    let content = Incoming {
        summary: "Ping".to_owned(),
        body: "pong".to_owned(),
        icon: Some(icon.clone()),
        ..Incoming::default()
    };
    let (list, _, _) = draw(&content, Duration::ZERO, WIDTH, measure, 20.0, &theme);
    let top = BORDER + INSET;
    assert_eq!(
        images(&list.items),
        [(
            Rect {
                x: top + PICTURE_MARGIN,
                y: top + (40.0 - PICTURE_SIZE) / 2.0,
                width: 16.0,
                height: 16.0
            },
            &icon
        )]
    );
    let plain = Incoming {
        icon: None,
        ..content
    };
    let (list, _, _) = draw(&plain, Duration::ZERO, WIDTH, measure, 20.0, &theme);
    assert!(images(&list.items).is_empty());
    assert!(
        text_xs(&list.items).contains(&("Ping", top)),
        "the text moves back"
    );
}

#[test]
fn a_picture_taller_than_the_text_centres_the_text_on_it() {
    let theme = theme();
    let content = Incoming {
        summary: "Ping".to_owned(),
        icon: Some(picture(1, 1)),
        ..Incoming::default()
    };
    let (list, size, _) = draw(&content, Duration::ZERO, WIDTH, measure, 10.0, &theme);
    let top = BORDER + INSET;
    let row = PICTURE_SIZE + 2.0 * PICTURE_MARGIN;
    assert_eq!(size.height, top + row + top);
    assert!(texts(&list.items).contains(&("Ping", top + (row - 10.0) / 2.0)));
    assert_eq!(images(&list.items)[0].0.y, top + PICTURE_MARGIN);
}
