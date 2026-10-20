//! The volume and mic modules' source on macOS: CoreAudio's default output
//! and input devices, their volume and mute, read with
//! `AudioObjectGetPropertyData` and changed with `AudioObjectSetPropertyData`.
//! [`Watch`] registers property listeners on the system object (the default
//! devices) and on the default devices themselves (volume and mute), which
//! CoreAudio calls on its own notification thread, so nothing is polled.
//!
//! A device's volume is its main element's when it has one, else its first
//! two channels', since many devices, the built-in output among them, have
//! only per-channel volumes.

use std::ffi::c_void;
use std::io;
use std::mem::size_of;
use std::ptr::NonNull;

use lyr_core::audio::{Sink, Source};
use objc2_core_audio::{
    AudioObjectAddPropertyListener, AudioObjectGetPropertyData, AudioObjectHasProperty,
    AudioObjectID, AudioObjectPropertyAddress, AudioObjectPropertyElement,
    AudioObjectPropertyScope, AudioObjectPropertySelector, AudioObjectRemovePropertyListener,
    AudioObjectSetPropertyData, kAudioDevicePropertyDeviceUID, kAudioDevicePropertyMute,
    kAudioDevicePropertyTransportType, kAudioDevicePropertyVolumeScalar,
    kAudioHardwarePropertyDefaultInputDevice, kAudioHardwarePropertyDefaultOutputDevice,
    kAudioObjectPropertyElementMain, kAudioObjectPropertyElementWildcard, kAudioObjectPropertyName,
    kAudioObjectPropertyScopeGlobal, kAudioObjectPropertyScopeInput,
    kAudioObjectPropertyScopeOutput, kAudioObjectSystemObject, kAudioObjectUnknown,
};
use objc2_core_foundation::{CFRetained, CFString};

use super::coreaudio::{bluetooth, mean, port};
use super::level::{percent, stepped};

/// Which way sound goes through a default device.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flow {
    /// The default output, the volume module's.
    Output,
    /// The default input, the mic module's.
    Input,
}

impl Flow {
    const fn scope(self) -> AudioObjectPropertyScope {
        match self {
            Self::Output => kAudioObjectPropertyScopeOutput,
            Self::Input => kAudioObjectPropertyScopeInput,
        }
    }

    const fn default_device(self) -> AudioObjectPropertySelector {
        match self {
            Self::Output => kAudioHardwarePropertyDefaultOutputDevice,
            Self::Input => kAudioHardwarePropertyDefaultInputDevice,
        }
    }
}

/// The channels a device without a main volume is read and set through.
const CHANNELS: [AudioObjectPropertyElement; 2] = [1, 2];

/// The system object, which owns the default devices.
const SYSTEM: AudioObjectID = kAudioObjectSystemObject.cast_unsigned();

fn address(
    selector: AudioObjectPropertySelector,
    scope: AudioObjectPropertyScope,
    element: AudioObjectPropertyElement,
) -> AudioObjectPropertyAddress {
    AudioObjectPropertyAddress {
        mSelector: selector,
        mScope: scope,
        mElement: element,
    }
}

/// An `OSStatus` as an error naming what failed.
fn checked(status: i32, what: &str) -> io::Result<()> {
    if status == 0 {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "CoreAudio: {what}: OSStatus {status}"
        )))
    }
}

fn has(object: AudioObjectID, at: AudioObjectPropertyAddress) -> bool {
    // SAFETY: `at` is a live address for the call.
    unsafe { AudioObjectHasProperty(object, NonNull::from(&at)) }
}

/// Reads a plain value (a number or an object id) of type `T`.
///
/// # Safety
///
/// `T` must be the type CoreAudio writes for `at`'s selector.
unsafe fn get<T: Copy + Default>(
    object: AudioObjectID,
    at: AudioObjectPropertyAddress,
    what: &str,
) -> io::Result<T> {
    let mut value = T::default();
    let mut size = u32::try_from(size_of::<T>()).expect("a property value fits in 32 bits");
    // SAFETY: `value` is a live `T` of `size` bytes, which the caller
    // vouches is the property's type; `at` and `size` are live.
    let status = unsafe {
        AudioObjectGetPropertyData(
            object,
            NonNull::from(&at),
            0,
            std::ptr::null(),
            NonNull::from(&mut size),
            NonNull::from(&mut value).cast(),
        )
    };
    checked(status, what)?;
    Ok(value)
}

