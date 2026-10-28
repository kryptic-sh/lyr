//! The notification daemon's store (`docs/plan/09-notifications.md`): what
//! lyr keeps of each notification, the ids it hands out, when one expires,
//! whether it is shown as a toast, and the history's order.
//!
//! The rules come from the Desktop Notifications Specification 1.3 and, where
//! the spec leaves a choice, from swaync, which lyr replaces. The D-Bus side
//! is in `lyr-sources`.

use std::time::{Duration, Instant};

use crate::draw::Picture;

/// How urgent a notification is: the `urgency` hint's byte.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Urgency {
    Low,
    #[default]
    Normal,
    Critical,
}

impl Urgency {
    /// The hint's byte: 0 low, 1 normal, 2 critical. Anything else is
    /// normal, the spec's default.
    #[must_use]
    pub const fn from_byte(byte: u8) -> Self {
        match byte {
            0 => Self::Low,
            2 => Self::Critical,
            _ => Self::Normal,
        }
    }
}

/// When a notification closes by itself: `Notify`'s `expire_timeout`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Expiry {
    /// -1: the daemon's own timeout for the urgency.
    Default,
    /// 0: never.
    Never,
    /// Milliseconds after it is shown.
    After(Duration),
}

impl Expiry {
    /// From `expire_timeout`: -1 and any other negative value the daemon's
    /// default, 0 never, and anything else that many milliseconds.
    #[must_use]
    pub fn from_millis(timeout: i32) -> Self {
        match u64::try_from(timeout) {
            Err(_) => Self::Default,
            Ok(0) => Self::Never,
            Ok(millis) => Self::After(Duration::from_millis(millis)),
        }
    }
}

/// Why a notification closed: `NotificationClosed`'s reason.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CloseReason {
    Expired = 1,
    Dismissed = 2,
    /// By `CloseNotification`.
    Closed = 3,
    Undefined = 4,
}

/// One notification as an application sent it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Incoming {
    pub app_name: String,
    /// The notification it replaces, or 0 for none.
    pub replaces_id: u32,
    pub app_icon: String,
    /// `app_icon`'s picture, when it was found.
    pub icon: Option<Picture>,
    /// The notification's image: from `image-data`, else `image-path`, else
    /// the deprecated `icon_data`, as the spec orders them.
    pub image: Option<Picture>,
    pub summary: String,
    pub body: String,
    /// Action id and label pairs.
    pub actions: Vec<(String, String)>,
    pub urgency: Urgency,
    pub category: Option<String>,
    pub desktop_entry: Option<String>,
    /// Shown, but not kept in the history.
    pub transient: bool,
    /// Not closed when an action is invoked.
    pub resident: bool,
    /// How far along what it reports is, 0 – 100: the `value` hint, drawn
    /// as a progress bar.
    pub progress: Option<u8>,
    pub expiry: Option<Expiry>,
}

/// A notification lyr holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Notification {
    pub id: u32,
    pub content: Incoming,
    /// When it arrived, as a count that only grows, so the history can be
    /// ordered newest first.
    pub arrival: u64,
    /// When it arrived, for its "2 mins ago".
    pub received: Instant,
}

impl Notification {
    /// What the history groups it under: the `desktop-entry` hint, else the
    /// app name, as swaync's `name_id` is.
    #[must_use]
    pub fn group(&self) -> &str {
        self.content
            .desktop_entry
            .as_deref()
            .unwrap_or(&self.content.app_name)
    }
}

/// swaync's `timeout` in the dotfiles: a normal notification's lifetime.
pub const NORMAL_TIMEOUT: Duration = Duration::from_secs(10);
/// swaync's `timeout-low`.
pub const LOW_TIMEOUT: Duration = Duration::from_secs(5);

/// The notifications lyr holds, and its do-not-disturb switch.
#[derive(Clone, Debug, Default)]
pub struct Store {
    notifications: Vec<Notification>,
    last_id: u32,
    arrivals: u64,
    pub dnd: bool,
}

