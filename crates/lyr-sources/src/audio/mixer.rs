//! The volume and mic modules' and the audio menu's source on Linux, through
//! crcbl's mixer (`crcbl-mixer`), which speaks the sound server's own protocol
//! over its socket: nothing to install beside the server, no `pactl` and no C
//! client library. Readings follow waybar 0.15.0's `AudioBackend`
//! (`src/util/audio_backend.cpp`) for the default sink and source.

use std::sync::Arc;

use crcbl_mixer::{Action, Device, Direction, Interest, Mixer, MixerError, MixerState, StreamId};
use lyr_core::audio::{Sink, Source};
use lyr_core::audio_menu::{Kind, Node, Nodes, Setting};
use tokio::sync::{Notify, oneshot, watch};

use crate::bus::publish;

/// The default sink and source, as the volume and mic modules show them.
pub type Reading = (Option<Sink>, Option<Source>);

/// Which way sound goes: out to a sink, as the volume module's, or in from
/// a source, as the mic module's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flow {
    Output,
    Input,
}

impl From<Flow> for Direction {
    fn from(flow: Flow) -> Self {
        match flow {
            Flow::Output => Self::Output,
            Flow::Input => Self::Input,
        }
    }
}

/// What a control acts on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    /// The default device of a flow: the volume or the mic module's.
    Default(Flow),
    /// A device of a flow, by its name.
    Device(Flow, String),
    /// A stream of a flow, by its id: the number `pactl` calls its index.
    Stream(Flow, u32),
}

/// The default sink and source in `state`.
#[must_use]
pub fn reading(state: &MixerState) -> Reading {
    let source = state.default_device(Direction::Input);
    let sink = state.default_device(Direction::Output).map(|sink| Sink {
        volume: sink.volume,
        muted: sink.muted,
        name: sink.name.clone(),
        source_name: source.map(|source| source.name.clone()).unwrap_or_default(),
        port: format!(
            "{}{}",
            sink.active_port.as_deref().unwrap_or("Unknown"),
            sink.form_factor.as_deref().unwrap_or_default()
        ),
        description: sink.description.clone(),
        bluetooth: sink.bluetooth,
    });
    let source = source.map(|source| Source {
        volume: source.volume,
        muted: source.muted,
        description: source.description.clone(),
    });
    (sink, source)
}

/// What the audio menu lists from `state`: devices by name, streams by id,
/// and the configuration tab's cards and the devices with ports to choose.
#[must_use]
pub fn nodes(state: &MixerState) -> Nodes {
    let devices = |direction| {
        state
            .devices
            .iter()
            .filter(|device| device.direction == direction)
            .map(|device| Node {
                id: device.name.clone(),
                name: device.description.clone(),
                volume: device.volume,
                muted: device.muted,
                default: device.default,
                device: None,
            })
            .collect()
    };
    let streams = |direction| {
        state
            .streams
            .iter()
            .filter(|stream| stream.direction == direction)
            .map(|stream| Node {
                id: stream.id.0.to_string(),
                // Its application, else what it plays, else its number.
                name: [Some(&stream.application), stream.title.as_ref()]
                    .into_iter()
                    .flatten()
                    .find(|name| !name.is_empty())
                    .map_or_else(|| format!("Stream {}", stream.id.0), String::clone),
                volume: stream.volume,
                muted: stream.muted,
                default: false,
                device: stream
                    .device
                    .and_then(|id| state.device(id))
                    .map(|device| device.name.clone()),
            })
            .collect()
    };
    let profiles = state.cards.iter().map(|card| Setting {
        id: card.name.clone(),
        name: card.description.clone(),
        kind: Kind::Profile,
        // In the server's order, the active one listed even if marked
        // unavailable.
        options: card
            .profiles
            .iter()
            .filter(|profile| {
                profile.available || card.active_profile.as_ref() == Some(&profile.name)
            })
            .map(|profile| (profile.name.clone(), profile.description.clone()))
            .collect(),
        active: card.active_profile.clone().unwrap_or_default(),
    });
    Nodes {
        playback: streams(Direction::Output),
        recording: streams(Direction::Input),
        outputs: devices(Direction::Output),
        inputs: devices(Direction::Input),
        settings: Some(
            profiles
                .chain(state.devices.iter().filter_map(ports))
                .collect(),
        ),
    }
}

/// A device's port setting, when it has more than one port to choose: its
/// ports in the server's order, those it reports as not available left out
/// unless active.
fn ports(device: &Device) -> Option<Setting> {
    let active = device.active_port.clone()?;
    let options: Vec<_> = device
        .ports
        .iter()
        .filter(|port| port.available || port.name == active)
        .map(|port| (port.name.clone(), port.description.clone()))
        .collect();
    (options.len() > 1).then(|| Setting {
        id: device.name.clone(),
        name: device.description.clone(),
        kind: match device.direction {
            Direction::Output => Kind::OutputPort,
            Direction::Input => Kind::InputPort,
        },
        options,
        active,
    })
}

/// The system mixer, and a note of each change it reports.
pub struct Audio {
    mixer: Mixer,
    changed: Arc<Notify>,
}

