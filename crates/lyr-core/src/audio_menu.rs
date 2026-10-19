//! The audio menu (`docs/plan/10-popups-and-menus.md#audio-menu`): the
//! playback and recording streams and the output and input devices, a tab
//! each, every row with its mute, its volume stepped down and up, and on a
//! device "make default". The platforms' sources fill it in
//! (`lyr_sources::audio`). wiremix's fifth tab, card configuration, waits on
//! a source that has cards (Linux).

use lyr_config::Theme;

use crate::draw::{DrawList, Size};
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
}

impl Tab {
    /// Every tab, in the order the menu shows them, wiremix's.
    pub const ALL: [Self; 4] = [Self::Playback, Self::Recording, Self::Output, Self::Input];

    /// Its label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Playback => "Playback",
            Self::Recording => "Recording",
            Self::Output => "Output devices",
            Self::Input => "Input devices",
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
}

/// Everything the menu lists, a list per tab.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Nodes {
    pub playback: Vec<Node>,
    pub recording: Vec<Node>,
    pub outputs: Vec<Node>,
    pub inputs: Vec<Node>,
}

impl Nodes {
    /// The rows `tab` shows.
    #[must_use]
    pub fn of(&self, tab: Tab) -> &[Node] {
        match tab {
            Tab::Playback => &self.playback,
            Tab::Recording => &self.recording,
            Tab::Output => &self.outputs,
            Tab::Input => &self.inputs,
        }
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
    /// A device's "make default" button.
    Default(usize),
}

/// The step buttons' labels.
const DOWN: &str = "-";
const UP: &str = "+";
/// The default button's label.
const MAKE_DEFAULT: &str = "Default";

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
    let tabs_width = Tab::ALL
        .iter()
        .map(|&each| {
            if each == tab {
                measure(each.label())
            } else {
                button_width(each.label())
            }
        })
        .sum::<f32>()
        + GAP * (Tab::ALL.len() - 1) as f32;
    let name_width = rows
        .iter()
        .map(|node| measure(&node.name))
        .fold(measure(tab.empty()), f32::max);
    // Each column as wide as its widest label, so the rows line up.
    let mute_width = button_width("Unmute");
    let level_width = measure("100%");
    let controls_width = mute_width
        + GAP
        + button_width(DOWN)
        + GAP
        + level_width
        + GAP
        + button_width(UP)
        + if tab.devices() {
            GAP + button_width(MAKE_DEFAULT)
        } else {
            0.0
        };
    let width = 2.0 * PADDING + tabs_width.max(name_width + GAP + controls_width);
    let mut items = Vec::new();
    let mut regions = Regions::default();
    let text =
        |text: String, x: f32, y: f32, colour| menu::text(text, x, y, measure, line_height, colour);
    let mut button = |items: &mut Vec<_>, x: f32, y: f32, label: &str, target| {
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
    for each in Tab::ALL {
        x += if each == tab {
            items.push(text(each.label().to_owned(), x, y, theme.workspace_active));
            measure(each.label())
        } else {
            button(&mut items, x, y, each.label(), Target::Tab(each))
        } + GAP;
    }
    y += row + 2.0 * GAP;
    if rows.is_empty() {
        items.push(text(tab.empty().to_owned(), PADDING, y, theme.foreground));
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
        button(&mut items, x, y, mute, Target::Mute(index));
        x += mute_width + GAP;
        x += button(&mut items, x, y, DOWN, Target::Step(index, Step::Down)) + GAP;
        let level = format!("{}%", node.volume);
        // Right-aligned in its column, as a number reads.
        items.push(text(
            level.clone(),
            x + level_width - measure(&level),
            y,
            theme.foreground,
        ));
        x += level_width + GAP;
        x += button(&mut items, x, y, UP, Target::Step(index, Step::Up)) + GAP;
        if tab.devices() && !node.default {
            button(&mut items, x, y, MAKE_DEFAULT, Target::Default(index));
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
