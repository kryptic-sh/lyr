//! The Bluetooth controller as the module's source and the Bluetooth menu
//! (`docs/plan/10-popups-and-menus.md#bluetooth-menu`) see it: the adapter,
//! its devices, and what the menu does to them. The platforms' sources fill
//! it in (`lyr_sources::bluetooth`).

use lyr_config::{Colour, Theme};

use crate::draw::{DrawList, Item, Size, switch};
use crate::hit::{Rect, Regions};

/// The adapter's properties the module uses (`org.bluez.Adapter1`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Adapter {
    /// Its object path, which identifies it.
    pub path: String,
    pub powered: bool,
    /// The menu's switches, and whether a scan is running.
    pub discoverable: bool,
    pub pairable: bool,
    pub discovering: bool,
    /// `PowerState`, which BlueZ marks experimental and may not send;
    /// `off-blocked` is an rfkill block.
    pub power_state: Option<String>,
}

/// A device's properties the module uses (`org.bluez.Device1`, and
/// `org.bluez.Battery1` when the device reports its battery).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Device {
    /// Its object path, which identifies it.
    pub path: String,
    pub alias: String,
    pub connected: bool,
    /// Paired with this machine; a device a scan found is not.
    pub paired: bool,
    pub trusted: bool,
    pub battery: Option<u8>,
}

/// One of the adapter's switches in the Bluetooth menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Switch {
    Powered,
    Discoverable,
    Pairable,
}

/// What the Bluetooth menu does to a device.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeviceAction {
    Connect,
    Disconnect,
    Pair,
    Trust,
    Untrust,
}

/// What the pairing agent asks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Question {
    /// Does the device show this passkey?
    Confirm(u32),
    /// May the device pair?
    Authorize,
    /// May the device connect to a service?
    AuthorizeService,
}

/// A question waiting at the top of the menu, about the device named
/// `device`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ask<'a> {
    pub device: &'a str,
    pub question: Question,
}

impl Ask<'_> {
    /// The question in words.
    #[must_use]
    pub fn text(&self) -> String {
        let device = self.device;
        match self.question {
            Question::Confirm(passkey) => format!("Pair with {device}? Passkey {passkey:06}"),
            Question::Authorize => format!("Let {device} pair?"),
            Question::AuthorizeService => format!("Let {device} connect?"),
        }
    }
}

/// What a click in the menu lands on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    /// Yes or No to the agent's question.
    Answer(bool),
    Switch(Switch),
    /// The scan toggle.
    Scan,
    /// A device's button, by the device's index in the order drawn
    /// ([`order`]).
    Device(usize, DeviceAction),
    /// A device's Forget button.
    Forget(usize),
}

/// The menu's padding, around its rows.
pub const PADDING: f32 = 8.0;
/// The space between rows, and between a row's buttons.
pub const GAP: f32 = 6.0;
/// Around a button's label.
const BUTTON_PADDING: (f32, f32) = (8.0, 2.0);

/// The devices in the order the menu lists them: paired first, then those a
/// scan found, each part by name.
#[must_use]
pub fn order(devices: &[Device]) -> Vec<&Device> {
    let mut ordered: Vec<&Device> = devices.iter().collect();
    ordered.sort_by(|a, b| b.paired.cmp(&a.paired).then_with(|| a.alias.cmp(&b.alias)));
    ordered
}

/// One of a device's buttons.
#[derive(Clone, Copy)]
enum Button {
    Action(DeviceAction),
    Forget,
}

impl Button {
    /// What a click on it lands on, for the device at `index`.
    const fn target(self, index: usize) -> Target {
        match self {
            Self::Action(action) => Target::Device(index, action),
            Self::Forget => Target::Forget(index),
        }
    }
}

/// A device's buttons, left to right: connect or disconnect, pair while it
/// is not paired, trust or untrust, then Forget.
fn buttons(device: &Device) -> Vec<(&'static str, Button)> {
    let mut buttons = vec![if device.connected {
        ("Disconnect", Button::Action(DeviceAction::Disconnect))
    } else {
        ("Connect", Button::Action(DeviceAction::Connect))
    }];
    if !device.paired {
        buttons.push(("Pair", Button::Action(DeviceAction::Pair)));
    }
    buttons.push(if device.trusted {
        ("Untrust", Button::Action(DeviceAction::Untrust))
    } else {
        ("Trust", Button::Action(DeviceAction::Trust))
    });
    buttons.push(("Forget", Button::Forget));
    buttons
}

