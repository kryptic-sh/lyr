//! `lyr --stats` (`docs/plan/12-performance-and-power.md#checking-it`): the
//! main loop's wake-ups, the events it applied and the frames it presented on
//! each output, logged once a minute. The idle check reads that line: a bar
//! showing only the clock should wake once a minute and present only the
//! clock's frame. With or without it, each bar's present logs at `debug` how
//! long after the wake-up that changed it the frame went out (plan 12's
//! event-to-pixels target).

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
    /// When the loop woke with something that may change a bar, until a
    /// frame shows it or nothing is left to draw.
    changed_at: Option<Instant>,
}

impl Stats {
    pub fn new(now: Instant) -> Self {
        Self {
            since: now,
            wakeups: 0,
            events: 0,
            presents: BTreeMap::new(),
            changed_at: None,
        }
    }

    /// Notes that the loop woke at `at` with something that may change a
    /// bar: an event, a source's update or the clock's tick. The first such
    /// wake-up since the last frame counts.
    pub fn changed(&mut self, at: Instant) {
        self.changed_at.get_or_insert(at);
    }

    /// Counts a frame presented on the output named `monitor` at `now`, and
    /// logs how long after the change it shows it went out, returning that.
    pub fn presented(&mut self, monitor: Option<&str>, now: Instant) -> Option<Duration> {
        let monitor = monitor.unwrap_or("?");
        *self.presents.entry(monitor.to_owned()).or_default() += 1;
        let delay = now.saturating_duration_since(self.changed_at?);
        tracing::debug!(
            "presented on {monitor} {:.1} ms after the wake-up that changed it",
            delay.as_secs_f64() * 1000.0
        );
        Some(delay)
    }

    /// Ends a draw: the change is shown, or showed nothing, unless a bar is
    /// still `waiting` to present it (the window system has not asked for
    /// its frame yet).
    pub fn drawn(&mut self, waiting: bool) {
        if !waiting {
            self.changed_at = None;
        }
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
        *self = Self {
            changed_at: self.changed_at,
            ..Self::new(now)
        };
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
    fn a_present_is_timed_from_the_first_wake_up_that_changed_it() {
        let start = Instant::now();
        let ms = Duration::from_millis;
        let mut stats = Stats::new(start);
        assert_eq!(stats.presented(Some("DP-1"), start), None, "no change");
        stats.changed(start);
        stats.changed(start + ms(5));
        assert_eq!(stats.presented(Some("DP-1"), start + ms(9)), Some(ms(9)));
        // A bar still waiting for its frame keeps the change's time, and
        // the stats line does not drop it.
        stats.drawn(true);
        assert!(stats.log_if_due(start + PERIOD));
        assert_eq!(
            stats.presented(Some("HDMI-A-1"), start + PERIOD),
            Some(PERIOD)
        );
        // Once nothing is left to draw, the next present is timed afresh.
        stats.drawn(false);
        assert_eq!(stats.presented(Some("DP-1"), start + PERIOD), None);
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
        let now = Instant::now();
        stats.presented(Some("DP-1"), now);
        stats.presented(Some("HDMI-A-1"), now);
        stats.presented(Some("DP-1"), now);
        stats.presented(None, now);
        assert_eq!(
            stats.line(PERIOD),
            "stats over 60 s: 2 wake-ups, 5 events, presents: ? 1 DP-1 2 HDMI-A-1 1"
        );
    }
}
