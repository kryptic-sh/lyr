//! The battery source on macOS: IOKit's power sources
//! (`IOPSCopyPowerSourcesInfo`), the first internal battery's description
//! read again each time macOS reports a power source change, which
//! `IOPSNotificationCreateRunLoopSource` delivers to a run loop on a thread
//! of its own, so nothing is polled.

use std::ffi::{CStr, c_void};
use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread::JoinHandle;

use objc2_core_foundation::{
    CFArray, CFBoolean, CFDictionary, CFNumber, CFRetained, CFRunLoop, CFString, CFType,
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
    // SAFETY: `context` is the `OnChange` the watch's thread owns, which it
    // frees only after its run loop, the only caller, has stopped.
    let on_change = unsafe { &*context.cast::<OnChange>() };
    on_change();
}

/// The watch thread's run loop, as the one handle another thread may hold.
struct RunLoop(CFRetained<CFRunLoop>);

// SAFETY: the handle is used from another thread only through `stop`, and
// Core Foundation's run loop functions are thread-safe (Apple's Threading
// Programming Guide, "Run Loop Objects"); `CFRunLoopStop` and
// `CFRunLoopWakeUp` exist to be called from other threads.
unsafe impl Send for RunLoop {}

impl RunLoop {
    /// Stops the loop if it is running, and wakes it so it notices.
    fn stop(&self) {
        self.0.stop();
        self.0.wake_up();
    }
}

/// The thread running the power source notification, calling its callback
/// while it lives.
pub struct Watch {
    run_loop: RunLoop,
    /// Set before stopping, so a loop stopped before it began running does
    /// not run again.
    stopping: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl std::fmt::Debug for Watch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Watch").finish_non_exhaustive()
    }
}

impl Watch {
    /// Calls `on_change` whenever macOS reports a power source change, on a
    /// thread of the watch's own, until dropped.
    ///
    /// # Errors
    ///
    /// When the thread cannot start or IOKit makes no notification source.
    pub fn new(on_change: impl Fn() + Send + 'static) -> io::Result<Self> {
        let (started, start) = mpsc::channel();
        let stopping = Arc::new(AtomicBool::new(false));
        let stop = stopping.clone();
        let thread = std::thread::Builder::new()
            .name("lyr battery".to_owned())
            .spawn(move || {
                let context: *mut OnChange = Box::into_raw(Box::new(Box::new(on_change)));
                // SAFETY: `changed` reads `context` as the `OnChange` it is,
                // and the box is freed below, after the loop has stopped.
                let source =
                    unsafe { IOPSNotificationCreateRunLoopSource(Some(changed), context.cast()) };
                let run_loop = CFRunLoop::current();
                let (Some(source), Some(run_loop)) = (source, run_loop) else {
                    // SAFETY: nothing else holds the box.
                    drop(unsafe { Box::from_raw(context) });
                    let _gone = started.send(Err(io::Error::other(
                        "IOKit made no power source notification",
                    )));
                    return;
                };
                // SAFETY: the default mode is a static CF string.
                run_loop.add_source(Some(&*source), unsafe { kCFRunLoopDefaultMode });
                let _gone = started.send(Ok(RunLoop(run_loop)));
                while !stop.load(Ordering::Acquire) {
                    CFRunLoop::run();
                }
                drop(source);
                // SAFETY: the loop, the only caller of `changed`, has stopped
                // and its source is gone.
                drop(unsafe { Box::from_raw(context) });
            })?;
        let run_loop = start
            .recv()
            .map_err(|_| io::Error::other("the battery thread stopped"))??;
        Ok(Self {
            run_loop,
            stopping,
            thread: Some(thread),
        })
    }
}

impl Drop for Watch {
    fn drop(&mut self) {
        self.stopping.store(true, Ordering::Release);
        let Some(thread) = self.thread.take() else {
            return;
        };
        // A stop that lands before the loop starts running is lost, so it is
        // sent until the thread has seen the flag and ended.
        while !thread.is_finished() {
            self.run_loop.stop();
            std::thread::yield_now();
        }
        let _joined = thread.join();
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
