use std::ops::Range;
use std::time::Duration;

use lyr_config::{Platform, Theme, from_sources};

use super::{
    Area, BORDER, INSET, PICTURE_MARGIN, PICTURE_PADDING, PICTURE_SIZE, PROGRESS_HEIGHT,
    PROGRESS_ROW, Placed, TIME_GAP, WIDTH, draw, relative_time,
};
use crate::draw::{Item, Picture, TextStyle};
use crate::hit::Rect;
use crate::markup::{Span, parse};
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

/// A stand-in for lyr-render's layout, measuring as [`measure`] does: lines
/// break at spaces, which go on no line where they break it, and at a line
/// feed; a final one starts no line.
pub fn lay_out(spans: &[Span], width: f32) -> Vec<Vec<Placed>> {
    // Each char: its span, its bytes there, the char, its width.
    let mut chars = Vec::new();
    for (index, span) in spans.iter().enumerate() {
        let each = if span.style.bold { 15.0 } else { 10.0 };
        for (at, c) in span.text.char_indices() {
            chars.push((index, at..at + c.len_utf8(), c, each));
        }
    }
    let width_of = |run: &[(usize, Range<usize>, char, f32)]| -> f32 {
        run.iter().map(|&(_, _, _, each)| each).sum()
    };
    let mut lines = Vec::new();
    for paragraph in chars.split(|&(_, _, c, _)| c == '\n') {
        let mut line = Vec::new();
        let mut spaces: &[_] = &[];
        for run in paragraph.chunk_by(|a, b| (a.2 == ' ') == (b.2 == ' ')) {
            if run[0].2 == ' ' {
                spaces = run;
                continue;
            }
            if line.is_empty() {
            } else if width_of(&line) + width_of(spaces) + width_of(run) > width {
                lines.push(placed(&std::mem::take(&mut line)));
            } else {
                line.extend_from_slice(spaces);
            }
            line.extend_from_slice(run);
            spaces = &[];
        }
        lines.push(placed(&line));
    }
    if chars.last().is_none_or(|&(_, _, c, _)| c == '\n') {
        lines.pop();
    }
    lines
}

/// `line`'s chars as stretches of one span each.
fn placed(line: &[(usize, Range<usize>, char, f32)]) -> Vec<Placed> {
    let mut x = 0.0;
    line.chunk_by(|a, b| a.0 == b.0)
        .map(|run| {
            let width = run.iter().map(|&(_, _, _, each)| each).sum();
            let placed = Placed {
                span: run[0].0,
                text: run[0].1.start..run[run.len() - 1].1.end,
                x,
                width,
            };
            x += width;
            placed
        })
        .collect()
}

#[test]
fn the_stand_in_layout_wraps_as_lyr_render_s_does() {
    let lines = |text: &str| -> Vec<Vec<String>> {
        let spans = parse(text);
        lay_out(&spans, 100.0)
            .iter()
            .map(|line| {
                line.iter()
                    .map(|placed| spans[placed.span].text[placed.text.clone()].to_owned())
                    .collect()
            })
            .collect()
    };
    assert_eq!(lines("the quick brown fox"), [["the quick"], ["brown fox"]]);
    assert_eq!(lines("one\ntwo\n"), [["one"], ["two"]]);
    assert!(lines("").is_empty());
    assert_eq!(lines("a <b>b</b>"), [vec!["a ", "b"]]);
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
    let (list, size, _) = draw(
        &content,
        Duration::ZERO,
        WIDTH,
        measure,
        &lay_out,
        20.0,
        &theme,
    );
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
    let (list, size, _) = draw(
        &no_body,
        Duration::ZERO,
        WIDTH,
        measure,
        &lay_out,
        20.0,
        &theme,
    );
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
        match draw(
            &content,
            Duration::ZERO,
            WIDTH,
            measure,
            &lay_out,
            20.0,
            &theme,
        )
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
        &lay_out,
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
    let (list, _, _) = draw(
        &content,
        Duration::ZERO,
        WIDTH,
        measure,
        &lay_out,
        20.0,
        &theme,
    );
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
        &lay_out,
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
            (Area::Action(1), BORDER + INSET, y, width, height),
            (
                Area::Action(2),
                BORDER + INSET + width + 8.0,
                y,
                width,
                height
            )
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
    let (_, size, buttons) = draw(
        &content,
        Duration::ZERO,
        WIDTH,
        measure,
        &lay_out,
        20.0,
        &theme,
    );
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
    let (list, _, _) = draw(
        &content,
        Duration::ZERO,
        WIDTH,
        measure,
        &lay_out,
        20.0,
        &theme,
    );
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
            ("a ", left, 20.0, TextStyle::PLAIN, false),
            ("b", left + 20.0, 15.0, bold, false),
            (" ", left + 35.0, 10.0, TextStyle::PLAIN, false),
            ("link", left + 45.0, 40.0, underlined, true),
        ],
        "each stretch where the layout puts it, a space in its own span"
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
    let (list, _, _) = draw(
        &content,
        Duration::ZERO,
        WIDTH,
        measure,
        &lay_out,
        20.0,
        &theme,
    );
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
    let (list, size, buttons) = draw(
        &content,
        Duration::ZERO,
        WIDTH,
        measure,
        &lay_out,
        20.0,
        &theme,
    );
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
    let (list, _, _) = draw(
        &content,
        Duration::ZERO,
        WIDTH,
        measure,
        &lay_out,
        20.0,
        &theme,
    );
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
    let (list, _, _) = draw(
        &plain,
        Duration::ZERO,
        WIDTH,
        measure,
        &lay_out,
        20.0,
        &theme,
    );
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
    let (list, size, _) = draw(
        &content,
        Duration::ZERO,
        WIDTH,
        measure,
        &lay_out,
        10.0,
        &theme,
    );
    let top = BORDER + INSET;
    let row = PICTURE_SIZE + 2.0 * PICTURE_MARGIN;
    assert_eq!(size.height, top + row + top);
    assert!(texts(&list.items).contains(&("Ping", top + (row - 10.0) / 2.0)));
    assert_eq!(images(&list.items)[0].0.y, top + PICTURE_MARGIN);
}

