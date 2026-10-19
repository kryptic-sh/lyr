//! The volume and mic on Windows, through WASAPI: the default output and
//! input endpoints (the `eConsole` role, Windows' "default device"), each
//! one's scalar volume and mute (`IAudioEndpointVolume`), friendly name and
//! form factor ([`super::win32`] maps them). [`Watch`] reports a change of
//! either default endpoint (`IMMNotificationClient`) or of its volume
//! (`IAudioEndpointVolumeCallback`); its callbacks only signal, and the
//! caller reads, on a thread that has joined COM's multithreaded apartment
//! ([`init`]).

use std::io;
use std::sync::Arc;

use lyr_core::audio::{Sink, Source};
use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
use windows::Win32::Foundation::{ERROR_NOT_FOUND, PROPERTYKEY};
use windows::Win32::Media::Audio::Endpoints::{
    IAudioEndpointVolume, IAudioEndpointVolumeCallback, IAudioEndpointVolumeCallback_Impl,
};
use windows::Win32::Media::Audio::{
    AUDIO_VOLUME_NOTIFICATION_DATA, DEVICE_STATE, EDataFlow, ERole, IMMDevice, IMMDeviceEnumerator,
    IMMNotificationClient, IMMNotificationClient_Impl, MMDeviceEnumerator,
    PKEY_AudioEndpoint_FormFactor, eCapture, eConsole, eRender,
};
use windows::Win32::System::Com::StructuredStorage::PROPVARIANT;
use windows::Win32::System::Com::{
    CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoTaskMemFree, STGM_READ,
};
use windows::core::{HRESULT, PCWSTR, implement};

use super::win32::{percent, port, stepped};

/// Joins the calling thread to COM's multithreaded apartment, which the
/// audio objects are made and called in.
///
/// # Errors
///
/// When COM refuses, as it does a thread already in another apartment.
pub fn init() -> io::Result<()> {
    // SAFETY: no reserved pointer; the thread stays in the apartment.
    unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }
        .ok()
        .map_err(io::Error::other)
}

/// Which endpoint: the output the volume module shows, or the input the
/// mic module shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flow {
    Output,
    Input,
}

impl Flow {
    const fn data_flow(self) -> EDataFlow {
        match self {
            Self::Output => eRender,
            Self::Input => eCapture,
        }
    }
}

/// The device enumerator, on a thread that has called [`init`].
pub struct Audio {
    enumerator: IMMDeviceEnumerator,
}

