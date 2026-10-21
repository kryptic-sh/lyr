use lyr_config::{Battery, Platform, from_sources};

use super::{Level, Reading, Status, TimeTo, view};

/// The battery table the bar ships with: the dotfiles' waybar settings.
fn defaults() -> Battery {
    from_sources(None, Platform::Linux)
        .expect("defaults")
        .battery
}

const fn reading(capacity: u8, status: Status, time_to: TimeTo) -> Reading {
    Reading {
        capacity,
        status,
        power: Some(7.5),
        time_to,
    }
}

#[test]
fn discharging_shows_the_ramp_icon_for_the_capacity() {
    let config = defaults();
    let at = |capacity| {
        view(
            &reading(capacity, Status::Discharging, TimeTo::Unknown),
            &config,
        )
        .text
    };
    assert_eq!(at(5), "\u{F244}");
    assert_eq!(at(45), "\u{F242}");
    assert_eq!(at(95), "\u{F240}");
}

#[test]
fn charging_adds_the_bolt_and_plugged_shows_the_plug_alone() {
    let config = defaults();
    let charging = view(&reading(45, Status::Charging, TimeTo::Unknown), &config);
    assert_eq!(charging.text, "\u{F242} \u{F140B}");
    let plugged = view(&reading(80, Status::Plugged, TimeTo::Unknown), &config);
    assert_eq!(plugged.text, "\u{EE9E}");
}

#[test]
fn plugged_in_and_idle_shows_the_plug_full_or_not() {
    let config = defaults();
    let full = view(&reading(100, Status::Full, TimeTo::Unknown), &config);
    assert_eq!(full.text, "\u{EE9E}");
    let held = view(&reading(80, Status::NotCharging, TimeTo::Unknown), &config);
    assert_eq!(held.text, "\u{EE9E}");
    // On battery it is the ramp's icon, never the plug.
    let discharging = view(&reading(100, Status::Discharging, TimeTo::Unknown), &config);
    assert_eq!(discharging.text, "\u{F240}");
}

#[test]
fn a_format_for_full_wins_over_the_plug() {
    let mut config = defaults();
    config.format_full = Some(lyr_config::Format::parse("{icon} full").expect("valid"));
    let full = view(&reading(100, Status::Full, TimeTo::Unknown), &config);
    assert_eq!(full.text, "\u{F240} full");
}

#[test]
fn the_level_is_the_lowest_threshold_reached() {
    let config = defaults();
    let level = |capacity| {
        view(
            &reading(capacity, Status::Discharging, TimeTo::Unknown),
            &config,
        )
        .level
    };
    assert_eq!(level(31), None);
    assert_eq!(level(30), Some(Level::Warning));
    assert_eq!(level(11), Some(Level::Warning));
    assert_eq!(level(10), Some(Level::Critical));
    assert_eq!(level(0), Some(Level::Critical));
}

#[test]
fn the_tooltip_gives_the_time_to_empty_or_full() {
    let config = defaults();
    let empty = view(
        &reading(50, Status::Discharging, TimeTo::Empty(2.5)),
        &config,
    );
    assert_eq!(empty.tooltip, "50% 7.5W Empty in 2 h 30 min");
    let full = view(&reading(50, Status::Charging, TimeTo::Full(0.25)), &config);
    assert_eq!(full.tooltip, "50% 7.5W Full in 0 h 15 min");
    // 1.0125 h is 1 h 0.75 min: waybar truncates the minutes, never rounds.
    let truncated = view(
        &reading(50, Status::Discharging, TimeTo::Empty(1.0125)),
        &config,
    );
    assert_eq!(truncated.tooltip, "50% 7.5W Empty in 1 h 0 min");
}

#[test]
fn with_no_estimate_the_tooltip_names_the_status() {
    let tooltip = view(
        &reading(80, Status::NotCharging, TimeTo::Unknown),
        &defaults(),
    )
    .tooltip;
    assert_eq!(tooltip, "80% 7.5W Not charging");
}

#[test]
fn no_power_reading_is_zero_watts_as_in_waybar() {
    let tooltip = view(
        &Reading {
            power: None,
            ..reading(80, Status::Discharging, TimeTo::Unknown)
        },
        &defaults(),
    )
    .tooltip;
    assert_eq!(tooltip, "80% 0W Discharging");
}

#[test]
fn an_estimate_under_a_minute_has_an_empty_time_as_in_waybar() {
    let tooltip = view(
        &reading(80, Status::Discharging, TimeTo::Empty(0.001)),
        &defaults(),
    )
    .tooltip;
    assert_eq!(tooltip, "80% 7.5W Empty in ");
}
