//! The config's shape: what each table holds and its types.
//!
//! Every table refuses unknown keys, so a misspelt key is an error rather than
//! a setting that silently does nothing.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::actions::{On, PowerMenu};
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
    pub power: Power,
    pub clock: Clock,
    pub battery: Battery,
    pub power_profile: PowerProfile,
    pub network: Network,
    pub bluetooth: Bluetooth,
    pub volume: Volume,
    pub mic: Mic,
    pub notifications: Notifications,
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

/// The power button (`docs/plan/07-modules.md#power-button`).
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Power {
    /// No placeholders: the button is a glyph.
    pub format: Format,
    pub tooltip_format: Format,
    /// The menu's entries set to a command instead of lyr's own action.
    #[serde(default)]
    pub menu: PowerMenu,
    /// What a click or a scroll does.
    #[serde(default)]
    pub on: On,
}

/// The notification bell (`docs/plan/09-notifications.md#the-bell-module`)
/// and the daemon behind it.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Notifications {
    /// Take `org.freedesktop.Notifications` from a daemon that already has
    /// it, such as swaync. Off, lyr leaves notifications to that daemon.
    #[serde(default)]
    pub replace: bool,
    /// `{icon}`.
    pub format: Format,
    /// By state: `notification` and `none`, and their `dnd-` forms while
    /// do-not-disturb is on, waybar's names for swaync's states.
    pub icons: NamedIcons,
    /// What a click or a scroll does.
    #[serde(default)]
    pub on: On,
}

/// The clock module.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Clock {
    /// A strftime format.
    pub format: String,
    /// What a click or a scroll does.
    #[serde(default)]
    pub on: On,
}

/// The battery module (`docs/plan/07-modules.md#battery`), with waybar's keys
/// in lyr's spelling.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Battery {
    /// `{icon}`, `{capacity}`, `{power}` and `{time}`.
    pub format: Format,
    /// Used while charging instead of `format`.
    #[serde(default)]
    pub format_charging: Option<Format>,
    /// Used while on mains power but not charging (held at a charge limit).
    #[serde(default)]
    pub format_plugged: Option<Format>,
    /// Used at 100 % instead of `format`.
    #[serde(default)]
    pub format_full: Option<Format>,
    /// The icon ramp, emptiest first.
    pub icons: Vec<String>,
    /// `{timeTo}`, `{capacity}`, `{power}` and `{time}`.
    pub tooltip_format: Format,
    /// At or below this percentage the module is in its warning state.
    pub warning: u8,
    /// At or below this percentage the module is in its critical state.
    pub critical: u8,
    /// What a click or a scroll does.
    #[serde(default)]
    pub on: On,
}

/// Icons by name, with `default` for a name that has none: waybar's
/// `format-icons` object.
pub type NamedIcons = BTreeMap<String, Icon>;

/// One entry of [`NamedIcons`]: a single icon, or a ramp chosen by a
/// percentage (the volume's `default`).
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum Icon {
    One(String),
    Ramp(Vec<String>),
}

/// The power-profile module (`docs/plan/07-modules.md#power-profile`).
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PowerProfile {
    /// `{icon}`, `{profile}` and `{driver}`.
    pub format: Format,
    /// The same placeholders as `format`.
    pub tooltip_format: Format,
    /// By profile name (`performance`, `balanced`, `power-saver`).
    pub icons: NamedIcons,
    /// What a click or a scroll does.
    #[serde(default)]
    pub on: On,
}

/// The network module (`docs/plan/07-modules.md#network`). Each state's
/// format, when set, replaces the plain one in that state, as waybar's
/// `format-<state>` does.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Network {
    /// `{icon}`, `{ifname}`, `{ipaddr}`, `{cidr}`, `{gwaddr}`, `{essid}` and
    /// `{signalStrength}`, as every format below.
    pub format: Format,
    #[serde(default)]
    pub format_wifi: Option<Format>,
    #[serde(default)]
    pub format_ethernet: Option<Format>,
    #[serde(default)]
    pub format_linked: Option<Format>,
    #[serde(default)]
    pub format_disconnected: Option<Format>,
    #[serde(default)]
    pub format_disabled: Option<Format>,
    pub tooltip_format: Format,
    #[serde(default)]
    pub tooltip_format_wifi: Option<Format>,
    #[serde(default)]
    pub tooltip_format_ethernet: Option<Format>,
    #[serde(default)]
    pub tooltip_format_linked: Option<Format>,
    #[serde(default)]
    pub tooltip_format_disconnected: Option<Format>,
    #[serde(default)]
    pub tooltip_format_disabled: Option<Format>,
    /// By state (`wifi`, `ethernet`, `linked`, `disconnected`, `disabled`).
    pub icons: NamedIcons,
    /// What a click or a scroll does.
    #[serde(default)]
    pub on: On,
}