impl Audio {
    /// The enumerator.
    ///
    /// # Errors
    ///
    /// When COM cannot make it.
    pub fn new() -> io::Result<Self> {
        // SAFETY: a CLSID constant and no outer object.
        let enumerator = unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) }
            .map_err(io::Error::other)?;
        Ok(Self { enumerator })
    }

    /// The default endpoint for `flow`, `None` when there is none.
    fn device(&self, flow: Flow) -> io::Result<Option<IMMDevice>> {
        // SAFETY: plain values.
        match unsafe {
            self.enumerator
                .GetDefaultAudioEndpoint(flow.data_flow(), eConsole)
        } {
            Ok(device) => Ok(Some(device)),
            Err(error) if error.code() == HRESULT::from_win32(ERROR_NOT_FOUND.0) => Ok(None),
            Err(error) => Err(io::Error::other(error)),
        }
    }

    /// The default endpoint for `flow` and its volume control.
    fn endpoint(&self, flow: Flow) -> io::Result<Option<(IMMDevice, IAudioEndpointVolume)>> {
        let Some(device) = self.device(flow)? else {
            return Ok(None);
        };
        // SAFETY: no activation parameters.
        let volume = unsafe { device.Activate(CLSCTX_ALL, None) }.map_err(io::Error::other)?;
        Ok(Some((device, volume)))
    }

    /// The default output, as the volume module reads it.
    ///
    /// # Errors
    ///
    /// When the endpoint does not answer.
    pub fn sink(&self) -> io::Result<Option<Sink>> {
        let Some((device, volume)) = self.endpoint(Flow::Output)? else {
            return Ok(None);
        };
        let form_factor = property(&device, &PKEY_AudioEndpoint_FormFactor)
            .and_then(|value| u32::try_from(&value).ok())
            .unwrap_or_default();
        let (level, muted) = level(&volume)?;
        Ok(Some(Sink {
            volume: level,
            muted,
            name: id(&device).unwrap_or_default(),
            source_name: self
                .device(Flow::Input)?
                .and_then(|input| id(&input))
                .unwrap_or_default(),
            port: port(form_factor).to_owned(),
            description: name(&device),
            // Not read yet: a Bluetooth endpoint shows as any other.
            bluetooth: false,
        }))
    }

    /// The default input, as the mic module reads it.
    ///
    /// # Errors
    ///
    /// When the endpoint does not answer.
    pub fn source(&self) -> io::Result<Option<Source>> {
        let Some((device, volume)) = self.endpoint(Flow::Input)? else {
            return Ok(None);
        };
        let (level, muted) = level(&volume)?;
        Ok(Some(Source {
            volume: level,
            muted,
            description: name(&device),
        }))
    }

    /// Mutes the default endpoint for `flow`, or unmutes it.
    ///
    /// # Errors
    ///
    /// When there is no such endpoint or it refuses.
    pub fn toggle_mute(&self, flow: Flow) -> io::Result<()> {
        let (_, volume) = self
            .endpoint(flow)?
            .ok_or_else(|| io::Error::other("no endpoint"))?;
        // SAFETY: plain values; no event context.
        unsafe {
            let muted = volume.GetMute().map_err(io::Error::other)?.as_bool();
            volume
                .SetMute(!muted, std::ptr::null())
                .map_err(io::Error::other)
        }
    }

    /// Moves the default endpoint for `flow`'s volume by `delta` percent.
    ///
    /// # Errors
    ///
    /// When there is no such endpoint or it refuses.
    pub fn step(&self, flow: Flow, delta: i32) -> io::Result<()> {
        let (_, volume) = self
            .endpoint(flow)?
            .ok_or_else(|| io::Error::other("no endpoint"))?;
        // SAFETY: plain values; no event context.
        unsafe {
            let scalar = volume
                .GetMasterVolumeLevelScalar()
                .map_err(io::Error::other)?;
            volume
                .SetMasterVolumeLevelScalar(stepped(scalar, delta), std::ptr::null())
                .map_err(io::Error::other)
        }
    }
}

/// An endpoint's volume as a percent, and whether it is muted.
fn level(volume: &IAudioEndpointVolume) -> io::Result<(u32, bool)> {
    // SAFETY: plain reads.
    unsafe {
        Ok((
            percent(
                volume
                    .GetMasterVolumeLevelScalar()
                    .map_err(io::Error::other)?,
            ),
            volume.GetMute().map_err(io::Error::other)?.as_bool(),
        ))
    }
}

/// A device's property, `None` when it has no such property.
fn property(device: &IMMDevice, key: &PROPERTYKEY) -> Option<PROPVARIANT> {
    // SAFETY: read-only access; `key` is a constant.
    unsafe { device.OpenPropertyStore(STGM_READ).ok()?.GetValue(key).ok() }
}

/// A device's friendly name, such as "Speakers (Realtek(R) Audio)".
fn name(device: &IMMDevice) -> String {
    property(device, &PKEY_Device_FriendlyName).map_or_else(String::new, |value| value.to_string())
}

/// A device's endpoint id.
fn id(device: &IMMDevice) -> Option<String> {
    // SAFETY: the id is a string COM allocated, read once and freed once.
    unsafe {
        let id = device.GetId().ok()?;
        let text = id.to_string().ok();
        CoTaskMemFree(Some(id.0.cast_const().cast()));
        text
    }
}

/// What a change calls.
type OnChange = Arc<dyn Fn() + Send + Sync>;

/// Says a default endpoint changed.
#[implement(IMMNotificationClient)]
struct DeviceChanges {
    on_change: OnChange,
}

impl IMMNotificationClient_Impl for DeviceChanges_Impl {
    fn OnDeviceStateChanged(
        &self,
        _device: &PCWSTR,
        _state: DEVICE_STATE,
    ) -> windows::core::Result<()> {
        Ok(())
    }

    fn OnDeviceAdded(&self, _device: &PCWSTR) -> windows::core::Result<()> {
        Ok(())
    }

    fn OnDeviceRemoved(&self, _device: &PCWSTR) -> windows::core::Result<()> {
        Ok(())
    }

    fn OnDefaultDeviceChanged(
        &self,
        _flow: EDataFlow,
        role: ERole,
        _device: &PCWSTR,
    ) -> windows::core::Result<()> {
        // The modules follow the console role, which each change reports
        // alongside the others.
        if role == eConsole {
            (self.on_change)();
        }
        Ok(())
    }

