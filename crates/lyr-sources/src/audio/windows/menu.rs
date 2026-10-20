//! What the audio menu lists on Windows: every active endpoint of each flow,
//! and the streams on them, which WASAPI calls audio sessions
//! (`IAudioSessionManager2`). A session whose program has quit, expired,
//! is left out; one whose program is open but quiet is listed, as Windows'
//! own volume mixer lists it. A stream off the default endpoint is shown as
//! [`super::super::win32::stream_row`] says.

use std::io;

use lyr_core::audio_menu::{Node, Nodes};
use windows::Win32::Foundation::S_OK;
use windows::Win32::Media::Audio::{
    AudioSessionStateExpired, DEVICE_STATE_ACTIVE, IAudioSessionControl2, IAudioSessionManager2,
    IMMDevice, ISimpleAudioVolume,
};
use windows::Win32::System::Com::CLSCTX_ALL;
use windows::core::Interface as _;

use super::super::win32::{session_name, stream_row};
use super::{Audio, Control, Flow, id, name, take, volume};

/// A stream and its volume control.
struct Session {
    /// Its instance id, which names it to [`Audio::stream`].
    id: String,
    name: String,
    volume: ISimpleAudioVolume,
}

impl Audio {
    /// Everything the audio menu lists.
    ///
    /// # Errors
    ///
    /// When an endpoint or a session does not answer.
    pub fn nodes(&self) -> io::Result<Nodes> {
        Ok(Nodes {
            playback: self.stream_nodes(Flow::Output)?,
            recording: self.stream_nodes(Flow::Input)?,
            outputs: self.device_nodes(Flow::Output)?,
            inputs: self.device_nodes(Flow::Input)?,
        })
    }

    /// `flow`'s active endpoints.
    fn devices(&self, flow: Flow) -> io::Result<Vec<IMMDevice>> {
        // SAFETY: plain values and reads.
        unsafe {
            let devices = self
                .enumerator
                .EnumAudioEndpoints(flow.data_flow(), DEVICE_STATE_ACTIVE)
                .map_err(io::Error::other)?;
            let count = devices.GetCount().map_err(io::Error::other)?;
            (0..count)
                .map(|index| devices.Item(index).map_err(io::Error::other))
                .collect()
        }
    }

    fn device_nodes(&self, flow: Flow) -> io::Result<Vec<Node>> {
        let default = self.device(flow)?.and_then(|device| id(&device));
        self.devices(flow)?
            .iter()
            .map(|device| {
                let (volume, muted) = Control::Endpoint(volume(device)?).shown()?;
                let id = id(device).unwrap_or_default();
                Ok(Node {
                    default: default.as_ref() == Some(&id),
                    id,
                    name: name(device),
                    volume,
                    muted,
                    device: None,
                })
            })
            .collect()
    }

    /// The streams on every one of `flow`'s endpoints.
    fn sessions(&self, flow: Flow) -> io::Result<Vec<Session>> {
        let default = self.device(flow)?.and_then(|device| id(&device));
        let mut sessions = Vec::new();
        for device in self.devices(flow)? {
            let elsewhere = (id(&device) != default).then(|| name(&device));
            // SAFETY: no activation parameters; plain reads after.
            unsafe {
                let manager: IAudioSessionManager2 = device
                    .Activate(CLSCTX_ALL, None)
                    .map_err(io::Error::other)?;
                let listed = manager.GetSessionEnumerator().map_err(io::Error::other)?;
                for index in 0..listed.GetCount().map_err(io::Error::other)? {
                    let control = listed.GetSession(index).map_err(io::Error::other)?;
                    if control.GetState().map_err(io::Error::other)? == AudioSessionStateExpired {
                        continue;
                    }
                    let control: IAudioSessionControl2 =
                        control.cast().map_err(io::Error::other)?;
                    let system_sounds = control.IsSystemSoundsSession() == S_OK;
                    let display = control.GetDisplayName().ok().and_then(take);
                    let program = control
                        .GetProcessId()
                        .ok()
                        .and_then(crate::program::windows::path);
                    let Some(name) = stream_row(
                        session_name(
                            system_sounds,
                            display.as_deref().unwrap_or_default(),
                            program.as_deref(),
                        ),
                        system_sounds,
                        elsewhere.as_deref(),
                    ) else {
                        continue;
                    };
                    sessions.push(Session {
                        id: control
                            .GetSessionInstanceIdentifier()
                            .ok()
                            .and_then(take)
                            .unwrap_or_default(),
                        name,
                        volume: control.cast().map_err(io::Error::other)?,
                    });
                }
            }
        }
        Ok(sessions)
    }

    fn stream_nodes(&self, flow: Flow) -> io::Result<Vec<Node>> {
        self.sessions(flow)?
            .into_iter()
            .map(|session| {
                let (volume, muted) = Control::Stream(session.volume).shown()?;
                Ok(Node {
                    id: session.id,
                    name: session.name,
                    volume,
                    muted,
                    default: false,
                    // No public API moves a session to another endpoint.
                    device: None,
                })
            })
            .collect()
    }

    /// The volume control of the stream on `flow` whose instance id is `id`.
    pub(super) fn stream(&self, flow: Flow, id: &str) -> io::Result<ISimpleAudioVolume> {
        self.sessions(flow)?
            .into_iter()
            .find(|session| session.id == id)
            .map(|session| session.volume)
            .ok_or_else(|| io::Error::other("the stream has gone"))
    }
}
