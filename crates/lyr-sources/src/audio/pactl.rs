//! The volume and mic modules' and the audio menu's source on Linux,
//! through `pactl`, the
//! PulseAudio client that pipewire-pulse serves as PulseAudio does, so no
//! client library is linked (`libpulse-binding` is a question in
//! `docs/backlog.md`). Changes come from `pactl subscribe`, read again only
//! for sink, source and server events; readings follow waybar 0.15.0's
//! `AudioBackend` (`src/util/audio_backend.cpp`) for the default sink and
//! source.

use std::collections::HashMap;
use std::io;
use std::process::Stdio;

use std::sync::atomic::{AtomicBool, Ordering};

use lyr_core::audio::{Sink, Source};
use lyr_core::audio_menu::{Card, Node, Nodes};
use serde::Deserialize;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::watch;

use crate::bus::{Follow, publish};

/// PulseAudio's `PA_VOLUME_NORM`: a channel at 100 %.
const VOLUME_NORM: u64 = 0x1_0000;

/// What a sink's monitor source is called when the sink is Bluetooth
/// (waybar's `isBluetooth`: PulseAudio's, PipeWire's, and BlueZ's own).
const BLUETOOTH_MONITORS: [&str; 3] = ["a2dp_sink", "a2dp-sink", "bluez"];

/// The default sink and source, as the volume and mic modules show them.
pub type Reading = (Option<Sink>, Option<Source>);

/// Which way sound goes: out to a sink, as the volume module's, or in from
/// a source, as the mic module's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flow {
    Output,
    Input,
}

/// What a control acts on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target {
    /// The default device of a flow: the volume or the mic module's.
    Default(Flow),
    /// A device of a flow, by its name.
    Device(Flow, String),
    /// A stream of a flow, a sink input or a source output, by its index.
    Stream(Flow, u32),
}

impl Target {
    /// `pactl`'s commands that set its mute and volume, and its name for it.
    fn commands(&self) -> (&'static str, &'static str, String) {
        match self {
            Self::Default(Flow::Output) => (
                "set-sink-mute",
                "set-sink-volume",
                "@DEFAULT_SINK@".to_owned(),
            ),
            Self::Default(Flow::Input) => (
                "set-source-mute",
                "set-source-volume",
                "@DEFAULT_SOURCE@".to_owned(),
            ),
            Self::Device(Flow::Output, name) => ("set-sink-mute", "set-sink-volume", name.clone()),
            Self::Device(Flow::Input, name) => {
                ("set-source-mute", "set-source-volume", name.clone())
            }
            Self::Stream(Flow::Output, index) => (
                "set-sink-input-mute",
                "set-sink-input-volume",
                index.to_string(),
            ),
            Self::Stream(Flow::Input, index) => (
                "set-source-output-mute",
                "set-source-output-volume",
                index.to_string(),
            ),
        }
    }
}

#[derive(Deserialize)]
struct Info {
    default_sink_name: String,
    default_source_name: String,
}

#[derive(Deserialize)]
struct Channel {
    value: u64,
}

/// A sink or a source, as `pactl --format=json list` writes it.
#[derive(Deserialize)]
struct Device {
    /// What a stream names it by; only the menu needs it.
    index: Option<u32>,
    name: String,
    description: String,
    mute: bool,
    volume: HashMap<String, Channel>,
    /// A sink's; sources have none.
    #[serde(default)]
    monitor_source: String,
    active_port: Option<String>,
    #[serde(default)]
    properties: HashMap<String, serde_json::Value>,
}

/// `pa_cvolume_avg` (the channels' mean, in integers) as a percent of
/// `PA_VOLUME_NORM`, rounded half up as waybar's `std::round` does.
fn percent(volume: &HashMap<String, Channel>) -> u32 {
    let Some(channels) = u64::try_from(volume.len()).ok().filter(|&n| n > 0) else {
        return 0;
    };
    let average = volume.values().map(|channel| channel.value).sum::<u64>() / channels;
    u32::try_from((average * 100 + VOLUME_NORM / 2) / VOLUME_NORM).unwrap_or(u32::MAX)
}