impl Network {
    /// Every format with its key, for validation.
    pub(crate) fn formats(&self) -> [(&'static str, Option<&Format>); 12] {
        [
            ("network.format", Some(&self.format)),
            ("network.format_wifi", self.format_wifi.as_ref()),
            ("network.format_ethernet", self.format_ethernet.as_ref()),
            ("network.format_linked", self.format_linked.as_ref()),
            (
                "network.format_disconnected",
                self.format_disconnected.as_ref(),
            ),
            ("network.format_disabled", self.format_disabled.as_ref()),
            ("network.tooltip_format", Some(&self.tooltip_format)),
            (
                "network.tooltip_format_wifi",
                self.tooltip_format_wifi.as_ref(),
            ),
            (
                "network.tooltip_format_ethernet",
                self.tooltip_format_ethernet.as_ref(),
            ),
            (
                "network.tooltip_format_linked",
                self.tooltip_format_linked.as_ref(),
            ),
            (
                "network.tooltip_format_disconnected",
                self.tooltip_format_disconnected.as_ref(),
            ),
            (
                "network.tooltip_format_disabled",
                self.tooltip_format_disabled.as_ref(),
            ),
        ]
    }
}

/// The bluetooth module (`docs/plan/07-modules.md#bluetooth`).
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Bluetooth {
    /// `{icon}`, `{status}`, `{num_connections}`, `{device_alias}` and
    /// `{device_battery_percentage}`, as every format below.
    pub format: Format,
    #[serde(default)]
    pub format_on: Option<Format>,
    #[serde(default)]
    pub format_off: Option<Format>,
    #[serde(default)]
    pub format_disabled: Option<Format>,
    #[serde(default)]
    pub format_connected: Option<Format>,
    /// Used while a connected device reports its battery.
    #[serde(default)]
    pub format_connected_battery: Option<Format>,
    pub tooltip_format: Format,
    /// By state (`off`, `on`, `connected`, `disabled`).
    pub icons: NamedIcons,
    /// What a click or a scroll does.
    #[serde(default)]
    pub on: On,
}

/// The volume module (`docs/plan/07-modules.md#volume`): the default sink.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Volume {
    /// `{icon}`, `{volume}` and `{desc}`, as every format below.
    pub format: Format,
    pub format_muted: Format,
    /// Used for a Bluetooth sink instead of `format`.
    #[serde(default)]
    pub format_bluetooth: Option<Format>,
    /// Used for a muted Bluetooth sink instead of `format_muted`.
    #[serde(default)]
    pub format_bluetooth_muted: Option<Format>,
    /// Unset shows the sink's description, as waybar does.
    #[serde(default)]
    pub tooltip_format: Option<Format>,
    /// By sink name, source name or port kind (`headphone`, `speaker`, `hdmi`,
    /// `headset`, `hands-free`, `portable`, `car`, `hifi`, `phone`), each also
    /// with `-muted`; `default` is usually a ramp by volume.
    pub icons: NamedIcons,
    /// Percent per `volume_up` or `volume_down`, as `pactl ... +5%` steps.
    pub scroll_step: u32,
    /// What a click or a scroll does.
    #[serde(default)]
    pub on: On,
}

/// The mic module (`docs/plan/07-modules.md#mic`): the default source.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Mic {
    /// `{volume}` and `{desc}`, as every format below.
    pub format: Format,
    pub format_muted: Format,
    /// Unset shows the source's description.
    #[serde(default)]
    pub tooltip_format: Option<Format>,
    /// Percent per `volume_up` or `volume_down`, as `pactl ... +5%` steps.
    pub scroll_step: u32,
    /// What a click or a scroll does.
    #[serde(default)]
    pub on: On,
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
    /// A notification toast: swaync's `.notification` in the dotfiles.
    pub toast_background: Colour,
    pub toast_foreground: Colour,
    /// A normal notification's border.
    pub toast_border: Colour,
    pub toast_border_low: Colour,
    pub toast_border_critical: Colour,
    /// When it arrived.
    pub toast_time: Colour,
}
