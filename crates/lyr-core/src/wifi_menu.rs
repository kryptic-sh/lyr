//! The Wi-Fi menu (`docs/plan/10-popups-and-menus.md#wi-fi-menu`): the
//! networks a scan found, one row per SSID, what joining one takes, and
//! drawing the menu. The platforms' sources fill it in
//! (`lyr_sources::network`).

use std::time::Duration;

use lyr_config::Theme;

use crate::draw::{DrawList, Size, switch};
use crate::hit::{Rect, Regions};
use crate::menu::{self, GAP, PADDING, button_width};

/// NetworkManager's `NM80211ApFlags` privacy bit.
const PRIVACY: u32 = 0x1;
/// NetworkManager's `NM80211ApSecurityFlags` key-management bits.
const KEY_PSK: u32 = 0x100;
const KEY_802_1X: u32 = 0x200;
const KEY_SAE: u32 = 0x400;
const KEY_OWE: u32 = 0x800;

/// How an access point is joined, least demanding first, so the most
/// demanding of a network's access points is the greatest.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Security {
    /// No password. An OWE transition-mode access point's open side is
    /// joined this way.
    Open,
    /// Opportunistic Wireless Encryption: encrypted, with no password.
    Owe,
    /// WPA or WPA2 with a password, and WPA3 transition mode.
    Psk,
    /// WPA3 alone.
    Sae,
    /// WEP, which the menu refuses.
    Wep,
    /// 802.1X, which the menu refuses.
    Enterprise,
}

impl Security {
    /// From an access point's `Flags`, `WpaFlags` and `RsnFlags`.
    #[must_use]
    pub const fn of(flags: u32, wpa_flags: u32, rsn_flags: u32) -> Self {
        let keys = wpa_flags | rsn_flags;
        if keys & KEY_802_1X != 0 {
            Self::Enterprise
        } else if keys & KEY_PSK != 0 {
            Self::Psk
        } else if keys & KEY_SAE != 0 {
            Self::Sae
        } else if keys & KEY_OWE != 0 {
            Self::Owe
        } else if keys == 0 && flags & PRIVACY != 0 {
            Self::Wep
        } else {
            Self::Open
        }
    }

    /// Why the menu will not join it, as `.menu-wifi` would not.
    #[must_use]
    pub const fn refusal(self) -> Option<&'static str> {
        match self {
            Self::Wep => Some("WEP networks are not supported"),
            Self::Enterprise => Some("Enterprise (802.1X) networks are not supported"),
            _ => None,
        }
    }

    /// Whether joining it takes a password.
    #[must_use]
    pub const fn needs_password(self) -> bool {
        matches!(self, Self::Psk | Self::Sae)
    }
}

/// Whether NetworkManager takes `password` for a network secured with
/// `security`: a WPA-PSK key is 8 to 63 bytes, or 64 hexadecimal digits;
/// SAE sets no length, but takes no empty one.
#[must_use]
pub fn password_fits(security: Security, password: &str) -> bool {
    match security {
        Security::Psk => {
            (8..=63).contains(&password.len())
                || (password.len() == 64 && password.bytes().all(|byte| byte.is_ascii_hexdigit()))
        }
        Security::Sae => !password.is_empty(),
        _ => true,
    }
}

/// How old a scan may be for the list to be used without a fresh one: the
/// cutoff `nmcli --rescan auto` uses.
pub const FRESH_SCAN: Duration = Duration::from_secs(30);

/// Whether a scan that ended `last_scan` after boot is fresh `now` after
/// boot. `None` is a device that never scanned.
#[must_use]
pub fn scan_is_fresh(last_scan: Option<Duration>, now: Duration) -> bool {
    last_scan.is_some_and(|last| now.saturating_sub(last) < FRESH_SCAN)
}

/// NetworkManager's `NM_DEVICE_STATE_REASON_*` codes the menu tells apart.
const REASON_IP_CONFIG_UNAVAILABLE: u32 = 5;
const REASON_NO_SECRETS: u32 = 7;
const REASON_SUPPLICANT_TIMEOUT: u32 = 11;
const REASON_SSID_NOT_FOUND: u32 = 53;

