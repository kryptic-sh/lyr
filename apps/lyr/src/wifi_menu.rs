//! The Wi-Fi menu (`docs/plan/10-popups-and-menus.md#wi-fi-menu`): a panel
//! at the top left, under the network module where a bar shows one, taking
//! the keyboard. It asks for a password in its header, confirms Forget, and
//! shows a join's progress and failure there; the radio switch, joining and
//! forgetting are the caller's to carry out. `Escape` (once a question is
//! dismissed), losing focus and the same click or binding again close it.
//!
//! Its panel is [`MenuPanel`]'s.

use std::time::{Duration, Instant};

use crcbl_shell::{ButtonState, KeyCode, Shell, ShellEvent, TextPurpose, WindowId};
use lyr_core::wifi_menu::{
    self, Header, Network, Scan, Shown, Target, Wifi, password_fits, scan_is_fresh,
};
use lyr_render::text::Text;

use crate::menu_panel::{Event, MenuPanel, Place};
use crate::tooltips::Look;

/// How long the menu shows "Scanning…" at most: NetworkManager can take a
/// scan request and never run it. lyr's own choice.
pub const SCAN_WAIT: Duration = Duration::from_secs(15);
/// A click on the bar this soon after the menu opened is the second half
/// of the double click that opened it, and leaves it open: GTK's default
/// `gtk-double-click-time`.
pub const DOUBLE_CLICK: Duration = Duration::from_millis(400);

/// The open menu.
pub struct WifiMenu {
    panel: MenuPanel<Target>,
    /// The rows, as drawn.
    networks: Vec<Network>,
    /// What the header shows.
    showing: Option<Showing>,
    /// What it showed when last drawn.
    shown: Option<Showing>,
    /// What has been typed into the password field.
    typed: String,
    waiting: Waiting,
    /// The list's state when last drawn: "Scanning…" stays on screen past
    /// [`SCAN_WAIT`] until something draws the menu again.
    drawn_scan: Scan,
}

/// The wait for a fresh scan, which a rescan on opening the menu starts.
#[derive(Clone, Copy, Debug)]
struct Waiting {
    opened: Instant,
    /// The last scan's end when the menu opened, and whether it was recent
    /// enough to use: a later one ends the wait.
    scan_at_open: Option<Duration>,
    fresh_at_open: bool,
}

impl Waiting {
    /// The list's state at `now`: the device's, or waiting for a fresh scan
    /// until one lands or [`SCAN_WAIT`] is up.
    fn scan(&self, wifi: &Wifi, now: Instant) -> Scan {
        if !wifi.available {
            Scan::Unavailable
        } else if self.fresh_at_open
            || wifi.last_scan != self.scan_at_open
            || now >= self.opened + SCAN_WAIT
        {
            Scan::Ready
        } else {
            Scan::Scanning
        }
    }
}

/// What the menu's header shows.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Showing {
    /// The password for this network; `again` after a saved profile's
    /// password failed.
    Password {
        network: Network,
        again: bool,
    },
    /// Forget this network?
    Forget(String),
    /// This network is being joined.
    Joining(String),
    Error(String),
}

/// What an event did to the menu.
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    Kept,
    /// It is to close.
    Close,
    /// The window system is closing its window.
    Closed,
    /// The radio is to switch to the other state.
    ToggleRadio,
    /// The network is to be joined, with the password typed for it.
    Join(Network, Option<String>),
    /// The network's saved profile is to take this password and join.
    NewPassword(Network, String),
    /// The saved profiles of the network with this SSID are to go.
    Forget(String),
    /// The menu is to be drawn again.
    Redraw,
}

/// How a join the menu started ended. Only Linux has a source that joins.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Joined {
    Up,
    /// It failed, for this reason in words.
    Failed(String),
    /// A saved profile's password failed: the menu offers to replace it.
    NewPassword,
}