/// Writes a plain value of type `T`.
///
/// # Safety
///
/// `T` must be the type CoreAudio reads for `at`'s selector.
unsafe fn set<T: Copy>(
    object: AudioObjectID,
    at: AudioObjectPropertyAddress,
    mut value: T,
    what: &str,
) -> io::Result<()> {
    let size = u32::try_from(size_of::<T>()).expect("a property value fits in 32 bits");
    // SAFETY: `value` is a live `T` of `size` bytes, which the caller
    // vouches is the property's type.
    let status = unsafe {
        AudioObjectSetPropertyData(
            object,
            NonNull::from(&at),
            0,
            std::ptr::null(),
            size,
            NonNull::from(&mut value).cast(),
        )
    };
    checked(status, what)
}

/// A device's string property, such as its name; CoreAudio hands back a
/// string the caller owns.
fn text(device: AudioObjectID, selector: AudioObjectPropertySelector) -> Option<String> {
    let at = address(
        selector,
        kAudioObjectPropertyScopeGlobal,
        kAudioObjectPropertyElementMain,
    );
    // SAFETY: the name and UID properties are a `CFStringRef`, which an
    // `Option<NonNull<_>>` has the layout of.
    let string: Option<NonNull<CFString>> = unsafe { get(device, at, "a device's name") }.ok()?;
    // SAFETY: CoreAudio returns the string retained for the caller, who
    // releases it, as `CFRetained` does when dropped.
    let string = unsafe { CFRetained::from_raw(string?) };
    Some(string.to_string())
}

/// The default device of `flow`, or `None` when there is none.
fn default_device(flow: Flow) -> io::Result<Option<AudioObjectID>> {
    let at = address(
        flow.default_device(),
        kAudioObjectPropertyScopeGlobal,
        kAudioObjectPropertyElementMain,
    );
    // SAFETY: the default device properties are an `AudioObjectID`.
    let device: AudioObjectID = unsafe { get(SYSTEM, at, "the default device") }?;
    Ok((device != kAudioObjectUnknown).then_some(device))
}

/// The elements a device's volume is read and set through: its main
/// element, else whichever of its first two channels have a volume.
fn volume_elements(device: AudioObjectID, flow: Flow) -> Vec<AudioObjectPropertyElement> {
    let at = |element| address(kAudioDevicePropertyVolumeScalar, flow.scope(), element);
    if has(device, at(kAudioObjectPropertyElementMain)) {
        return vec![kAudioObjectPropertyElementMain];
    }
    CHANNELS
        .into_iter()
        .filter(|&element| has(device, at(element)))
        .collect()
}

/// A device's volume, 0 – 1, or `None` with no volume control.
fn volume(device: AudioObjectID, flow: Flow) -> io::Result<Option<f32>> {
    let channels = volume_elements(device, flow)
        .into_iter()
        .map(|element| {
            let at = address(kAudioDevicePropertyVolumeScalar, flow.scope(), element);
            // SAFETY: the volume scalar is a `Float32`.
            unsafe { get::<f32>(device, at, "the volume") }
        })
        .collect::<io::Result<Vec<_>>>()?;
    Ok(mean(&channels))
}

fn mute_address(flow: Flow) -> AudioObjectPropertyAddress {
    address(
        kAudioDevicePropertyMute,
        flow.scope(),
        kAudioObjectPropertyElementMain,
    )
}

/// Whether a device is muted; one with no mute control is not.
fn muted(device: AudioObjectID, flow: Flow) -> io::Result<bool> {
    if !has(device, mute_address(flow)) {
        return Ok(false);
    }
    // SAFETY: mute is a `UInt32`.
    let muted: u32 = unsafe { get(device, mute_address(flow), "mute") }?;
    Ok(muted != 0)
}

fn transport(device: AudioObjectID) -> u32 {
    let at = address(
        kAudioDevicePropertyTransportType,
        kAudioObjectPropertyScopeGlobal,
        kAudioObjectPropertyElementMain,
    );
    // SAFETY: the transport type is a `UInt32`.
    unsafe { get(device, at, "the transport type") }.unwrap_or_default()
}