/// Why joining failed, in words, from the device's state reason.
#[must_use]
pub fn failure_text(ssid: &str, reason: u32) -> String {
    match reason {
        // NetworkManager reports a wrong password and an association
        // timeout alike.
        REASON_NO_SECRETS => format!("Wrong password for {ssid}, or it did not answer"),
        REASON_SUPPLICANT_TIMEOUT => format!("{ssid} did not answer in time"),
        REASON_SSID_NOT_FOUND => format!("{ssid} was not found"),
        REASON_IP_CONFIG_UNAVAILABLE => format!("{ssid} gave no address"),
        _ => format!("Joining {ssid} failed (reason {reason})"),
    }
}

/// What to do with a profile after joining with it failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Recovery {
    /// Keep it, and offer to replace its password.
    NewPassword,
    /// Delete it, so the next attempt starts afresh.
    Delete,
    /// Keep it as it is.
    Keep,
}

/// What to do after joining a network secured with `security` failed for
/// `reason`, with a profile that was `saved` before, or made for this
/// attempt: a saved profile is kept, and offered a new password when the
/// secrets failed; a new one is deleted when its secrets failed, or when it
/// is open, and kept otherwise (a DHCP timeout says nothing against it).
#[must_use]
pub fn recovery(saved: bool, security: Security, reason: u32) -> Recovery {
    let secrets = reason == REASON_NO_SECRETS;
    if saved {
        if secrets && security.needs_password() {
            Recovery::NewPassword
        } else {
            Recovery::Keep
        }
    } else if secrets || security == Security::Open {
        Recovery::Delete
    } else {
        Recovery::Keep
    }
}

/// One access point a scan found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccessPoint {
    /// Its object path, which identifies it.
    pub path: String,
    /// Empty for a hidden network.
    pub ssid: String,
    /// Percent.
    pub strength: u8,
    pub security: Security,
}

/// One row of the menu: every access point with one SSID.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Network {
    pub ssid: String,
    /// The strongest access point's.
    pub strength: u8,
    /// The most demanding among its access points, so a spoofed open twin
    /// cannot lower what joining asks for.
    pub security: Security,
    /// The strongest access point, which joining asks for.
    pub point: String,
    /// The network the device is on.
    pub in_use: bool,
    /// A saved profile joins it.
    pub known: bool,
}

/// The menu's rows from `points`, strongest first: one per SSID, hidden
/// networks left out. `active` is the access point in use, `known` the
/// SSIDs saved profiles join.
#[must_use]
pub fn networks(points: &[AccessPoint], active: Option<&str>, known: &[String]) -> Vec<Network> {
    let mut networks: Vec<Network> = Vec::new();
    for point in points.iter().filter(|point| !point.ssid.is_empty()) {
        let in_use = active == Some(point.path.as_str());
        match networks
            .iter_mut()
            .find(|network| network.ssid == point.ssid)
        {
            Some(network) => {
                if point.strength > network.strength {
                    network.strength = point.strength;
                    network.point.clone_from(&point.path);
                }
                network.security = network.security.max(point.security);
                network.in_use |= in_use;
            }
            None => networks.push(Network {
                ssid: point.ssid.clone(),
                strength: point.strength,
                security: point.security,
                point: point.path.clone(),
                in_use,
                known: known.contains(&point.ssid),
            }),
        }
    }
    networks.sort_by(|a, b| {
        b.strength
            .cmp(&a.strength)
            .then_with(|| a.ssid.cmp(&b.ssid))
    });
    networks
}

/// The Wi-Fi radio's switches (`WirelessEnabled` and
/// `WirelessHardwareEnabled`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Radio {
    pub enabled: bool,
    /// False while a hardware switch blocks the radio, which the menu's
    /// switch cannot lift.
    pub hardware_enabled: bool,
}

/// A saved Wi-Fi client profile.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Profile {
    /// Its object path, which identifies it.
    pub path: String,
    pub ssid: String,
}

/// What the Wi-Fi menu shows.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Wifi {
    /// The Wi-Fi device's object path; `None` when there is none.
    pub device: Option<String>,
    pub radio: Radio,
    /// False while the device is unmanaged, or unavailable while its
    /// supplicant is down, when it cannot scan.
    pub available: bool,
    /// When the last scan ended, since boot; `None` if it never scanned.
    pub last_scan: Option<Duration>,
    pub points: Vec<AccessPoint>,
    /// The access point in use.
    pub active: Option<String>,
    pub profiles: Vec<Profile>,
}

