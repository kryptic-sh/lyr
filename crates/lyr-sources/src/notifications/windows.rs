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

/// The names the sender of a notification goes by, for finding its windows
/// by their program's name (`lyr_wm::app::is_app`): its AppUserModelID
/// `aumid`; the program a path-like ID names, without `.exe`, as Windows
/// makes one for a program that sets none
/// (`{1AC14E77-…}\WindowsPowerShell\v1.0\powershell.exe`); a packaged app's
/// package name, without its publisher's hash and the app's id
/// (`Microsoft.WindowsTerminal` from
/// `Microsoft.WindowsTerminal_8wekyb3d8bbwe!App`); and its display name
/// `app_name`. Empty ones are left out.
#[must_use]
pub fn sender_names(aumid: &str, app_name: &str) -> Vec<String> {
    let program = (aumid.contains(['\\', '/'])).then(|| crate::program::name(aumid));
    let package = aumid
        .split_once('!')
        .map(|(family, _)| family.rsplit_once('_').map_or(family, |(name, _)| name));
    let mut names: Vec<String> = Vec::new();
    for name in [Some(aumid), program, package, Some(app_name)]
        .into_iter()
        .flatten()
        .map(str::trim)
        .filter(|name| !name.is_empty())
    {
        if !names.iter().any(|held| held == name) {
            names.push(name.to_owned());
        }
    }
    names
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

    use super::{received, sender_names};

    #[test]
    fn a_sender_is_named_by_its_id_its_program_or_package_and_its_name() {
        assert_eq!(
            sender_names("Chrome", "Google Chrome"),
            ["Chrome", "Google Chrome"]
        );
        assert_eq!(
            sender_names(
                "{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}\\WindowsPowerShell\\v1.0\\powershell.exe",
                "Windows PowerShell",
            )[1..],
            ["powershell", "Windows PowerShell"],
            "a path's program"
        );
        assert_eq!(
            sender_names("Microsoft.WindowsTerminal_8wekyb3d8bbwe!App", "Terminal")[1..],
            ["Microsoft.WindowsTerminal", "Terminal"],
            "a package's name"
        );
        assert_eq!(sender_names("App", "App"), ["App"], "once each");
        assert_eq!(sender_names(" ", ""), Vec::<String>::new());
    }

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
