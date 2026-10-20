//! The audio menu (`docs/plan/10-popups-and-menus.md#audio-menu`): the
//! playback and recording streams and the output and input devices, a tab
//! each, every row with its mute, its volume as a slider stepped down and up,
//! on a device "make default", and on a stream that can move the device it
//! plays on, which a click moves on to the next; wiremix's fifth tab sets
//! cards' profiles and devices' ports where the source lists them (Linux).
//! The platforms' sources fill it in (`lyr_sources::audio`).

use lyr_config::Theme;

use crate::draw::{DrawList, Item, Size};
use crate::hit::{Rect, Regions};
use crate::menu::{self, button_width};
pub use crate::menu::{GAP, PADDING};

/// One of the menu's tabs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Playback,
    Recording,
    Output,
    Input,
    /// The sound cards' profiles and the devices' ports, where the platform
    /// has them.
    Configuration,
}

impl Tab {
    /// Every tab, in the order the menu shows them, wiremix's.
    pub const ALL: [Self; 5] = [
        Self::Playback,
        Self::Recording,
        Self::Output,
        Self::Input,
        Self::Configuration,
    ];

    /// Its label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Playback => "Playback",
            Self::Recording => "Recording",
            Self::Output => "Output devices",
            Self::Input => "Input devices",
            Self::Configuration => "Configuration",
        }
    }

    /// Whether its rows are devices, which can be made the default.
    #[must_use]
    pub const fn devices(self) -> bool {
        matches!(self, Self::Output | Self::Input)
    }

    /// What it shows with no rows.
    const fn empty(self) -> &'static str {
        match self {
            Self::Playback => "Nothing is playing",
            Self::Recording => "Nothing is recording",
            Self::Output | Self::Input => "No devices",
            Self::Configuration => "Nothing to configure",
        }
    }
}

/// A stream or a device.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node {
    /// What identifies it to its source.
    pub id: String,
    pub name: String,
    /// Percent.
    pub volume: u32,
    pub muted: bool,
    /// The default device; never set on a stream.
    pub default: bool,
    /// The device a stream plays to or records from, by its id, where the
    /// source can move it to another; `None` on a device, and on a stream
    /// that cannot move.
    pub device: Option<String>,
}

/// What a configuration row chooses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A sound card's profile, such as "Analog Stereo Output + Mono Input".
    Profile,
    /// An output device's port, such as "Headphones" or "Speaker".
    OutputPort,
    /// An input device's port.
    InputPort,
}

/// A row of the configuration tab: a card or a device, and the option of
/// its kind it has chosen among those available.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Setting {
    /// What identifies the card or device to its source.
    pub id: String,
    pub name: String,
    pub kind: Kind,
    /// The options available, by id and description, in the order a click
    /// steps through them.
    pub options: Vec<(String, String)>,
    /// The chosen option's id.
    pub active: String,
}

impl Setting {
    /// The chosen option's description, else its id.
    #[must_use]
    pub fn chosen(&self) -> &str {
        self.options
            .iter()
            .find(|(id, _)| *id == self.active)
            .map_or(&self.active, |(_, description)| description)
    }
}

/// Everything the menu lists, a list per tab.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Nodes {
    pub playback: Vec<Node>,
    pub recording: Vec<Node>,
    pub outputs: Vec<Node>,
    pub inputs: Vec<Node>,
    /// The configuration tab's rows, cards' profiles and devices' ports, or
    /// `None` where the platform has nothing to configure, which hides it.
    pub settings: Option<Vec<Setting>>,
}

impl Nodes {
    /// The rows `tab` shows; the configuration tab has none of these.
    #[must_use]
    pub fn of(&self, tab: Tab) -> &[Node] {
        match tab {
            Tab::Playback => &self.playback,
            Tab::Recording => &self.recording,
            Tab::Output => &self.outputs,
            Tab::Input => &self.inputs,
            Tab::Configuration => &[],
        }
    }

    /// The tabs to show: every one, but the configuration tab only where
    /// there is something to configure.
    pub fn tabs(&self) -> impl Iterator<Item = Tab> + '_ {
        Tab::ALL
            .into_iter()
            .filter(|&tab| tab != Tab::Configuration || self.settings.is_some())
    }
}

