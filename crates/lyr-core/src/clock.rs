//! The clock module (`docs/plan/07-modules.md#clock`).
//!
//! The text is a strftime format (jiff's, with POSIX's `%c`, `%x`, `%X` and
//! `%r`), as waybar's `{:%Y/%m/%d %H:%M}` is. The finest unit the format shows
//! decides how often the clock ticks, so a minute format wakes lyr once a
//! minute rather than every second.

use std::time::Duration;

use jiff::fmt::strtime::{BrokenDownTime, Config, PosixCustom};
use jiff::{Timestamp, ZonedDateTime, tz::TimeZone};

/// How often the clock's text can change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tick {
    Minute,
    Second,
}

impl Tick {
    /// The finest unit `format` shows: a second when its text differs between
    /// two moments a second and a half apart in the same minute (`%S`, `%T`,
    /// `%c`, `%s`, `%f`, …), a minute otherwise.
    #[must_use]
    pub fn of(format: &str) -> Self {
        let at = |millis| {
            Timestamp::from_millisecond(millis)
                .map(|moment| format_at(&moment.to_zoned_datetime(TimeZone::UTC), format))
        };
        // 2026-01-01T00:00:00Z and 1.5 s later.
        let start = 1_767_225_600_000;
        match (at(start), at(start + 1_500)) {
            (Ok(Ok(first)), Ok(Ok(second))) if first == second => Self::Minute,
            _ => Self::Second,
        }
    }

    /// How long from `now` until the text next changes: the start of the
    /// next minute or second, so the clock flips on the boundary.
    #[must_use]
    pub fn until_next(self, now: &ZonedDateTime) -> Duration {
        let into_second = Duration::from_nanos(u64::from(now.subsec_nanosecond().unsigned_abs()));
        let into_tick = match self {
            Self::Second => into_second,
            Self::Minute => {
                Duration::from_secs(u64::from(now.second().unsigned_abs())) + into_second
            }
        };
        let period = match self {
            Self::Second => Duration::from_secs(1),
            Self::Minute => Duration::from_secs(60),
        };
        period.saturating_sub(into_tick)
    }
}

/// `now` under the strftime `format`, with POSIX's `%c`, `%x`, `%X` and `%r`
/// (`Sun Jul  8 00:34:59 2001` rather than jiff's default locale), as chrono
/// and the C library print them. Fails on a specifier jiff does not know.
///
/// # Errors
///
/// When `format` has a conversion specifier jiff does not support.
pub fn format_at(now: &ZonedDateTime, format: &str) -> Result<String, jiff::Error> {
    let config = Config::new().custom(PosixCustom::new());
    BrokenDownTime::from(now).to_string_with_config(&config, format)
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
/// The config has already checked that `format` formats ([`format_at`]); were
/// it to fail, the text is the error, so it shows rather than going blank.
#[must_use]
pub fn view(now: &ZonedDateTime, zone: Option<&str>, format: &str) -> View {
    let shown = |format| format_at(now, format).unwrap_or_else(|error| error.to_string());
    let offset = format!("UTC{}", shown("%:z"));
    View {
        text: shown(format),
        tooltip: zone.map_or_else(|| offset.clone(), |zone| format!("{zone}, {offset}")),
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use jiff::ZonedDateTime;
    use jiff::fmt::strtime;

    use super::{Tick, format_at, view};

    /// The time `text` names (RFC 3339), in a fixed zone of its offset.
    fn at(text: &str) -> ZonedDateTime {
        strtime::parse("%Y-%m-%dT%H:%M:%S%.f%:z", text)
            .and_then(|tm| tm.to_zoned_datetime())
            .expect("a valid time")
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
        for format in ["%H:%M:%S", "%T", "%X", "%c", "%r", "%s", "%.3f", "%f"] {
            assert_eq!(Tick::of(format), Tick::Second, "{format}");
        }
    }

    #[test]
    fn the_locale_specifiers_read_as_posix() {
        let now = at("2026-10-05T17:05:09+08:00");
        let shown = |format| format_at(&now, format).expect("a known format");
        assert_eq!(shown("%c"), "Mon Oct  5 17:05:09 2026");
        assert_eq!(shown("%x"), "10/05/26");
        assert_eq!(shown("%X"), "17:05:09");
        assert_eq!(shown("%r"), "05:05:09 PM");
    }

    #[test]
    fn an_unknown_specifier_is_an_error() {
        let now = at("2026-10-05T17:05:09+08:00");
        assert!(format_at(&now, "%+").is_err());
        assert!(format_at(&now, "%Y %K").is_err());
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
