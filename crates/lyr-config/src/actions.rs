//! What a click or a scroll on a module does
//! (`docs/plan/05-layout-and-config.md`).
//!
//! A module's `[<module>.on]` table binds each pointer action to a built-in
//! action or to a command. A built-in acts on the module it is bound in, so
//! `toggle_mute` on the mic mutes the microphone.

use serde::Deserialize;

/// The pointer actions of one module. Unset does nothing.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct On {
    #[serde(default)]
    pub click: Option<Binding>,
    #[serde(default)]
    pub click_right: Option<Binding>,
    #[serde(default)]
    pub click_middle: Option<Binding>,
    #[serde(default)]
    pub scroll_up: Option<Binding>,
    #[serde(default)]
    pub scroll_down: Option<Binding>,
}

impl On {
    /// Every binding with the key it sits at under `module`.
    pub(crate) fn bindings(&self, module: &str) -> Vec<(String, &Binding)> {
        [
            ("click", &self.click),
            ("click_right", &self.click_right),
            ("click_middle", &self.click_middle),
            ("scroll_up", &self.scroll_up),
            ("scroll_down", &self.scroll_down),
        ]
        .into_iter()
        .filter_map(|(key, binding)| {
            binding
                .as_ref()
                .map(|binding| (format!("{module}.on.{key}"), binding))
        })
        .collect()
    }
}

/// One pointer action's effect.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum Binding {
    /// A built-in action, by name.
    Builtin(Builtin),
    /// `{ exec = "…" }`: run a command through the platform shell. A config
    /// file is code: the command runs as the user (SECURITY.md).
    Exec { exec: String },
}

/// `{ exec = "…" }` on its own: a command run through the platform shell.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Exec {
    pub exec: String,
}

/// The power menu's entries set to a command
/// (`docs/plan/10-popups-and-menus.md#power-menu`). An unset one does what
/// lyr does itself.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PowerMenu {
    #[serde(default)]
    pub lock: Option<Exec>,
    #[serde(default)]
    pub sleep: Option<Exec>,
    #[serde(default)]
    pub hibernate: Option<Exec>,
    #[serde(default)]
    pub shutdown: Option<Exec>,
    #[serde(default)]
    pub logout: Option<Exec>,
    #[serde(default)]
    pub reboot: Option<Exec>,
}

impl PowerMenu {
    /// Every command set, with its key.
    pub(crate) fn commands(&self) -> impl Iterator<Item = (&'static str, &Exec)> {
        [
            ("lock", &self.lock),
            ("sleep", &self.sleep),
            ("hibernate", &self.hibernate),
            ("shutdown", &self.shutdown),
            ("logout", &self.logout),
            ("reboot", &self.reboot),
        ]
        .into_iter()
        .filter_map(|(key, exec)| exec.as_ref().map(|exec| (key, exec)))
    }
}

/// The built-in actions.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Builtin {
    /// Mute or unmute the module's device (volume and mic).
    ToggleMute,
    /// Raise the module's device by its `scroll_step` (volume and mic).
    VolumeUp,
    /// Lower the module's device by its `scroll_step` (volume and mic).
    VolumeDown,
    /// Switch to the next power profile.
    CycleProfile,
    /// Switch to the previous power profile.
    CycleProfileBack,
    OpenClockPanel,
    OpenPowerMenu,
    OpenWifiMenu,
    OpenBluetoothMenu,
    OpenAudioMenu,
    ToggleDnd,
}

impl Builtin {
    /// The name a config file uses.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::ToggleMute => "toggle_mute",
            Self::VolumeUp => "volume_up",
            Self::VolumeDown => "volume_down",
            Self::CycleProfile => "cycle_profile",
            Self::CycleProfileBack => "cycle_profile_back",
            Self::OpenClockPanel => "open_clock_panel",
            Self::OpenPowerMenu => "open_power_menu",
            Self::OpenWifiMenu => "open_wifi_menu",
            Self::OpenBluetoothMenu => "open_bluetooth_menu",
            Self::OpenAudioMenu => "open_audio_menu",
            Self::ToggleDnd => "toggle_dnd",
        }
    }

    /// Whether this action means something on `module`. The ones that act on
    /// the module's own state only fit modules that have it; opening a panel
    /// or a menu fits anywhere.
    pub(crate) fn fits(self, module: &str) -> bool {
        match self {
            Self::ToggleMute | Self::VolumeUp | Self::VolumeDown => {
                matches!(module, "volume" | "mic")
            }
            Self::CycleProfile | Self::CycleProfileBack => module == "power_profile",
            Self::OpenClockPanel
            | Self::OpenPowerMenu
            | Self::OpenWifiMenu
            | Self::OpenBluetoothMenu
            | Self::OpenAudioMenu
            | Self::ToggleDnd => true,
        }
    }
}
