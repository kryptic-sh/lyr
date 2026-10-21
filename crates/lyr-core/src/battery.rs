//! The battery module's view: what the bar shows for a battery reading
//! (`docs/plan/07-modules.md#battery`).
//!
//! It follows waybar 0.15.0's battery module (`src/modules/battery.cpp`) for
//! the parts the dotfiles use, so the bar reads the same after the switch.

use lyr_config::{Battery as Config, Format};

use crate::icon::ramp;

/// What the battery is doing, as the source resolved it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Charging,
    Discharging,
    /// On mains power and not charging, below 100 % (held at a charge limit).
    Plugged,
    /// At 100 %, on mains power: every platform reports it only there.
    Full,
    /// On mains power and neither charging nor discharging.
    NotCharging,
    Unknown,
}

impl Status {
    /// The name waybar shows when there is no time estimate.
    const fn name(self) -> &'static str {
        match self {
            Self::Charging => "Charging",
            Self::Discharging => "Discharging",
            Self::Plugged => "Plugged",
            Self::Full => "Full",
            Self::NotCharging => "Not charging",
            Self::Unknown => "Unknown",
        }
    }
}

/// How long until the battery is empty or full, in hours.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TimeTo {
    Empty(f32),
    Full(f32),
    /// No estimate: the battery is idle, or the platform gives none.
    Unknown,
}

/// One battery reading.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Reading {
    /// 0 – 100.
    pub capacity: u8,
    pub status: Status,
    /// Watts, when the platform reports the draw; shown as 0 when it does
    /// not, as waybar shows it.
    pub power: Option<f32>,
    pub time_to: TimeTo,
}

/// How urgent the charge level is, which colours the module.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Warning,
    Critical,
}

/// What the module draws.
#[derive(Clone, Debug, PartialEq)]
pub struct View {
    pub text: String,
    pub tooltip: String,
    pub level: Option<Level>,
}

/// The view of `reading` under `config`.
#[must_use]
pub fn view(reading: &Reading, config: &Config) -> View {
    // waybar's `getState`: the lowest threshold the capacity is at or under.
    let level = if reading.capacity <= config.critical {
        Some(Level::Critical)
    } else if reading.capacity <= config.warning {
        Some(Level::Warning)
    } else {
        None
    };
    // Plugged in and idle shows as plugged, full or not, where waybar shows
    // a full or not-charging battery in `format`, as if on battery.
    let format = match reading.status {
        Status::Charging => config.format_charging.as_ref(),
        Status::Plugged | Status::NotCharging => config.format_plugged.as_ref(),
        Status::Full => config
            .format_full
            .as_ref()
            .or(config.format_plugged.as_ref()),
        Status::Discharging | Status::Unknown => None,
    }
    .unwrap_or(&config.format);

    let icon = ramp(&config.icons, u32::from(reading.capacity), 100).map_or("", String::as_str);
    let capacity = reading.capacity.to_string();
    // waybar's power is the batteries' total draw, 0 when none reports one.
    let power = reading.power.unwrap_or(0.0).to_string();
    let time = match reading.time_to {
        TimeTo::Empty(hours) | TimeTo::Full(hours) => hours_and_minutes(hours),
        TimeTo::Unknown => String::new(),
    };
    let time_to = match reading.time_to {
        TimeTo::Empty(_) => format!("Empty in {time}"),
        TimeTo::Full(_) => format!("Full in {time}"),
        TimeTo::Unknown => reading.status.name().to_owned(),
    };

    let fill = |format: &Format| {
        format.render(|name| match name {
            "icon" => Some(icon),
            "capacity" => Some(capacity.as_str()),
            "power" => Some(power.as_str()),
            "time" => Some(time.as_str()),
            "timeTo" => Some(time_to.as_str()),
            _ => None,
        })
    };
    View {
        text: fill(format),
        tooltip: fill(&config.tooltip_format),
        level,
    }
}

/// waybar's `formatTimeRemaining` with its default `{H} h {M} min`: whole
/// hours, then the whole minutes left over, and nothing at all when both are
/// zero.
fn hours_and_minutes(hours: f32) -> String {
    let hours = hours.abs();
    let whole = hours.trunc();
    // `as` truncates, which is what waybar's `static_cast<uint16_t>` does.
    let (full, minutes) = (whole as u32, (60.0 * (hours - whole)) as u32);
    if full == 0 && minutes == 0 {
        String::new()
    } else {
        format!("{full} h {minutes} min")
    }
}

#[cfg(test)]
mod tests;
