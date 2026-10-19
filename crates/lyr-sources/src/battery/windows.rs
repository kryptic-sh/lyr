//! The battery source on Windows: `GetSystemPowerStatus`, read again each
//! time the power source or the remaining charge changes, which
//! `PowerSettingRegisterNotification` reports to a callback, so nothing is
//! polled and no window is needed.

use std::ffi::c_void;
use std::io;
use std::mem::ManuallyDrop;
use std::ptr;

use windows_sys::Win32::Foundation::{ERROR_SUCCESS, HANDLE};
use windows_sys::Win32::System::Power::{
    DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS, GetSystemPowerStatus, PowerSettingRegisterNotification,
    PowerSettingUnregisterNotification, SYSTEM_POWER_STATUS,
};
use windows_sys::Win32::System::SystemServices::{
    GUID_ACDC_POWER_SOURCE, GUID_BATTERY_PERCENTAGE_REMAINING,
};
use windows_sys::Win32::UI::WindowsAndMessaging::DEVICE_NOTIFY_CALLBACK;
use windows_sys::core::GUID;

use super::win32::PowerStatus;

/// The power status now.
///
/// # Errors
///
/// When Windows cannot report it.
pub fn status() -> io::Result<PowerStatus> {
    let mut status = SYSTEM_POWER_STATUS::default();
    // SAFETY: `status` is a local the call fills.
    if unsafe { GetSystemPowerStatus(&raw mut status) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(PowerStatus {
        ac_line_status: status.ACLineStatus,
        battery_flag: status.BatteryFlag,
        battery_life_percent: status.BatteryLifePercent,
        battery_life_time: status.BatteryLifeTime,
    })
}

/// What a change calls.
type OnChange = Box<dyn Fn() + Send + Sync>;

/// Calls the [`OnChange`] its registration was given.
unsafe extern "system" fn changed(
    context: *const c_void,
    _kind: u32,
    _setting: *const c_void,
) -> u32 {
    // SAFETY: `context` is the `OnChange` a `Watch` keeps alive until it
    // unregisters, which stops the calls first.
    let on_change = unsafe { &*context.cast::<OnChange>() };
    on_change();
    ERROR_SUCCESS
}

/// The settings whose changes a reading follows: on mains power or not,
/// and how much charge is left.
const SETTINGS: [GUID; 2] = [GUID_ACDC_POWER_SOURCE, GUID_BATTERY_PERCENTAGE_REMAINING];

/// The registrations calling `on_change` while it lives.
pub struct Watch {
    handles: Vec<isize>,
    /// What each registration was handed: the callback's context, boxed
    /// twice so the address it holds stays put, and the parameters naming
    /// it. Freed only once every registration is gone ([`Drop`]).
    on_change: ManuallyDrop<Box<OnChange>>,
    parameters: ManuallyDrop<Box<DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS>>,
}

impl Watch {
    /// Calls `on_change` on a system thread each time the power source or the
    /// remaining charge changes, until the watch is dropped.
    ///
    /// # Errors
    ///
    /// When Windows refuses a registration; those made before it are undone.
    pub fn new(on_change: impl Fn() + Send + Sync + 'static) -> io::Result<Self> {
        let on_change: Box<OnChange> = Box::new(Box::new(on_change));
        let context = ptr::from_ref::<OnChange>(&on_change)
            .cast_mut()
            .cast::<c_void>();
        let mut watch = Self {
            handles: Vec::new(),
            on_change: ManuallyDrop::new(on_change),
            parameters: ManuallyDrop::new(Box::new(DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS {
                Callback: Some(changed),
                Context: context,
            })),
        };
        let recipient = ptr::from_mut::<DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS>(&mut watch.parameters)
            .cast::<c_void>() as HANDLE;
        for setting in &SETTINGS {
            let mut handle: *mut c_void = ptr::null_mut();
            // SAFETY: the GUID is a constant, the parameters live in a box the
            // watch keeps until every registration is gone, and `handle` is a
            // local the call writes.
            let error = unsafe {
                PowerSettingRegisterNotification(
                    setting,
                    DEVICE_NOTIFY_CALLBACK,
                    recipient,
                    &raw mut handle,
                )
            };
            if error != ERROR_SUCCESS {
                // Dropping `watch` undoes the registrations made so far.
                return Err(io::Error::from_raw_os_error(error as i32));
            }
            watch.handles.push(handle as isize);
        }
        Ok(watch)
    }
}

impl std::fmt::Debug for Watch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Watch")
            .field("handles", &self.handles)
            .finish_non_exhaustive()
    }
}

impl Drop for Watch {
    fn drop(&mut self) {
        let mut all_gone = true;
        for &handle in &self.handles {
            // SAFETY: each handle came from `PowerSettingRegisterNotification`
            // and is unregistered once.
            all_gone &= unsafe { PowerSettingUnregisterNotification(handle) } == ERROR_SUCCESS;
        }
        // A registration that would not go can still call back, so what it
        // was handed is leaked rather than freed under it.
        if all_gone {
            // SAFETY: no registration holds them any more, and they are
            // dropped once.
            unsafe {
                ManuallyDrop::drop(&mut self.on_change);
                ManuallyDrop::drop(&mut self.parameters);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Watch, status};

    #[test]
    fn the_status_reads_and_a_watch_registers_both_settings() {
        // Whatever this machine's battery is, the call answers.
        status().expect("a power status");
        let watch = Watch::new(|| {}).expect("registered");
        assert_eq!(watch.handles.len(), 2);
        assert!(watch.handles.iter().all(|&handle| handle != 0));
    }
}