/// The default sink and source from `pactl --format=json`'s `info`,
/// `list sinks` and `list sources`.
///
/// # Errors
///
/// When any of them is not the JSON `pactl` writes.
pub fn reading(info: &str, sinks: &str, sources: &str) -> serde_json::Result<Reading> {
    let info: Info = serde_json::from_str(info)?;
    let sinks: Vec<Device> = serde_json::from_str(sinks)?;
    let sources: Vec<Device> = serde_json::from_str(sources)?;
    let sink = sinks
        .into_iter()
        .find(|sink| sink.name == info.default_sink_name)
        .map(|sink| {
            let form_factor = sink
                .properties
                .get("device.form_factor")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            Sink {
                volume: percent(&sink.volume),
                muted: sink.mute,
                port: format!(
                    "{}{form_factor}",
                    sink.active_port.as_deref().unwrap_or("Unknown")
                ),
                bluetooth: BLUETOOTH_MONITORS
                    .iter()
                    .any(|monitor| sink.monitor_source.contains(monitor)),
                description: sink.description,
                source_name: info.default_source_name.clone(),
                name: sink.name,
            }
        });
    let source = sources
        .into_iter()
        .find(|source| source.name == info.default_source_name)
        .map(|source| Source {
            volume: percent(&source.volume),
            muted: source.mute,
            description: source.description,
        });
    Ok((sink, source))
}

/// A sink input or a source output, as `pactl --format=json list` writes
/// it.
#[derive(Deserialize)]
struct Stream {
    index: u32,
    /// The index of the sink a sink input plays to, or of the source a
    /// source output records from.
    #[serde(alias = "source")]
    sink: Option<u32>,
    mute: bool,
    volume: HashMap<String, Channel>,
    #[serde(default)]
    properties: HashMap<String, serde_json::Value>,
}

impl Stream {
    /// What the menu calls it: its application, else its media, else its
    /// program, as `pactl` reports them.
    fn name(&self) -> String {
        [
            "application.name",
            "media.name",
            "application.process.binary",
        ]
        .iter()
        .find_map(|key| self.properties.get(*key)?.as_str())
        .map_or_else(|| format!("Stream {}", self.index), str::to_owned)
    }

    /// Its row, its device named by `name_of` from the device's index.
    fn node(&self, name_of: impl Fn(u32) -> Option<String>) -> Node {
        Node {
            id: self.index.to_string(),
            name: self.name(),
            volume: percent(&self.volume),
            muted: self.mute,
            default: false,
            device: self.sink.and_then(name_of),
        }
    }
}

/// A card, as `pactl --format=json list cards` writes it.
#[derive(Deserialize)]
struct CardJson {
    name: String,
    active_profile: String,
    profiles: HashMap<String, ProfileJson>,
    #[serde(default)]
    properties: HashMap<String, serde_json::Value>,
}

#[derive(Deserialize)]
struct ProfileJson {
    description: String,
    available: bool,
    priority: u32,
}

impl CardJson {
    /// Its row: named by its description, its available profiles best
    /// first by PulseAudio's priority (a JSON object keeps no order), the
    /// active one listed even if marked unavailable.
    fn card(self) -> Card {
        let mut profiles: Vec<_> = self
            .profiles
            .into_iter()
            .filter(|(id, profile)| profile.available || *id == self.active_profile)
            .collect();
        profiles.sort_by(|(a, x), (b, y)| y.priority.cmp(&x.priority).then_with(|| a.cmp(b)));
        Card {
            name: self
                .properties
                .get("device.description")
                .and_then(serde_json::Value::as_str)
                .map_or_else(|| self.name.clone(), str::to_owned),
            id: self.name,
            profiles: profiles
                .into_iter()
                .map(|(id, profile)| (id, profile.description))
                .collect(),
            active: self.active_profile,
        }
    }
}

/// Whether a source is a sink's monitor, which the menu leaves out as
/// wiremix does: `pactl` marks it by its device class.
fn monitor(device: &Device) -> bool {
    device
        .properties
        .get("device.class")
        .and_then(serde_json::Value::as_str)
        == Some("monitor")
}

