//! Which notifications have a toast up, newest on top, and when each goes
//! (`docs/plan/09-notifications.md#toasts`). A toast leaves when its timeout
//! is up, while the pointer is off it: hovering pauses the timeout and
//! leaving starts it again with what remained, and shows the toast's close
//! button.

use std::time::{Duration, Instant};

use lyr_config::Theme;

use super::{MARGIN, WIDTH, close_button, draw};
use crate::draw::{DrawList, Size};
use crate::hit::{Rect, Regions};
use crate::notifications::Store;

/// What a click on the stack lands on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    /// A toast, by its notification's id.
    Body(u32),
    /// An action button, by the action's index in the notification's
    /// actions.
    Action(u32, usize),
    /// The close button of the toast under the pointer.
    Close(u32),
}

impl Part {
    /// The notification it belongs to.
    #[must_use]
    pub const fn id(self) -> u32 {
        match self {
            Self::Body(id) | Self::Action(id, _) | Self::Close(id) => id,
        }
    }
}

/// One toast that is up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Up {
    id: u32,
    /// When it goes, or `None` for never (a critical notification) or while
    /// the pointer pauses it.
    until: Option<Instant>,
    /// What remained of its timeout when the pointer paused it.
    paused: Option<Duration>,
}

/// The toasts that are up.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Stack {
    /// Newest first.
    up: Vec<Up>,
    hovered: Option<u32>,
}

impl Stack {
    /// The notification `id` arrived in `store` or replaced one: it goes on
    /// top with its full timeout, if it gets a toast at all.
    pub fn notified(&mut self, store: &Store, id: u32, panel_open: bool, now: Instant) {
        self.up.retain(|up| up.id != id);
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
        self.up.insert(0, Up { id, until, paused });
    }

    /// The notification `id` closed: its toast goes.
    pub fn closed(&mut self, id: u32) {
        self.up.retain(|up| up.id != id);
    }

    /// Takes down the toasts whose time is up and returns their ids.
    pub fn expire(&mut self, now: Instant) -> Vec<u32> {
        let (gone, kept) = self
            .up
            .iter()
            .partition(|up| up.until.is_some_and(|until| until <= now));
        self.up = kept;
        gone.into_iter().map(|up: Up| up.id).collect()
    }

    /// The pointer is over the toast `id`, or over none: that one's timeout
    /// pauses, and the one it left resumes with what remained. Returns
    /// whether it moved to another toast, which shows another close button.
    pub fn hover(&mut self, id: Option<u32>, now: Instant) -> bool {
        if id == self.hovered {
            return false;
        }
        for up in &mut self.up {
            if Some(up.id) == self.hovered
                && let Some(left) = up.paused.take()
            {
                up.until = Some(now + left);
            }
            if Some(up.id) == id
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
        self.up.iter().filter_map(|up| up.until).min()
    }

    /// When a toast's "N mins ago" next changes, at `now` or later: a whole
    /// minute after it arrived in `store`.
    #[must_use]
    pub fn next_label_change(&self, store: &Store, now: Instant) -> Option<Instant> {
        self.up
            .iter()
            .filter_map(|up| store.get(up.id))
            .map(|held| {
                let minutes = now.saturating_duration_since(held.received).as_secs() / 60;
                held.received + Duration::from_secs((minutes + 1) * 60)
            })
            .min()
    }

    /// The ids of the toasts that are up, newest first.
    pub fn ids(&self) -> impl Iterator<Item = u32> + '_ {
        self.up.iter().map(|up| up.id)
    }

    /// The stack drawn from `store` at `now`, newest on top and [`MARGIN`]
    /// apart: its draw list, its size, and each toast's and action button's
    /// region. Empty when no toast is up.
    #[must_use]
    pub fn draw(
        &self,
        store: &Store,
        now: Instant,
        measure: impl Fn(&str) -> f32,
        line_height: f32,
        theme: &Theme,
    ) -> (DrawList, Size, Regions<Part>) {
        let mut list = DrawList::default();
        let mut regions = Regions::default();
        let mut size = Size {
            width: 0.0,
            height: 0.0,
        };
        for notification in self.ids().filter_map(|id| store.get(id)) {
            let elapsed = now.saturating_duration_since(notification.received);
            let (toast, toast_size, buttons) = draw(
                &notification.content,
                elapsed,
                WIDTH,
                &measure,
                line_height,
                theme,
            );
            let top = if size.height > 0.0 {
                size.height + MARGIN
            } else {
                0.0
            };
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
            for (index, rect) in buttons {
                regions.push(
                    Rect {
                        y: rect.y + top,
                        ..rect
                    },
                    Part::Action(notification.id, index),
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
            size = Size {
                width: size.width.max(toast_size.width),
                height: top + toast_size.height,
            };
        }
        (list, size, regions)
    }
}

#[cfg(test)]
mod tests;
