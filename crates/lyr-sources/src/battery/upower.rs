//! Reading UPower's display device (`org.freedesktop.UPower.Device`, the
//! composite battery at `/org/freedesktop/UPower/devices/DisplayDevice`).
//!
//! The property values are UPower's documented ones
//! (`upower.freedesktop.org/docs/Device.html`, read 2026-10-18). Turning them
//! into a [`Reading`] is pure, so it is tested on every platform; the D-Bus
//! connection that fills [`Device`] is Linux-only.

use lyr_core::battery::{Reading, Status, TimeTo};

/// `Type` for a battery.
const TYPE_BATTERY: u32 = 2;

/// `State` values.
const STATE_CHARGING: u32 = 1;
const STATE_DISCHARGING: u32 = 2;
const STATE_EMPTY: u32 = 3;
const STATE_FULLY_CHARGED: u32 = 4;
const STATE_PENDING_CHARGE: u32 = 5;
const STATE_PENDING_DISCHARGE: u32 = 6;

const SECONDS_PER_HOUR: f32 = 3600.0;

/// The display device's properties the battery module uses.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Device {
    /// `Type`.
    pub kind: u32,
    /// `IsPresent`.
    pub present: bool,
    /// `State`.
    pub state: u32,
    /// `Percentage`, 0 – 100.
    pub percentage: f64,
    /// `EnergyRate` in watts: positive while discharging, negative while
    /// charging.
    pub energy_rate: f64,
    /// `TimeToEmpty` in seconds, 0 when unknown.
    pub time_to_empty: i64,
    /// `TimeToFull` in seconds, 0 when unknown.
    pub time_to_full: i64,
}

impl Device {
    /// The battery reading, or `None` when there is no battery to show, so the
    /// module hides itself.
    #[must_use]
    pub fn reading(&self) -> Option<Reading> {
        if self.kind != TYPE_BATTERY || !self.present {
            return None;
        }
        // `as` saturates a float into the integer range; the clamp keeps a
        // driver's 100.4 % at 100.
        let capacity = self.percentage.round().clamp(0.0, 100.0) as u8;
        let status = match self.state {
            // waybar reports a charging battery at 100 % as full.
            STATE_CHARGING if capacity == 100 => Status::Full,
            STATE_CHARGING => Status::Charging,
            STATE_DISCHARGING | STATE_EMPTY => Status::Discharging,
            STATE_FULLY_CHARGED => Status::Full,
            // On mains power and holding: a charge limit, or waiting to start.
            STATE_PENDING_CHARGE => Status::Plugged,
            STATE_PENDING_DISCHARGE => Status::NotCharging,
            _ => Status::Unknown,
        };
        let hours = |seconds: i64| seconds as f32 / SECONDS_PER_HOUR;
        let time_to = match status {
            Status::Discharging if self.time_to_empty > 0 => {
                TimeTo::Empty(hours(self.time_to_empty))
            }
            Status::Charging if self.time_to_full > 0 => TimeTo::Full(hours(self.time_to_full)),
            _ => TimeTo::Unknown,
        };
        Some(Reading {
            capacity,
            status,
            power: Some(self.energy_rate.abs() as f32),
            time_to,
        })
    }
}

#[cfg(test)]
mod tests {
    use lyr_core::battery::{Status, TimeTo};

    use super::Device;

    fn battery(state: u32, percentage: f64) -> Device {
        Device {
            kind: 2,
            present: true,
            state,
            percentage,
            energy_rate: 7.5,
            time_to_empty: 9000,
            time_to_full: 1800,
        }
    }

    #[test]
    fn no_battery_hides_the_module() {
        let line_power = Device {
            kind: 1,
            ..battery(2, 50.0)
        };
        assert_eq!(line_power.reading(), None);
        let removed = Device {
            present: false,
            ..battery(2, 50.0)
        };
        assert_eq!(removed.reading(), None);
    }

    #[test]
    fn upower_states_map_onto_the_views_statuses() {
        let status = |state, percentage| battery(state, percentage).reading().map(|r| r.status);
        assert_eq!(status(1, 50.0), Some(Status::Charging));
        assert_eq!(
            status(1, 100.0),
            Some(Status::Full),
            "charging at 100 % is full"
        );
        assert_eq!(status(2, 50.0), Some(Status::Discharging));
        assert_eq!(status(3, 0.0), Some(Status::Discharging));
        assert_eq!(status(4, 100.0), Some(Status::Full));
        assert_eq!(status(5, 80.0), Some(Status::Plugged));
        assert_eq!(status(6, 80.0), Some(Status::NotCharging));
        assert_eq!(status(0, 80.0), Some(Status::Unknown));
    }

    #[test]
    fn the_estimate_is_in_hours_and_only_for_the_direction_of_travel() {
        let time = |state| battery(state, 50.0).reading().map(|r| r.time_to);
        assert_eq!(time(2), Some(TimeTo::Empty(2.5)));
        assert_eq!(time(1), Some(TimeTo::Full(0.5)));
        assert_eq!(time(5), Some(TimeTo::Unknown));
        let unknown = Device {
            time_to_empty: 0,
            ..battery(2, 50.0)
        };
        assert_eq!(unknown.reading().map(|r| r.time_to), Some(TimeTo::Unknown));
    }

    #[test]
    fn capacity_is_rounded_and_held_to_0_to_100_and_power_is_a_magnitude() {
        let reading = |percentage, rate| {
            Device {
                energy_rate: rate,
                ..battery(1, percentage)
            }
            .reading()
            .expect("a battery")
        };
        assert_eq!(reading(49.6, -12.0).capacity, 50);
        assert_eq!(reading(100.4, -12.0).capacity, 100);
        assert_eq!(reading(-3.0, -12.0).capacity, 0);
        assert_eq!(
            reading(50.0, -12.0).power,
            Some(12.0),
            "charging is negative"
        );
    }
}
