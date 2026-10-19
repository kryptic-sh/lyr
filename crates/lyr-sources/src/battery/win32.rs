//! `GetSystemPowerStatus`'s `SYSTEM_POWER_STATUS` as the view's reading,
//! on every platform's tests; [`super::windows`] reads it. The field values
//! are the ones Microsoft's `SYSTEM_POWER_STATUS` reference documents.

use lyr_core::battery::{Reading, Status, TimeTo};

/// `ACLineStatus`: on mains power.
const AC_ONLINE: u8 = 1;
/// `ACLineStatus`: off mains power.
const AC_OFFLINE: u8 = 0;
/// `BatteryFlag` bits: charging, and no system battery. The byte is 255 when
/// the status cannot be read, which has the no-battery bit set too.
const FLAG_CHARGING: u8 = 8;
const FLAG_NO_BATTERY: u8 = 128;
/// `BatteryLifePercent` when it is not known.
const PERCENT_UNKNOWN: u8 = 255;
/// `BatteryLifeTime` when it is not known, as on mains power.
const TIME_UNKNOWN: u32 = u32::MAX;

/// The `SYSTEM_POWER_STATUS` fields lyr reads.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PowerStatus {
    pub ac_line_status: u8,
    pub battery_flag: u8,
    pub battery_life_percent: u8,
    /// Seconds of charge left.
    pub battery_life_time: u32,
}

impl PowerStatus {
    /// The battery reading, or `None` when there is no battery to show, so the
    /// module hides itself. Windows reports no draw, so `power` is `None`, and
    /// no time to full.
    #[must_use]
    pub fn reading(&self) -> Option<Reading> {
        if self.battery_flag & FLAG_NO_BATTERY != 0 || self.battery_life_percent == PERCENT_UNKNOWN
        {
            return None;
        }
        let capacity = self.battery_life_percent.min(100);
        let status = if self.battery_flag & FLAG_CHARGING != 0 {
            // waybar reports a charging battery at 100 % as full.
            if capacity == 100 {
                Status::Full
            } else {
                Status::Charging
            }
        } else {
            match self.ac_line_status {
                AC_ONLINE if capacity == 100 => Status::Full,
                // On mains power and not charging: held at a charge limit.
                AC_ONLINE => Status::Plugged,
                AC_OFFLINE => Status::Discharging,
                _ => Status::Unknown,
            }
        };
        let time_to = match status {
            Status::Discharging if self.battery_life_time != TIME_UNKNOWN => {
                TimeTo::Empty(self.battery_life_time as f32 / 3600.0)
            }
            _ => TimeTo::Unknown,
        };
        Some(Reading {
            capacity,
            status,
            power: None,
            time_to,
        })
    }
}

#[cfg(test)]
mod tests {
    use lyr_core::battery::{Status, TimeTo};

    use super::PowerStatus;

    fn status(ac: u8, flag: u8, percent: u8, seconds: u32) -> PowerStatus {
        PowerStatus {
            ac_line_status: ac,
            battery_flag: flag,
            battery_life_percent: percent,
            battery_life_time: seconds,
        }
    }

    #[test]
    fn no_battery_or_an_unknown_one_hides_the_module() {
        assert_eq!(status(1, 128, 100, u32::MAX).reading(), None, "a desktop");
        assert_eq!(status(1, 255, 50, u32::MAX).reading(), None, "unknown");
        assert_eq!(status(1, 1, 255, u32::MAX).reading(), None);
    }

    #[test]
    fn the_status_follows_the_charging_bit_then_the_mains() {
        let of = |ac, flag, percent| {
            status(ac, flag, percent, u32::MAX)
                .reading()
                .map(|r| r.status)
        };
        assert_eq!(of(1, 8 | 1, 80), Some(Status::Charging));
        assert_eq!(
            of(1, 8, 100),
            Some(Status::Full),
            "charging at 100 % is full"
        );
        assert_eq!(of(1, 1, 100), Some(Status::Full));
        assert_eq!(of(1, 1, 80), Some(Status::Plugged));
        assert_eq!(of(0, 2, 20), Some(Status::Discharging));
        assert_eq!(of(255, 1, 50), Some(Status::Unknown));
    }

    #[test]
    fn a_discharging_battery_has_its_time_left() {
        let reading = status(0, 1, 60, 5400).reading().expect("a battery");
        assert_eq!(reading.capacity, 60);
        assert_eq!(reading.time_to, TimeTo::Empty(1.5));
        assert_eq!(reading.power, None, "Windows reports no draw");
        assert_eq!(
            status(0, 1, 60, u32::MAX).reading().map(|r| r.time_to),
            Some(TimeTo::Unknown)
        );
        assert_eq!(
            status(1, 8, 60, 5400).reading().map(|r| r.time_to),
            Some(TimeTo::Unknown),
            "no time to full"
        );
        assert_eq!(
            status(0, 1, 104, 60).reading().map(|r| r.capacity),
            Some(100)
        );
    }
}
