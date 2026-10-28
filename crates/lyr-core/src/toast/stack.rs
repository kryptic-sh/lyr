//! Which notifications have a toast up, newest on top, and when each goes
//! (`docs/plan/09-notifications.md#toasts`). A toast leaves when its timeout
//! is up, while the pointer is off it: hovering pauses the timeout and
//! leaving starts it again with what remained, and shows the toast's close
//! button.
//!
//! A toast comes and goes as swaync 0.12.6's floating notifications do
//! (its `AnimatedList`, `AnimatedListItem`): over [`ANIMATION`], eased out
//! quintically, an arriving toast's place grows from nothing while the toast
//! slides in from the right and fades in, and a leaving one's place shrinks
//! while it fades out. A leaving toast is drawn from what it showed, since
//! its notification may be gone from the store.

use std::time::{Duration, Instant};

use lyr_config::Theme;

use super::{Area, LayOut, MARGIN, WIDTH, close_button, draw};
use crate::draw::{DrawList, Item, Size, TextStyle};
use crate::hit::{Rect, Regions};
use crate::notifications::{Notification, Store};

/// How long a toast takes to come up or go: swaync 0.12.6's
/// `ANIMATION_DURATION`.
pub const ANIMATION: Duration = Duration::from_millis(400);

/// What a click on the stack lands on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    /// A toast, by its notification's id.
    Body(u32),
    /// An action button, by the action's index in the notification's
    /// actions.
    Action(u32, usize),
    /// A link in the body, by its index among the body's links.
    Link(u32, usize),
    /// The close button of the toast under the pointer.
    Close(u32),
}

impl Part {
    /// The notification it belongs to.
    #[must_use]
    pub const fn id(self) -> u32 {
        match self {
            Self::Body(id) | Self::Action(id, _) | Self::Link(id, _) | Self::Close(id) => id,
        }
    }
}

/// A toast on its way up or out, since when.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Motion {
    Arriving(Instant),
    Leaving(Instant),
}

/// One toast in the stack.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Up {
    /// The notification as its toast shows it.
    shown: Notification,
    /// When it goes, or `None` for never (a critical notification) or while
    /// the pointer pauses it.
    until: Option<Instant>,
    /// What remained of its timeout when the pointer paused it.
    paused: Option<Duration>,
    /// `None` once it is still.
    motion: Option<Motion>,
}

impl Up {
    const fn id(&self) -> u32 {
        self.shown.id
    }

    const fn leaving(&self) -> bool {
        matches!(self.motion, Some(Motion::Leaving(_)))
    }

    /// How much of its place it takes at `now`, 0 – 1.
    fn shown_part(&self, now: Instant) -> f32 {
        let along = |since: Instant| {
            (now.saturating_duration_since(since).as_secs_f32() / ANIMATION.as_secs_f32()).min(1.0)
        };
        match self.motion {
            None => 1.0,
            Some(Motion::Arriving(since)) => ease_out_quint(along(since)),
            Some(Motion::Leaving(since)) => 1.0 - ease_out_quint(along(since)),
        }
    }
}

/// What a click on `area` of the toast for `id` stands for.
const fn part_of(area: Area, id: u32) -> Part {
    match area {
        Area::Action(index) => Part::Action(id, index),
        Area::Link(index) => Part::Link(id, index),
    }
}

/// libadwaita's `ADW_EASE_OUT_QUINT`, which swaync's animation takes.
fn ease_out_quint(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(5)
}

/// The toasts that are up.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Stack {
    /// Newest first, with those still leaving where they were.
    up: Vec<Up>,
    hovered: Option<u32>,
}

impl Stack {
    /// The notification `id` arrived in `store` or replaced one: it goes on
    /// top with its full timeout, if it gets a toast at all, coming up
    /// unless its toast was up already.
    pub fn notified(&mut self, store: &Store, id: u32, panel_open: bool, now: Instant) {
        let was_up = self
            .up
            .iter()
            .find(|up| up.id() == id && !up.leaving())
            .map(|up| up.motion);
        self.up.retain(|up| up.id() != id);
        let Some(notification) = store.get(id) else {
            return;
        };
        if !store.shows_toast(notification, panel_open) {
            return;
        }
        let timeout = Store::timeout(notification);
        let (until, paused) = if self.hovered == Some(id) {
            (None, timeout)
        } else {
            (timeout.map(|timeout| now + timeout), None)
        };
        self.up.insert(
            0,
            Up {
                shown: notification.clone(),
                until,
                paused,
                motion: was_up.unwrap_or(Some(Motion::Arriving(now))),
            },
        );
    }

    /// The notification `id` closed at `now`: its toast goes.
    pub fn closed(&mut self, id: u32, now: Instant) {
        for up in &mut self.up {
            if up.id() == id && !up.leaving() {
                up.motion = Some(Motion::Leaving(now));
            }
        }
    }

    /// Takes down the toasts whose time is up at `now` and returns their
    /// ids.
    pub fn expire(&mut self, now: Instant) -> Vec<u32> {
        let mut gone = Vec::new();
        for up in &mut self.up {
            if !up.leaving() && up.until.is_some_and(|until| until <= now) {
                up.motion = Some(Motion::Leaving(now));
                gone.push(up.id());
            }
        }
        gone
    }

    /// Ends the motions over by `now`: an arrived toast stands still and a
    /// left one goes. Returns whether any did, which changes what is drawn.
    pub fn settle(&mut self, now: Instant) -> bool {
        let over = |since: Instant| now.saturating_duration_since(since) >= ANIMATION;
        let before = self.up.len();
        self.up
            .retain(|up| !matches!(up.motion, Some(Motion::Leaving(since)) if over(since)));
        let mut changed = self.up.len() != before;
        for up in &mut self.up {
            if let Some(Motion::Arriving(since)) = up.motion
                && over(since)
            {
                up.motion = None;
                changed = true;
            }
        }
        changed
    }

