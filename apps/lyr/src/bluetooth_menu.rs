//! The Bluetooth menu (`docs/plan/10-popups-and-menus.md#bluetooth-menu`): a
//! panel at the top left, under the Bluetooth module where a bar shows one,
//! taking the keyboard. A click on a switch or a device's button is the
//! caller's to carry out, as is Enter on the one the arrow keys have moved
//! to, and so are the answer to the pairing agent's question and a paired
//! device's new name, what is typed coming through the window system's text
//! input; `Escape` (once a rename is cancelled), losing focus and the same
//! click or binding again close it.
//!
//! It is a panel rather than a popup so `lyr panel bluetooth` can open it:
//! a Wayland popup's grab needs the serial of the input that opened it. It is
//! recreated when its size changes (a scan finding a device), since crcbl
//! 0.5.0 cannot resize a panel.

use crcbl_shell::{
    ButtonState, CloseReply, KeyCode, LogicalSize, Margins, PanelAnchor, PanelDesc, PanelKeyboard,
    PanelLayer, Reserve, Shell, ShellEvent, TextPurpose, WindowDesc, WindowId, WindowRole,
};
use lyr_core::bluetooth_menu::{
    self, Adapter, Ask, Device, Header, PIN_LENGTH, Question, Reply, Target, order,
};
use lyr_core::draw::{DrawList, Size};
use lyr_core::hit::Regions;
use lyr_core::pointer::Press;
use lyr_render::surface::Surface;
use lyr_render::text::Text;
use lyr_render::translate::translate;
use lyr_shell::bars::NAMESPACE;

use crate::input::Pointer;
use crate::toasts::EDGE_MARGIN;
use crate::tooltips::Look;

