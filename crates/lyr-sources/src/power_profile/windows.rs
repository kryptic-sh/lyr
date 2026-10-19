//! The power-profile source on Windows: the effective power mode, which
//! `PowerRegisterForEffectivePowerModeNotifications` (Windows 10 1809 and
//! later) reports to a callback when registered and at each change, so
//! nothing polls. Setting the mode is undocumented (plan 16's question 13)
//! and not built.

use std::ffi::c_void;
use std::io;
use std::mem::ManuallyDrop;
use std::ptr;

use windows_sys::Win32::System::Power::{
    EFFECTIVE_POWER_MODE, EFFECTIVE_POWER_MODE_V2, PowerRegisterForEffectivePowerModeNotifications,
    PowerUnregisterFromEffectivePowerModeNotifications,
};

/// What a change calls, with the mode.
type OnChange = Box<dyn Fn(i32) + Send + Sync>;

/// Calls the [`OnChange`] its registration was given.
unsafe extern "system" fn changed(mode: EFFECTIVE_POWER_MODE, context: *const c_void) {
    // SAFETY: the context a `Watch` registered and keeps until it
    // unregisters.
    (unsafe { &*context.cast::<OnChange>() })(mode);
}

/// The registration calling `on_change` while it lives.
pub struct Watch {
    handle: *mut c_void,
    /// The callback's context, boxed twice so its address stays put; freed
    /// only once the registration is gone ([`Drop`]).
    on_change: ManuallyDrop<Box<OnChange>>,
}

impl Watch {
    /// Calls `on_change`, on a system thread, with the effective power mode
    /// now and at each change, until the watch is dropped.
    ///
    /// # Errors
    ///
    /// When Windows refuses the registration, as one before 1809 would.
    pub fn new(on_change: impl Fn(i32) + Send + Sync + 'static) -> io::Result<Self> {
        let on_change: Box<OnChange> = Box::new(Box::new(on_change));
        let context = ptr::from_ref::<OnChange>(&on_change).cast::<c_void>();
        let mut handle: *mut c_void = ptr::null_mut();
        // SAFETY: the callback matches the type, the context lives in the
        // watch until it unregisters, and `handle` is a local the call writes.
        let result = unsafe {
            PowerRegisterForEffectivePowerModeNotifications(
                EFFECTIVE_POWER_MODE_V2,
                Some(changed),
                context,
                &raw mut handle,
            )
        };
        if result < 0 {
            return Err(io::Error::from_raw_os_error(result));
        }
        Ok(Self {
            handle,
            on_change: ManuallyDrop::new(on_change),
        })
    }
}

impl std::fmt::Debug for Watch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Watch").finish_non_exhaustive()
    }
}

impl Drop for Watch {
    fn drop(&mut self) {
        // SAFETY: the handle came from the registration and is unregistered
        // once. A failure leaves a registration that can still call back, so
        // its context is leaked rather than freed under it.
        if unsafe { PowerUnregisterFromEffectivePowerModeNotifications(self.handle) } >= 0 {
            // SAFETY: nothing holds the context any more, dropped once.
            unsafe { ManuallyDrop::drop(&mut self.on_change) };
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::time::Duration;

    use super::Watch;
    use crate::power_profile::win32::profile;

    #[test]
    fn the_mode_comes_when_registered() {
        let (sender, modes) = mpsc::channel();
        let watch = Watch::new(move |mode| {
            let _ = sender.send(mode);
        })
        .expect("registered");
        let mode = modes
            .recv_timeout(Duration::from_secs(5))
            .expect("the mode within 5 s of registering");
        assert!(profile(mode).is_some(), "a mode the table knows: {mode}");
        drop(watch);
    }
}