/// The menu for `adapter` and its `devices`: its draw list, its size, and
/// what each click lands on, its text measured by `measure` in lines
/// `line_height` tall.
#[must_use]
pub fn draw(
    adapter: &Adapter,
    devices: &[Device],
    ask: Option<Ask<'_>>,
    focus: Option<Target>,
    measure: &dyn Fn(&str) -> f32,
    line_height: f32,
    theme: &Theme,
) -> (DrawList, Size, Regions<Target>) {
    let (across, down) = BUTTON_PADDING;
    let row = line_height + 2.0 * down;
    let switches = [
        ("Power", Target::Switch(Switch::Powered), adapter.powered),
        (
            "Discoverable",
            Target::Switch(Switch::Discoverable),
            adapter.discoverable,
        ),
        (
            "Pairable",
            Target::Switch(Switch::Pairable),
            adapter.pairable,
        ),
        ("Scan", Target::Scan, adapter.discovering),
    ];
    let ordered = order(devices);
    let name = |device: &Device| match device.battery {
        Some(battery) => format!("{} {battery}%", device.alias),
        None => device.alias.clone(),
    };
    let label_width = switches
        .iter()
        .map(|(label, _, _)| measure(label))
        .chain(ordered.iter().map(|device| measure(&name(device))))
        .fold(0.0, f32::max);
    let button_width = |label: &str| measure(label) + 2.0 * across;
    let buttons_width = ordered
        .iter()
        .map(|device| {
            buttons(device)
                .iter()
                .map(|(label, _)| button_width(label) + GAP)
                .sum::<f32>()
        })
        .fold(2.0 * row, f32::max);
    let answers = [("Yes", true), ("No", false)];
    let ask_width = ask.map_or(0.0, |ask| {
        measure(&ask.text())
            + answers
                .iter()
                .map(|(label, _)| GAP + button_width(label))
                .sum::<f32>()
    });
    let width = 2.0 * PADDING + (label_width + GAP + buttons_width).max(ask_width);
    let mut items = Vec::new();
    let mut regions = Regions::default();
    let text = |text: String, x: f32, y: f32, colour| Item::Text {
        rect: Rect {
            x,
            y: y + down,
            width: measure(&text),
            height: line_height,
        },
        text,
        colour,
    };
    let mut y = PADDING;
    // The agent's question first, where it cannot be missed.
    if let Some(ask) = ask {
        items.push(text(ask.text(), PADDING, y, theme.workspace_active));
        let mut x = width - PADDING;
        for (label, answer) in answers.iter().rev() {
            x -= button_width(label);
            let rect = Rect {
                x,
                y,
                width: button_width(label),
                height: row,
            };
            items.push(Item::Box {
                rect,
                radius: theme.radius as f32,
                fill: theme.toast_background,
                border: Some((theme.border_width as f32, theme.panel_border)),
            });
            items.push(text((*label).to_owned(), x + across, y, theme.foreground));
            regions.push(rect, Target::Answer(*answer));
            x -= GAP;
        }
        y += row + 2.0 * GAP;
    }
    for (label, target, on) in switches {
        items.push(text(label.to_owned(), PADDING, y, theme.foreground));
        let rect = Rect {
            x: width - PADDING - 2.0 * row,
            y,
            width: 2.0 * row,
            height: row,
        };
        items.extend(switch(rect, on, theme));
        regions.push(rect, target);
        y += row + GAP;
    }
    for (index, device) in ordered.iter().enumerate() {
        y += if index == 0 { GAP } else { 0.0 };
        let colour = if device.connected {
            theme.workspace_active
        } else {
            theme.foreground
        };
        items.push(text(name(device), PADDING, y, colour));
        let mut x = PADDING + label_width + GAP;
        for (label, button) in buttons(device) {
            let rect = Rect {
                x,
                y,
                width: button_width(label),
                height: row,
            };
            items.push(Item::Box {
                rect,
                radius: theme.radius as f32,
                fill: theme.toast_background,
                border: Some((theme.border_width as f32, theme.panel_border)),
            });
            items.push(text(label.to_owned(), x + across, y, theme.foreground));
            regions.push(rect, button.target(index));
            x += rect.width + GAP;
        }
        y += row + GAP;
    }
    let size = Size {
        width,
        height: y - GAP + PADDING,
    };
    let mut list = DrawList {
        items: vec![Item::Box {
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: size.width,
                height: size.height,
            },
            radius: theme.radius as f32,
            fill: theme.module_background,
            border: Some((theme.border_width as f32, theme.border)),
        }],
    };
    list.items.extend(items);
    // The keyboard's place: an outline in the active colour.
    if let Some(rect) = focus.and_then(|focus| regions.rect(&focus)) {
        list.items.push(Item::Box {
            rect,
            radius: theme.radius as f32,
            fill: Colour::TRANSPARENT,
            border: Some((theme.border_width as f32, theme.workspace_active)),
        });
    }
    (list, size, regions)
}

#[cfg(test)]
mod tests;
