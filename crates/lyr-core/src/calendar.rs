//! The clock panel's calendar maths (`docs/plan/10-popups-and-menus.md#clock-panel`):
//! a month's rows of days, each row's week number, and shifting the view by a
//! month or a year. It follows waybar 0.15.0's `clock.cpp`, whose calendar the
//! panel carries over.

pub mod draw;

use jiff::Span;
use jiff::civil::{Date, Weekday};

/// One row of a month: its week number, and the day of the month in each of
/// the seven columns, `None` outside the month.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Week {
    pub number: u32,
    pub days: [Option<i8>; 7],
}

/// The seven weekdays in column order, starting from `first`.
#[must_use]
pub fn weekdays(first: Weekday) -> [Weekday; 7] {
    let mut day = first;
    std::array::from_fn(|_| {
        let this = day;
        day = day.next();
        this
    })
}

/// The rows of `month` in `year` (`month` from 1), weeks starting on `first`.
/// `None` for a month that does not exist.
#[must_use]
pub fn month(year: i16, month: i8, first: Weekday) -> Option<Vec<Week>> {
    let start = Date::new(year, month, 1).ok()?;
    let length = start.days_in_month();
    // Columns before the 1st.
    let offset = start.weekday().since(first);
    let rows = (offset + length + 6) / 7;
    (0..rows)
        .map(|row| {
            let days = std::array::from_fn(|column| {
                // `column` is below 7.
                let day = row * 7 + column as i8 - offset + 1;
                (1..=length).contains(&day).then_some(day)
            });
            // waybar numbers the first row by the 1st and every other row by
            // its first day.
            let numbered = if row == 0 {
                start
            } else {
                start.with().day(row * 7 + 1 - offset).build().ok()?
            };
            Some(Week {
                number: week_number(numbered, first),
                days,
            })
        })
        .collect()
}

/// The week of the year `date` falls in, as waybar shows it with no
/// `iso8601`: strftime's `%W` (weeks from the first Monday) when weeks start
/// on Monday, `%U` (from the first Sunday) otherwise. Both count the days
/// before that first day as week 0.
#[must_use]
pub fn week_number(date: Date, first: Weekday) -> u32 {
    let from = if first == Weekday::Monday {
        Weekday::Monday
    } else {
        Weekday::Sunday
    };
    // POSIX strftime: (tm_yday + 7 - days since the week's first day) / 7,
    // tm_yday counting from 0.
    let yday = u32::from(date.day_of_year().unsigned_abs()) - 1;
    (yday + 7 - u32::from(date.weekday().since(from).unsigned_abs())) / 7
}

/// What the calendar shows: a year of twelve months, or one month.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Year,
    Month,
}

/// The month the view shows after moving `steps` units of `mode` from
/// `shown` (a year in year mode, a month in month mode, backwards when
/// negative), as waybar's `on-scroll` does. `None` past jiff's range.
#[must_use]
pub fn shift(shown: Date, mode: Mode, steps: i32) -> Option<Date> {
    let unit = match mode {
        Mode::Year => 12,
        Mode::Month => 1,
    };
    let months = i64::from(steps) * unit;
    shown
        .first_of_month()
        .checked_add(Span::new().try_months(months).ok()?)
        .ok()
}

#[cfg(test)]
mod tests;
