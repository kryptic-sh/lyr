//! Windows: Windows' own notifications, copied into a store for the clock
//! panel to list, and the panel's Clear All and close buttons carried out on
//! them.
//!
//! Windows tells an unpackaged program nothing when its notifications
//! change, so they are read again every [`POLL`], and at once after a
//! dismissal. A store lyr may not read, which Windows' notification-access
//! privacy setting decides, is tried again at the same rate, so the panel
//! lists them once access is granted; access taken away again drops lyr's
//! handle on the store, the last snapshot still listed, until access is
//! granted again.

use std::hash::Hash;
use std::sync::{Arc, Mutex, PoisonError, mpsc};
use std::time::{Duration, Instant, SystemTime};

use crcbl_shell::ShellWaker;
use lyr_core::notifications::{Mirror, Store};
use lyr_sources::notifications::windows::{NotificationError, Notifications, incoming};

use super::{Update, deliver};

/// How often Windows' notifications are read.
const POLL: Duration = Duration::from_secs(2);

/// What the bar asks of Windows' notifications, by lyr's ids.
#[derive(Debug)]
pub(super) enum Work {
    Dismiss(u32),
    Clear,
}

/// Starts the thread that reads Windows' notifications into a store, hands
/// the store to the main loop once it can be read and again after every
/// change, and carries out what the bar asks. It ends with the bar.
///
/// # Errors
///
/// When the thread cannot start.
pub(super) fn start(
    sender: mpsc::Sender<Update>,
    waker: Option<ShellWaker>,
) -> std::io::Result<mpsc::Sender<Work>> {
    let (work, incoming_work) = mpsc::channel();
    std::thread::Builder::new()
        .name("lyr notifications".to_owned())
        .spawn(move || {
            let store = Arc::new(Mutex::new(Store::default()));
            let mut mirror = Mirror::default();
            let mut opened: Option<Notifications> = None;
            // Whether the main loop has the store: from the first read, empty
            // or not, so the panel shows its centre.
            let mut handed = false;
            // The last failure logged, so one repeated is logged once.
            let mut failure: Option<String> = None;
            loop {
                if opened.is_none() {
                    match Notifications::open() {
                        Ok(store) => {
                            tracing::info!("notifications: reading Windows' notifications");
                            opened = Some(store);
                        }
                        Err(error) => {
                            fail(
                                &mut failure,
                                format!("Windows' notifications cannot be read: {error}"),
                            );
                        }
                    }
                }
                if let Some(notifications) = &opened {
                    match notifications.list() {
                        Ok(listed) => {
                            failure = None;
                            let now = (Instant::now(), SystemTime::now());
                            let listed = listed.iter().map(|shown| incoming(shown, now)).collect();
                            let changed = mirror.apply(
                                &mut store.lock().unwrap_or_else(PoisonError::into_inner),
                                listed,
                            );
                            if changed || !handed {
                                if !deliver(
                                    &sender,
                                    waker.as_ref(),
                                    Update::Notifications(Arc::clone(&store)),
                                ) {
                                    break;
                                }
                                handed = true;
                            }
                        }
                        Err(error) => {
                            // `list` says only that it failed; `open` reads
                            // whether access was taken away, and the store is
                            // opened again once it is granted.
                            if matches!(
                                Notifications::open(),
                                Err(NotificationError::Denied | NotificationError::Undecided)
                            ) {
                                opened = None;
                            }
                            // The last snapshot stays listed rather than none.
                            fail(
                                &mut failure,
                                format!("reading Windows' notifications: {error}"),
                            );
                        }
                    }
                }
                let first = match incoming_work.recv_timeout(POLL) {
                    Ok(first) => first,
                    Err(mpsc::RecvTimeoutError::Timeout) => continue,
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                };
                let asked: Vec<Work> = std::iter::once(first)
                    .chain(incoming_work.try_iter())
                    .collect();
                let Some(notifications) = &opened else {
                    fail(
                        &mut failure,
                        "not dismissing: Windows' notifications cannot be read".to_owned(),
                    );
                    continue;
                };
                for id in asked.iter().flat_map(|asked| targets(asked, &mirror)) {
                    if let Err(error) = notifications.dismiss(id) {
                        fail(&mut failure, format!("dismissing a notification: {error}"));
                    }
                }
            }
        })?;
    Ok(work)
}

/// The keys `asked` dismisses among those `mirror` holds: the one lyr's id
/// names, or every one.
fn targets<K: Copy + Eq + Hash>(asked: &Work, mirror: &Mirror<K>) -> Vec<K> {
    match *asked {
        Work::Dismiss(id) => mirror.key(id).copied().into_iter().collect(),
        Work::Clear => mirror.keys().copied().collect(),
    }
}

/// Logs `what` unless it is the `last` failure logged, and returns whether
/// it was logged.
fn fail(last: &mut Option<String>, what: String) -> bool {
    if last.as_ref() == Some(&what) {
        return false;
    }
    tracing::warn!("notifications: {what}");
    *last = Some(what);
    true
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use lyr_core::notifications::{Incoming, Mirror, Store};

    use super::{Work, fail, targets};

    #[test]
    fn a_dismissal_names_its_own_key_and_clear_all_of_them() {
        let mut store = Store::default();
        let mut mirror = Mirror::default();
        let now = Instant::now();
        mirror.apply(
            &mut store,
            vec![
                (7_u8, Incoming::default(), now),
                (9, Incoming::default(), now),
            ],
        );
        let id = |key| {
            (1..=2)
                .find(|&id| mirror.key(id) == Some(&key))
                .expect("listed")
        };
        assert_eq!(targets(&Work::Dismiss(id(9)), &mirror), [9]);
        assert_eq!(
            targets(&Work::Dismiss(3), &mirror),
            Vec::<u8>::new(),
            "an id lyr did not hand out dismisses nothing"
        );
        let mut all = targets(&Work::Clear, &mirror);
        all.sort_unstable();
        assert_eq!(all, [7, 9]);
    }

    #[test]
    fn a_failure_is_logged_once_until_another_comes() {
        let mut last = None;
        assert!(fail(&mut last, "a".to_owned()));
        assert!(!fail(&mut last, "a".to_owned()), "the same again");
        assert!(fail(&mut last, "b".to_owned()));
        assert!(fail(&mut last, "a".to_owned()), "a after b is new");
    }
}