/// The open menu.
pub struct BluetoothMenu {
    window: WindowId,
    /// From the output's left edge.
    left: f64,
    size: Size,
    list: DrawList,
    regions: Regions<Target>,
    /// The keyboard's place, by the target's index in the order drawn.
    focus: Option<usize>,
    /// The devices' paths, in the order drawn.
    paths: Vec<String>,
    /// The device being renamed, by its path, until the name is sent or
    /// the pairing agent asks something.
    renaming: Option<String>,
    /// What the header shows.
    showing: Option<Showing>,
    /// What has been typed into the header's field.
    typed: String,
    pointer: Pointer,
    scale: f32,
    surface: Option<Surface>,
    dirty: bool,
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

/// The panel's window, `size` big, `left` from the output's left edge.
fn create(shell: &mut dyn Shell, size: Size, left: f64) -> anyhow::Result<WindowId> {
    Ok(shell.create_window(&WindowDesc {
        title: NAMESPACE,
        app_id: NAMESPACE,
        size: LogicalSize {
            width: f64::from(size.width),
            height: f64::from(size.height),
        },
        transparent: true,
        role: WindowRole::Panel(PanelDesc {
            namespace: NAMESPACE,
            monitor: None,
            layer: PanelLayer::Overlay,
            anchor: PanelAnchor::TOP | PanelAnchor::LEFT,
            reserve: Reserve::None,
            margins: Margins {
                top: EDGE_MARGIN,
                right: 0.0,
                bottom: 0.0,
                left,
            },
            keyboard: PanelKeyboard::Exclusive,
        }),
        ..WindowDesc::default()
    })?)
}

impl BluetoothMenu {
    /// Opens the menu for `adapter` and its `devices`, `left` from the
    /// output's left edge.
    pub fn open(
        shell: &mut dyn Shell,
        left: f64,
        look: &Look<'_>,
        adapter: &Adapter,
        devices: &[Device],
        ask: Option<Ask<'_>>,
    ) -> anyhow::Result<Self> {
        let (list, size, regions) = bluetooth_menu::draw(
            adapter,
            devices,
            ask.map(Header::Ask),
            None,
            look.measure,
            look.line_height,
            look.theme,
        );
        let menu = Self {
            window: create(shell, size, left)?,
            left,
            size,
            list,
            regions,
            focus: None,
            paths: paths(devices),
            renaming: None,
            showing: ask.map(|ask| Showing::Ask(ask.device.to_owned(), ask.question)),
            typed: String::new(),
            pointer: Pointer::default(),
            scale: 1.0,
            surface: None,
            dirty: false,
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
        Ok(shell.set_text_input(self.window, self.field())?)
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
        self.window == window
    }

    /// Whether it has a frame to present.
    pub fn dirty(&self) -> bool {
        self.dirty
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
        self.paths = paths(devices);
        let (list, size, regions) = bluetooth_menu::draw(
            adapter,
            devices,
            self.header(ask, devices),
            self.focused(),
            look.measure,
            look.line_height,
            look.theme,
        );
        let count = regions.targets().count();
        self.focus = self.focus.filter(|_| count > 0).map(|at| at.min(count - 1));
        let resized = size != self.size;
        if resized {
            if let Some(surface) = self.surface.take() {
                surface.destroy()?;
            }
            shell.destroy_window(self.window)?;
            self.window = create(shell, size, self.left)?;
            self.size = size;
        }
        if resized || new_header {
            self.take_typing(shell)?;
        }
        self.list = list;
        self.regions = regions;
        self.dirty = true;
        Ok(())
    }

    /// Applies an event for the menu's window.
    pub fn event(&mut self, shell: &mut dyn Shell, event: &ShellEvent) -> anyhow::Result<Outcome> {
        if event.window() != Some(self.window) {
            return Ok(Outcome::Kept);
        }
        match *event {
            ShellEvent::Resized {
                size, scale_factor, ..
            }
            | ShellEvent::ScaleFactorChanged {
                size, scale_factor, ..
            } => {
                let device = (size.width, size.height);
                self.scale = scale_factor as f32;
                match &mut self.surface {
                    Some(surface) => surface.resize(device, self.scale)?,
                    None => {
                        self.surface = Some(Surface::open(shell, self.window, device, self.scale)?);
                    }
                }
                self.dirty = true;
            }
            ShellEvent::CloseRequested { window } => {
                shell.reply_close_request(window, CloseReply::Close)?;
                return Ok(Outcome::Closed);
            }
            ShellEvent::Key {
                key_code: Some(KeyCode::Escape),
                state: ButtonState::Pressed,
                ..
            } if self.renaming.is_some() => {
                self.renaming = None;
                return Ok(Outcome::Redraw);
            }
            ShellEvent::Focus { focused: false, .. }
            | ShellEvent::Key {
                key_code: Some(KeyCode::Escape),
                state: ButtonState::Pressed,
                ..
            } => return Ok(Outcome::Close),
            ShellEvent::TextCommit { ref text, .. } if self.field().is_some() => {
                let room = match self.field() {
                    Some(TextPurpose::Pin) => PIN_LENGTH - self.typed.chars().count(),
                    _ => usize::MAX,
                };
                self.typed
                    .extend(text.chars().filter(|c| !c.is_control()).take(room));
                return Ok(Outcome::Redraw);
            }
            ShellEvent::Key {
                key_code: Some(KeyCode::Backspace),
                state: ButtonState::Pressed,
                ..
            } if self.field().is_some() => {
                self.typed.pop();
                return Ok(Outcome::Redraw);
            }
            ShellEvent::Key {
                key_code: Some(key),
                state: ButtonState::Pressed,
                ..
            } => {
                let count = self.regions.targets().count();
                if count == 0 {
                    return Ok(Outcome::Kept);
                }
                match key {
                    KeyCode::ArrowDown => {
                        self.focus = Some(self.focus.map_or(0, |at| (at + 1).min(count - 1)));
                        return Ok(Outcome::Redraw);
                    }
                    KeyCode::ArrowUp => {
                        self.focus = Some(self.focus.map_or(0, |at| at.saturating_sub(1)));
                        return Ok(Outcome::Redraw);
                    }
                    // With nothing chosen, Enter sends what was typed.
                    KeyCode::Enter | KeyCode::NumpadEnter => match self.focused() {
                        Some(target) => return Ok(self.act(target)),
                        None if self.field().is_some() => {
                            return Ok(self.act(Target::Answer(true)));
                        }
                        None => {}
                    },
                    _ => {}
                }
            }
            _ => {
                let presses = self.pointer.handle(event, self.scale);
                if presses.contains(&Press::Left)
                    && let Some(&target) = self.pointer.target(&self.regions)
                {
                    return Ok(self.act(target));
                }
            }
        }
        Ok(Outcome::Kept)
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

    /// The target the keyboard is on.
    fn focused(&self) -> Option<Target> {
        self.regions.targets().nth(self.focus?).copied()
    }

    /// Draws the menu if it has a frame to present.
    pub fn draw(&mut self, text: &Text) -> anyhow::Result<()> {
        if let (true, Some(surface)) = (self.dirty, &mut self.surface) {
            self.dirty = !surface.draw(&translate(&self.list, text))?;
        }
        Ok(())
    }

    /// Closes the menu: its surface, then its window unless the window
    /// system has closed it already (`window_gone`).
    pub fn close(self, shell: &mut dyn Shell, window_gone: bool) -> anyhow::Result<()> {
        if let Some(surface) = self.surface {
            surface.destroy()?;
        }
        if !window_gone {
            shell.destroy_window(self.window)?;
        }
        Ok(())
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
