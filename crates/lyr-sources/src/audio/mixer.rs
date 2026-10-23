//! The volume and mic modules' and the audio menu's source on Linux and
//! Windows, through crcbl's mixer (`crcbl-mixer`): on Linux it speaks the sound
//! server's own protocol over its socket, nothing to install beside the server,
//! no `pactl` and no C client library; on Windows it reads WASAPI. Readings
//! follow waybar 0.15.0's `AudioBackend` (`src/util/audio_backend.cpp`) for
//! the default sink and source. `Audio` follows the mixer on Linux's async
//! sources; Windows' thread holds a [`Mixer`] itself and calls the same
//! controls.

#[cfg(target_os = "linux")]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(target_os = "linux")]
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crcbl_mixer::{Action, Device, Direction, MixerState, StreamId};
pub use crcbl_mixer::{Interest, Mixer, MixerError};
use lyr_core::audio::{Sink, Source};
use lyr_core::audio_menu::{Kind, Node, Nodes, Setting};
#[cfg(target_os = "linux")]
use tokio::sync::{Notify, oneshot, watch};

#[cfg(target_os = "linux")]
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

/// What a platform's mixer does beyond volumes, mutes and defaults, which
/// decides what the audio menu draws.
#[derive(Clone, Copy, Debug)]
struct Platform {
    /// Whether a stream can be moved to another device: the menu then shows
    /// the device a stream plays to as a button that moves it, and otherwise
    /// in the stream's name.
    moves_streams: bool,
    /// Whether it reports cards and ports, which the configuration tab
    /// lists.
    configures: bool,
}

/// PipeWire and PulseAudio do all of it.
#[cfg(any(target_os = "linux", test))]
const LINUX: Platform = Platform {
    moves_streams: true,
    configures: true,
};

/// WASAPI reports no cards and no ports, and has no public API to move a
/// stream.
#[cfg(any(windows, test))]
const WINDOWS: Platform = Platform {
    moves_streams: false,
    configures: false,
};

#[cfg(target_os = "linux")]
const HERE: Platform = LINUX;
#[cfg(windows)]
const HERE: Platform = WINDOWS;

/// The name crcbl's mixer gives Windows' own sounds' session
/// (`crcbl_mixer`'s `windows::session`).
const SYSTEM_SOUNDS: &str = "System sounds";

/// What the audio menu lists from `state`: devices by name, streams by id,
/// and the configuration tab's cards and the devices with ports to choose,
/// as this platform's mixer can act on them.
#[must_use]
pub fn nodes(state: &MixerState) -> Nodes {
    nodes_on(state, HERE)
}

