//! When a tooltip shows (`docs/plan/10-popups-and-menus.md#tooltips`), timed
//! as GTK 3 times waybar's: 500 ms of hovering shows it, and for 500 ms after
//! one hides, moving onto another target shows that one's after 60 ms
//! ("browse mode"). A click hides it until the pointer moves to another
//! target. The constants are `gtktooltip.c`'s at GTK 3.24.43.

use std::time::{Duration, Instant};

/// How long the pointer rests on a target before its tooltip shows.
pub const HOVER_DELAY: Duration = Duration::from_millis(500);
/// The delay instead while browsing from one tooltip to the next.
pub const BROWSE_DELAY: Duration = Duration::from_millis(60);
/// How long browse mode lasts after a tooltip hides.
pub const BROWSE_WINDOW: Duration = Duration::from_millis(500);

/// The tooltip state of one bar, over targets of type `T`.
#[derive(Debug)]
pub struct Tooltip<T> {
    /// What the pointer is over.
    target: Option<T>,
    /// When its tooltip shows, while it is still to show.
    show_at: Option<Instant>,
    shown: bool,
    /// Until when a newly hovered target shows after [`BROWSE_DELAY`].
    browse_until: Option<Instant>,
}

impl<T> Default for Tooltip<T> {
    fn default() -> Self {
        Self {
            target: None,
            show_at: None,
            shown: false,
            browse_until: None,
        }
    }
}

impl<T: PartialEq> Tooltip<T> {
    /// The pointer is over `target` (`None`: over nothing with a tooltip, or
    /// off the bar). Returns whether the shown tooltip changed.
    pub fn hover(&mut self, target: Option<T>, now: Instant) -> bool {
        if target == self.target {
            return false;
        }
        let was_shown = self.hide(now);
        let browsing = self.browse_until.is_some_and(|until| now < until);
        self.show_at = target
            .is_some()
            .then(|| now + if browsing { BROWSE_DELAY } else { HOVER_DELAY });
        self.target = target;
        was_shown
    }

    /// A click: the tooltip hides and stays hidden until the pointer moves
    /// to another target. Returns whether one was shown.
    pub fn press(&mut self, now: Instant) -> bool {
        let was_shown = self.hide(now);
        self.show_at = None;
        was_shown
    }

    /// Shows the tooltip if its delay is up. Returns whether it changed.
    pub fn tick(&mut self, now: Instant) -> bool {
        if self.show_at.is_some_and(|at| at <= now) {
            self.show_at = None;
            self.shown = true;
            return true;
        }
        false
    }

    /// When [`Self::tick`] next has something to do.
    #[must_use]
    pub const fn next_wake(&self) -> Option<Instant> {
        self.show_at
    }

    /// The target whose tooltip is showing.
    #[must_use]
    pub fn shown(&self) -> Option<&T> {
        self.target.as_ref().filter(|_| self.shown)
    }

    fn hide(&mut self, now: Instant) -> bool {
        if !self.shown {
            return false;
        }
        self.shown = false;
        self.browse_until = Some(now + BROWSE_WINDOW);
        true
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{BROWSE_DELAY, BROWSE_WINDOW, HOVER_DELAY, Tooltip};

    const MS: Duration = Duration::from_millis(1);

    #[test]
    fn a_tooltip_shows_after_the_hover_delay() {
        let start = Instant::now();
        let mut tooltip = Tooltip::default();
        assert!(!tooltip.hover(Some("clock"), start));
        assert_eq!(tooltip.next_wake(), Some(start + HOVER_DELAY));
        assert!(!tooltip.tick(start + HOVER_DELAY - MS));
        assert_eq!(tooltip.shown(), None);
        assert!(tooltip.tick(start + HOVER_DELAY));
        assert_eq!(tooltip.shown(), Some(&"clock"));
        assert_eq!(tooltip.next_wake(), None);
    }

    #[test]
    fn moving_on_while_one_shows_browses_to_the_next_quickly() {
        let start = Instant::now();
        let mut tooltip = Tooltip::default();
        tooltip.hover(Some("clock"), start);
        tooltip.tick(start + HOVER_DELAY);
        let moved = start + HOVER_DELAY + 10 * MS;
        assert!(tooltip.hover(Some("battery"), moved), "the clock's hides");
        assert_eq!(tooltip.shown(), None);
        assert_eq!(tooltip.next_wake(), Some(moved + BROWSE_DELAY));
    }

    #[test]
    fn browse_mode_ends_after_its_window() {
        let start = Instant::now();
        let mut tooltip = Tooltip::default();
        tooltip.hover(Some("clock"), start);
        tooltip.tick(start + HOVER_DELAY);
        let left = start + HOVER_DELAY;
        tooltip.hover(None, left);
        assert_eq!(tooltip.next_wake(), None, "nothing to show off a target");
        let back = left + BROWSE_WINDOW;
        tooltip.hover(Some("battery"), back);
        assert_eq!(tooltip.next_wake(), Some(back + HOVER_DELAY));
    }

    #[test]
    fn leaving_before_the_delay_shows_nothing() {
        let start = Instant::now();
        let mut tooltip = Tooltip::default();
        tooltip.hover(Some("clock"), start);
        assert!(!tooltip.hover(None, start + 100 * MS));
        assert!(!tooltip.tick(start + HOVER_DELAY));
        assert_eq!(tooltip.shown(), None);
        // A tooltip that never showed starts no browse mode.
        let next = start + HOVER_DELAY;
        tooltip.hover(Some("battery"), next);
        assert_eq!(tooltip.next_wake(), Some(next + HOVER_DELAY));
    }

    #[test]
    fn a_click_hides_it_until_the_pointer_moves_on() {
        let start = Instant::now();
        let mut tooltip = Tooltip::default();
        tooltip.hover(Some("clock"), start);
        tooltip.tick(start + HOVER_DELAY);
        let clicked = start + HOVER_DELAY + MS;
        assert!(tooltip.press(clicked));
        assert_eq!(tooltip.shown(), None);
        assert!(!tooltip.hover(Some("clock"), clicked + MS), "same target");
        assert!(!tooltip.tick(clicked + HOVER_DELAY * 2));
        tooltip.hover(Some("battery"), clicked + 2 * MS);
        assert_eq!(tooltip.next_wake(), Some(clicked + 2 * MS + BROWSE_DELAY));
    }

    #[test]
    fn a_click_before_the_delay_keeps_it_from_showing() {
        let start = Instant::now();
        let mut tooltip = Tooltip::default();
        tooltip.hover(Some("clock"), start);
        assert!(!tooltip.press(start + 100 * MS));
        assert_eq!(tooltip.next_wake(), None);
        assert!(!tooltip.tick(start + HOVER_DELAY));
    }
}
