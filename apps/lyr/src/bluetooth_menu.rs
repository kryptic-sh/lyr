//! The Bluetooth menu (`docs/plan/10-popups-and-menus.md#bluetooth-menu`): a
//! panel at the top left, under the Bluetooth module where a bar shows one,
//! taking the keyboard. A click on a switch or a device's button is the
//! caller's to carry out, as is Enter on the one the arrow keys have moved
//! to, and so are the answer to the pairing agent's question and a paired
//! device's new name, what is typed coming through the window system's text
//! input; `Escape` (once a rename is cancelled), losing focus and the same
//! click or binding again close it.
//!
//! Its panel is [`MenuPanel`]'s, recreated when its size changes (a scan
//! finding a device).

use crcbl_shell::{Shell, ShellEvent, TextPurpose, WindowId};
use lyr_core::bluetooth_menu::{
    self, Adapter, Ask, Device, Header, PIN_LENGTH, Question, Reply, Target, order,
};
use lyr_render::text::Text;

use crate::keys::{self, Key};
use crate::menu_panel::{Event, MenuPanel, Place};
use crate::tooltips::Look;

/// The open menu.
pub struct BluetoothMenu {
    panel: MenuPanel<Target>,
    /// The devices' paths, in the order drawn.
    paths: Vec<String>,
    /// The device being renamed, by its path, until the name is sent or
    /// the pairing agent asks something.
    renaming: Option<String>,
    /// What the header shows.
    showing: Option<Showing>,
    /// What has been typed into the header's field.
    typed: String,
}

/// What the menu's header shows.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Showing {
    /// The pairing agent's question, by the device's name.
    Ask(String, Question),
    /// A new name for the device at this path.
    Rename(String),
}

/// What an event did to the menu.
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    Kept,
    /// It is to close.
    Close,
    /// The window system is closing its window.
    Closed,
    /// A switch or a device's button was clicked, or chosen with Enter.
    Act(Target),
    /// The pairing agent's question was answered.
    Answer(Reply),
    /// The device at this path is to take this name.
    Rename(String, String),
    /// The keyboard moved its place or typed: the menu is to be drawn again.
    Redraw,
}

impl BluetoothMenu {
    /// Opens the menu for `adapter` and its `devices` at `place`.
    pub fn open(
        shell: &mut dyn Shell,
        place: Place,
        look: &Look<'_>,
        adapter: &Adapter,
        devices: &[Device],
        ask: Option<Ask<'_>>,
    ) -> anyhow::Result<Self> {
        let drawn = bluetooth_menu::draw(
            adapter,
            devices,
            ask.map(Header::Ask),
            None,
            look.measure,
            look.line_height,
            look.theme,
        );
        let menu = Self {
            panel: MenuPanel::open(shell, place, look.line_height, drawn)?,
            paths: paths(devices),
            renaming: None,
            showing: ask.map(|ask| Showing::Ask(ask.device.to_owned(), ask.question)),
            typed: String::new(),
        };
        menu.take_typing(shell)?;
        Ok(menu)
    }

    /// What the header's field takes, when it has one.
    const fn field(&self) -> Option<TextPurpose> {
        match self.showing {
            Some(Showing::Ask(_, Question::Pin)) => Some(TextPurpose::Pin),
            Some(Showing::Rename(_)) => Some(TextPurpose::Normal),
            Some(Showing::Ask(..)) | None => None,
        }
    }

    /// Has the window system's text input on while the header has a field.
    fn take_typing(&self, shell: &mut dyn Shell) -> anyhow::Result<()> {
        self.panel.set_typing(shell, self.field())
    }

    /// Brings the header up to date with the agent's question `ask` and
    /// the `devices`, returning whether it changed; what was typed goes
    /// when it does.
    fn follow(&mut self, ask: Option<Ask<'_>>, devices: &[Device]) -> bool {
        // The agent's question comes first, and a gone device's rename goes.
        if ask.is_some()
            || self
                .renaming
                .as_ref()
                .is_some_and(|path| !devices.iter().any(|device| device.path == *path))
        {
            self.renaming = None;
        }
        let showing = match ask {
            Some(ask) => Some(Showing::Ask(ask.device.to_owned(), ask.question)),
            None => self.renaming.clone().map(Showing::Rename),
        };
        let changed = showing != self.showing;
        if changed {
            self.showing = showing;
            self.typed.clear();
        }
        changed
    }

