//! The volume and mic on Windows, through WASAPI: the default output and
//! input endpoints (the `eConsole` role, Windows' "default device"), each
//! one's scalar volume and mute (`IAudioEndpointVolume`), friendly name and
//! form factor ([`super::win32`] maps them). [`Watch`] reports a change of
//! either default endpoint (`IMMNotificationClient`) or of its volume
//! (`IAudioEndpointVolumeCallback`); its callbacks only signal, and the
//! caller reads, on a thread that has joined COM's multithreaded apartment
//! ([`init`]). `menu` reads what the audio menu lists.

mod menu;

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
    IMMNotificationClient, IMMNotificationClient_Impl, ISimpleAudioVolume, MMDeviceEnumerator,
    PKEY_AudioEndpoint_FormFactor, eCapture, eConsole, eRender,
};
use windows::Win32::System::Com::StructuredStorage::PROPVARIANT;
use windows::Win32::System::Com::{
    CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoTaskMemFree, STGM_READ,
};
use windows::core::{HRESULT, HSTRING, PCWSTR, PWSTR, implement};

use super::level::{percent, scalar, stepped};
use super::win32::port;

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

/// What a mute or a step acts on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Which {
    /// The default endpoint for a flow: the volume or the mic module's.
    Default(Flow),
    /// An endpoint, by its id.
    Device(String),
    /// A stream on one of `flow`'s endpoints, by its session's instance id.
    Stream(Flow, String),
}

/// A volume control: an endpoint's or a stream's.
enum Control {
    Endpoint(IAudioEndpointVolume),
    Stream(ISimpleAudioVolume),
}

impl Control {
    /// The scalar volume, 0 – 1, and whether it is muted.
    fn read(&self) -> io::Result<(f32, bool)> {
        // SAFETY: plain reads.
        let read = unsafe {
            match self {
                Self::Endpoint(volume) => volume
                    .GetMasterVolumeLevelScalar()
                    .and_then(|scalar| Ok((scalar, volume.GetMute()?.as_bool()))),
                Self::Stream(volume) => volume
                    .GetMasterVolume()
                    .and_then(|scalar| Ok((scalar, volume.GetMute()?.as_bool()))),
            }
        };
        read.map_err(io::Error::other)
    }

    /// The volume as the percent the views show, and whether it is muted.
    fn shown(&self) -> io::Result<(u32, bool)> {
        let (scalar, muted) = self.read()?;
        Ok((percent(scalar), muted))
    }

    fn set_mute(&self, muted: bool) -> io::Result<()> {
        // SAFETY: plain values; no event context.
        let set = unsafe {
            match self {
                Self::Endpoint(volume) => volume.SetMute(muted, std::ptr::null()),
                Self::Stream(volume) => volume.SetMute(muted, std::ptr::null()),
            }
        };
        set.map_err(io::Error::other)
    }

    fn set_scalar(&self, scalar: f32) -> io::Result<()> {
        // SAFETY: plain values; no event context.
        let set = unsafe {
            match self {
                Self::Endpoint(volume) => {
                    volume.SetMasterVolumeLevelScalar(scalar, std::ptr::null())
                }
                Self::Stream(volume) => volume.SetMasterVolume(scalar, std::ptr::null()),
            }
        };
        set.map_err(io::Error::other)
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
        let volume = volume(&device)?;
        Ok(Some((device, volume)))
    }

    /// The volume control `which` names.
    fn control(&self, which: &Which) -> io::Result<Control> {
        match which {
            Which::Default(flow) => self
                .endpoint(*flow)?
                .map(|(_, volume)| Control::Endpoint(volume))
                .ok_or_else(|| io::Error::other("no endpoint")),
            Which::Device(id) => {
                // SAFETY: the id is a string that lives across the call.
                let device = unsafe { self.enumerator.GetDevice(&HSTRING::from(id.as_str())) }
                    .map_err(io::Error::other)?;
                Ok(Control::Endpoint(volume(&device)?))
            }
            Which::Stream(flow, id) => Ok(Control::Stream(self.stream(*flow, id)?)),
        }
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
        let (level, muted) = Control::Endpoint(volume).shown()?;
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
        let (level, muted) = Control::Endpoint(volume).shown()?;
        Ok(Some(Source {
            volume: level,
            muted,
            description: name(&device),
        }))
    }