/// What the audio menu lists, from `pactl --format=json`'s `info`,
/// `list sinks`, `list sources`, `list sink-inputs`,
/// `list source-outputs` and `list cards`.
///
/// # Errors
///
/// When any of them is not the JSON `pactl` writes.
pub fn nodes(
    info: &str,
    sinks: &str,
    sources: &str,
    sink_inputs: &str,
    source_outputs: &str,
    cards: &str,
) -> serde_json::Result<Nodes> {
    let info: Info = serde_json::from_str(info)?;
    let sinks: Vec<Device> = serde_json::from_str(sinks)?;
    let sources: Vec<Device> = serde_json::from_str(sources)?;
    let devices = |devices: &[Device], default: &str| -> Vec<Node> {
        devices
            .iter()
            .filter(|device| !monitor(device))
            .map(|device| Node {
                id: device.name.clone(),
                name: device.description.clone(),
                volume: percent(&device.volume),
                muted: device.mute,
                default: device.name == default,
                device: None,
            })
            .collect()
    };
    let streams = |text: &str, devices: &[Device]| -> serde_json::Result<Vec<Node>> {
        let streams: Vec<Stream> = serde_json::from_str(text)?;
        let name_of = |index| {
            devices
                .iter()
                .find(|device| device.index == Some(index))
                .map(|device| device.name.clone())
        };
        Ok(streams.iter().map(|stream| stream.node(name_of)).collect())
    };
    Ok(Nodes {
        playback: streams(sink_inputs, &sinks)?,
        recording: streams(source_outputs, &sources)?,
        outputs: devices(&sinks, &info.default_sink_name),
        inputs: devices(&sources, &info.default_source_name),
        cards: Some(
            serde_json::from_str::<Vec<CardJson>>(cards)?
                .into_iter()
                .map(CardJson::card)
                .collect(),
        ),
    })
}

/// Whether a `pactl subscribe` line is a change the modules or the menu's
/// devices and cards show: a sink, a source, a card, or the server's
/// defaults, not a stream's.
#[must_use]
pub fn shows(line: &str) -> bool {
    [" on sink #", " on source #", " on card #", " on server"]
        .iter()
        .any(|on| line.contains(on))
}

/// Whether a `pactl subscribe` line is a stream's change, which only the
/// open menu shows.
#[must_use]
pub fn streams(line: &str) -> bool {
    [" on sink-input #", " on source-output #"]
        .iter()
        .any(|on| line.contains(on))
}

/// `pactl`'s volume argument for a step of `delta` percent, or `None` for
/// no step. Relative and with no ceiling at 100 %, as the dotfiles'
/// `pactl set-sink-volume @DEFAULT_SINK@ +5%` has none (plan 07); `pactl`
/// stops a step down at 0.
#[must_use]
pub fn step_argument(delta: i32) -> Option<String> {
    match delta.signum() {
        1 => Some(format!("+{delta}%")),
        -1 => Some(format!("{delta}%")),
        _ => None,
    }
}