impl WifiMenu {
    /// Opens the menu showing `wifi` at `place`, at `now`, `since_boot` after boot (`None` when it is not known, which
    /// treats the last scan as stale).
    pub fn open(
        shell: &mut dyn Shell,
        place: Place,
        look: &Look<'_>,
        wifi: &Wifi,
        now: Instant,
        since_boot: Option<Duration>,
    ) -> anyhow::Result<Self> {
        let waiting = Waiting {
            opened: now,
            scan_at_open: wifi.last_scan,
            fresh_at_open: since_boot.is_some_and(|boot| scan_is_fresh(wifi.last_scan, boot)),
        };
        let networks = wifi.networks();
        let drawn_scan = waiting.scan(wifi, now);
        let drawn = wifi_menu::draw(
            &Shown {
                radio: wifi.radio,
                scan: drawn_scan,
                networks: &networks,
                header: None,
            },
            None,
            look.measure,
            look.line_height,
            look.theme,
        );
        Ok(Self {
            panel: MenuPanel::open(shell, place, drawn)?,
            networks,
            showing: None,
            shown: None,
            typed: String::new(),
            waiting,
            drawn_scan,
        })
    }

    /// The list's state at `now`.
    fn scan(&self, wifi: &Wifi, now: Instant) -> Scan {
        self.waiting.scan(wifi, now)
    }

    /// When the menu has to be drawn again on its own: when the wait for a
    /// scan is up, while it still shows the wait. Past that time it stays
    /// due, until a redraw shows the list.
    pub fn next_wake(&self) -> Option<Instant> {
        (self.drawn_scan == Scan::Scanning).then_some(self.waiting.opened + SCAN_WAIT)
    }

    /// When it opened.
    pub const fn opened(&self) -> Instant {
        self.waiting.opened
    }