    /// Mutes what `which` names, or unmutes it.
    ///
    /// # Errors
    ///
    /// When there is no such endpoint or stream, or it refuses.
    pub fn toggle_mute(&self, which: &Which) -> io::Result<()> {
        let control = self.control(which)?;
        let (_, muted) = control.read()?;
        control.set_mute(!muted)
    }

    /// Sets the volume of what `which` names to `percent`, held to 100.
    ///
    /// # Errors
    ///
    /// When there is no such endpoint or stream, or it refuses.
    pub fn set(&self, which: &Which, percent: u32) -> io::Result<()> {
        self.control(which)?.set_scalar(scalar(percent))
    }

    /// Moves the volume of what `which` names by `delta` percent.
    ///
    /// # Errors
    ///
    /// When there is no such endpoint or stream, or it refuses.
    pub fn step(&self, which: &Which, delta: i32) -> io::Result<()> {
        let control = self.control(which)?;
        let (scalar, _) = control.read()?;
        control.set_scalar(stepped(scalar, delta))
    }
}

/// An endpoint's volume control.
fn volume(device: &IMMDevice) -> io::Result<IAudioEndpointVolume> {
    // SAFETY: no activation parameters.
    unsafe { device.Activate(CLSCTX_ALL, None) }.map_err(io::Error::other)
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
    // SAFETY: a plain read; `take` frees the string.
    take(unsafe { device.GetId() }.ok()?)
}

/// A string COM allocated for the caller, read and then freed.
fn take(text: PWSTR) -> Option<String> {
    // SAFETY: COM hands over a terminated string, freed here once, after it
    // was read.
    unsafe {
        let read = text.to_string().ok();
        CoTaskMemFree(Some(text.0.cast_const().cast()));
        read
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
    use super::{Audio, Flow, Watch, Which, init};

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

    /// Steps by 0 % through every kind of control, which finds each by its
    /// id and sets the volume it already has, so the machine's volume stays
    /// as it was.
    #[test]
    fn the_menu_s_nodes_read_and_each_finds_its_control() {
        init().expect("in the multithreaded apartment");
        let audio = Audio::new().expect("the enumerator");
        let nodes = audio.nodes().expect("the nodes read");
        let sink = audio.sink().expect("the output answers");
        let defaults = nodes.outputs.iter().filter(|node| node.default).count();
        assert_eq!(defaults, usize::from(sink.is_some()), "one default output");
        if let Some(sink) = &sink {
            let default = nodes.outputs.iter().find(|node| node.default);
            assert_eq!(default.map(|node| &node.id), Some(&sink.name));
        }
        let mut stepped = 0;
        for (which, before) in nodes
            .outputs
            .iter()
            .chain(&nodes.inputs)
            .map(|node| (Which::Device(node.id.clone()), node.volume))
            .chain(
                nodes
                    .playback
                    .iter()
                    .map(|node| (Which::Stream(Flow::Output, node.id.clone()), node.volume)),
            )
        {
            audio.step(&which, 0).expect("found and set");
            stepped += 1;
            let after = audio.nodes().expect("the nodes read");
            let now = after
                .outputs
                .iter()
                .chain(&after.inputs)
                .chain(&after.playback)
                .find(|node| match &which {
                    Which::Device(id) | Which::Stream(_, id) => node.id == *id,
                    Which::Default(_) => false,
                })
                .map(|node| node.volume);
            assert_eq!(now, Some(before), "{which:?}");
        }
        assert_eq!(
            stepped,
            nodes.outputs.len() + nodes.inputs.len() + nodes.playback.len()
        );
        assert!(
            audio
                .step(&Which::Stream(Flow::Output, "no such stream".to_owned()), 0)
                .is_err()
        );
    }
}
