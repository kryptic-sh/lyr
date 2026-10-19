use chrono::{NaiveDate, Weekday};
use lyr_config::{Platform, Theme, from_sources};

use super::draw;
use crate::calendar::Mode;
use crate::draw::{Item, TextStyle};
use crate::hit::Rect;

fn theme() -> Theme {
    from_sources(None, Platform::Linux)
        .expect("the defaults")
        .theme
}

/// Every character is 10 px wide, so a cell ("31", "Mo") is 20 px, a gap 10,
/// the week column ("W00") 30, and a month block 7 * 20 + 7 * 10 + 30 = 240.
fn measure(text: &str, _: TextStyle) -> f32 {
    text.chars().count() as f32 * 10.0
}

const LINE: f32 = 20.0;

fn date(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).expect("a date")
}

/// Each text drawn, with its rect.
fn texts(items: &[Item]) -> Vec<(&str, Rect)> {
    items
        .iter()
        .filter_map(|item| match item {
            Item::Text { text, rect, .. } => Some((text.as_str(), *rect)),
            _ => None,
        })
        .collect()
}

fn find<'a>(items: &'a [Item], wanted: &str) -> Vec<&'a Item> {
    items
        .iter()
        .filter(|item| matches!(item, Item::Text { text, .. } if text == wanted))
        .collect()
}

#[test]
fn a_month_has_its_title_weekdays_and_weeks() {
    let theme = theme();
    let today = date(2026, 10, 19);
    let (list, size) = draw(
        today,
        Mode::Month,
        Weekday::Mon,
        today,
        &measure,
        LINE,
        &theme,
    );
    // October 2026 starts on a Thursday and takes five rows.
    assert_eq!((size.width, size.height), (240.0, 7.0 * LINE));
    let drawn = texts(&list.items);
    assert_eq!(drawn[0].0, "October 2026");
    let header: Vec<&str> = drawn[1..8].iter().map(|(text, _)| *text).collect();
    assert_eq!(header, ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"]);
    let weeks: Vec<&str> = drawn
        .iter()
        .map(|(text, _)| *text)
        .filter(|text| text.starts_with('W') && text.len() == 3 && text != &"We")
        .collect();
    assert_eq!(weeks, ["W39", "W40", "W41", "W42", "W43"]);
}

#[test]
fn days_are_right_aligned_in_their_column() {
    let theme = theme();
    let today = date(2026, 10, 19);
    let (list, _) = draw(
        today,
        Mode::Month,
        Weekday::Mon,
        today,
        &measure,
        LINE,
        &theme,
    );
    let right = |day: &str| {
        let [Item::Text { rect, .. }] = find(&list.items, day)[..] else {
            panic!("one {day}");
        };
        (rect.x + rect.width, rect.y)
    };
    // The 1st is a Thursday, the fourth column; the 8th sits under it.
    let (one, one_y) = right("1");
    let (eight, eight_y) = right("8");
    assert_eq!(one, 3.0 * 30.0 + 20.0);
    assert_eq!(eight, one);
    assert_eq!((one_y, eight_y), (2.0 * LINE, 3.0 * LINE));
}

#[test]
fn today_is_coloured_and_underlined() {
    let theme = theme();
    let today = date(2026, 10, 19);
    let (list, _) = draw(
        today,
        Mode::Month,
        Weekday::Mon,
        today,
        &measure,
        LINE,
        &theme,
    );
    let [Item::Text { colour, style, .. }] = find(&list.items, "19")[..] else {
        panic!("one 19");
    };
    assert_eq!(*colour, theme.calendar_today);
    assert!(style.bold && style.underline);
    let [Item::Text { colour, style, .. }] = find(&list.items, "18")[..] else {
        panic!("one 18");
    };
    assert_eq!(*colour, theme.calendar_days);
    assert!(style.bold && !style.underline);
    // Another year's 19 October is not today.
    let (list, _) = draw(
        date(2025, 10, 1),
        Mode::Month,
        Weekday::Mon,
        today,
        &measure,
        LINE,
        &theme,
    );
    assert!(
        !list
            .items
            .iter()
            .any(|item| matches!(item, Item::Text { style, .. } if style.underline))
    );
}

#[test]
fn a_year_is_twelve_months_three_across() {
    let theme = theme();
    let today = date(2026, 10, 19);
    let (list, size) = draw(
        today,
        Mode::Year,
        Weekday::Mon,
        today,
        &measure,
        LINE,
        &theme,
    );
    let titles: Vec<(&str, Rect)> = texts(&list.items)
        .into_iter()
        .filter(|(text, _)| text.ends_with(" 2026"))
        .collect();
    assert_eq!(titles.len(), 12);
    // Three blocks with two three-space gaps between them.
    assert_eq!(size.width, 3.0 * 240.0 + 2.0 * 30.0);
    let at = |name: &str| {
        titles
            .iter()
            .find(|(text, _)| text.starts_with(name))
            .map(|(_, rect)| (rect.x, rect.y))
            .expect("a title")
    };
    assert_eq!(at("February"), (270.0, 0.0));
    // January to March 2026 take five, five and six rows: eight lines with
    // their title and weekdays, then a blank line.
    assert_eq!(at("April"), (0.0, 9.0 * LINE));
    // April to June take five rows each (seven lines), and July to September
    // eight lines for August, the tallest, in the middle.
    assert_eq!(at("October"), (0.0, (9.0 + 8.0 + 8.0 + 1.0) * LINE));
    // The first row of January 2026 is week 0, written with two digits.
    assert_eq!(find(&list.items, "W00").len(), 1);
    assert!(size.height > 0.0);
}

#[test]
fn spaces_are_measured_between_letters() {
    // A measure that, like a text layout, leaves whitespace at the ends out.
    let trimmed = |text: &str, style| measure(text.trim(), style);
    let theme = theme();
    let today = date(2026, 10, 19);
    let (_, spaced) = draw(
        today,
        Mode::Month,
        Weekday::Mon,
        today,
        &measure,
        LINE,
        &theme,
    );
    let (_, trimmed) = draw(
        today,
        Mode::Month,
        Weekday::Mon,
        today,
        &trimmed,
        LINE,
        &theme,
    );
    assert_eq!(trimmed.width, spaced.width);
}

#[test]
fn it_is_all_bold_and_measured_so() {
    let theme = theme();
    let today = date(2026, 10, 19);
    // A measure that sees only bold text.
    let bold_only = |text: &str, style: TextStyle| {
        if style.bold {
            measure(text, style)
        } else {
            0.0
        }
    };
    let (list, size) = draw(
        today,
        Mode::Year,
        Weekday::Mon,
        today,
        &bold_only,
        LINE,
        &theme,
    );
    assert_eq!(size.width, 3.0 * 240.0 + 2.0 * 30.0, "measured in bold");
    assert!(
        list.items
            .iter()
            .all(|item| matches!(item, Item::Text { style, .. } if style.bold))
    );
}