    /// The header for the agent's question `ask` or the rename, with what
    /// was typed.
    fn header<'a>(&'a self, ask: Option<Ask<'a>>, devices: &'a [Device]) -> Option<Header<'a>> {
        if let Some(ask) = ask {
            return Some(Header::Ask(Ask {
                typed: &self.typed,
                ..ask
            }));
        }
        let path = self.renaming.as_ref()?;
        let device = devices.iter().find(|device| device.path == *path)?;
        Some(Header::Rename {
            device: &device.alias,
            typed: &self.typed,
        })
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

    /// Draws the menu again for `adapter` and `devices` as they are now,
    /// recreating its window if its size changed.
    pub fn refresh(
        &mut self,
        shell: &mut dyn Shell,
        look: &Look<'_>,
        adapter: &Adapter,
        devices: &[Device],
        ask: Option<Ask<'_>>,
    ) -> anyhow::Result<()> {
        let new_header = self.follow(ask, devices);
        // A new question's Enter answers it, wherever the outline was.
        if new_header {
            self.panel.unfocus();
        }
        self.paths = paths(devices);
        let drawn = bluetooth_menu::draw(
            adapter,
            devices,
            self.header(ask, devices),
            self.panel.focused(),
            look.measure,
            look.line_height,
            look.theme,
        );
        self.panel.show(shell, drawn)?;
        if new_header {
            self.take_typing(shell)?;
        }
        Ok(())
    }

    /// Applies an event for the menu's window.
    pub fn event(&mut self, shell: &mut dyn Shell, event: &ShellEvent) -> anyhow::Result<Outcome> {
        if !event.window().is_some_and(|window| self.panel.owns(window)) {
            return Ok(Outcome::Kept);
        }
        match *event {
            ShellEvent::Key { .. }
                if self.renaming.is_some() && keys::pressed(event) == Some(Key::Escape) =>
            {
                self.renaming = None;
                return Ok(Outcome::Redraw);
            }
            ShellEvent::TextCommit { ref text, .. } if self.field().is_some() => {
                let room = match self.field() {
                    Some(TextPurpose::Pin) => PIN_LENGTH - self.typed.chars().count(),
                    _ => usize::MAX,
                };
                self.typed
                    .extend(text.chars().filter(|c| !c.is_control()).take(room));
                return Ok(Outcome::Redraw);
            }
            ShellEvent::Key { .. }
                if self.field().is_some() && keys::pressed(event) == Some(Key::Backspace) =>
            {
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
            // With nothing chosen, Enter sends what was typed.
            Event::Enter if self.field().is_some() => self.act(Target::Answer(true)),
            Event::Enter => Outcome::Kept,
        })
    }

    /// What choosing `target` does: a rename started, sent or cancelled,
    /// an answer to the pairing agent, which for a PIN is what was typed
    /// (nothing yet answers nothing), or the caller's to carry out.
    fn act(&mut self, target: Target) -> Outcome {
        let pin = matches!(self.showing, Some(Showing::Ask(_, Question::Pin)));
        match (target, &self.showing) {
            (Target::Rename(index), _) => match self.paths.get(index) {
                Some(path) => {
                    self.renaming = Some(path.clone());
                    Outcome::Redraw
                }
                None => Outcome::Kept,
            },
            (Target::Answer(yes), Some(Showing::Rename(path))) => {
                let outcome = if yes {
                    Outcome::Rename(path.clone(), self.typed.clone())
                } else {
                    Outcome::Redraw
                };
                self.renaming = None;
                outcome
            }
            (Target::Answer(false), _) => Outcome::Answer(Reply::No),
            (Target::Answer(true), _) if pin && self.typed.is_empty() => Outcome::Kept,
            (Target::Answer(true), _) if pin => Outcome::Answer(Reply::Pin(self.typed.clone())),
            (Target::Answer(true), _) => Outcome::Answer(Reply::Yes),
            (target, _) => Outcome::Act(target),
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

/// The `devices`' paths, in the order the menu draws them.
fn paths(devices: &[Device]) -> Vec<String> {
    order(devices)
        .into_iter()
        .map(|device| device.path.clone())
        .collect()
}

#[cfg(test)]
mod tests;