impl Audio {
    /// Connects to the sound server and reads it, on a thread of its own:
    /// the mixer waits there for the server's whole first picture. It
    /// follows devices only until [`follow_streams`](Self::follow_streams).
    ///
    /// # Errors
    ///
    /// When no sound server the mixer speaks to is running, or it could not
    /// be read.
    pub async fn open() -> Result<Self, MixerError> {
        let changed = Arc::new(Notify::new());
        let (opened, mixer) = oneshot::channel();
        let waker = Arc::clone(&changed);
        std::thread::spawn(move || {
            // Nobody waits any more when the receiver is gone.
            let _ = opened.send(Mixer::open(move || waker.notify_one()));
        });
        let mixer = mixer
            .await
            .map_err(|_| MixerError::Protocol("the mixer's thread ended".to_owned()))??;
        mixer.set_interest(Interest::Devices);
        Ok(Self { mixer, changed })
    }

    /// The modules' reading and the menu's nodes now.
    #[must_use]
    pub fn read(&self) -> (Reading, Nodes) {
        let state = self.mixer.state();
        (reading(&state), nodes(&state))
    }

    /// Sends what [`read`](Self::read) reads to `states` now and after every
    /// change the mixer reports, until every receiver is dropped.
    pub async fn watch(&self, states: &watch::Sender<(Reading, Nodes)>) {
        states.send_replace(self.read());
        loop {
            self.changed.notified().await;
            if !publish(states, self.read()) {
                break;
            }
        }
    }

    /// Hands `states` what changed since [`watch`](Self::watch) last sent:
    /// for the menu opening, whose streams were not followed while it was
    /// closed.
    pub fn refresh(&self, states: &watch::Sender<(Reading, Nodes)>) {
        publish(states, self.read());
    }

    /// Whether a stream's change is a change [`watch`](Self::watch) sends:
    /// only the open menu shows streams.
    pub fn follow_streams(&self, follow: bool) {
        self.mixer.set_interest(if follow {
            Interest::DevicesAndStreams
        } else {
            Interest::Devices
        });
    }

    /// Mutes or unmutes `target`.
    ///
    /// # Errors
    ///
    /// When the target is gone or the server could not be told.
    pub fn toggle_mute(&self, target: &Target) -> Result<(), MixerError> {
        self.apply(target, &Action::ToggleMute)
    }

    /// Moves `target`'s volume by `delta` percent, held to the mixer's
    /// range.
    ///
    /// # Errors
    ///
    /// As [`toggle_mute`](Self::toggle_mute).
    pub fn step(&self, target: &Target, delta: i32) -> Result<(), MixerError> {
        self.apply(target, &Action::Step(delta))
    }

    /// Sets `target`'s volume to `percent`.
    ///
    /// # Errors
    ///
    /// As [`toggle_mute`](Self::toggle_mute).
    pub fn set(&self, target: &Target, percent: u32) -> Result<(), MixerError> {
        self.apply(target, &Action::SetVolume(percent))
    }

    /// Moves the stream `id` of `flow` to the device `name`.
    ///
    /// # Errors
    ///
    /// As [`toggle_mute`](Self::toggle_mute).
    pub fn move_stream(&self, flow: Flow, id: u32, name: &str) -> Result<(), MixerError> {
        let device = self.device(flow, name)?;
        self.mixer.apply(
            crcbl_mixer::Target::Stream(StreamId(id)),
            &Action::MoveTo(device),
        )
    }

    /// Chooses `option` for the card or device `id`: its profile or its
    /// port, as `kind` says.
    ///
    /// # Errors
    ///
    /// As [`toggle_mute`](Self::toggle_mute), and when `option` is not one
    /// it has.
    pub fn choose(&self, kind: Kind, id: &str, option: &str) -> Result<(), MixerError> {
        match kind {
            Kind::Profile => {
                let card = self
                    .mixer
                    .state()
                    .cards
                    .iter()
                    .find(|card| card.name == id)
                    .ok_or(MixerError::NoSuchTarget)?
                    .id;
                self.mixer.apply(
                    crcbl_mixer::Target::Card(card),
                    &Action::SetProfile(option.to_owned()),
                )
            }
            Kind::OutputPort | Kind::InputPort => {
                let flow = if kind == Kind::OutputPort {
                    Flow::Output
                } else {
                    Flow::Input
                };
                self.apply(
                    &Target::Device(flow, id.to_owned()),
                    &Action::SetPort(option.to_owned()),
                )
            }
        }
    }

    /// Makes the device `name` of `flow` the default.
    ///
    /// # Errors
    ///
    /// As [`toggle_mute`](Self::toggle_mute).
    pub fn make_default(&self, flow: Flow, name: &str) -> Result<(), MixerError> {
        self.apply(&Target::Device(flow, name.to_owned()), &Action::MakeDefault)
    }

    fn apply(&self, target: &Target, action: &Action) -> Result<(), MixerError> {
        let target = match target {
            Target::Default(flow) => crcbl_mixer::Target::Default((*flow).into()),
            Target::Device(flow, name) => crcbl_mixer::Target::Device(self.device(*flow, name)?),
            Target::Stream(_, id) => crcbl_mixer::Target::Stream(StreamId(*id)),
        };
        self.mixer.apply(target, action)
    }

    /// The id of the device `name` of `flow`.
    fn device(&self, flow: Flow, name: &str) -> Result<crcbl_mixer::DeviceId, MixerError> {
        let direction = flow.into();
        self.mixer
            .state()
            .devices
            .iter()
            .find(|device| device.direction == direction && device.name == name)
            .map(|device| device.id)
            .ok_or(MixerError::NoSuchTarget)
    }
}

#[cfg(test)]
mod tests;
