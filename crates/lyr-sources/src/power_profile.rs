//! The power-profile module's source (`docs/plan/07-modules.md#power-profile`).
//!
//! [`Profiles`] turns power-profiles-daemon's properties into what the view
//! and the click need, on every platform's tests; [`linux`] reads them over
//! D-Bus. Windows and macOS come with milestones M9 and M10.

use lyr_core::power_profile::Profile;

#[cfg(target_os = "linux")]
pub mod linux;

/// One entry of the daemon's `Profiles` property: its `Profile` and `Driver`
/// keys.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub profile: String,
    pub driver: String,
}

/// The daemon's state: the profile in use and the ones a click cycles
/// through, in the daemon's order (power-saver, balanced, performance).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Profiles {
    pub active: Profile,
    pub available: Vec<String>,
}

impl Profiles {
    /// From `ActiveProfile` and `Profiles`. The active profile's driver is
    /// its entry's; `None` when the daemon lists no such profile, which it
    /// never should.
    #[must_use]
    pub fn new(active: &str, entries: &[Entry]) -> Option<Self> {
        let entry = entries.iter().find(|entry| entry.profile == active)?;
        Some(Self {
            active: Profile {
                name: entry.profile.clone(),
                driver: entry.driver.clone(),
            },
            available: entries.iter().map(|entry| entry.profile.clone()).collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{Entry, Profiles};

    fn entries() -> Vec<Entry> {
        [
            ("power-saver", "amd_pstate"),
            ("balanced", "amd_pstate"),
            ("performance", "platform_profile"),
        ]
        .map(|(profile, driver)| Entry {
            profile: profile.to_owned(),
            driver: driver.to_owned(),
        })
        .to_vec()
    }

    #[test]
    fn the_active_profile_takes_its_entrys_driver() {
        let profiles = Profiles::new("performance", &entries()).expect("listed");
        assert_eq!(profiles.active.name, "performance");
        assert_eq!(profiles.active.driver, "platform_profile");
        assert_eq!(
            profiles.available,
            ["power-saver", "balanced", "performance"]
        );
    }

    #[test]
    fn an_unlisted_active_profile_has_no_state() {
        assert_eq!(Profiles::new("turbo", &entries()), None);
    }
}
