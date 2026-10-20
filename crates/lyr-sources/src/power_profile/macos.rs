//! The power-profile module's source on macOS: Low Power Mode, read from
//! `NSProcessInfo` and read again on its
//! `NSProcessInfoPowerStateDidChangeNotification`. Read-only: setting the
//! mode needs `pmset` as root (plan 07).

use std::ptr::NonNull;

use block2::RcBlock;
use lyr_core::power_profile::Profile;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, NSObjectProtocol, ProtocolObject};
use objc2_foundation::{
    NSNotification, NSNotificationCenter, NSProcessInfo,
    NSProcessInfoPowerStateDidChangeNotification,
};

use super::lowpower::profile;

/// The profile now.
#[must_use]
pub fn read() -> Profile {
    profile(NSProcessInfo::processInfo().isLowPowerModeEnabled())
}

/// The notification observer, calling its callback while it lives.
pub struct Watch {
    observer: Retained<ProtocolObject<dyn NSObjectProtocol>>,
}

impl std::fmt::Debug for Watch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Watch").finish_non_exhaustive()
    }
}

impl Watch {
    /// Calls `on_change` with the profile each time Low Power Mode turns on
    /// or off, on the thread that posts the notification, until dropped.
    pub fn new(on_change: impl Fn(Profile) + Send + Sync + 'static) -> Self {
        let block = RcBlock::new(move |_: NonNull<NSNotification>| on_change(read()));
        // SAFETY: the name is the notification's own static; no object and
        // no queue, so the block runs where the notification is posted, and
        // it is `Send + Sync` for that.
        let observer = unsafe {
            NSNotificationCenter::defaultCenter().addObserverForName_object_queue_usingBlock(
                Some(NSProcessInfoPowerStateDidChangeNotification),
                None,
                None,
                &block,
            )
        };
        Self { observer }
    }
}

impl Drop for Watch {
    fn drop(&mut self) {
        let observer: &AnyObject = (*self.observer).as_ref();
        // SAFETY: the observer is the one `addObserverForName…` returned.
        unsafe { NSNotificationCenter::defaultCenter().removeObserver(observer) };
    }
}

#[cfg(test)]
mod tests {
    use super::{Watch, read};

    #[test]
    fn the_mode_reads_and_a_watch_starts_and_stops() {
        let profile = read();
        assert!(["power-saver", "balanced"].contains(&profile.name.as_str()));
        drop(Watch::new(|_| {}));
    }
}
