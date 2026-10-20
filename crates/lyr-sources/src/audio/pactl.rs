//! The volume and mic modules' source on Linux, through `pactl`, the
//! PulseAudio client that pipewire-pulse serves as PulseAudio does, so no
//! client library is linked (`libpulse-binding` is a question in
//! `docs/backlog.md`). Changes come from `pactl subscribe`, read again only
//! for sink, source and server events; readings follow waybar 0.15.0's
//! `AudioBackend` (`src/util/audio_backend.cpp`) for the default sink and
//! source.

use std::collections::HashMap;
use std::io;
use std::process::Stdio;

use lyr_core::audio::{Sink, Source};
use serde::Deserialize;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::watch;

use crate::bus::{Follow, publish};

/// PulseAudio's `PA_VOLUME_NORM`: a channel at 100 %.
const VOLUME_NORM: u64 = 0x1_0000;

/// The most a wheel step raises a volume to, in percent: waybar's
/// `max-volume` default, which `pa_cvolume_inc_clamp` holds a step to.
const MAX_STEP_PERCENT: u32 = 100;

/// What a sink's monitor source is called when the sink is Bluetooth
/// (waybar's `isBluetooth`: PulseAudio's, PipeWire's, and BlueZ's own).
const BLUETOOTH_MONITORS: [&str; 3] = ["a2dp_sink", "a2dp-sink", "bluez"];

/// The default sink and source, as the volume and mic modules show them.
pub type Reading = (Option<Sink>, Option<Source>);

/// Which default device a control acts on: the volume module's output or
/// the mic module's input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flow {
    Output,
    Input,
}

impl Flow {
    /// `pactl`'s name for the default device of this flow.
    const fn default_device(self) -> &'static str {
        match self {
            Self::Output => "@DEFAULT_SINK@",
            Self::Input => "@DEFAULT_SOURCE@",
        }
    }

    /// `pactl`'s commands that set this flow's mute and volume.
    const fn commands(self) -> (&'static str, &'static str) {
        match self {
            Self::Output => ("set-sink-mute", "set-sink-volume"),
            Self::Input => ("set-source-mute", "set-source-volume"),
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
struct Node {
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
    let sinks: Vec<Node> = serde_json::from_str(sinks)?;
    let sources: Vec<Node> = serde_json::from_str(sources)?;
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

/// Whether a `pactl subscribe` line is a change the modules show: a sink, a
/// source, or the server's defaults, not a stream's.
#[must_use]
pub fn shows(line: &str) -> bool {
    [" on sink #", " on source #", " on server"]
        .iter()
        .any(|on| line.contains(on))
}

/// `pactl`'s volume argument for a step of `delta` percent from `current`:
/// up to [`MAX_STEP_PERCENT`] and no further, and never down from a volume
/// already above it; `None` when the step changes nothing.
#[must_use]
pub fn step_argument(current: u32, delta: i32) -> Option<String> {
    match delta.signum() {
        1 if current >= MAX_STEP_PERCENT => None,
        1 => Some(match current.saturating_add(delta.unsigned_abs()) {
            target if target >= MAX_STEP_PERCENT => format!("{MAX_STEP_PERCENT}%"),
            _ => format!("+{delta}%"),
        }),
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

/// The default sink and source now.
///
/// # Errors
///
/// When `pactl` cannot run or reach the server, or writes something else.
pub async fn read() -> io::Result<Reading> {
    let (info, sinks, sources) = tokio::try_join!(
        pactl(&["--format=json", "info"]),
        pactl(&["--format=json", "list", "sinks"]),
        pactl(&["--format=json", "list", "sources"]),
    )?;
    reading(&info, &sinks, &sources).map_err(io::Error::other)
}

/// Sends the reading to `readings` now and after every change the modules
/// show, until `pactl subscribe` ends or every receiver is dropped.
///
/// # Errors
///
/// When `pactl` cannot run, or a read fails.
pub async fn watch(readings: &watch::Sender<Reading>) -> io::Result<()> {
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
    let wanted = |line: io::Result<String>| line.map(|line| shows(&line));
    readings.send_replace(changes.during(read(), wanted).await??);
    while changes.changed(wanted).await? {
        if !publish(readings, changes.during(read(), wanted).await??) {
            break;
        }
    }
    Ok(())
}

/// Mutes or unmutes `flow`'s default device.
///
/// # Errors
///
/// When `pactl` fails.
pub async fn toggle_mute(flow: Flow) -> io::Result<()> {
    let (mute, _) = flow.commands();
    pactl(&[mute, flow.default_device(), "toggle"])
        .await
        .map(drop)
}

/// Moves `flow`'s default device by `delta` percent from `current`, as
/// [`step_argument`] has it.
///
/// # Errors
///
/// When `pactl` fails.
pub async fn step(flow: Flow, current: u32, delta: i32) -> io::Result<()> {
    let Some(volume) = step_argument(current, delta) else {
        return Ok(());
    };
    let (_, set) = flow.commands();
    // `--` so a step down is not read as an option.
    pactl(&[set, flow.default_device(), "--", &volume])
        .await
        .map(drop)
}

#[cfg(test)]
mod tests;
