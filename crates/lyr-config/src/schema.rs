//! The config's shape: what each table holds and its types.
//!
//! Every table refuses unknown keys, so a misspelt key is an error rather than
//! a setting that silently does nothing.

use serde::Deserialize;

use crate::colour::Colour;
use crate::format::Format;

/// The whole config, after the user file is layered over the defaults. It is
/// built a table at a time (`typed` in the crate root), so that an error names
/// its table.
#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    pub bar: Bar,
    pub layout: Layout,
    pub workspaces: Workspaces,
    pub clock: Clock,
    pub theme: Theme,
}

/// One bar per output: where it sits and how tall it is.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Bar {
    pub position: Position,
    /// Logical pixels.
    pub height: u32,
    /// Output names to put a bar on. Empty means every output.
    pub output: Vec<String>,
    /// The layer-shell layer. Wayland only; ignored elsewhere.
    pub layer: Layer,
}

/// Which screen edge the bar is on.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Position {
    Top,
    Bottom,
}

/// The Wayland layer-shell layer the bar is placed in.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Layer {
    Background,
    Bottom,
    Top,
    Overlay,
}

/// The three zones, each a list of modules in the order they are drawn.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Layout {
    pub left: Vec<Item>,
    pub center: Vec<Item>,
    pub right: Vec<Item>,
}

impl Layout {
    /// Every zone's items, with the key path each one sits at, so an error
    /// can name it.
    pub(crate) fn items(&self) -> impl Iterator<Item = (String, &Item)> {
        [
            ("left", &self.left),
            ("center", &self.center),
            ("right", &self.right),
        ]
        .into_iter()
        .flat_map(|(zone, items)| {
            items
                .iter()
                .enumerate()
                .map(move |(i, item)| (format!("layout.{zone}[{i}]"), item))
        })
    }
}

/// One entry in a zone: a module, a fixed gap, or a thin rule.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum Item {
    /// `"separator"` or a module name.
    Named(String),
    /// `{ spacer = 20 }`: an empty gap this many logical pixels wide.
    Spacer { spacer: u32 },
}

/// The modules a zone can name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Module {
    Power,
    Battery,
    PowerProfile,
    Network,
    Bluetooth,
    Workspaces,
    Tray,
    Volume,
    Mic,
    Clock,
    Notifications,
}

impl Module {
    /// Every module, for error messages that list what a name could be.
    pub const ALL: [Self; 11] = [
        Self::Power,
        Self::Battery,
        Self::PowerProfile,
        Self::Network,
        Self::Bluetooth,
        Self::Workspaces,
        Self::Tray,
        Self::Volume,
        Self::Mic,
        Self::Clock,
        Self::Notifications,
    ];

    /// The name a zone uses for this module.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Power => "power",
            Self::Battery => "battery",
            Self::PowerProfile => "power_profile",
            Self::Network => "network",
            Self::Bluetooth => "bluetooth",
            Self::Workspaces => "workspaces",
            Self::Tray => "tray",
            Self::Volume => "volume",
            Self::Mic => "mic",
            Self::Clock => "clock",
            Self::Notifications => "notifications",
        }
    }

    /// The module a zone entry names, if any.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|module| module.name() == name)
    }
}

/// The workspace module (`docs/plan/06-workspaces.md`).
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Workspaces {
    /// Every bar shows every workspace, rather than its own output's.
    pub all_outputs: bool,
    /// A button's label. `{name}` is the workspace's name.
    pub format: Format,
    /// Scrolling over the module steps to the next or previous workspace.
    pub scroll: bool,
}

/// The clock module.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Clock {
    /// A strftime format.
    pub format: String,
}

/// Colours, font and box metrics (`docs/plan/04-rendering-and-theme.md`).
/// Lengths are logical pixels.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Theme {
    pub font: String,
    pub font_size: u32,
    pub foreground: Colour,
    pub bar_background: Colour,
    pub module_background: Colour,
    /// Applied to the whole module box, text included, like CSS `opacity`.
    pub module_opacity: f32,
    pub border: Colour,
    pub border_width: u32,
    pub radius: u32,
    pub module_padding_x: u32,
    pub module_margin_top: u32,
    pub module_margin_bottom: u32,
    pub module_margin_left: u32,
    pub module_min_width: u32,
    /// The right margin of the last module in the right zone.
    pub last_module_margin_right: u32,
    pub workspace_padding_x: u32,
    /// The text of the active and the hovered workspace button.
    pub workspace_active: Colour,
    /// Urgent workspace buttons. Unset leaves them looking like any other,
    /// as waybar's style does.
    #[serde(default)]
    pub workspace_urgent: Option<Colour>,
    pub battery_warning: Colour,
    pub battery_critical: Colour,
    pub volume: Colour,
    pub mic: Colour,
    pub tooltip_background: Colour,
    pub tooltip_foreground: Colour,
}
