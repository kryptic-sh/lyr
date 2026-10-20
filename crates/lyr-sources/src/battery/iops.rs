//! A macOS power source's description, from IOKit's `IOPSGetPowerSourceDescription`,
//! as the view's reading, on every platform's tests; [`super::macos`] reads
//! it. The keys and values are `IOPSKeys.h`'s, as `objc2-io-kit` 0.3.2 binds
//! them: capacities in the units `Max Capacity` gives (a percentage on
//! current Macs), times in minutes with `-1` while macOS is still estimating.

use lyr_core::battery::{Reading, Status, TimeTo};

/// The description's fields lyr reads, for one internal battery.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PowerSource {
    /// `Is Present`: a battery is in the bay.
    pub present: bool,
    /// `Current Capacity`.
    pub current_capacity: i64,
    /// `Max Capacity`.
    pub max_capacity: i64,
    /// `Is Charging`.
    pub charging: bool,
    /// `Is Charged`, which is only in the description once charging is done.
    pub charged: bool,
    /// `Power Source State` is `AC Power`, rather than `Battery Power`.
    pub on_ac: bool,
    /// `Time to Empty`, in minutes.
    pub time_to_empty: i64,
    /// `Time to Full Charge`, in minutes.
    pub time_to_full: i64,
}

impl PowerSource {
    /// The battery reading, or `None` when the bay is empty or its capacity
    /// is not reported, so the module hides itself. macOS reports no draw
    /// here, so `power` is `None`.
    #[must_use]
    pub fn reading(&self) -> Option<Reading> {
        if !self.present || self.max_capacity <= 0 {
            return None;
        }
        let percent = (self.current_capacity.clamp(0, self.max_capacity) * 100) / self.max_capacity;
        let capacity = u8::try_from(percent).unwrap_or(100);
        let status = if self.charging {
            Status::Charging
        } else if self.charged || (self.on_ac && capacity == 100) {
            Status::Full
        } else if self.on_ac {
            // On mains power and not charging: held at a charge limit
            // (optimized charging).
            Status::Plugged
        } else {
            Status::Discharging
        };
        // A negative time is one macOS is still estimating.
        let hours = |minutes: i64| (minutes >= 0).then(|| minutes as f32 / 60.0);
        let time_to = match status {
            Status::Discharging => hours(self.time_to_empty).map_or(TimeTo::Unknown, TimeTo::Empty),
            Status::Charging => hours(self.time_to_full).map_or(TimeTo::Unknown, TimeTo::Full),
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

    use super::PowerSource;

    fn battery(current: i64, charging: bool, on_ac: bool) -> PowerSource {
        PowerSource {
            present: true,
            current_capacity: current,
            max_capacity: 100,
            charging,
            charged: false,
            on_ac,
            time_to_empty: -1,
            time_to_full: -1,
        }
    }

    #[test]
    fn an_empty_bay_or_no_capacity_hides_the_module() {
        let mut absent = battery(80, false, false);
        absent.present = false;
        assert_eq!(absent.reading(), None);
        let mut unreported = battery(80, false, false);
        unreported.max_capacity = 0;
        assert_eq!(unreported.reading(), None);
    }

    #[test]
    fn the_capacity_is_a_percentage_of_the_maximum() {
        let mut older = battery(4200, false, false);
        older.max_capacity = 5600;
        assert_eq!(older.reading().map(|r| r.capacity), Some(75));
        assert_eq!(
            battery(130, false, false).reading().map(|r| r.capacity),
            Some(100),
            "a capacity over the maximum is held to it"
        );
    }

    #[test]
    fn the_status_follows_charging_then_the_mains() {
        let of = |source: PowerSource| source.reading().map(|r| r.status);
        assert_eq!(of(battery(60, true, true)), Some(Status::Charging));
        assert_eq!(of(battery(100, false, true)), Some(Status::Full));
        let mut charged = battery(98, false, true);
        charged.charged = true;
        assert_eq!(of(charged), Some(Status::Full), "macOS says it is charged");
        assert_eq!(
            of(battery(80, false, true)),
            Some(Status::Plugged),
            "held at a charge limit"
        );
        assert_eq!(of(battery(40, false, false)), Some(Status::Discharging));
    }

    #[test]
    fn times_are_in_hours_once_estimated() {
        let mut draining = battery(50, false, false);
        assert_eq!(
            draining.reading().map(|r| r.time_to),
            Some(TimeTo::Unknown),
            "still estimating"
        );
        draining.time_to_empty = 90;
        assert_eq!(
            draining.reading().map(|r| r.time_to),
            Some(TimeTo::Empty(1.5))
        );

        let mut filling = battery(50, true, true);
        filling.time_to_full = 30;
        filling.time_to_empty = 600;
        assert_eq!(
            filling.reading().map(|r| r.time_to),
            Some(TimeTo::Full(0.5))
        );
        assert_eq!(
            battery(80, false, true).reading().map(|r| r.time_to),
            Some(TimeTo::Unknown),
            "plugged in and idle"
        );
    }

    #[test]
    fn macos_reports_no_draw() {
        assert_eq!(
            battery(50, false, false).reading().map(|r| r.power),
            Some(None)
        );
    }
}
