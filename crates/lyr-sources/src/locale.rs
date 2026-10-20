//! The user's locale, as far as the clock panel's calendar needs it
//! (`docs/plan/10-popups-and-menus.md`): the first day of the week.

/// The locale's first day of the week, as days after Monday (0 for Monday,
/// 6 for Sunday); `None` when it cannot be read. Read from Windows'
/// `LOCALE_IFIRSTDAYOFWEEK` and macOS' current `NSCalendar`; Linux and the
/// BSDs answer `None` until lyr reads glibc's `nl_langinfo`, a dependency
/// question in `docs/backlog.md`.
#[must_use]
pub fn first_weekday() -> Option<u8> {
    #[cfg(windows)]
    return windows();
    #[cfg(target_os = "macos")]
    return from_apple(objc2_foundation::NSCalendar::currentCalendar().firstWeekday());
    #[cfg(not(any(windows, target_os = "macos")))]
    None
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
    use super::{first_weekday, from_apple, from_windows};

    #[test]
    fn windows_counts_from_monday() {
        assert_eq!(from_windows("0"), Some(0), "Monday");
        assert_eq!(from_windows("6"), Some(6), "Sunday");
        assert_eq!(from_windows("7"), None);
        assert_eq!(from_windows(""), None);
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
        if cfg!(any(windows, target_os = "macos")) {
            assert!(day.is_some_and(|day| day <= 6), "{day:?}");
        } else {
            assert_eq!(day, None);
        }
    }
}
