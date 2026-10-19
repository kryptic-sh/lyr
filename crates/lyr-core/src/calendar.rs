//! The clock panel's calendar maths (`docs/plan/10-popups-and-menus.md#clock-panel`):
//! a month's rows of days, each row's week number, and shifting the view by a
//! month or a year. It follows waybar 0.15.0's `clock.cpp`, whose calendar the
//! panel carries over.

use chrono::{Datelike, Months, NaiveDate, Weekday};

/// One row of a month: its week number, and the day of the month in each of
/// the seven columns, `None` outside the month.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Week {
    pub number: u32,
    pub days: [Option<u32>; 7],
}

/// The seven weekdays in column order, starting from `first`.
#[must_use]
pub fn weekdays(first: Weekday) -> [Weekday; 7] {
    let mut day = first;
    std::array::from_fn(|_| {
        let this = day;
        day = day.succ();
        this
    })
}

/// The rows of `month` in `year` (`month` from 1), weeks starting on `first`.
/// `None` for a month that does not exist.
#[must_use]
pub fn month(year: i32, month: u32, first: Weekday) -> Option<Vec<Week>> {
    let start = NaiveDate::from_ymd_opt(year, month, 1)?;
    let length = start.checked_add_months(Months::new(1))?.pred_opt()?.day();
    // Columns before the 1st.
    let offset = start.weekday().days_since(first);
    let rows = (offset + length).div_ceil(7);
    (0..rows)
        .map(|row| {
            let days = std::array::from_fn(|column| {
                let day = (row * 7 + column as u32).checked_sub(offset)? + 1;
                (day <= length).then_some(day)
            });
            // waybar numbers the first row by the 1st and every other row by
            // its first day.
            let numbered = if row == 0 {
                start
            } else {
                start.with_day(row * 7 + 1 - offset)?
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
pub fn week_number(date: NaiveDate, first: Weekday) -> u32 {
    let from = if first == Weekday::Mon {
        Weekday::Mon
    } else {
        Weekday::Sun
    };
    // POSIX strftime: (tm_yday + 7 - days since the week's first day) / 7.
    (date.ordinal0() + 7 - date.weekday().days_since(from)) / 7
}

/// What the calendar shows: a year of twelve months, or one month.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Year,
    Month,
}

/// The month the view shows after moving `steps` units of `mode` from
/// `shown` (a year in year mode, a month in month mode, backwards when
/// negative), as waybar's `on-scroll` does. `None` past chrono's range.
#[must_use]
pub fn shift(shown: NaiveDate, mode: Mode, steps: i32) -> Option<NaiveDate> {
    let unit = match mode {
        Mode::Year => 12,
        Mode::Month => 1,
    };
    let months = Months::new(steps.unsigned_abs().checked_mul(unit)?);
    let first = shown.with_day(1)?;
    if steps < 0 {
        first.checked_sub_months(months)
    } else {
        first.checked_add_months(months)
    }
}

#[cfg(test)]
mod tests;