/// [`nodes`] as `platform` draws them. Where streams cannot move, a stream
/// on a device other than its flow's default is named with that device, and
/// Windows' own sounds there, which every endpoint has and which play on the
/// default, are left out.
fn nodes_on(state: &MixerState, platform: Platform) -> Nodes {
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
            .filter_map(|stream| {
                // Its application, else what it plays, else its number.
                let name = [Some(&stream.application), stream.title.as_ref()]
                    .into_iter()
                    .flatten()
                    .find(|name| !name.is_empty())
                    .map_or_else(|| format!("Stream {}", stream.id.0), String::clone);
                let device = stream.device.and_then(|id| state.device(id));
                let (name, device) = if platform.moves_streams {
                    (name, device.map(|device| device.name.clone()))
                } else {
                    match device.filter(|device| !device.default) {
                        None => (name, None),
                        Some(_) if stream.application == SYSTEM_SOUNDS => return None,
                        Some(device) => (format!("{name} ({})", device.description), None),
                    }
                };
                Some(Node {
                    id: stream.id.0.to_string(),
                    name,
                    volume: stream.volume,
                    muted: stream.muted,
                    default: false,
                    device,
                })
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
        settings: platform.configures.then(|| {
            profiles
                .chain(state.devices.iter().filter_map(ports))
                .collect()
        }),
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
#[cfg(target_os = "linux")]
pub struct Audio {
    /// Replaced by [`reconnect`](Self::reconnect) once the server has gone.
    mixer: Mutex<Mixer>,
    changed: Arc<Notify>,
    /// Whether streams are followed, kept for the mixer a reconnect opens.
    streams: AtomicBool,
}

#[cfg(target_os = "linux")]
impl Audio {
    /// Connects to the sound server and reads it. It follows devices only
    /// until [`follow_streams`](Self::follow_streams).
    ///
    /// # Errors
    ///
    /// When no sound server the mixer speaks to is running, or it could not
    /// be read.
    pub async fn open() -> Result<Self, MixerError> {
        let changed = Arc::new(Notify::new());
        let mixer = connect(&changed, Interest::Devices).await?;
        Ok(Self {
            mixer: Mutex::new(mixer),
            changed,
            streams: AtomicBool::new(false),
        })
    }

    /// Whether the sound server is still there: once it has gone, as it does
    /// when it restarts, the mixer holds its last state and refuses
    /// changes until [`reconnect`](Self::reconnect).
    #[must_use]
    pub fn is_connected(&self) -> bool {
        self.mixer().is_connected()
    }

    /// Connects to the sound server again, once, in place of the one that
    /// went away.
    ///
    /// # Errors
    ///
    /// As [`open`](Self::open): the server may not be back yet.
    pub async fn reconnect(&self) -> Result<(), MixerError> {
        let mixer = connect(&self.changed, self.interest()).await?;
        *self.mixer() = mixer;
        Ok(())
    }

    /// The modules' reading and the menu's nodes now.
    #[must_use]
    pub fn read(&self) -> (Reading, Nodes) {
        let state = self.mixer().state();
        (reading(&state), nodes(&state))
    }

    /// Sends what [`read`](Self::read) reads to `states` now and after every
    /// change the mixer reports, until every receiver is dropped.
    ///
    /// # Errors
    ///
    /// [`MixerError::Disconnected`] when the sound server goes away; `states`
    /// then holds no reading and no nodes, and [`reconnect`](Self::reconnect)
    /// follows it again once it is back.
    pub async fn watch(&self, states: &watch::Sender<(Reading, Nodes)>) -> Result<(), MixerError> {
        states.send_replace(self.read());
        loop {
            self.changed.notified().await;
            if !self.is_connected() {
                // What it last read is no longer true, and changes are refused.
                states.send_replace(Default::default());
                return Err(MixerError::Disconnected);
            }
            if !publish(states, self.read()) {
                return Ok(());
            }
        }
    }

    /// Hands `states` what changed since [`watch`](Self::watch) last sent:
    /// for the menu opening, whose streams were not followed while it was
    /// closed.
    pub fn refresh(&self, states: &watch::Sender<(Reading, Nodes)>) {
        if self.is_connected() {
            publish(states, self.read());
        }
    }

    /// Whether a stream's change is a change [`watch`](Self::watch) sends:
    /// only the open menu shows streams.
    pub fn follow_streams(&self, follow: bool) {
        self.streams.store(follow, Ordering::Relaxed);
        self.mixer().set_interest(self.interest());
    }

    fn interest(&self) -> Interest {
        if self.streams.load(Ordering::Relaxed) {
            Interest::DevicesAndStreams
        } else {
            Interest::Devices
        }
    }

    fn mixer(&self) -> MutexGuard<'_, Mixer> {
        // A panic while it was held left nothing half-changed: the guard only
        // ever reads or swaps whole mixers.
        self.mixer.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Mutes or unmutes `target`; see [`toggle_mute`].
    ///
    /// # Errors
    ///
    /// As [`toggle_mute`].
    pub fn toggle_mute(&self, target: &Target) -> Result<(), MixerError> {
        toggle_mute(&self.mixer(), target)
    }

    /// Moves `target`'s volume by `delta` percent; see [`step`].
    ///
    /// # Errors
    ///
    /// As [`toggle_mute`].
    pub fn step(&self, target: &Target, delta: i32) -> Result<(), MixerError> {
        step(&self.mixer(), target, delta)
    }

    /// Sets `target`'s volume to `percent`; see [`set`].
    ///
    /// # Errors
    ///
    /// As [`toggle_mute`].
    pub fn set(&self, target: &Target, percent: u32) -> Result<(), MixerError> {
        set(&self.mixer(), target, percent)
    }

    /// Moves the stream `id` of `flow` to the device `name`; see
    /// [`move_stream`].
    ///
    /// # Errors
    ///
    /// As [`toggle_mute`].
    pub fn move_stream(&self, flow: Flow, id: u32, name: &str) -> Result<(), MixerError> {
        move_stream(&self.mixer(), flow, id, name)
    }

    /// Chooses `option` for the card or device `id`; see [`choose`].
    ///
    /// # Errors
    ///
    /// As [`choose`].
    pub fn choose(&self, kind: Kind, id: &str, option: &str) -> Result<(), MixerError> {
        choose(&self.mixer(), kind, id, option)
    }

    /// Makes the device `name` of `flow` the default; see [`make_default`].
    ///
    /// # Errors
    ///
    /// As [`toggle_mute`].
    pub fn make_default(&self, flow: Flow, name: &str) -> Result<(), MixerError> {
        make_default(&self.mixer(), flow, name)
    }
}

/// Mutes or unmutes `target` in `mixer`.
///
/// # Errors
///
/// When the target is gone or the server could not be told.
pub fn toggle_mute(mixer: &Mixer, target: &Target) -> Result<(), MixerError> {
    apply(mixer, target, &Action::ToggleMute)
}

/// Moves `target`'s volume by `delta` percent, held to the mixer's range.
///
/// # Errors
///
/// As [`toggle_mute`].
pub fn step(mixer: &Mixer, target: &Target, delta: i32) -> Result<(), MixerError> {
    apply(mixer, target, &Action::Step(delta))
}

/// Sets `target`'s volume to `percent`.
///
/// # Errors
///
/// As [`toggle_mute`].
pub fn set(mixer: &Mixer, target: &Target, percent: u32) -> Result<(), MixerError> {
    apply(mixer, target, &Action::SetVolume(percent))
}

/// Moves the stream `id` of `flow` to the device `name`.
///
/// # Errors
///
/// As [`toggle_mute`], and on Windows, which cannot move a stream.
pub fn move_stream(mixer: &Mixer, flow: Flow, id: u32, name: &str) -> Result<(), MixerError> {
    let device = device(mixer, flow, name)?;
    mixer.apply(
        crcbl_mixer::Target::Stream(StreamId(id)),
        &Action::MoveTo(device),
    )
}

/// Chooses `option` for the card or device `id`: its profile or its port,
/// as `kind` says.
///
/// # Errors
///
/// As [`toggle_mute`], and when `option` is not one it has.
pub fn choose(mixer: &Mixer, kind: Kind, id: &str, option: &str) -> Result<(), MixerError> {
    match kind {
        Kind::Profile => {
            let card = mixer
                .state()
                .cards
                .iter()
                .find(|card| card.name == id)
                .ok_or(MixerError::NoSuchTarget)?
                .id;
            mixer.apply(
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
            apply(
                mixer,
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
/// As [`toggle_mute`].
pub fn make_default(mixer: &Mixer, flow: Flow, name: &str) -> Result<(), MixerError> {
    apply(
        mixer,
        &Target::Device(flow, name.to_owned()),
        &Action::MakeDefault,
    )
}

fn apply(mixer: &Mixer, target: &Target, action: &Action) -> Result<(), MixerError> {
    let target = match target {
        Target::Default(flow) => crcbl_mixer::Target::Default((*flow).into()),
        Target::Device(flow, name) => crcbl_mixer::Target::Device(device(mixer, *flow, name)?),
        Target::Stream(_, id) => crcbl_mixer::Target::Stream(StreamId(*id)),
    };
    mixer.apply(target, action)
}

/// The id of the device `name` of `flow` in `mixer`.
fn device(mixer: &Mixer, flow: Flow, name: &str) -> Result<crcbl_mixer::DeviceId, MixerError> {
    let direction = flow.into();
    mixer
        .state()
        .devices
        .iter()
        .find(|device| device.direction == direction && device.name == name)
        .map(|device| device.id)
        .ok_or(MixerError::NoSuchTarget)
}

/// Opens a mixer following `interest`, waking `changed` on each change, on a
/// thread of its own: the mixer waits there for the server's whole first
/// picture.
#[cfg(target_os = "linux")]
async fn connect(changed: &Arc<Notify>, interest: Interest) -> Result<Mixer, MixerError> {
    let (opened, mixer) = oneshot::channel();
    let waker = Arc::clone(changed);
    std::thread::spawn(move || {
        // Nobody waits any more when the receiver is gone.
        let _ = opened.send(Mixer::open(move || waker.notify_one()));
    });
    let mixer = mixer
        .await
        .map_err(|_| MixerError::Protocol("the mixer's thread ended".to_owned()))??;
    mixer.set_interest(interest);
    Ok(mixer)
}

#[cfg(test)]
mod tests;