/// Runs `pactl` with `args` and returns what it printed.
async fn pactl(args: &[&str]) -> io::Result<String> {
    let output = Command::new("pactl")
        .args(args)
        .stdin(Stdio::null())
        .output()
        .await?;
    if !output.status.success() {
        return Err(io::Error::other(format!(
            "`pactl {}` failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    String::from_utf8(output.stdout).map_err(io::Error::other)
}

/// The modules' reading and the menu's nodes now.
///
/// # Errors
///
/// When `pactl` cannot run or reach the server, or writes something else.
pub async fn read() -> io::Result<(Reading, Nodes)> {
    let (info, sinks, sources, sink_inputs, source_outputs, cards) = tokio::try_join!(
        pactl(&["--format=json", "info"]),
        pactl(&["--format=json", "list", "sinks"]),
        pactl(&["--format=json", "list", "sources"]),
        pactl(&["--format=json", "list", "sink-inputs"]),
        pactl(&["--format=json", "list", "source-outputs"]),
        pactl(&["--format=json", "list", "cards"]),
    )?;
    let read = reading(&info, &sinks, &sources).and_then(|reading| {
        Ok((
            reading,
            nodes(
                &info,
                &sinks,
                &sources,
                &sink_inputs,
                &source_outputs,
                &cards,
            )?,
        ))
    });
    read.map_err(io::Error::other)
}

/// Sends what [`read`] reads to `states` now and after every change the
/// modules show, and every stream's change too while `menu` is set, until
/// `pactl subscribe` ends or every receiver is dropped.
///
/// # Errors
///
/// When `pactl` cannot run, or a read fails.
pub async fn watch(states: &watch::Sender<(Reading, Nodes)>, menu: &AtomicBool) -> io::Result<()> {
    // Subscribed before the first read, so a change in between is not lost.
    let mut subscribed = Command::new("pactl")
        .arg("subscribe")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()?;
    let stdout = subscribed
        .stdout
        .take()
        .ok_or_else(|| io::Error::other("`pactl subscribe` has no output"))?;
    let lines = futures_util::stream::unfold(BufReader::new(stdout).lines(), |mut lines| async {
        let line = lines.next_line().await.transpose()?;
        Some((line, lines))
    });
    let mut changes = Follow::new(Box::pin(lines));
    let wanted = |line: io::Result<String>| {
        line.map(|line| shows(&line) || (menu.load(Ordering::Relaxed) && streams(&line)))
    };
    states.send_replace(changes.during(read(), wanted).await??);
    while changes.changed(wanted).await? {
        if !publish(states, changes.during(read(), wanted).await??) {
            break;
        }
    }
    Ok(())
}

/// Reads again and hands `states` what changed, as [`watch`] does on a
/// change: for the menu opening, whose streams were not followed while it
/// was closed.
///
/// # Errors
///
/// When the read fails.
pub async fn refresh(states: &watch::Sender<(Reading, Nodes)>) -> io::Result<()> {
    publish(states, read().await?);
    Ok(())
}

/// Mutes or unmutes `target`.
///
/// # Errors
///
/// When `pactl` fails.
pub async fn toggle_mute(target: &Target) -> io::Result<()> {
    let (mute, _, id) = target.commands();
    pactl(&[mute, &id, "toggle"]).await.map(drop)
}

/// Moves `target`'s volume by `delta` percent, as [`step_argument`] has it.
///
/// # Errors
///
/// When `pactl` fails.
pub async fn step(target: &Target, delta: i32) -> io::Result<()> {
    let Some(volume) = step_argument(delta) else {
        return Ok(());
    };
    let (_, set, id) = target.commands();
    // `--` so a step down is not read as an option.
    pactl(&[set, &id, "--", &volume]).await.map(drop)
}

/// Sets `target`'s volume to `percent`.
///
/// # Errors
///
/// When `pactl` fails.
pub async fn set(target: &Target, percent: u32) -> io::Result<()> {
    let (_, set, id) = target.commands();
    pactl(&[set, &id, &format!("{percent}%")]).await.map(drop)
}

/// Moves the stream `index` of `flow` to the device `name`.
///
/// # Errors
///
/// When `pactl` fails.
pub async fn move_stream(flow: Flow, index: u32, name: &str) -> io::Result<()> {
    let command = match flow {
        Flow::Output => "move-sink-input",
        Flow::Input => "move-source-output",
    };
    pactl(&[command, &index.to_string(), name]).await.map(drop)
}

/// Switches the card `card` to its profile `profile`.
///
/// # Errors
///
/// When `pactl` fails.
pub async fn set_profile(card: &str, profile: &str) -> io::Result<()> {
    pactl(&["set-card-profile", card, profile]).await.map(drop)
}

/// Makes the device `name` of `flow` the default.
///
/// # Errors
///
/// When `pactl` fails.
pub async fn make_default(flow: Flow, name: &str) -> io::Result<()> {
    let command = match flow {
        Flow::Output => "set-default-sink",
        Flow::Input => "set-default-source",
    };
    pactl(&[command, name]).await.map(drop)
}

#[cfg(test)]
mod tests;