/// The default output, as the volume module reads it.
///
/// # Errors
///
/// When CoreAudio does not answer.
pub fn sink() -> io::Result<Option<Sink>> {
    let Some(device) = default_device(Flow::Output)? else {
        return Ok(None);
    };
    let transport = transport(device);
    Ok(Some(Sink {
        volume: volume(device, Flow::Output)?.map_or(0, percent),
        muted: muted(device, Flow::Output)?,
        name: text(device, kAudioDevicePropertyDeviceUID).unwrap_or_default(),
        source_name: default_device(Flow::Input)?
            .and_then(|input| text(input, kAudioDevicePropertyDeviceUID))
            .unwrap_or_default(),
        port: port(transport).to_owned(),
        description: text(device, kAudioObjectPropertyName).unwrap_or_default(),
        bluetooth: bluetooth(transport),
    }))
}

/// The default input, as the mic module reads it.
///
/// # Errors
///
/// When CoreAudio does not answer.
pub fn source() -> io::Result<Option<Source>> {
    let Some(device) = default_device(Flow::Input)? else {
        return Ok(None);
    };
    Ok(Some(Source {
        volume: volume(device, Flow::Input)?.map_or(0, percent),
        muted: muted(device, Flow::Input)?,
        description: text(device, kAudioObjectPropertyName).unwrap_or_default(),
    }))
}

/// The default device of `flow`, or an error saying there is none.
fn required(flow: Flow) -> io::Result<AudioObjectID> {
    default_device(flow)?.ok_or_else(|| io::Error::other(format!("no default {flow:?} device")))
}

/// Mutes the default device of `flow`, or unmutes it.
///
/// # Errors
///
/// When there is no such device, it has no mute control, or it refuses.
pub fn toggle_mute(flow: Flow) -> io::Result<()> {
    let device = required(flow)?;
    if !has(device, mute_address(flow)) {
        return Err(io::Error::other(format!(
            "the default {flow:?} device has no mute"
        )));
    }
    let muted = muted(device, flow)?;
    // SAFETY: mute is a `UInt32`.
    unsafe { set(device, mute_address(flow), u32::from(!muted), "mute") }
}

/// Moves the volume of the default device of `flow` by `delta` percent,
/// held to 0 – 100: CoreAudio's scalar volume does not boost.
///
/// # Errors
///
/// When there is no such device, it has no volume control, or it refuses.
pub fn step(flow: Flow, delta: i32) -> io::Result<()> {
    let device = required(flow)?;
    let now = volume(device, flow)?
        .ok_or_else(|| io::Error::other(format!("the default {flow:?} device has no volume")))?;
    set_volume(device, flow, stepped(now, delta))
}

/// Sets every element the volume is read through, so the channels stay
/// level.
fn set_volume(device: AudioObjectID, flow: Flow, value: f32) -> io::Result<()> {
    let elements = volume_elements(device, flow);
    if elements.is_empty() {
        return Err(io::Error::other(format!(
            "the default {flow:?} device has no volume"
        )));
    }
    for element in elements {
        let at = address(kAudioDevicePropertyVolumeScalar, flow.scope(), element);
        // SAFETY: the volume scalar is a `Float32`.
        unsafe { set(device, at, value, "the volume") }?;
    }
    Ok(())
}

/// What a change calls, from CoreAudio's notification thread, in a sized
/// box so a thin pointer to it can be the listeners' client data.
struct Callback(Box<dyn Fn() + Send + Sync>);

/// Calls the [`Callback`] its listener was registered with.
unsafe extern "C-unwind" fn changed(
    _object: AudioObjectID,
    _count: u32,
    _addresses: NonNull<AudioObjectPropertyAddress>,
    context: *mut c_void,
) -> i32 {
    // SAFETY: `context` is a `Watch`'s leaked `&'static Callback`, which is
    // never freed, so a call still in flight after a listener is removed
    // reads live memory.
    let callback = unsafe { &*context.cast::<Callback>() };
    (callback.0)();
    0
}

/// The properties a watch listens to on the system object: the defaults.
fn system_addresses() -> [AudioObjectPropertyAddress; 2] {
    [Flow::Output, Flow::Input].map(|flow| {
        address(
            flow.default_device(),
            kAudioObjectPropertyScopeGlobal,
            kAudioObjectPropertyElementMain,
        )
    })
}

