//! The network module (`docs/plan/07-modules.md#network`), following waybar
//! 0.15.0's `network.cpp`.

use lyr_config::{Format, Network as Config};

use crate::icon::pick;

/// The link's state, decided as waybar's `getNetworkState` does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    /// No interface with a carrier, and the radio blocked by rfkill.
    Disabled,
    /// No interface with a carrier.
    Disconnected,
    /// A carrier but no address yet.
    Linked,
    /// An address on a link with no SSID: wired, or anything not Wi-Fi.
    Ethernet,
    /// An address on a Wi-Fi network.
    Wifi,
}

impl State {
    /// The name waybar uses, which the icon lookup takes.
    const fn name(self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::Disconnected => "disconnected",
            Self::Linked => "linked",
            Self::Ethernet => "ethernet",
            Self::Wifi => "wifi",
        }
    }
}

/// One reading of the interface the module follows. Fields the state does not
/// have are empty, as waybar leaves them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Link {
    pub ifname: String,
    /// IPv4, the address waybar shows by default.
    pub ipaddr: String,
    /// The prefix length.
    pub cidr: String,
    pub gwaddr: String,
    pub essid: String,
    /// Percent.
    pub signal_strength: u8,
    /// A Wi-Fi link whose network name the platform withholds, as macOS does
    /// without Location permission. Elsewhere the name decides, as in
    /// waybar, and this stays false.
    pub wireless: bool,
}

/// What a source read: the interface with the default route, if any, and
/// whether rfkill blocks the Wi-Fi radio.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Reading {
    pub link: Option<Link>,
    pub radio_blocked: bool,
}

impl Reading {
    /// The link's state, as waybar's `getNetworkState` decides it: with no
    /// interface, disabled while the radio is blocked and disconnected
    /// otherwise; then linked until it has an address, and Wi-Fi when it has
    /// an SSID, or is Wi-Fi with its SSID withheld.
    #[must_use]
    pub fn state(&self) -> State {
        match &self.link {
            None if self.radio_blocked => State::Disabled,
            None => State::Disconnected,
            Some(link) if link.ipaddr.is_empty() => State::Linked,
            Some(link) if link.essid.is_empty() && !link.wireless => State::Ethernet,
            Some(_) => State::Wifi,
        }
    }

    /// The view of this reading under `config`.
    #[must_use]
    pub fn view(&self, config: &Config) -> View {
        let none = Link::default();
        view(self.state(), self.link.as_ref().unwrap_or(&none), config)
    }
}

/// What the module draws.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct View {
    pub text: String,
    pub tooltip: String,
}

/// The view of `link` in `state` under `config`.
#[must_use]
pub fn view(state: State, link: &Link, config: &Config) -> View {
    let (format, tooltip_format) = match state {
        State::Wifi => (&config.format_wifi, &config.tooltip_format_wifi),
        State::Ethernet => (&config.format_ethernet, &config.tooltip_format_ethernet),
        State::Linked => (&config.format_linked, &config.tooltip_format_linked),
        State::Disconnected => (
            &config.format_disconnected,
            &config.tooltip_format_disconnected,
        ),
        State::Disabled => (&config.format_disabled, &config.tooltip_format_disabled),
    };
    let icon = pick(
        &config.icons,
        &[state.name()],
        u32::from(link.signal_strength),
    );
    let signal = link.signal_strength.to_string();
    let fill = |format: &Format| {
        format.render(|name| match name {
            "icon" => Some(icon),
            "ifname" => Some(link.ifname.as_str()),
            "ipaddr" => Some(link.ipaddr.as_str()),
            "cidr" => Some(link.cidr.as_str()),
            "gwaddr" => Some(link.gwaddr.as_str()),
            "essid" => Some(link.essid.as_str()),
            "signalStrength" => Some(signal.as_str()),
            _ => None,
        })
    };
    View {
        text: fill(format.as_ref().unwrap_or(&config.format)),
        tooltip: fill(tooltip_format.as_ref().unwrap_or(&config.tooltip_format)),
    }
}

#[cfg(test)]
mod tests {
    use lyr_config::{Format, Network, Platform, from_sources};

    use super::{Link, Reading, State, view};

    fn defaults() -> Network {
        from_sources(None, Platform::Linux)
            .expect("defaults")
            .network
    }

    fn link() -> Link {
        Link {
            ifname: "wlan0".to_owned(),
            ipaddr: "192.168.1.20".to_owned(),
            cidr: "24".to_owned(),
            gwaddr: "192.168.1.1".to_owned(),
            essid: "home".to_owned(),
            signal_strength: 71,
            wireless: false,
        }
    }

    #[test]
    fn each_state_has_its_icon() {
        let config = defaults();
        let text = |state| view(state, &link(), &config).text;
        assert_eq!(text(State::Wifi), "\u{F1EB}");
        assert_eq!(text(State::Ethernet), "\u{EF09}");
        assert_eq!(text(State::Disconnected), "\u{F015B}");
        // waybar's icons have no `linked` or `disabled` entry, so both take
        // the default, the wired icon.
        assert_eq!(text(State::Linked), "\u{EF09}");
        assert_eq!(text(State::Disabled), "\u{EF09}");
    }

    #[test]
    fn a_state_format_replaces_the_plain_one_in_that_state_only() {
        let mut config = defaults();
        config.format_wifi = Some(Format::parse("{essid}").expect("valid"));
        assert_eq!(view(State::Wifi, &link(), &config).text, "home");
        assert_eq!(view(State::Ethernet, &link(), &config).text, "\u{EF09}");
    }

    #[test]
    fn each_state_has_its_tooltip() {
        let config = defaults();
        let tooltip = |state| view(state, &link(), &config).tooltip;
        assert_eq!(tooltip(State::Wifi), "wlan0 192.168.1.20/24 home (71%)");
        assert_eq!(tooltip(State::Ethernet), "wlan0 192.168.1.20/24");
        assert_eq!(tooltip(State::Disconnected), "YOU IN THE WOODS?");
    }

    #[test]
    fn a_reading_s_state_follows_waybar() {
        let reading = |link: Option<Link>, radio_blocked| Reading {
            link,
            radio_blocked,
        };
        assert_eq!(reading(None, false).state(), State::Disconnected);
        assert_eq!(reading(None, true).state(), State::Disabled);
        assert_eq!(reading(Some(link()), true).state(), State::Wifi);
        let wired = Link {
            essid: String::new(),
            ..link()
        };
        assert_eq!(reading(Some(wired.clone()), false).state(), State::Ethernet);
        let withheld = Link {
            wireless: true,
            ..wired
        };
        assert_eq!(
            reading(Some(withheld), false).state(),
            State::Wifi,
            "Wi-Fi with its name withheld"
        );
        let waiting = Link {
            ipaddr: String::new(),
            ..link()
        };
        assert_eq!(reading(Some(waiting), false).state(), State::Linked);
        assert_eq!(
            reading(None, false).view(&defaults()).tooltip,
            "YOU IN THE WOODS?"
        );
    }
}
