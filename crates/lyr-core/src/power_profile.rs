//! The power-profile module (`docs/plan/07-modules.md#power-profile`),
//! following waybar 0.15.0's `power_profiles_daemon.cpp`.

use lyr_config::{Format, PowerProfile as Config};

use crate::icon::pick;

/// The active profile, as power-profiles-daemon names it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Profile {
    /// `performance`, `balanced` or `power-saver`.
    pub name: String,
    /// The driver behind it, such as `amd_pstate`.
    pub driver: String,
}

/// What the module draws.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct View {
    pub text: String,
    pub tooltip: String,
}

/// The view of `profile` under `config`.
#[must_use]
pub fn view(profile: &Profile, config: &Config) -> View {
    let icon = pick(&config.icons, &[profile.name.as_str()], 0);
    let fill = |format: &Format| {
        format.render(|name| match name {
            "icon" => Some(icon),
            "profile" => Some(profile.name.as_str()),
            "driver" => Some(profile.driver.as_str()),
            _ => None,
        })
    };
    View {
        text: fill(&config.format),
        tooltip: fill(&config.tooltip_format),
    }
}

/// Which way a click moves through the profiles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    /// A left click.
    Next,
    /// Any other button.
    Previous,
}

/// The profile a click switches to: the one after (or before) `active` among
/// `available`, wrapping at both ends, as waybar does. `None` when `active` is
/// not one of them.
#[must_use]
pub fn cycle<'a>(available: &'a [String], active: &str, direction: Direction) -> Option<&'a str> {
    let at = available.iter().position(|name| name == active)?;
    let len = available.len();
    let to = match direction {
        Direction::Next => (at + 1) % len,
        Direction::Previous => (at + len - 1) % len,
    };
    Some(available[to].as_str())
}

#[cfg(test)]
mod tests {
    use lyr_config::{Platform, from_sources};

    use super::{Direction, Profile, cycle, view};

    fn profile(name: &str) -> Profile {
        Profile {
            name: name.to_owned(),
            driver: "amd_pstate".to_owned(),
        }
    }

    #[test]
    fn each_profile_has_its_icon_and_the_tooltip_names_it() {
        let config = from_sources(None, Platform::Linux)
            .expect("defaults")
            .power_profile;
        assert_eq!(view(&profile("performance"), &config).text, "\u{F04C5}");
        assert_eq!(view(&profile("balanced"), &config).text, "\u{F24E}");
        assert_eq!(view(&profile("power-saver"), &config).text, "\u{F06C}");
        assert_eq!(
            view(&profile("custom"), &config).text,
            "\u{F04C5}",
            "an unknown profile takes the default icon"
        );
        assert_eq!(
            view(&profile("balanced"), &config).tooltip,
            "Profile: balanced\nDriver: amd_pstate"
        );
    }

    #[test]
    fn clicks_cycle_through_the_profiles_and_wrap() {
        let available: Vec<String> = ["power-saver", "balanced", "performance"]
            .map(str::to_owned)
            .to_vec();
        assert_eq!(
            cycle(&available, "balanced", Direction::Next),
            Some("performance")
        );
        assert_eq!(
            cycle(&available, "performance", Direction::Next),
            Some("power-saver")
        );
        assert_eq!(
            cycle(&available, "power-saver", Direction::Previous),
            Some("performance")
        );
        assert_eq!(
            cycle(&available, "balanced", Direction::Previous),
            Some("power-saver")
        );
        assert_eq!(cycle(&available, "turbo", Direction::Next), None);
    }
}
