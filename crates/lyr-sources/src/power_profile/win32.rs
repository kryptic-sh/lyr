//! Windows' effective power mode as the module's profile, on every
//! platform's tests; [`super::windows`] reads it. The values are
//! `EFFECTIVE_POWER_MODE`'s from Microsoft's reference, spelled out here, as
//! plan 11 asks of Windows ABI values, so this compiles everywhere.

use lyr_core::power_profile::Profile;

/// `EffectivePowerModeBatterySaver` … `EffectivePowerModeMixedReality`.
const BATTERY_SAVER: i32 = 0;
const BETTER_BATTERY: i32 = 1;
const BALANCED: i32 = 2;
const HIGH_PERFORMANCE: i32 = 3;
const MAX_PERFORMANCE: i32 = 4;
const GAME_MODE: i32 = 5;
const MIXED_REALITY: i32 = 6;

/// The profile Windows' effective power mode `mode` is: power-profiles-daemon's
/// name for its kind, which the module's icons are keyed by, and Windows'
/// own name for the mode in place of the daemon's driver. `None` for a mode
/// this table does not know.
#[must_use]
pub fn profile(mode: i32) -> Option<Profile> {
    let (name, driver) = match mode {
        BATTERY_SAVER => ("power-saver", "Battery Saver"),
        BETTER_BATTERY => ("power-saver", "Better Battery"),
        BALANCED => ("balanced", "Balanced"),
        HIGH_PERFORMANCE => ("performance", "High Performance"),
        MAX_PERFORMANCE => ("performance", "Max Performance"),
        GAME_MODE => ("performance", "Game Mode"),
        MIXED_REALITY => ("balanced", "Mixed Reality"),
        _ => return None,
    };
    Some(Profile {
        name: name.to_owned(),
        driver: driver.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::profile;

    #[test]
    fn each_mode_is_its_kind_with_its_own_name() {
        let of = |mode| profile(mode).map(|p| (p.name, p.driver));
        let pair = |name: &str, driver: &str| Some((name.to_owned(), driver.to_owned()));
        assert_eq!(of(0), pair("power-saver", "Battery Saver"));
        assert_eq!(of(1), pair("power-saver", "Better Battery"));
        assert_eq!(of(2), pair("balanced", "Balanced"));
        assert_eq!(of(3), pair("performance", "High Performance"));
        assert_eq!(of(4), pair("performance", "Max Performance"));
        assert_eq!(of(5), pair("performance", "Game Mode"));
        assert_eq!(of(6), pair("balanced", "Mixed Reality"));
        assert_eq!(of(7), None);
        assert_eq!(of(-1), None);
    }
}
