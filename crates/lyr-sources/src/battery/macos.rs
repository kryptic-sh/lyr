//! The battery source on macOS: IOKit's power sources
//! (`IOPSCopyPowerSourcesInfo`), the first internal battery's description
//! read again each time macOS reports a power source change, which
//! `IOPSNotificationCreateRunLoopSource` delivers to a run loop on a thread
//! of its own, so nothing is polled.

use std::ffi::{CStr, c_void};
use std::io;

use objc2_core_foundation::{
    CFArray, CFBoolean, CFDictionary, CFNumber, CFRetained, CFRunLoopSource, CFString, CFType,
    kCFRunLoopDefaultMode,
};
use objc2_io_kit::{
    IOPSCopyPowerSourcesInfo, IOPSCopyPowerSourcesList, IOPSGetPowerSourceDescription,
    IOPSNotificationCreateRunLoopSource, kIOPSACPowerValue, kIOPSCurrentCapacityKey,
    kIOPSInternalBatteryType, kIOPSIsChargedKey, kIOPSIsChargingKey, kIOPSIsPresentKey,
    kIOPSMaxCapacityKey, kIOPSPowerSourceStateKey, kIOPSTimeToEmptyKey, kIOPSTimeToFullChargeKey,
    kIOPSTypeKey,
};

use super::iops::PowerSource;
use crate::macos_runloop::RunLoopThread;

/// A power source's description, keyed by `IOPSKeys.h`'s strings.
type Description = CFDictionary<CFString, CFType>;

/// The first internal battery's description, or `None` when the Mac has no
/// battery (or macOS lists none).
#[must_use]
pub fn power_source() -> Option<PowerSource> {
    let info = IOPSCopyPowerSourcesInfo()?;
    // SAFETY: `info` is the blob `IOPSCopyPowerSourcesInfo` returned, which
    // is what the list and each description are read from.
    let list = unsafe { IOPSCopyPowerSourcesList(Some(&*info)) }?;
    // SAFETY: the list holds power source handles, each a CF object.
    let list: &CFArray<CFType> = unsafe { list.cast_unchecked() };
    list.iter().find_map(|source| {
        // SAFETY: `source` came from `info`'s own list.
        let description = unsafe { IOPSGetPowerSourceDescription(Some(&*info), Some(&*source)) }?;
        // SAFETY: a description is a dictionary keyed by CFStrings, whose
        // values are CF objects; each read checks the value's type.
        let description: &Description = unsafe { description.cast_unchecked() };
        (text(description, kIOPSTypeKey)? == utf8(kIOPSInternalBatteryType)?)
            .then(|| read(description))
    })
}

/// The fields lyr reads from one description; a missing one reads as
/// absent, zero or false, which `PowerSource::reading` turns into no
/// battery or no estimate.
fn read(description: &Description) -> PowerSource {
    let number = |key| number(description, key).unwrap_or(0);
    let flag = |key| flag(description, key).unwrap_or(false);
    PowerSource {
        present: flag(kIOPSIsPresentKey),
        current_capacity: number(kIOPSCurrentCapacityKey),
        max_capacity: number(kIOPSMaxCapacityKey),
        charging: flag(kIOPSIsChargingKey),
        charged: flag(kIOPSIsChargedKey),
        on_ac: text(description, kIOPSPowerSourceStateKey).as_deref() == utf8(kIOPSACPowerValue),
        time_to_empty: number(kIOPSTimeToEmptyKey),
        time_to_full: number(kIOPSTimeToFullChargeKey),
    }
}

/// A key or value from `IOPSKeys.h` as text; every one is ASCII.
fn utf8(key: &CStr) -> Option<&str> {
    key.to_str().ok()
}

fn value(description: &Description, key: &CStr) -> Option<CFRetained<CFType>> {
    description.get(&CFString::from_str(utf8(key)?))
}

fn number(description: &Description, key: &CStr) -> Option<i64> {
    value(description, key)?
        .downcast::<CFNumber>()
        .ok()?
        .as_i64()
}

fn flag(description: &Description, key: &CStr) -> Option<bool> {
    Some(
        value(description, key)?
            .downcast::<CFBoolean>()
            .ok()?
            .as_bool(),
    )
}

fn text(description: &Description, key: &CStr) -> Option<String> {
    Some(
        value(description, key)?
            .downcast::<CFString>()
            .ok()?
            .to_string(),
    )
}

/// What a change calls.
type OnChange = Box<dyn Fn() + Send>;

/// Calls the [`OnChange`] its run loop source was made with.
unsafe extern "C-unwind" fn changed(context: *mut c_void) {
    // SAFETY: `context` is a `Registration`'s `OnChange`, which it frees only
    // after invalidating the source, the only caller.
    let on_change = unsafe { &*context.cast::<OnChange>() };
    on_change();
}

/// The notification source on the watch's run loop and the callback it
/// calls, freed in that order.
struct Registration {
    source: CFRetained<CFRunLoopSource>,
    context: *mut OnChange,
}

impl Drop for Registration {
    fn drop(&mut self) {
        self.source.invalidate();
        // SAFETY: the box came from `Box::into_raw`, and the source that
        // called into it is invalidated, so nothing reads it again.
        drop(unsafe { Box::from_raw(self.context) });
    }
}

/// The thread running the power source notification, calling its callback
/// while it lives.
#[derive(Debug)]
pub struct Watch {
    /// Kept for its drop, which stops the thread.
    _thread: RunLoopThread,
}

impl Watch {
    /// Calls `on_change` whenever macOS reports a power source change, on a
    /// thread of the watch's own, until dropped.
    ///
    /// # Errors
    ///
    /// When the thread cannot start or IOKit makes no notification source.
    pub fn new(on_change: impl Fn() + Send + 'static) -> io::Result<Self> {
        RunLoopThread::spawn("lyr battery", move |run_loop| {
            let context: *mut OnChange = Box::into_raw(Box::new(Box::new(on_change)));
            // SAFETY: `changed` reads `context` as the `OnChange` it is, which
            // the registration frees only after invalidating the source.
            let source =
                unsafe { IOPSNotificationCreateRunLoopSource(Some(changed), context.cast()) };
            let Some(source) = source else {
                // SAFETY: nothing else holds the box.
                drop(unsafe { Box::from_raw(context) });
                return Err(io::Error::other("IOKit made no power source notification"));
            };
            // SAFETY: the default mode is a static CF string.
            run_loop.add_source(Some(&*source), unsafe { kCFRunLoopDefaultMode });
            Ok(Registration { source, context })
        })
        .map(|thread| Self { _thread: thread })
    }
}

#[cfg(test)]
mod tests {
    use super::{Watch, power_source};

    #[test]
    fn the_battery_reads_and_a_watch_starts_and_stops() {
        // CI's Macs have no battery; a laptop has one. Either reads.
        if let Some(source) = power_source() {
            assert!(source.max_capacity >= 0);
        }
        let watch = Watch::new(|| {}).expect("a notification source");
        drop(watch);
    }
}
