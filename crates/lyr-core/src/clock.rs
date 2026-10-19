//! The clock module (`docs/plan/07-modules.md#clock`).
//!
//! The text is a strftime format (chrono's), as waybar's `{:%Y/%m/%d %H:%M}`
//! is. The finest unit the format shows decides how often the clock ticks, so
//! a minute format wakes lyr once a minute rather than every second.

use std::time::Duration;

use chrono::format::{Fixed, Item, Numeric, StrftimeItems};
use chrono::{DateTime, FixedOffset, Timelike};

/// How often the clock's text can change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tick {
    Minute,
    Second,
}

impl Tick {
    /// The finest unit `format` shows: a second when any of its items shows
    /// seconds or finer (`%S`, `%T`, `%c`, `%s`, `%+`, …), a minute otherwise.
    #[must_use]
    pub fn of(format: &str) -> Self {
        let shows_seconds = StrftimeItems::new(format).any(|item| match item {
            Item::Numeric(numeric, _) => {
                matches!(
                    numeric,
                    Numeric::Second | Numeric::Nanosecond | Numeric::Timestamp
                )
            }
            Item::Fixed(fixed) => matches!(
                fixed,
                Fixed::Nanosecond
                    | Fixed::Nanosecond3
                    | Fixed::Nanosecond6
                    | Fixed::Nanosecond9
                    | Fixed::RFC2822
                    | Fixed::RFC3339
            ),
            _ => false,
        });
        if shows_seconds {
            Self::Second
        } else {
            Self::Minute
        }
    }

    /// How long from `now` until the text next changes: the start of the
    /// next minute or second, so the clock flips on the boundary.
    #[must_use]
    pub fn until_next(self, now: &DateTime<FixedOffset>) -> Duration {
        let into_second = Duration::from_nanos(u64::from(now.nanosecond() % 1_000_000_000));
        let into_tick = match self {
            Self::Second => into_second,
            Self::Minute => Duration::from_secs(u64::from(now.second())) + into_second,
        };
        let period = match self {
            Self::Second => Duration::from_secs(1),
            Self::Minute => Duration::from_secs(60),
        };
        period.saturating_sub(into_tick)
    }
}

/// What the module draws.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct View {
    pub text: String,
    /// The time zone and its offset, such as `Asia/Manila, UTC+08:00`, or
    /// the offset alone when the zone's name is not known.
    pub tooltip: String,
}

/// The view of `now` in the zone named `zone`, under the strftime `format`.
/// The config has already checked that `format` parses.
#[must_use]
pub fn view(now: &DateTime<FixedOffset>, zone: Option<&str>, format: &str) -> View {
    let offset = format!("UTC{}", now.format("%:z"));
    View {
        text: now.format(format).to_string(),
        tooltip: zone.map_or_else(|| offset.clone(), |zone| format!("{zone}, {offset}")),
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use chrono::{DateTime, FixedOffset};

    use super::{Tick, view};

    fn at(text: &str) -> DateTime<FixedOffset> {
        DateTime::parse_from_rfc3339(text).expect("a valid time")
    }

    #[test]
    fn the_default_format_reads_as_waybars() {
        let shown = view(
            &at("2026-10-19T09:05:42.5+08:00"),
            Some("Asia/Manila"),
            "%Y/%m/%d %H:%M",
        );
        assert_eq!(shown.text, "2026/10/19 09:05");
        assert_eq!(shown.tooltip, "Asia/Manila, UTC+08:00");
    }

    #[test]
    fn a_negative_offset_is_shown_with_its_sign() {
        let shown = view(
            &at("2026-10-19T09:05:00-03:30"),
            Some("America/St_Johns"),
            "%H",
        );
        assert_eq!(shown.tooltip, "America/St_Johns, UTC-03:30");
    }

    #[test]
    fn an_unknown_zone_leaves_the_offset_alone() {
        let shown = view(&at("2026-10-19T09:05:00+08:00"), None, "%H");
        assert_eq!(shown.tooltip, "UTC+08:00");
    }

    #[test]
    fn the_tick_follows_the_finest_unit_shown() {
        assert_eq!(Tick::of("%Y/%m/%d %H:%M"), Tick::Minute);
        assert_eq!(Tick::of("%a %e %b"), Tick::Minute);
        for format in ["%H:%M:%S", "%T", "%X", "%c", "%r", "%s", "%+", "%.3f"] {
            assert_eq!(Tick::of(format), Tick::Second, "{format}");
        }
    }

    #[test]
    fn the_next_tick_lands_on_the_boundary() {
        let now = at("2026-10-19T09:05:42.25+08:00");
        assert_eq!(Tick::Minute.until_next(&now), Duration::from_millis(17_750));
        assert_eq!(Tick::Second.until_next(&now), Duration::from_millis(750));
        let on_the_minute = at("2026-10-19T09:06:00+08:00");
        assert_eq!(
            Tick::Minute.until_next(&on_the_minute),
            Duration::from_secs(60)
        );
    }
}