impl Wifi {
    /// The menu's rows.
    #[must_use]
    pub fn networks(&self) -> Vec<Network> {
        let known: Vec<String> = self
            .profiles
            .iter()
            .map(|profile| profile.ssid.clone())
            .collect();
        networks(&self.points, self.active.as_deref(), &known)
    }

    /// The saved profiles that join `ssid`.
    #[must_use]
    pub fn profiles_for(&self, ssid: &str) -> Vec<&Profile> {
        self.profiles
            .iter()
            .filter(|profile| profile.ssid == ssid)
            .collect()
    }
}

/// Whether the list can be trusted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scan {
    /// The last scan is recent.
    Ready,
    /// A fresh scan is awaited.
    Scanning,
    /// The device is unmanaged, or unavailable while its supplicant is down.
    Unavailable,
}

/// The row at the top of the menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Header<'a> {
    /// The password for `ssid`, `typed` so far. `again` when a saved
    /// profile's password failed and the menu offers to replace it.
    Password {
        ssid: &'a str,
        typed: &'a str,
        again: bool,
    },
    /// Forget `ssid`'s profiles?
    Forget { ssid: &'a str },
    /// Joining `ssid`, while every row waits.
    Joining { ssid: &'a str },
    /// What went wrong, until OK.
    Error(&'a str),
}

impl Header<'_> {
    /// What it says.
    #[must_use]
    pub fn text(&self) -> String {
        match self {
            Self::Password {
                ssid, again: false, ..
            } => format!("Password for {ssid}"),
            Self::Password {
                ssid, again: true, ..
            } => format!("Joining {ssid} failed. New password"),
            Self::Forget { ssid } => format!("Forget {ssid}?"),
            Self::Joining { ssid } => format!("Joining {ssid}\u{2026}"),
            Self::Error(message) => (*message).to_owned(),
        }
    }

    /// Its buttons, left to right, with what each answers.
    const fn answers(&self) -> &'static [(&'static str, bool)] {
        match self {
            Self::Password { .. } => &[("OK", true), ("Cancel", false)],
            Self::Forget { .. } => &[("Yes", true), ("No", false)],
            Self::Joining { .. } => &[],
            Self::Error(_) => &[("OK", true)],
        }
    }
}

/// How a typed password is shown: a dot for each character.
const MASK: char = '\u{2022}';
/// How many characters the password field shows at once; a longer one
/// scrolls in it.
const FIELD_CHARS: usize = 16;
/// The mark of the network in use (Nerd Font's check).
const IN_USE: &str = "\u{F00C}";

/// What a click in the menu lands on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    /// OK or Yes (`true`), Cancel or No (`false`), to the header.
    Answer(bool),
    /// The radio switch.
    Radio,
    /// A network's Connect button, by its index in the rows.
    Join(usize),
    /// A known network's Forget button.
    Forget(usize),
}

/// What the menu shows.
#[derive(Clone, Copy, Debug)]
pub struct Shown<'a> {
    pub radio: Radio,
    pub scan: Scan,
    pub networks: &'a [Network],
    pub header: Option<Header<'a>>,
}

impl Shown<'_> {
    /// What stands in for the list, when it is not shown.
    fn status(&self) -> Option<&'static str> {
        if !self.radio.hardware_enabled {
            Some("Wi-Fi is blocked by a hardware switch")
        } else if !self.radio.enabled {
            Some("Wi-Fi is off")
        } else {
            match self.scan {
                Scan::Ready => None,
                Scan::Scanning => Some("Scanning\u{2026}"),
                Scan::Unavailable => Some("Wi-Fi cannot scan: the device is unavailable"),
            }
        }
    }
}

/// A row's label: the network's name and signal, marked when in use.
fn label(network: &Network) -> String {
    let mark = if network.in_use {
        format!("{IN_USE} ")
    } else {
        String::new()
    };
    format!("{mark}{} {}%", network.ssid, network.strength)
}

/// A row's buttons: Connect unless it is in use, then Forget when it is
/// known.
fn buttons(network: &Network, index: usize) -> Vec<(&'static str, Target)> {
    let mut buttons = Vec::new();
    if !network.in_use {
        buttons.push(("Connect", Target::Join(index)));
    }
    if network.known {
        buttons.push(("Forget", Target::Forget(index)));
    }
    buttons
}