    /// Whether a toast is coming or going, so each frame differs.
    #[must_use]
    pub fn moving(&self) -> bool {
        self.up.iter().any(|up| up.motion.is_some())
    }

    /// The pointer is over the toast `id`, or over none: that one's timeout
    /// pauses, and the one it left resumes with what remained. Returns
    /// whether it moved to another toast, which shows another close button.
    pub fn hover(&mut self, id: Option<u32>, now: Instant) -> bool {
        if id == self.hovered {
            return false;
        }
        for up in &mut self.up {
            if Some(up.id()) == self.hovered
                && let Some(left) = up.paused.take()
            {
                up.until = Some(now + left);
            }
            if Some(up.id()) == id
                && let Some(until) = up.until.take()
            {
                up.paused = Some(until.saturating_duration_since(now));
            }
        }
        self.hovered = id;
        true
    }

    /// When [`Self::expire`] next has something to do.
    #[must_use]
    pub fn next_wake(&self) -> Option<Instant> {
        self.staying().filter_map(|up| up.until).min()
    }

    /// When a toast's "N mins ago" next changes, at `now` or later: a whole
    /// minute after its notification arrived.
    #[must_use]
    pub fn next_label_change(&self, now: Instant) -> Option<Instant> {
        self.staying()
            .map(|up| {
                let received = up.shown.received;
                let minutes = now.saturating_duration_since(received).as_secs() / 60;
                received + Duration::from_secs((minutes + 1) * 60)
            })
            .min()
    }

    /// The toasts not on their way out.
    fn staying(&self) -> impl Iterator<Item = &Up> {
        self.up.iter().filter(|up| !up.leaving())
    }

    /// The ids of the toasts that are up and not leaving, newest first.
    pub fn ids(&self) -> impl Iterator<Item = u32> + '_ {
        self.staying().map(Up::id)
    }

    /// The stack drawn at `now`, newest on top and [`MARGIN`] apart: its
    /// draw list, its size, and each toast's and action button's region,
    /// as far as an arriving one is drawn, and none for a leaving one. The size is every toast's at full height, those coming and
    /// going included, so the window keeps one size while they move and
    /// changes it as one arrives and once one has left. Empty when no toast
    /// is up.
    #[must_use]
    pub fn draw(
        &self,
        now: Instant,
        measure: impl Fn(&str, TextStyle) -> f32,
        lay_out: &LayOut<'_>,
        line_height: f32,
        theme: &Theme,
    ) -> (DrawList, Size, Regions<Part>) {
        let mut list = DrawList::default();
        let mut regions = Regions::default();
        let mut size = Size {
            width: 0.0,
            height: 0.0,
        };
        // Where the next toast's place starts.
        let mut top = 0.0;
        for up in &self.up {
            let notification = &up.shown;
            let elapsed = now.saturating_duration_since(notification.received);
            let (toast, toast_size, areas) = draw(
                &notification.content,
                elapsed,
                WIDTH,
                &measure,
                lay_out,
                line_height,
                theme,
            );
            size = Size {
                width: size.width.max(toast_size.width),
                height: size.height
                    + toast_size.height
                    + if size.height > 0.0 { MARGIN } else { 0.0 },
            };
            let part = up.shown_part(now);
            if up.motion.is_some() {
                let slide = match up.motion {
                    Some(Motion::Arriving(_)) => toast_size.width * (1.0 - part),
                    _ => 0.0,
                };
                let clip = Rect {
                    x: 0.0,
                    y: top,
                    width: toast_size.width,
                    height: toast_size.height * part,
                };
                // An arriving toast takes clicks where it is drawn so far.
                if !up.leaving() {
                    regions.push(Rect { x: slide, ..clip }, Part::Body(notification.id));
                    for (area, rect) in areas {
                        let rect = Rect {
                            x: rect.x + slide,
                            y: rect.y + top,
                            ..rect
                        };
                        if rect.y + rect.height <= clip.y + clip.height {
                            regions.push(rect, part_of(area, notification.id));
                        }
                    }
                }
                list.items.push(Item::Clip {
                    rect: clip,
                    items: vec![Item::Group {
                        opacity: part,
                        items: toast
                            .items
                            .into_iter()
                            .map(|item| item.offset(slide, top))
                            .collect(),
                    }],
                });
                top += (toast_size.height + MARGIN) * part;
                continue;
            }
            list.items
                .extend(toast.items.into_iter().map(|item| item.offset(0.0, top)));
            regions.push(
                Rect {
                    x: 0.0,
                    y: top,
                    width: toast_size.width,
                    height: toast_size.height,
                },
                Part::Body(notification.id),
            );
            for (area, rect) in areas {
                regions.push(
                    Rect {
                        y: rect.y + top,
                        ..rect
                    },
                    part_of(area, notification.id),
                );
            }
            if self.hovered == Some(notification.id) {
                let (items, rect) = close_button(toast_size.width, &measure, line_height, theme);
                list.items
                    .extend(items.into_iter().map(|item| item.offset(0.0, top)));
                regions.push(
                    Rect {
                        y: rect.y + top,
                        ..rect
                    },
                    Part::Close(notification.id),
                );
            }
            top += toast_size.height + MARGIN;
        }
        (list, size, regions)
    }
}

#[cfg(test)]
mod tests;
