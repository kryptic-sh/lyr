//! Windows: Windows' own notifications, copied into a store for the clock
//! panel to list, and the panel's Clear All and close buttons carried out on
//! them.
//!
//! Windows tells an unpackaged program nothing when its notifications
//! change, so they are read again every [`POLL`], and at once after a
//! dismissal. A store lyr may not read yet, which Windows' notification-access
//! privacy setting decides, is tried again at the same rate, so the panel
//! lists them once access is granted.

use std::sync::{Arc, Mutex, PoisonError, mpsc};
use std::time::{Duration, Instant, SystemTime};

use crcbl_shell::ShellWaker;
use lyr_core::notifications::{Mirror, Store};
use lyr_sources::notifications::windows::{Notifications, incoming};

use super::Update;

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
                        Err(error) => fail(
                            &mut failure,
                            format!("Windows' notifications cannot be read: {error}"),
                        ),
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
                                if !deliver(&sender, waker.as_ref(), &store) {
                                    break;
                                }
                                handed = true;
                            }
                        }
                        // The last snapshot stays listed rather than none.
                        Err(error) => fail(
                            &mut failure,
                            format!("reading Windows' notifications: {error}"),
                        ),
                    }
                }
                let first = match incoming_work.recv_timeout(POLL) {
                    Ok(first) => first,
                    Err(mpsc::RecvTimeoutError::Timeout) => continue,
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                };
                let Some(notifications) = &opened else {
                    continue;
                };
                for asked in std::iter::once(first).chain(incoming_work.try_iter()) {
                    let ids = match asked {
                        Work::Dismiss(id) => mirror.key(id).copied().into_iter().collect(),
                        Work::Clear => mirror.keys().copied().collect::<Vec<_>>(),
                    };
                    for id in ids {
                        if let Err(error) = notifications.dismiss(id) {
                            tracing::warn!("notifications: dismissing one: {error}");
                        }
                    }
                }
            }
        })?;
    Ok(work)
}

/// Logs `what` unless it is the `last` failure logged.
fn fail(last: &mut Option<String>, what: String) {
    if last.as_ref() != Some(&what) {
        tracing::warn!("notifications: {what}");
        *last = Some(what);
    }
}

/// Hands `store` to the main loop and wakes it. `false` once the main loop
/// has gone.
fn deliver(
    sender: &mpsc::Sender<Update>,
    waker: Option<&ShellWaker>,
    store: &Arc<Mutex<Store>>,
) -> bool {
    let delivered = sender
        .send(Update::Notifications(Arc::clone(store)))
        .is_ok();
    if let Some(waker) = waker {
        waker.wake();
    }
    delivered
}
