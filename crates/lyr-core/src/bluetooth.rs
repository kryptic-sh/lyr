//! The bluetooth module (`docs/plan/07-modules.md#bluetooth`), following
//! waybar 0.15.0's `bluetooth.cpp`.
//!
//! With no controller at all the module is hidden, as battery is on a machine
//! with no battery, so that case never reaches the view.

use lyr_config::{Bluetooth as Config, Format};

use crate::icon::named;

/// The controller's state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// Blocked by rfkill.
    Disabled,
    /// Powered off.
    Off,
    /// Powered on with nothing connected.
    On,
    /// Powered on with at least one device connected.
    Connected,
}

impl State {
    /// The name waybar uses, which `{status}` and the icon lookup take.
    const fn name(self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::Off => "off",
            Self::On => "on",
            Self::Connected => "connected",
        }
    }
}

/// The device the module shows: the one that connected last.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Device {
    pub alias: String,
    /// Percent, when the device reports it.
    pub battery: Option<u8>,
}

/// One reading of the controller.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reading {
    pub state: State,
    pub connections: usize,
    pub device: Option<Device>,
}

/// What the module draws.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct View {
    pub text: String,
    pub tooltip: String,
}

/// The view of `reading` under `config`.
#[must_use]
pub fn view(reading: &Reading, config: &Config) -> View {
    let state = reading.state;
    let battery = match (state, &reading.device) {
        (State::Connected, Some(device)) => device.battery,
        _ => None,
    };
    // waybar looks the icon up by state, then by whether rfkill blocks the
    // radio, then falls back to `default`.
    let blocked = if state == State::Disabled {
        "disabled"
    } else {
        "enabled"
    };
    let icon = named(&config.icons, &[state.name(), blocked]);
    let state_format = match state {
        State::Disabled => config.format_disabled.as_ref(),
        State::Off => config.format_off.as_ref(),
        State::On => config.format_on.as_ref(),
        State::Connected => config.format_connected.as_ref(),
    };
    let (format, label_icon) = match (battery, &config.format_connected_battery) {
        // With a battery waybar takes the icon from the ramp at the battery's
        // level; `icons` is named rather than a ramp, so that is `default`.
        (Some(_), Some(format)) => (format, named(&config.icons, &[])),
        _ => (state_format.unwrap_or(&config.format), icon),
    };

    let connections = reading.connections.to_string();
    let alias = reading
        .device
        .as_ref()
        .map_or("", |device| device.alias.as_str());
    let percentage = battery.unwrap_or(0).to_string();
    let fill = |format: &Format, icon: &str| {
        format.render(|name| match name {
            "icon" => Some(icon),
            "status" => Some(state.name()),
            "num_connections" => Some(connections.as_str()),
            "device_alias" => Some(alias),
            "device_battery_percentage" => Some(percentage.as_str()),
            _ => None,
        })
    };
    View {
        text: fill(format, label_icon),
        tooltip: fill(&config.tooltip_format, icon),
    }
}

#[cfg(test)]
mod tests {
    use lyr_config::{Bluetooth, Format, Platform, from_sources};

    use super::{Device, Reading, State, view};

    fn defaults() -> Bluetooth {
        from_sources(None, Platform::Linux)
            .expect("defaults")
            .bluetooth
    }

    fn reading(state: State, battery: Option<u8>) -> Reading {
        Reading {
            state,
            connections: usize::from(state == State::Connected),
            device: (state == State::Connected).then(|| Device {
                alias: "Buds".to_owned(),
                battery,
            }),
        }
    }

    #[test]
    fn the_icon_follows_the_state() {
        let config = defaults();
        let text = |state| view(&reading(state, None), &config).text;
        assert_eq!(text(State::On), "\u{F00AF}");
        assert_eq!(text(State::Connected), "\u{F00AF}");
        assert_eq!(text(State::Off), "\u{F00B2}");
        assert_eq!(text(State::Disabled), "\u{F00B2}");
    }

    #[test]
    fn a_connected_device_with_a_battery_is_named_with_its_level() {
        let shown = view(&reading(State::Connected, Some(70)), &defaults());
        assert_eq!(shown.text, "\u{F00AF} Buds 70%");
    }

    #[test]
    fn a_state_format_replaces_the_plain_one_in_that_state_only() {
        let mut config = defaults();
        config.format_off = Some(Format::parse("{icon} off").expect("valid"));
        assert_eq!(
            view(&reading(State::Off, None), &config).text,
            "\u{F00B2} off"
        );
        assert_eq!(view(&reading(State::On, None), &config).text, "\u{F00AF}");
    }

    #[test]
    fn the_tooltip_names_the_state() {
        let config = defaults();
        assert_eq!(
            view(&reading(State::Off, None), &config).tooltip,
            "Bluetooth off"
        );
        assert_eq!(
            view(&reading(State::Connected, Some(70)), &config).tooltip,
            "Bluetooth connected"
        );
    }
}
