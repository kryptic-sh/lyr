//! The volume and mic modules (`docs/plan/07-modules.md#volume`), following
//! waybar 0.15.0's `pulseaudio.cpp`. waybar draws both from one module; lyr
//! has one per device, so the mic shows the source's description in its
//! tooltip, not the sink's.

use lyr_config::{Format, Mic, Volume};

use crate::icon::pick;

/// The port kinds waybar looks for in a sink's port name and form factor, in
/// the order it tries them (`ports` in `pulseaudio.cpp`).
const PORTS: [&str; 9] = [
    "headphone",
    "speaker",
    "hdmi",
    "headset",
    "hands-free",
    "portable",
    "car",
    "hifi",
    "phone",
];

/// The default sink.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Sink {
    /// Percent; above 100 when boosted.
    pub volume: u32,
    pub muted: bool,
    /// The sink's name, such as `alsa_output.pci-0000_00_1f.3.analog-stereo`.
    pub name: String,
    /// The default source's name, which waybar also tries as an icon name.
    pub source_name: String,
    /// The active port's name followed by the form factor, as waybar joins
    /// them, such as `analog-output-headphones` + `headset`.
    pub port: String,
    pub description: String,
    pub bluetooth: bool,
}

/// The default source.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Source {
    /// Percent.
    pub volume: u32,
    pub muted: bool,
    pub description: String,
}

/// What a module draws.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct View {
    pub text: String,
    pub tooltip: String,
}

/// The names waybar tries for the volume icon, in order (`getPulseIcon`):
/// the sink, the source, the first port kind the port mentions, each with a
/// `-muted` form first while muted, and `default-muted` last.
fn icon_names(sink: &Sink) -> Vec<String> {
    let mut names = Vec::new();
    let muted = |names: &mut Vec<String>, name: &str| {
        if sink.muted {
            names.push(format!("{name}-muted"));
        }
    };
    muted(&mut names, &sink.name);
    names.push(sink.name.clone());
    names.push(sink.source_name.clone());
    let port = sink.port.to_lowercase();
    if let Some(kind) = PORTS.into_iter().find(|kind| port.contains(kind)) {
        muted(&mut names, kind);
        names.push(kind.to_owned());
    }
    if sink.muted {
        names.push("default-muted".to_owned());
    }
    names
}

/// The volume module's view of `sink`.
#[must_use]
pub fn volume(sink: &Sink, config: &Volume) -> View {
    let format = match (sink.bluetooth, sink.muted) {
        (true, false) => config.format_bluetooth.as_ref().unwrap_or(&config.format),
        // waybar falls back to the plain muted format, never to the unmuted
        // Bluetooth one.
        (true, true) => config
            .format_bluetooth_muted
            .as_ref()
            .unwrap_or(&config.format_muted),
        (false, false) => &config.format,
        (false, true) => &config.format_muted,
    };
    let names = icon_names(sink);
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    let icon = pick(&config.icons, &names, sink.volume);
    let volume = sink.volume.to_string();
    let fill = |format: &Format| {
        format.render(|name| match name {
            "icon" => Some(icon),
            "volume" => Some(volume.as_str()),
            "desc" => Some(sink.description.as_str()),
            _ => None,
        })
    };
    View {
        text: fill(format),
        tooltip: config
            .tooltip_format
            .as_ref()
            .map_or_else(|| sink.description.clone(), fill),
    }
}

/// The mic module's view of `source`.
#[must_use]
pub fn mic(source: &Source, config: &Mic) -> View {
    let format = if source.muted {
        &config.format_muted
    } else {
        &config.format
    };
    let volume = source.volume.to_string();
    let fill = |format: &Format| {
        format.render(|name| match name {
            "volume" => Some(volume.as_str()),
            "desc" => Some(source.description.as_str()),
            _ => None,
        })
    };
    View {
        text: fill(format),
        tooltip: config
            .tooltip_format
            .as_ref()
            .map_or_else(|| source.description.clone(), fill),
    }
}

#[cfg(test)]
mod tests;