/// The properties a watch listens to on a default device: its volume, on
/// any element, and its mute.
fn device_addresses(flow: Flow) -> [AudioObjectPropertyAddress; 2] {
    [
        address(
            kAudioDevicePropertyVolumeScalar,
            flow.scope(),
            kAudioObjectPropertyElementWildcard,
        ),
        mute_address(flow),
    ]
}

/// The property listeners calling a callback, on the system object and on
/// the default devices it follows.
pub struct Watch {
    /// The callback, leaked so it outlives every call CoreAudio makes: one
    /// allocation per watch, for the life of the process.
    context: &'static Callback,
    /// The listeners on default devices, to move when a default changes.
    followed: Vec<(AudioObjectID, AudioObjectPropertyAddress)>,
}

impl std::fmt::Debug for Watch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Watch").finish_non_exhaustive()
    }
}

impl Watch {
    /// Calls `on_change` whenever a default device changes, or the default
    /// devices' volume or mute does, until dropped.
    ///
    /// # Errors
    ///
    /// When CoreAudio refuses a listener.
    pub fn new(on_change: impl Fn() + Send + Sync + 'static) -> io::Result<Self> {
        let context: &'static Callback = Box::leak(Box::new(Callback(Box::new(on_change))));
        let mut watch = Self {
            context,
            followed: Vec::new(),
        };
        for at in system_addresses() {
            watch.listen(SYSTEM, at)?;
        }
        watch.follow()?;
        Ok(watch)
    }

    /// Moves the device listeners onto the current default devices; called
    /// when a default device changed.
    ///
    /// # Errors
    ///
    /// When CoreAudio refuses a listener or does not answer. A listener the
    /// old device would not give up (as when it was unplugged) is reported
    /// after the move is done.
    pub fn follow(&mut self) -> io::Result<()> {
        let mut removed = Ok(());
        for (device, at) in std::mem::take(&mut self.followed) {
            // Every listener is removed; the first refusal is the one told.
            let outcome = self.unlisten(device, at);
            if removed.is_ok() {
                removed = outcome;
            }
        }
        for flow in [Flow::Output, Flow::Input] {
            let Some(device) = default_device(flow)? else {
                continue;
            };
            for at in device_addresses(flow) {
                if has(device, at) || at.mElement == kAudioObjectPropertyElementWildcard {
                    self.listen(device, at)?;
                    self.followed.push((device, at));
                }
            }
        }
        removed
    }

    fn client_data(&self) -> *mut c_void {
        std::ptr::from_ref::<Callback>(self.context)
            .cast_mut()
            .cast()
    }

    fn listen(&self, object: AudioObjectID, at: AudioObjectPropertyAddress) -> io::Result<()> {
        // SAFETY: `changed` reads the client data as the leaked callback it
        // is; `at` is live for the call.
        let status = unsafe {
            AudioObjectAddPropertyListener(
                object,
                NonNull::from(&at),
                Some(changed),
                self.client_data(),
            )
        };
        checked(status, "adding a listener")
    }

    fn unlisten(&self, object: AudioObjectID, at: AudioObjectPropertyAddress) -> io::Result<()> {
        // SAFETY: the same listener and client data it was added with.
        let status = unsafe {
            AudioObjectRemovePropertyListener(
                object,
                NonNull::from(&at),
                Some(changed),
                self.client_data(),
            )
        };
        checked(status, "removing a listener")
    }
}

impl Drop for Watch {
    fn drop(&mut self) {
        // A drop has no one to report to. A listener that will not go keeps
        // calling the callback, which is leaked and so stays valid; the
        // calls then wake a main loop that has gone, which ignores them.
        for (device, at) in std::mem::take(&mut self.followed) {
            let _kept = self.unlisten(device, at);
        }
        for at in system_addresses() {
            let _kept = self.unlisten(SYSTEM, at);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Watch, sink, source};

    #[test]
    fn the_defaults_read_and_a_watch_starts_and_stops() {
        // CI's Macs may have no audio device; whatever there is reads.
        let output = sink().expect("CoreAudio answers");
        if let Some(sink) = output {
            assert!(sink.volume <= 100);
        }
        let _input = source().expect("CoreAudio answers");
        let mut watch = Watch::new(|| {}).expect("listeners");
        watch.follow().expect("follows the defaults");
        drop(watch);
    }
}