#[test]
fn a_link_is_an_area_on_every_line_it_wraps_to() {
    let theme = theme();
    let content = Incoming {
        summary: "x".to_owned(),
        body: "go <a href=\"https://x\">here now</a>".to_owned(),
        ..Incoming::default()
    };
    // 60 px of text: "go", "here" and "now" each take a line.
    let width = 2.0 * (BORDER + INSET) + 60.0;
    let (_, _, areas) = draw(
        &content,
        Duration::ZERO,
        width,
        measure,
        &lay_out,
        20.0,
        &theme,
    );
    let top = BORDER + INSET;
    let line = |y, width| Rect {
        x: top,
        y,
        width,
        height: 20.0,
    };
    assert_eq!(
        areas,
        [
            (Area::Link(0), line(top + 40.0, 40.0)),
            (Area::Link(0), line(top + 60.0, 30.0)),
        ]
    );
}

#[test]
fn a_progress_hint_is_a_bar_under_the_text_as_long_as_the_value() {
    let theme = theme();
    let progress = |percent, urgency| {
        let content = Incoming {
            summary: "Copying".to_owned(),
            urgency,
            progress: Some(percent),
            ..with_actions()
        };
        draw(
            &content,
            Duration::ZERO,
            WIDTH,
            measure,
            &lay_out,
            20.0,
            &theme,
        )
    };
    let bars = |items: &[Item]| -> Vec<(Rect, lyr_config::Colour)> {
        items
            .iter()
            .filter_map(|item| match item {
                Item::Box {
                    rect,
                    fill,
                    border: None,
                    ..
                } => Some((*rect, *fill)),
                _ => None,
            })
            .collect()
    };
    let top = BORDER + INSET;
    let content_width = WIDTH - 2.0 * top;
    let (list, size, buttons) = progress(40, Urgency::Normal);
    let row = top + 20.0;
    assert_eq!(
        bars(&list.items),
        [(
            Rect {
                x: top,
                y: row + (PROGRESS_ROW - PROGRESS_HEIGHT) / 2.0,
                width: content_width * 40.0 / 100.0,
                height: PROGRESS_HEIGHT,
            },
            theme.toast_progress
        )]
    );
    let (_, without, _) = draw(
        &with_actions(),
        Duration::ZERO,
        WIDTH,
        measure,
        &lay_out,
        20.0,
        &theme,
    );
    assert_eq!(
        size.height,
        without.height + PROGRESS_ROW,
        "a row of its own"
    );
    assert!(
        buttons.iter().all(|(_, rect)| rect.y > row + PROGRESS_ROW),
        "the buttons below it"
    );
    let (list, _, _) = progress(100, Urgency::Critical);
    assert_eq!(
        bars(&list.items)[0],
        (
            Rect {
                x: top,
                y: row + (PROGRESS_ROW - PROGRESS_HEIGHT) / 2.0,
                width: content_width,
                height: PROGRESS_HEIGHT,
            },
            theme.toast_progress_critical
        )
    );
    let (list, size, _) = progress(0, Urgency::Normal);
    assert!(bars(&list.items).is_empty(), "nothing filled");
    assert_eq!(size.height, without.height + PROGRESS_ROW, "the row kept");
}
