//! Windows' own notifications, read through `crcbl-notify` for the clock
//! panel to list: lyr is not the notification daemon there, so its store
//! holds a copy ([`lyr_core::notifications::Mirror`]) rather than what it
//! was sent.

use std::time::{Instant, SystemTime};

use crcbl_notify::{Notification, NotificationId};
use lyr_core::markup;
use lyr_core::notifications::{Expiry, Incoming};

pub use crcbl_notify::{NotificationError, Notifications};

/// `notification` as the store holds it, under its id, with the moment it
/// arrived on lyr's clock, given that `now` is the same moment on both
/// clocks. Its sender's AppUserModelID is its `desktop_entry`: what groups
/// it in the panel and finds the sender's windows. Windows' text is plain,
/// so the body is escaped for the markup a body is drawn as.
#[must_use]
pub fn incoming(
    notification: &Notification,
    now: (Instant, SystemTime),
) -> (NotificationId, Incoming, Instant) {
    let content = Incoming {
        app_name: notification.app.name.clone(),
        desktop_entry: Some(notification.app.id.clone()),
        summary: notification.title.clone(),
        body: markup::escape(&notification.body),
        // Windows takes it down, not lyr.
        expiry: Some(Expiry::Never),
        ..Incoming::default()
    };
    (
        notification.id,
        content,
        received(notification.arrived, now),
    )
}

/// The moment `arrived` on lyr's clock, given that `now` is the same moment
/// on both: as long before `now` as it was before the system's. One the
/// system's clock puts in the future, or before lyr's clock can reach, is
/// `now`.
fn received(arrived: SystemTime, (instant, system): (Instant, SystemTime)) -> Instant {
    system
        .duration_since(arrived)
        .ok()
        .and_then(|ago| instant.checked_sub(ago))
        .unwrap_or(instant)
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant, SystemTime};

    use super::received;

    #[test]
    fn arrival_keeps_its_distance_from_now() {
        let now = (Instant::now(), SystemTime::now());
        let ago = Duration::from_secs(90);
        assert_eq!(received(now.1 - ago, now), now.0 - ago);
        assert_eq!(received(now.1, now), now.0);
        assert_eq!(
            received(now.1 + ago, now),
            now.0,
            "a clock ahead of the system's is now"
        );
    }
}
