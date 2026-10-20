//! The user's locale, as far as the clock panel's calendar needs it
//! (`docs/plan/10-popups-and-menus.md`): the first day of the week.

/// The locale's first day of the week, as days after Monday (0 for Monday,
/// 6 for Sunday); `None` when it cannot be read. Read from Windows'
/// `LOCALE_IFIRSTDAYOFWEEK`, macOS' current `NSCalendar`, and elsewhere
/// glibc's `LC_TIME` through its `locale` utility ([`from_glibc`]); a
/// system without that utility (musl, the BSDs) answers `None`.
#[must_use]
pub fn first_weekday() -> Option<u8> {
    #[cfg(windows)]
    return windows();
    #[cfg(target_os = "macos")]
    return from_apple(objc2_foundation::NSCalendar::currentCalendar().firstWeekday());
    #[cfg(not(any(windows, target_os = "macos")))]
    return glibc();
}

/// `locale -k LC_TIME`'s output, as days after Monday: the weekday of the
/// `week-1stday` date, moved on by `first_weekday` - 1 days. These are the
/// two values waybar 0.15.0 reads through `nl_langinfo`
/// (`_NL_TIME_WEEK_1STDAY` and `_NL_TIME_FIRST_WEEKDAY`, in
/// `Clock::first_day_of_week`), the way `cal` does.
#[must_use]
pub fn from_glibc(output: &str) -> Option<u8> {
    use chrono::{Datelike, NaiveDate};

    let value = |key: &str| {
        output
            .lines()
            .find_map(|line| line.strip_prefix(key)?.strip_prefix('='))
            .map(str::trim)
    };
    // Always eight digits from glibc; chrono's `%Y` would take fewer.
    let start = value("week-1stday")
        .filter(|date| date.len() == 8 && date.bytes().all(|byte| byte.is_ascii_digit()))?;
    let start = NaiveDate::parse_from_str(start, "%Y%m%d").ok()?;
    let first: u32 = value("first_weekday")?.parse().ok()?;
    if !(1..=7).contains(&first) {
        return None;
    }
    let day = (start.weekday().num_days_from_monday() + first - 1) % 7;
    u8::try_from(day).ok()
}

/// [`from_glibc`] for the user's `LC_TIME`, by running `locale`, which reads
/// it from the environment as glibc's `setlocale` does.
#[cfg(not(any(windows, target_os = "macos")))]
fn glibc() -> Option<u8> {
    let output = std::process::Command::new("locale")
        .args(["-k", "LC_TIME"])
        .stderr(std::process::Stdio::null())
        .output()
        .ok()
        .filter(|output| output.status.success())?;
    from_glibc(&String::from_utf8_lossy(&output.stdout))
}

/// `LOCALE_IFIRSTDAYOFWEEK`'s value, "0" for Monday through "6" for Sunday,
/// as days after Monday.
#[must_use]
pub fn from_windows(value: &str) -> Option<u8> {
    value.parse().ok().filter(|day| *day <= 6)
}

/// `NSCalendar`'s `firstWeekday`, 1 for Sunday through 7 for Saturday, as
/// days after Monday.
#[must_use]
pub fn from_apple(value: usize) -> Option<u8> {
    let day = u8::try_from(value)
        .ok()
        .filter(|day| (1..=7).contains(day))?;
    Some((day + 5) % 7)
}

#[cfg(windows)]
fn windows() -> Option<u8> {
    use windows_sys::Win32::Globalization::{GetLocaleInfoEx, LOCALE_IFIRSTDAYOFWEEK};

    // The value is one digit; room for it, its terminator and some spare.
    let mut buffer = [0_u16; 8];
    // SAFETY: a null name is `LOCALE_NAME_USER_DEFAULT`; the call writes at
    // most the buffer's length, which it is given.
    let written = unsafe {
        GetLocaleInfoEx(
            std::ptr::null(),
            LOCALE_IFIRSTDAYOFWEEK,
            buffer.as_mut_ptr(),
            buffer.len() as i32,
        )
    };
    // The count includes the terminator; 0 is a failure.
    let length = usize::try_from(written).ok()?.checked_sub(1)?;
    from_windows(&String::from_utf16(buffer.get(..length)?).ok()?)
}

#[cfg(test)]
mod tests {
    use super::{first_weekday, from_apple, from_glibc, from_windows};

    #[test]
    fn windows_counts_from_monday() {
        assert_eq!(from_windows("0"), Some(0), "Monday");
        assert_eq!(from_windows("6"), Some(6), "Sunday");
        assert_eq!(from_windows("7"), None);
        assert_eq!(from_windows(""), None);
    }

    #[test]
    fn glibc_moves_on_from_its_week_start() {
        let output = |start: &str, first: &str| {
            format!("abday=\"Sun;Mon\"\nweek-ndays=7\nweek-1stday={start}\nfirst_weekday={first}\n")
        };
        // 1997-11-30 is a Sunday, glibc's usual `week-1stday`.
        assert_eq!(
            from_glibc(&output("19971130", "1")),
            Some(6),
            "en_US: Sunday"
        );
        assert_eq!(
            from_glibc(&output("19971130", "2")),
            Some(0),
            "de_DE: Monday"
        );
        assert_eq!(from_glibc(&output("19971130", "7")), Some(5), "Saturday");
        // 1997-12-01 is a Monday.
        assert_eq!(from_glibc(&output("19971201", "1")), Some(0));
        assert_eq!(from_glibc(&output("19971130", "8")), None);
        assert_eq!(from_glibc(&output("1997113", "1")), None, "not a date");
        assert_eq!(from_glibc("first_weekday=1\n"), None, "no week start");
    }

    #[test]
    fn apple_counts_from_sunday() {
        assert_eq!(from_apple(1), Some(6), "Sunday");
        assert_eq!(from_apple(2), Some(0), "Monday");
        assert_eq!(from_apple(7), Some(5), "Saturday");
        assert_eq!(from_apple(0), None);
        assert_eq!(from_apple(8), None);
    }

    #[test]
    fn this_machine_s_locale_answers_where_it_can_be_read() {
        let day = first_weekday();
        assert!(day.is_some_and(|day| day <= 6), "{day:?}");
    }
}