/// The menu showing `shown`: its draw list, its size, and what each click
/// lands on, its text measured by `measure` in lines `line_height` tall.
/// While a network is being joined, the rows take no clicks.
#[must_use]
pub fn draw(
    shown: &Shown<'_>,
    focus: Option<Target>,
    measure: &dyn Fn(&str) -> f32,
    line_height: f32,
    theme: &Theme,
) -> (DrawList, Size, Regions<Target>) {
    let row = menu::row(line_height);
    let across = menu::BUTTON_PADDING.0;
    let status = shown.status();
    let networks = if status.is_some() {
        &[][..]
    } else {
        shown.networks
    };
    let joining = matches!(shown.header, Some(Header::Joining { .. }));
    let label_width = networks
        .iter()
        .map(|network| measure(&label(network)))
        .chain(status.map(measure))
        .chain([measure("Wi-Fi")])
        .fold(0.0, f32::max);
    let buttons_width = networks
        .iter()
        .enumerate()
        .map(|(index, network)| {
            buttons(network, index)
                .iter()
                .map(|(label, _)| button_width(label, measure) + GAP)
                .sum::<f32>()
        })
        .fold(2.0 * row, f32::max);
    let field_width = measure(&MASK.to_string().repeat(FIELD_CHARS)) + 2.0 * across;
    let header_width = shown.header.map_or(0.0, |header| {
        let field = if matches!(header, Header::Password { .. }) {
            GAP + field_width
        } else {
            0.0
        };
        measure(&header.text())
            + field
            + header
                .answers()
                .iter()
                .map(|(label, _)| GAP + button_width(label, measure))
                .sum::<f32>()
    });
    let width = 2.0 * PADDING + (label_width + GAP + buttons_width).max(header_width);
    let text =
        |text: String, x: f32, y: f32, colour| menu::text(text, x, y, measure, line_height, colour);
    let mut items = Vec::new();
    let mut regions = Regions::default();
    let mut y = PADDING;
    if let Some(header) = shown.header {
        let colour = if matches!(header, Header::Error(_)) {
            theme.battery_critical
        } else {
            theme.workspace_active
        };
        items.push(text(header.text(), PADDING, y, colour));
        if let Header::Password { typed, .. } = header {
            let field = Rect {
                x: PADDING + measure(&header.text()) + GAP,
                y,
                width: field_width,
                height: row,
            };
            let masked: String = typed.chars().map(|_| MASK).collect();
            items.extend(menu::field(field, &masked, measure, line_height, theme));
        }
        let mut x = width - PADDING;
        for (label, answer) in header.answers().iter().rev() {
            x -= button_width(label, measure);
            let rect = Rect {
                x,
                y,
                width: button_width(label, measure),
                height: row,
            };
            items.extend(menu::button(rect, label, measure, line_height, theme));
            regions.push(rect, Target::Answer(*answer));
            x -= GAP;
        }
        y += row + 2.0 * GAP;
    }
    items.push(text("Wi-Fi".to_owned(), PADDING, y, theme.foreground));
    let rect = Rect {
        x: width - PADDING - 2.0 * row,
        y,
        width: 2.0 * row,
        height: row,
    };
    items.extend(switch(rect, shown.radio.enabled, theme));
    if shown.radio.hardware_enabled {
        regions.push(rect, Target::Radio);
    }
    y += row + GAP;
    if status.is_some() || !networks.is_empty() {
        y += GAP;
    }
    if let Some(status) = status {
        items.push(text(status.to_owned(), PADDING, y, theme.foreground));
        y += row + GAP;
    }
    for (index, network) in networks.iter().enumerate() {
        let colour = if network.in_use {
            theme.workspace_active
        } else {
            theme.foreground
        };
        items.push(text(label(network), PADDING, y, colour));
        let mut x = PADDING + label_width + GAP;
        for (label, target) in buttons(network, index) {
            let rect = Rect {
                x,
                y,
                width: button_width(label, measure),
                height: row,
            };
            items.extend(menu::button(rect, label, measure, line_height, theme));
            if !joining {
                regions.push(rect, target);
            }
            x += rect.width + GAP;
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