/// Which way a step moves a volume.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Down,
    Up,
}

/// What a click in the menu lands on. A row is by its index in the shown
/// tab's list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Tab(Tab),
    Mute(usize),
    Step(usize, Step),
    /// A row's slider: a click sets the volume where it lands.
    Level(usize),
    /// A device's "make default" button.
    Default(usize),
    /// A stream's device: a click moves it to the next of its flow.
    Route(usize),
    /// A configuration row's choice: a click moves it to the next option.
    Setting(usize),
}

/// The step buttons' labels.
const DOWN: &str = "-";
const UP: &str = "+";
/// The default button's label.
const MAKE_DEFAULT: &str = "Default";
/// A slider's width.
const SLIDER_WIDTH: f32 = 120.0;
/// A slider's track, as a share of the row's height.
const TRACK: f32 = 0.4;
/// The volume a full slider stands for: on Linux 150 %, wiremix's
/// `max_volume_percent` (plan 10), since PulseAudio's volumes go past 100 %;
/// elsewhere 100 %, where Windows' and macOS' endpoints stop.
pub const MAX_PERCENT: u32 = if cfg!(target_os = "linux") { 150 } else { 100 };

/// The volume a click `across` a slider sets, 0 – 1 of [`MAX_PERCENT`].
#[must_use]
pub fn level_at(across: f32) -> u32 {
    // `across` is 0 – 1, so the product is in range and `as` cannot wrap.
    (across.clamp(0.0, 1.0) * MAX_PERCENT as f32).round() as u32
}

/// The name `node`'s device goes by among `devices`, or its id when they
/// do not list it; `None` for a node with no device.
fn device_name<'a>(node: &'a Node, devices: &'a [Node]) -> Option<&'a str> {
    let id = node.device.as_deref()?;
    Some(
        devices
            .iter()
            .find(|device| device.id == id)
            .map_or(id, |device| device.name.as_str()),
    )
}

/// A slider filling `rect` at `volume` percent: its track, and its level
/// in the switch's on colour, or the border's when `muted`.
fn slider(rect: Rect, volume: u32, muted: bool, theme: &Theme) -> [Item; 2] {
    let track = Rect {
        y: rect.y + rect.height * (1.0 - TRACK) / 2.0,
        height: rect.height * TRACK,
        ..rect
    };
    let shown = volume.min(MAX_PERCENT) as f32 / MAX_PERCENT as f32;
    [
        Item::Box {
            rect: track,
            radius: track.height / 2.0,
            fill: theme.toast_background,
            border: Some((theme.border_width as f32, theme.panel_border)),
        },
        Item::Box {
            rect: Rect {
                width: track.width * shown,
                ..track
            },
            radius: track.height / 2.0,
            fill: if muted { theme.border } else { theme.switch_on },
            border: None,
        },
    ]
}