    fn OnPropertyValueChanged(
        &self,
        _device: &PCWSTR,
        _key: &PROPERTYKEY,
    ) -> windows::core::Result<()> {
        Ok(())
    }
}

/// Says an endpoint's volume or mute changed.
#[implement(IAudioEndpointVolumeCallback)]
struct VolumeChanges {
    on_change: OnChange,
}

impl IAudioEndpointVolumeCallback_Impl for VolumeChanges_Impl {
    fn OnNotify(&self, _data: *mut AUDIO_VOLUME_NOTIFICATION_DATA) -> windows::core::Result<()> {
        (self.on_change)();
        Ok(())
    }
}

/// The registrations calling `on_change` while it lives.
pub struct Watch {
    enumerator: IMMDeviceEnumerator,
    client: IMMNotificationClient,
    /// Each default endpoint's volume control with its registered callback.
    endpoints: Vec<(IAudioEndpointVolume, IAudioEndpointVolumeCallback)>,
    on_change: OnChange,
}

impl Watch {
    /// Calls `on_change`, on a COM thread, when a default endpoint changes or
    /// its volume or mute does, until the watch is dropped. After a change of
    /// endpoint, [`Watch::follow`] moves the volume callbacks to the new one.
    ///
    /// # Errors
    ///
    /// When a registration is refused.
    pub fn new(audio: &Audio, on_change: impl Fn() + Send + Sync + 'static) -> io::Result<Self> {
        let on_change: OnChange = Arc::new(on_change);
        let client: IMMNotificationClient = DeviceChanges {
            on_change: on_change.clone(),
        }
        .into();
        // SAFETY: the client is a COM object the enumerator holds a reference
        // to until it is unregistered.
        unsafe {
            audio
                .enumerator
                .RegisterEndpointNotificationCallback(&client)
        }
        .map_err(io::Error::other)?;
        let mut watch = Self {
            enumerator: audio.enumerator.clone(),
            client,
            endpoints: Vec::new(),
            on_change,
        };
        watch.follow(audio)?;
        Ok(watch)
    }

    /// Moves the volume callbacks to the default endpoints as they are now.
    ///
    /// # Errors
    ///
    /// When an endpoint refuses a registration.
    pub fn follow(&mut self, audio: &Audio) -> io::Result<()> {
        self.unregister_endpoints();
        for flow in [Flow::Output, Flow::Input] {
            let Some((_, volume)) = audio.endpoint(flow)? else {
                continue;
            };
            let callback: IAudioEndpointVolumeCallback = VolumeChanges {
                on_change: self.on_change.clone(),
            }
            .into();
            // SAFETY: the endpoint holds a reference to the callback until it
            // is unregistered.
            unsafe { volume.RegisterControlChangeNotify(&callback) }.map_err(io::Error::other)?;
            self.endpoints.push((volume, callback));
        }
        Ok(())
    }

    fn unregister_endpoints(&mut self) {
        for (volume, callback) in self.endpoints.drain(..) {
            // SAFETY: registered on this endpoint by `follow`. A failure leaves
            // a callback that only signals a change.
            let _unregistered = unsafe { volume.UnregisterControlChangeNotify(&callback) };
        }
    }
}

impl std::fmt::Debug for Watch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Watch")
            .field("endpoints", &self.endpoints.len())
            .finish_non_exhaustive()
    }
}

impl Drop for Watch {
    fn drop(&mut self) {
        self.unregister_endpoints();
        // SAFETY: registered by `new`. A failure leaves a client that only
        // signals a change, holding its own reference.
        let _unregistered = unsafe {
            self.enumerator
                .UnregisterEndpointNotificationCallback(&self.client)
        };
    }
}

#[cfg(test)]
mod tests {
    use super::{Audio, Watch, init};

    #[test]
    fn this_machine_s_endpoints_read_and_a_watch_registers() {
        init().expect("in the multithreaded apartment");
        let audio = Audio::new().expect("the enumerator");
        // A machine may have no output or input; whatever it has answers.
        let sink = audio.sink().expect("the output answers");
        let source = audio.source().expect("the input answers");
        if let Some(sink) = &sink {
            assert!(sink.volume <= 100);
        }
        if let Some(source) = &source {
            assert!(source.volume <= 100);
        }
        let watch = Watch::new(&audio, || {}).expect("registered");
        assert_eq!(
            watch.endpoints.len(),
            usize::from(sink.is_some()) + usize::from(source.is_some())
        );
    }
}
