//! The user's locale, as far as the clock panel's calendar needs it
//! (`docs/plan/10-popups-and-menus.md`): the first day of the week.

/// The locale's first day of the week, as days after Monday (0 for Monday,
/// 6 for Sunday); `None` when it cannot be read. Read from Windows'
/// `LOCALE_IFIRSTDAYOFWEEK`, macOS' current `NSCalendar`, and elsewhere
/// from the region of the POSIX locale the environment names
/// ([`from_posix`]), through CLDR's week data, in-process and the same on
/// glibc, musl and the BSDs.
#[must_use]
pub fn first_weekday() -> Option<u8> {
    #[cfg(windows)]
    return windows();
    #[cfg(target_os = "macos")]
    return from_apple(objc2_foundation::NSCalendar::currentCalendar().firstWeekday());
    #[cfg(not(any(windows, target_os = "macos")))]
    return from_posix(&posix_time_locale()?);
}

/// The locale `LC_TIME` follows, by POSIX's order: `LC_ALL`, else
/// `LC_TIME`, else `LANG`, the first of them set and not empty.
#[cfg(not(any(windows, target_os = "macos")))]
fn posix_time_locale() -> Option<String> {
    ["LC_ALL", "LC_TIME", "LANG"]
        .into_iter()
        .find_map(|name| std::env::var(name).ok().filter(|value| !value.is_empty()))
}

/// The first day of the week, as days after Monday, for a POSIX locale name
/// such as `en_US.UTF-8` or `de_DE@euro`: [`from_region`] of its territory,
/// or `None` for a name with none, such as `C` or `POSIX`.
#[must_use]
pub fn from_posix(locale: &str) -> Option<u8> {
    let name = locale.split(['.', '@']).next()?;
    let (_, region) = name.split_once('_')?;
    (!region.is_empty()).then(|| from_region(region))
}

/// The first day of the week, as days after Monday, in the territory
/// `region` (`US`, `DE`, `419`), as CLDR 47's week data has it
/// (`common/supplemental/supplementalData.xml`, `weekData`'s `firstDay`):
/// Monday for every territory it does not list otherwise, as its `001`
/// says.
#[must_use]
pub fn from_region(region: &str) -> u8 {
    const FRIDAY: &[&str] = &["MV"];
    const SATURDAY: &[&str] = &[
        "AF", "BH", "DJ", "DZ", "EG", "IQ", "IR", "JO", "KW", "LY", "OM", "QA", "SD", "SY",
    ];
    const SUNDAY: &[&str] = &[
        "AG", "AS", "BD", "BR", "BS", "BT", "BW", "BZ", "CA", "CO", "DM", "DO", "ET", "GT", "GU",
        "HK", "HN", "ID", "IL", "IN", "JM", "JP", "KE", "KH", "KR", "LA", "MH", "MM", "MO", "MT",
        "MX", "MZ", "NI", "NP", "PA", "PE", "PH", "PK", "PR", "PT", "PY", "SA", "SG", "SV", "TH",
        "TT", "TW", "UM", "US", "VE", "VI", "WS", "YE", "ZA", "ZW",
    ];
    if FRIDAY.contains(&region) {
        4
    } else if SATURDAY.contains(&region) {
        5
    } else if SUNDAY.contains(&region) {
        6
    } else {
        0
    }
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
    use super::{first_weekday, from_apple, from_posix, from_region, from_windows};

    #[test]
    fn windows_counts_from_monday() {
        assert_eq!(from_windows("0"), Some(0), "Monday");
        assert_eq!(from_windows("6"), Some(6), "Sunday");
        assert_eq!(from_windows("7"), None);
        assert_eq!(from_windows(""), None);
    }

    #[test]
    fn a_posix_locale_counts_by_its_region() {
        assert_eq!(from_posix("en_US.UTF-8"), Some(6), "Sunday");
        assert_eq!(from_posix("de_DE@euro"), Some(0), "Monday");
        assert_eq!(
            from_posix("sd_IN@devanagari"),
            Some(6),
            "the modifier is not the region"
        );
        assert_eq!(from_posix("ar_EG.UTF-8"), Some(5), "Saturday");
        assert_eq!(from_posix("dv_MV"), Some(4), "Friday");
        assert_eq!(from_posix("en_GB"), Some(0), "not the GB variant");
        assert_eq!(from_posix("es_419.UTF-8"), Some(0), "the world's Monday");
        assert_eq!(from_posix("C.UTF-8"), None, "no region");
        assert_eq!(from_posix("POSIX"), None);
        assert_eq!(from_posix("en_"), None);
    }

    #[test]
    fn regions_follow_cldr() {
        assert_eq!(from_region("MV"), 4);
        for region in ["AF", "SY", "IR"] {
            assert_eq!(from_region(region), 5, "{region}");
        }
        for region in ["AG", "US", "ZW", "PT", "SA", "YE"] {
            assert_eq!(from_region(region), 6, "{region}");
        }
        for region in ["AE", "FR", "XK", "us", ""] {
            assert_eq!(from_region(region), 0, "{region}");
        }
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
        // A POSIX locale with no region, such as a CI runner's `C.UTF-8`,
        // has no first weekday to read.
        #[cfg(not(any(windows, target_os = "macos")))]
        if super::posix_time_locale().is_none_or(|locale| !locale.contains('_')) {
            assert_eq!(day, None);
            return;
        }
        assert!(day.is_some_and(|day| day <= 6), "{day:?}");
    }
}
