//! Drawing the calendar (`docs/plan/10-popups-and-menus.md#clock-panel`):
//! waybar's month blocks, a title, the weekday names and a row per week with
//! its number on the right, in year mode twelve of them three across.
//!
//! waybar lays the calendar out as monospace text. lyr lays it out as a grid
//! of cells, each as wide as the widest day or weekday name in the calendar's
//! font, so the columns line up in a proportional font too. Names are English
//! and weeks start on the first weekday the caller gives (plan 16, 19).

use chrono::{Datelike, NaiveDate, Weekday};
use lyr_config::{Colour, Theme};

use super::{Mode, Week, month, weekdays};
use crate::draw::{DrawList, Item, Size};
use crate::hit::Rect;

/// Months across in year mode: waybar's `mode-mon-col` in the dotfiles.
pub const COLUMNS: usize = 3;
/// The underline under today's date.
const UNDERLINE: f32 = 1.0;

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// A weekday's name cut to two letters, as waybar cuts `%a`.
const fn short(day: Weekday) -> &'static str {
    match day {
        Weekday::Mon => "Mo",
        Weekday::Tue => "Tu",
        Weekday::Wed => "We",
        Weekday::Thu => "Th",
        Weekday::Fri => "Fr",
        Weekday::Sat => "Sa",
        Weekday::Sun => "Su",
    }
}

/// What every month block is drawn with: the font's measurements, the first
/// weekday, today and the theme.
struct Grid<'a> {
    measure: &'a dyn Fn(&str) -> f32,
    line_height: f32,
    /// One day's or weekday name's column.
    cell: f32,
    /// Between two columns: a space.
    gap: f32,
    /// The week number's column.
    week: f32,
    first: Weekday,
    today: NaiveDate,
    theme: &'a Theme,
}

impl Grid<'_> {
    /// One month block's width: seven day columns and the week number.
    fn width(&self) -> f32 {
        7.0 * self.cell + 7.0 * self.gap + self.week
    }

    /// Text right-aligned in the cell at `column` of the block at `x`.
    fn cell_text(&self, x: f32, y: f32, column: usize, text: &str, colour: Colour) -> Item {
        let width = (self.measure)(text);
        let right = x + (column as f32 + 1.0) * self.cell + column as f32 * self.gap;
        Item::Text {
            rect: Rect {
                x: right - width,
                y,
                width,
                height: self.line_height,
            },
            text: text.to_owned(),
            colour,
        }
    }
}

/// The calendar showing `shown`'s month or year in `mode`, weeks from
/// `first`, with `today` marked: its draw list and its size, its text
/// measured by `measure` in lines `line_height` tall.
#[must_use]
pub fn draw(
    shown: NaiveDate,
    mode: Mode,
    first: Weekday,
    today: NaiveDate,
    measure: &dyn Fn(&str) -> f32,
    line_height: f32,
    theme: &Theme,
) -> (DrawList, Size) {
    let cell = (1..=31)
        .map(|day| measure(&day.to_string()))
        .chain(weekdays(first).map(|day| measure(short(day))))
        .fold(0.0, f32::max);
    let grid = Grid {
        measure,
        line_height,
        cell,
        gap: spaces(measure, 1),
        week: measure("W00"),
        first,
        today,
        theme,
    };
    let months: Vec<u32> = match mode {
        Mode::Year => (1..=12).collect(),
        Mode::Month => vec![shown.month()],
    };
    let between = spaces(measure, 3);
    let mut items = Vec::new();
    let mut y = 0.0;
    let mut width: f32 = 0.0;
    for (index, row) in months.chunks(COLUMNS).enumerate() {
        // A blank line between rows of months, as waybar's year mode has.
        if index > 0 {
            y += line_height;
        }
        let mut lines: usize = 0;
        for (column, &number) in row.iter().enumerate() {
            let x = column as f32 * (grid.width() + between);
            let weeks = month(shown.year(), number, first).expect("a month of the year");
            lines = lines.max(weeks.len() + 2);
            block(&mut items, &grid, (x, y), (shown.year(), number), &weeks);
            width = width.max(x + grid.width());
        }
        y += lines as f32 * line_height;
    }
    let size = Size { width, height: y };
    (DrawList { items }, size)
}

/// The width of `count` spaces, measured between two letters: a font's
/// measure can leave trailing or lone whitespace out.
fn spaces(measure: &dyn Fn(&str) -> f32, count: usize) -> f32 {
    let spaced = format!("a{}a", " ".repeat(count));
    measure(&spaced) - measure("aa")
}

/// One month block at `at`: its title, weekday names and weeks.
fn block(
    items: &mut Vec<Item>,
    grid: &Grid<'_>,
    at: (f32, f32),
    (year, number): (i32, u32),
    weeks: &[Week],
) {
    let Grid {
        first,
        today,
        theme,
        ..
    } = *grid;
    let (x, y) = at;
    let lh = grid.line_height;
    let title = format!("{} {year}", MONTHS[number as usize - 1]);
    items.push(Item::Text {
        rect: Rect {
            x,
            y,
            width: (grid.measure)(&title),
            height: lh,
        },
        text: title,
        colour: theme.calendar_months,
    });
    for (column, day) in weekdays(first).into_iter().enumerate() {
        items.push(grid.cell_text(x, y + lh, column, short(day), theme.calendar_weekdays));
    }
    for (row, week) in weeks.iter().enumerate() {
        let row_y = y + lh * (row + 2) as f32;
        for (column, day) in week.days.iter().enumerate() {
            let Some(day) = day else { continue };
            let is_today = (today.year(), today.month(), today.day()) == (year, number, *day);
            let colour = if is_today {
                theme.calendar_today
            } else {
                theme.calendar_days
            };
            let text = grid.cell_text(x, row_y, column, &day.to_string(), colour);
            if is_today && let Item::Text { rect, .. } = &text {
                items.push(Item::Box {
                    rect: Rect {
                        y: rect.y + rect.height - UNDERLINE,
                        height: UNDERLINE,
                        ..*rect
                    },
                    radius: 0.0,
                    fill: theme.calendar_today,
                    border: None,
                });
            }
            items.push(text);
        }
        let number = format!("W{:02}", week.number);
        let week_x = x + 7.0 * grid.cell + 7.0 * grid.gap;
        items.push(Item::Text {
            rect: Rect {
                x: week_x,
                y: row_y,
                width: grid.week,
                height: lh,
            },
            text: number,
            colour: theme.calendar_weeks,
        });
    }
}

#[cfg(test)]
mod tests;