impl Store {
    /// Takes a notification that arrived at `now` and returns its id:
    /// `replaces_id` when that is set, as the spec requires, replacing the
    /// notification in place; otherwise the next id, never 0.
    pub fn notify(&mut self, incoming: Incoming, now: Instant) -> u32 {
        self.arrivals += 1;
        let id = if incoming.replaces_id == 0 {
            self.next_id()
        } else {
            incoming.replaces_id
        };
        let notification = Notification {
            id,
            content: incoming,
            arrival: self.arrivals,
            received: now,
        };
        match self.notifications.iter_mut().find(|held| held.id == id) {
            Some(held) => *held = notification,
            None => self.notifications.push(notification),
        }
        id
    }

    /// An id no notification holds, after the last one handed out. Ids wrap
    /// past `u32::MAX` and skip 0, which the spec forbids.
    fn next_id(&mut self) -> u32 {
        loop {
            self.last_id = self.last_id.checked_add(1).unwrap_or(1);
            if !self
                .notifications
                .iter()
                .any(|held| held.id == self.last_id)
            {
                return self.last_id;
            }
        }
    }

    /// Closes `id`, returning it, or `None` when lyr holds no such
    /// notification (the spec's empty error for `CloseNotification`).
    pub fn close(&mut self, id: u32) -> Option<Notification> {
        let index = self.notifications.iter().position(|held| held.id == id)?;
        Some(self.notifications.remove(index))
    }

    /// Closes every notification, as Clear All does, returning their ids.
    pub fn clear(&mut self) -> Vec<u32> {
        self.notifications.drain(..).map(|held| held.id).collect()
    }

    /// The notification `id`.
    #[must_use]
    pub fn get(&self, id: u32) -> Option<&Notification> {
        self.notifications.iter().find(|held| held.id == id)
    }

    /// How long `notification`'s toast stays: its own timeout, or swaync's
    /// for its urgency (critical never expires). `None` is never.
    #[must_use]
    pub fn timeout(notification: &Notification) -> Option<Duration> {
        match notification.content.expiry.unwrap_or(Expiry::Default) {
            Expiry::After(after) => Some(after),
            Expiry::Never => None,
            Expiry::Default => match notification.content.urgency {
                Urgency::Low => Some(LOW_TIMEOUT),
                Urgency::Normal => Some(NORMAL_TIMEOUT),
                Urgency::Critical => None,
            },
        }
    }

    /// Whether `notification` gets a toast: not while the clock panel is
    /// open, and not under do-not-disturb unless it is critical, as swaync
    /// does.
    #[must_use]
    pub fn shows_toast(&self, notification: &Notification, panel_open: bool) -> bool {
        !panel_open && (!self.dnd || notification.content.urgency == Urgency::Critical)
    }

    /// The history as the clock panel lists it: transient notifications left
    /// out, grouped by app, groups holding a critical notification first,
    /// then newest first, each group newest first. swaync's order for a panel
    /// at the top (`list_box_sort_func` at v0.12.6).
    #[must_use]
    pub fn history(&self) -> Vec<(&str, Vec<&Notification>)> {
        let mut groups: Vec<(&str, Vec<&Notification>)> = Vec::new();
        for held in self
            .notifications
            .iter()
            .filter(|held| !held.content.transient)
        {
            match groups.iter_mut().find(|(name, _)| *name == held.group()) {
                Some((_, members)) => members.push(held),
                None => groups.push((held.group(), vec![held])),
            }
        }
        for (_, members) in &mut groups {
            members.sort_by_key(|held| std::cmp::Reverse(held.arrival));
        }
        let critical = |members: &[&Notification]| {
            members
                .iter()
                .any(|held| held.content.urgency == Urgency::Critical)
        };
        groups.sort_by(|(_, a), (_, b)| {
            critical(b)
                .cmp(&critical(a))
                .then_with(|| b[0].arrival.cmp(&a[0].arrival))
        });
        groups
    }
}

#[cfg(test)]
mod tests;
