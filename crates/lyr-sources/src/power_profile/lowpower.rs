//! macOS's Low Power Mode as the module's profile, on every platform's tests;
//! [`super::macos`] reads it. macOS reports only whether the mode is on, not
//! which of its other energy modes is in use, so off is named as off.

use lyr_core::power_profile::Profile;

/// The profile Low Power Mode being `on` is: power-profiles-daemon's name for
/// its kind, which the module's icons are keyed by, and the mode's state in
/// place of the daemon's driver.
#[must_use]
pub fn profile(on: bool) -> Profile {
    let (name, driver) = if on {
        ("power-saver", "Low Power Mode")
    } else {
        ("balanced", "Low Power Mode off")
    };
    Profile {
        name: name.to_owned(),
        driver: driver.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::profile;

    #[test]
    fn low_power_is_the_power_saver_and_off_is_balanced() {
        let on = profile(true);
        assert_eq!(
            (on.name.as_str(), on.driver.as_str()),
            ("power-saver", "Low Power Mode")
        );
        let off = profile(false);
        assert_eq!(
            (off.name.as_str(), off.driver.as_str()),
            ("balanced", "Low Power Mode off")
        );
    }
}