    /// The header, from what it shows and what was typed.
    fn header(&self) -> Option<Header<'_>> {
        Some(match self.showing.as_ref()? {
            Showing::Password { network, again } => Header::Password {
                ssid: &network.ssid,
                typed: &self.typed,
                again: *again,
            },
            Showing::Forget(ssid) => Header::Forget { ssid },
            Showing::Joining(ssid) => Header::Joining { ssid },
            Showing::Error(message) => Header::Error(message),
        })
    }

    /// The menu's draw list for `wifi` at `now`.
    fn drawn(
        &self,
        look: &Look<'_>,
        wifi: &Wifi,
        now: Instant,
    ) -> (
        lyr_core::draw::DrawList,
        lyr_core::draw::Size,
        lyr_core::hit::Regions<Target>,
    ) {
        wifi_menu::draw(
            &Shown {
                radio: wifi.radio,
                scan: self.scan(wifi, now),
                networks: &self.networks,
                header: self.header(),
            },
            self.panel.focused(),
            look.measure,
            look.line_height,
            look.theme,
        )
    }

    /// Has the window system's text input on while the password is asked.
    fn take_typing(&self, shell: &mut dyn Shell) -> anyhow::Result<()> {
        let purpose =
            matches!(self.showing, Some(Showing::Password { .. })).then_some(TextPurpose::Password);
        self.panel.set_typing(shell, purpose)
    }

    /// Whether `window` is the menu.
    pub fn owns(&self, window: WindowId) -> bool {
        self.panel.owns(window)
    }

    /// Reads one shell event for its surface's frame request.
    pub fn observe(&mut self, event: &ShellEvent) {
        self.panel.observe(event);
    }

    /// Whether it has a frame to present that it can present now.
    pub fn ready(&self) -> bool {
        self.panel.ready()
    }

    /// Draws the menu again for `wifi` as it is at `now`.
    pub fn refresh(
        &mut self,
        shell: &mut dyn Shell,
        look: &Look<'_>,
        wifi: &Wifi,
        now: Instant,
    ) -> anyhow::Result<()> {
        // Joining keeps the rows as they were, so the one asked about stays.
        if !matches!(self.showing, Some(Showing::Joining(_))) {
            self.networks = wifi.networks();
        }
        // A new question's Enter answers it, wherever the outline was.
        if self.showing != self.shown {
            self.panel.unfocus();
            self.shown.clone_from(&self.showing);
        }
        let drawn = self.drawn(look, wifi, now);
        self.panel.show(shell, drawn)?;
        self.drawn_scan = self.scan(wifi, now);
        // A new header needs its text input set.
        self.take_typing(shell)
    }

    /// Shows how the join it started ended.
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    pub fn joined(&mut self, joined: Joined) {
        let network = match self.showing.take() {
            Some(Showing::Joining(ssid)) => self.networks.iter().find(|n| n.ssid == ssid).cloned(),
            other => {
                self.showing = other;
                return;
            }
        };
        self.typed.clear();
        self.showing = match (joined, network) {
            (Joined::Up, _) => None,
            (Joined::Failed(message), _) => Some(Showing::Error(message)),
            (Joined::NewPassword, Some(network)) => Some(Showing::Password {
                network,
                again: true,
            }),
            (Joined::NewPassword, None) => None,
        };
    }

    /// Applies an event for the menu's window.
    pub fn event(&mut self, shell: &mut dyn Shell, event: &ShellEvent) -> anyhow::Result<Outcome> {
        if !event.window().is_some_and(|window| self.panel.owns(window)) {
            return Ok(Outcome::Kept);
        }
        let typing = matches!(self.showing, Some(Showing::Password { .. }));
        match *event {
            ShellEvent::Key {
                key_code: Some(KeyCode::Escape),
                state: ButtonState::Pressed,
                ..
            } if matches!(
                self.showing,
                Some(Showing::Password { .. } | Showing::Forget(_) | Showing::Error(_))
            ) =>
            {
                self.showing = None;
                self.typed.clear();
                return Ok(Outcome::Redraw);
            }
            ShellEvent::TextCommit { ref text, .. } if typing => {
                // Spaces count, leading and trailing ones too.
                self.typed.extend(text.chars().filter(|c| !c.is_control()));
                return Ok(Outcome::Redraw);
            }
            ShellEvent::Key {
                key_code: Some(KeyCode::Backspace),
                state: ButtonState::Pressed,
                ..
            } if typing => {
                self.typed.pop();
                return Ok(Outcome::Redraw);
            }
            _ => {}
        }
        Ok(match self.panel.event(shell, event)? {
            Event::Kept => Outcome::Kept,
            Event::Close => Outcome::Close,
            Event::Closed => Outcome::Closed,
            Event::Redraw => Outcome::Redraw,
            Event::Pick(target) => self.act(target),
            Event::Enter if typing => self.act(Target::Answer(true)),
            Event::Enter => Outcome::Kept,
        })
    }

    /// What choosing `target` does.
    fn act(&mut self, target: Target) -> Outcome {
        match target {
            Target::Radio => Outcome::ToggleRadio,
            Target::Join(index) => {
                let Some(network) = self.networks.get(index).cloned() else {
                    return Outcome::Kept;
                };
                if let Some(refusal) = network.security.refusal() {
                    self.showing = Some(Showing::Error(format!("{}: {refusal}", network.ssid)));
                    return Outcome::Redraw;
                }
                if network.security.needs_password() && !network.known {
                    self.typed.clear();
                    self.showing = Some(Showing::Password {
                        network,
                        again: false,
                    });
                    return Outcome::Redraw;
                }
                self.showing = Some(Showing::Joining(network.ssid.clone()));
                Outcome::Join(network, None)
            }
            Target::Forget(index) => match self.networks.get(index) {
                Some(network) => {
                    self.showing = Some(Showing::Forget(network.ssid.clone()));
                    Outcome::Redraw
                }
                None => Outcome::Kept,
            },
            Target::Answer(yes) => {
                let showing = self.showing.take();
                match (showing, yes) {
                    (Some(Showing::Password { network, again }), true) => {
                        if !password_fits(network.security, &self.typed) {
                            self.showing = Some(Showing::Password { network, again });
                            return Outcome::Kept;
                        }
                        let password = std::mem::take(&mut self.typed);
                        self.showing = Some(Showing::Joining(network.ssid.clone()));
                        if again {
                            Outcome::NewPassword(network, password)
                        } else {
                            Outcome::Join(network, Some(password))
                        }
                    }
                    (Some(Showing::Forget(ssid)), true) => Outcome::Forget(ssid),
                    (Some(Showing::Joining(ssid)), _) => {
                        // Nothing answers a join under way.
                        self.showing = Some(Showing::Joining(ssid));
                        Outcome::Kept
                    }
                    _ => {
                        self.typed.clear();
                        Outcome::Redraw
                    }
                }
            }
        }
    }

    /// Draws the menu if it has a frame to present.
    pub fn draw(&mut self, shell: &mut dyn Shell, text: &Text) -> anyhow::Result<()> {
        self.panel.draw(shell, text)
    }

    /// Closes the menu: its surface, then its window unless the window
    /// system has closed it already (`window_gone`).
    pub fn close(self, shell: &mut dyn Shell, window_gone: bool) -> anyhow::Result<()> {
        self.panel.close(shell, window_gone)
    }
}

#[cfg(test)]
mod tests;