/// The menu showing `tab` of `nodes`: its draw list, its size, and what each
/// click lands on, its text measured by `measure` in lines `line_height`
/// tall. The shown tab is its label in the active colour; the others are
/// buttons.
#[must_use]
pub fn draw(
    nodes: &Nodes,
    tab: Tab,
    focus: Option<Target>,
    measure: &dyn Fn(&str) -> f32,
    line_height: f32,
    theme: &Theme,
) -> (DrawList, Size, Regions<Target>) {
    let row = menu::row(line_height);
    let button_width = |label: &str| button_width(label, measure);
    let rows = nodes.of(tab);
    let tabs_width = nodes
        .tabs()
        .map(|each| {
            if each == tab {
                measure(each.label())
            } else {
                button_width(each.label())
            }
        })
        .sum::<f32>()
        + GAP * (nodes.tabs().count() - 1) as f32;
    let settings = match (tab, &nodes.settings) {
        (Tab::Configuration, Some(settings)) => &settings[..],
        _ => &[],
    };
    let name_width = rows
        .iter()
        .map(|node| measure(&node.name))
        .chain(settings.iter().map(|setting| measure(&setting.name)))
        .fold(measure(tab.empty()), f32::max);
    // Each column as wide as its widest label, so the rows line up.
    let mute_width = button_width("Unmute");
    let level_width = measure("100%");
    let controls_width = mute_width
        + GAP
        + button_width(DOWN)
        + GAP
        + SLIDER_WIDTH
        + GAP
        + level_width
        + GAP
        + button_width(UP)
        + if tab.devices() {
            GAP + button_width(MAKE_DEFAULT)
        } else {
            0.0
        };
    // A stream's device by its name, as its flow's tab lists it.
    let devices = match tab {
        Tab::Playback => &nodes.outputs[..],
        Tab::Recording => &nodes.inputs[..],
        Tab::Output | Tab::Input | Tab::Configuration => &[],
    };
    let route_width = rows
        .iter()
        .filter_map(|node| device_name(node, devices))
        .map(|name| GAP + button_width(name))
        .fold(0.0, f32::max);
    let controls_width = if tab == Tab::Configuration {
        settings
            .iter()
            .map(|setting| button_width(setting.chosen()))
            .fold(0.0, f32::max)
    } else {
        controls_width + route_width
    };
    let width = 2.0 * PADDING + tabs_width.max(name_width + GAP + controls_width);
    let mut items = Vec::new();
    let mut regions = Regions::default();
    let text =
        |text: String, x: f32, y: f32, colour| menu::text(text, x, y, measure, line_height, colour);
    let button =
        |items: &mut Vec<_>, regions: &mut Regions<Target>, x: f32, y: f32, label: &str, target| {
            let rect = Rect {
                x,
                y,
                width: button_width(label),
                height: row,
            };
            items.extend(menu::button(rect, label, measure, line_height, theme));
            regions.push(rect, target);
            rect.width
        };
    let mut y = PADDING;
    let mut x = PADDING;
    for each in nodes.tabs() {
        x += if each == tab {
            items.push(text(each.label().to_owned(), x, y, theme.workspace_active));
            measure(each.label())
        } else {
            button(
                &mut items,
                &mut regions,
                x,
                y,
                each.label(),
                Target::Tab(each),
            )
        } + GAP;
    }
    y += row + 2.0 * GAP;
    if rows.is_empty() && settings.is_empty() {
        items.push(text(tab.empty().to_owned(), PADDING, y, theme.foreground));
        y += row + GAP;
    }
    for (index, setting) in settings.iter().enumerate() {
        items.push(text(setting.name.clone(), PADDING, y, theme.foreground));
        let x = PADDING + name_width + GAP;
        button(
            &mut items,
            &mut regions,
            x,
            y,
            setting.chosen(),
            Target::Setting(index),
        );
        y += row + GAP;
    }
    for (index, node) in rows.iter().enumerate() {
        let colour = if node.default {
            theme.workspace_active
        } else {
            theme.foreground
        };
        items.push(text(node.name.clone(), PADDING, y, colour));
        let mut x = PADDING + name_width + GAP;
        let mute = if node.muted { "Unmute" } else { "Mute" };
        button(&mut items, &mut regions, x, y, mute, Target::Mute(index));
        x += mute_width + GAP;
        x += button(
            &mut items,
            &mut regions,
            x,
            y,
            DOWN,
            Target::Step(index, Step::Down),
        ) + GAP;
        let rect = Rect {
            x,
            y,
            width: SLIDER_WIDTH,
            height: row,
        };
        items.extend(slider(rect, node.volume, node.muted, theme));
        regions.push(rect, Target::Level(index));
        x += SLIDER_WIDTH + GAP;
        let level = format!("{}%", node.volume);
        // Right-aligned in its column, as a number reads.
        items.push(text(
            level.clone(),
            x + level_width - measure(&level),
            y,
            theme.foreground,
        ));
        x += level_width + GAP;
        x += button(
            &mut items,
            &mut regions,
            x,
            y,
            UP,
            Target::Step(index, Step::Up),
        ) + GAP;
        if tab.devices() && !node.default {
            button(
                &mut items,
                &mut regions,
                x,
                y,
                MAKE_DEFAULT,
                Target::Default(index),
            );
        }
        if let Some(name) = device_name(node, devices) {
            button(&mut items, &mut regions, x, y, name, Target::Route(index));
        }
        y += row + GAP;
    }
    let size = Size {
        width,
        height: y - GAP + PADDING,
    };
    let focus = focus.and_then(|focus| regions.rect(&focus));
    (menu::frame(size, items, focus, theme), size, regions)
}

#[cfg(test)]
mod tests;
