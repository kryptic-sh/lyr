//! `lyr --stats` (`docs/plan/12-performance-and-power.md#checking-it`): the
//! main loop's wake-ups, the events it applied and the frames it presented on
//! each output, logged once a minute. The idle check reads that line: a bar
//! showing only the clock should wake once a minute and present only the
//! clock's frame.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

/// How often the counts are logged.
const PERIOD: Duration = Duration::from_secs(60);

/// The counts since they were last logged.
#[derive(Debug)]
pub struct Stats {
    since: Instant,
    pub wakeups: u64,
    /// Window-system events and source updates.
    pub events: u64,
    /// Frames presented, by output name.
    presents: BTreeMap<String, u64>,
}

impl Stats {
    pub fn new(now: Instant) -> Self {
        Self {
            since: now,
            wakeups: 0,
            events: 0,
            presents: BTreeMap::new(),
        }
    }

    /// Counts a frame presented on the output named `monitor`.
    pub fn presented(&mut self, monitor: Option<&str>) {
        *self
            .presents
            .entry(monitor.unwrap_or("?").to_owned())
            .or_default() += 1;
    }

    /// The line logged for the counts so far, over `elapsed`.
    fn line(&self, elapsed: Duration) -> String {
        let presents = if self.presents.is_empty() {
            " none".to_owned()
        } else {
            self.presents
                .iter()
                .map(|(monitor, count)| format!(" {monitor} {count}"))
                .collect()
        };
        format!(
            "stats over {:.0} s: {} wake-ups, {} events, presents:{presents}",
            elapsed.as_secs_f64(),
            self.wakeups,
            self.events
        )
    }

    /// Logs the counts and starts again once a period has passed. Called on
    /// wake-ups the loop has anyway, so the counting adds none of its own:
    /// the line comes with the first wake-up after the minute is up.
    pub fn log_if_due(&mut self, now: Instant) -> bool {
        let elapsed = now.duration_since(self.since);
        if elapsed < PERIOD {
            return false;
        }
        tracing::info!("{}", self.line(elapsed));
        *self = Self::new(now);
        true
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{PERIOD, Stats};

    #[test]
    fn the_counts_are_logged_once_a_period_and_start_again() {
        let start = Instant::now();
        let mut stats = Stats::new(start);
        stats.wakeups = 3;
        assert!(!stats.log_if_due(start + PERIOD - Duration::from_millis(1)));
        assert_eq!(stats.wakeups, 3, "kept until the period is up");
        assert!(stats.log_if_due(start + PERIOD));
        assert_eq!(stats.wakeups, 0, "counting starts again");
    }

    #[test]
    fn the_line_counts_presents_per_output() {
        let mut stats = Stats::new(Instant::now());
        assert_eq!(
            stats.line(PERIOD),
            "stats over 60 s: 0 wake-ups, 0 events, presents: none"
        );
        stats.wakeups = 2;
        stats.events = 5;
        stats.presented(Some("DP-1"));
        stats.presented(Some("HDMI-A-1"));
        stats.presented(Some("DP-1"));
        stats.presented(None);
        assert_eq!(
            stats.line(PERIOD),
            "stats over 60 s: 2 wake-ups, 5 events, presents: ? 1 DP-1 2 HDMI-A-1 1"
        );
    }
}
