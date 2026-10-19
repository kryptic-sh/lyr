use chrono::{Datelike, NaiveDate, Weekday};

use super::{Mode, Week, month, shift, week_number, weekdays};

fn date(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).expect("a date")
}

fn days(rows: &[Week]) -> Vec<[Option<u32>; 7]> {
    rows.iter().map(|week| week.days).collect()
}

const fn d(day: u32) -> Option<u32> {
    Some(day)
}

#[test]
fn a_month_is_laid_out_from_the_first_weekday() {
    // 1 October 2026 is a Thursday.
    let monday = month(2026, 10, Weekday::Mon).expect("a month");
    assert_eq!(
        days(&monday),
        [
            [None, None, None, d(1), d(2), d(3), d(4)],
            [d(5), d(6), d(7), d(8), d(9), d(10), d(11)],
            [d(12), d(13), d(14), d(15), d(16), d(17), d(18)],
            [d(19), d(20), d(21), d(22), d(23), d(24), d(25)],
            [d(26), d(27), d(28), d(29), d(30), d(31), None],
        ]
    );
    let sunday = month(2026, 10, Weekday::Sun).expect("a month");
    assert_eq!(sunday[0].days, [None, None, None, None, d(1), d(2), d(3)]);
    assert_eq!(
        sunday.last().expect("a row").days,
        [d(25), d(26), d(27), d(28), d(29), d(30), d(31)]
    );
    assert_eq!(sunday.len(), 5);
}

#[test]
fn a_month_that_fills_its_rows_has_no_spare_row() {
    // 1 February 2026 is a Sunday, and the month has 28 days.
    let rows = month(2026, 2, Weekday::Sun).expect("a month");
    assert_eq!(rows.len(), 4);
    assert_eq!(rows[3].days, [22, 23, 24, 25, 26, 27, 28].map(Some));
    // Starting on Monday it spills into a fifth row.
    assert_eq!(month(2026, 2, Weekday::Mon).expect("a month").len(), 5);
}

#[test]
fn a_leap_february_has_its_29th() {
    let rows = month(2028, 2, Weekday::Mon).expect("a month");
    let last = rows.last().expect("a row").days;
    assert!(last.contains(&d(29)) && !last.contains(&d(30)));
}

#[test]
fn rows_are_numbered_by_the_first_then_by_their_first_day() {
    // 1 January 2026 is a Thursday: the row it starts has its Monday in 2025,
    // but waybar numbers the row by the 1st, week 0.
    let rows = month(2026, 1, Weekday::Mon).expect("a month");
    let numbers: Vec<u32> = rows.iter().map(|week| week.number).collect();
    assert_eq!(numbers, [0, 1, 2, 3, 4]);
    // A month starting on the first weekday: 1 June 2026 is a Monday.
    let rows = month(2026, 6, Weekday::Mon).expect("a month");
    assert_eq!(rows[0].days[0], d(1));
    assert_eq!(
        rows.iter().map(|week| week.number).collect::<Vec<_>>(),
        [22, 23, 24, 25, 26]
    );
}

#[test]
fn week_numbers_match_strftime() {
    // chrono's own %W and %U are the reference, every day of a decade.
    let mut day = date(2020, 1, 1);
    while day.year() < 2031 {
        for (first, spec) in [
            (Weekday::Mon, "%W"),
            (Weekday::Sun, "%U"),
            (Weekday::Sat, "%U"),
        ] {
            let expected: u32 = day.format(spec).to_string().parse().expect("a number");
            assert_eq!(week_number(day, first), expected, "{day} from {first}");
        }
        day = day.succ_opt().expect("a next day");
    }
}

#[test]
fn the_header_starts_on_the_first_weekday() {
    use Weekday::{Fri, Mon, Sat, Sun, Thu, Tue, Wed};
    assert_eq!(weekdays(Sun), [Sun, Mon, Tue, Wed, Thu, Fri, Sat]);
    assert_eq!(weekdays(Mon), [Mon, Tue, Wed, Thu, Fri, Sat, Sun]);
}

#[test]
fn scrolling_moves_by_the_mode_s_unit() {
    let shown = date(2026, 10, 19);
    assert_eq!(shift(shown, Mode::Year, -1), Some(date(2025, 10, 1)));
    assert_eq!(shift(shown, Mode::Year, 2), Some(date(2028, 10, 1)));
    assert_eq!(shift(shown, Mode::Month, 3), Some(date(2027, 1, 1)));
    assert_eq!(shift(shown, Mode::Month, -10), Some(date(2025, 12, 1)));
    assert_eq!(shift(shown, Mode::Month, 0), Some(date(2026, 10, 1)));
}

#[test]
fn a_month_that_does_not_exist_has_no_rows() {
    assert_eq!(month(2026, 13, Weekday::Mon), None);
    assert_eq!(month(2026, 0, Weekday::Mon), None);
}
