//! The power menu's popup (`docs/plan/10-popups-and-menus.md#power-menu`):
//! anchored to the module that opened it, taking the pointer and the
//! keyboard while open. Arrow keys and the pointer move the selection; Enter
//! or a click carries the entry out and closes the menu, and `Escape` or a
//! click outside it closes it (both reported by crcbl as a close request).

use crcbl_shell::{
    ButtonState, CloseReply, KeyCode, LogicalPoint, LogicalRect, LogicalSize, PopupDesc,
    PopupInput, PopupPlacement, Shell, ShellEvent, WindowDesc, WindowId, WindowRole,
};
use lyr_config::Position;
use lyr_core::draw::DrawList;
use lyr_core::hit::{Rect, Regions};
use lyr_core::power_menu::{Entry, Menu};
use lyr_render::surface::Surface;
use lyr_render::text::Text;
use lyr_render::translate::translate;
use lyr_shell::bars::NAMESPACE;

use crate::input::Pointer;
use crate::tooltips::Look;

/// The command the config sets for `entry`, or `None` for lyr's own action.
pub fn configured(menu: &lyr_config::PowerMenu, entry: Entry) -> Option<&lyr_config::Exec> {
    match entry {
        Entry::Lock => menu.lock.as_ref(),
        Entry::Sleep => menu.sleep.as_ref(),
        Entry::Hibernate => menu.hibernate.as_ref(),
        Entry::Shutdown => menu.shutdown.as_ref(),
        Entry::Logout => menu.logout.as_ref(),
        Entry::Reboot => menu.reboot.as_ref(),
    }
}

/// The open power menu.
pub struct PowerMenu {
    window: WindowId,
    /// The bar it opened from.
    bar: WindowId,
    menu: Menu,
    list: DrawList,
    regions: Regions<Entry>,
    pointer: Pointer,
    /// Device pixels per logical pixel, from its first resize.
    scale: f32,
    /// Opened on the window's first resize.
    surface: Option<Surface>,
    /// Whether it must be drawn: it changed, or has not presented since.
    dirty: bool,
}

/// What an event did to the menu.
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Nothing the caller acts on.
    Kept,
    /// An entry was chosen; the menu is to close and carry it out.
    Chosen(Entry),
    /// The window system is closing the menu's window, with nothing chosen:
    /// `Escape`, or a click outside it.
    Dismissed,
}

impl PowerMenu {
    /// Opens a menu of `entries` anchored to `anchor` on the bar `bar`.
    /// `None` when there are no entries.
    pub fn open(
        shell: &mut dyn Shell,
        bar: WindowId,
        anchor: Rect,
        position: Position,
        entries: Vec<Entry>,
        look: &Look<'_>,
    ) -> anyhow::Result<Option<Self>> {
        let Some(menu) = Menu::new(entries) else {
            return Ok(None);
        };
        let (list, size, regions) = menu.draw(look.measure, look.line_height, look.theme);
        let window = shell.create_window(&WindowDesc {
            title: NAMESPACE,
            app_id: NAMESPACE,
            size: LogicalSize {
                width: f64::from(size.width),
                height: f64::from(size.height),
            },
            role: WindowRole::Popup(PopupDesc {
                parent: bar,
                anchor: LogicalRect {
                    origin: LogicalPoint {
                        x: f64::from(anchor.x),
                        y: f64::from(anchor.y),
                    },
                    size: LogicalSize {
                        width: f64::from(anchor.width),
                        height: f64::from(anchor.height),
                    },
                },
                placement: match position {
                    Position::Top => PopupPlacement::Below,
                    Position::Bottom => PopupPlacement::Above,
                },
                input: PopupInput::Grab,
            }),
            ..WindowDesc::default()
        })?;
        Ok(Some(Self {
            window,
            bar,
            menu,
            list,
            regions,
            pointer: Pointer::default(),
            scale: 1.0,
            surface: None,
            dirty: false,
        }))
    }

    /// Whether `window` is this menu.
    pub fn owns(&self, window: WindowId) -> bool {
        self.window == window
    }

    /// Whether the menu opened from the bar `bar`.
    pub fn opened_from(&self, bar: WindowId) -> bool {
        self.bar == bar
    }

    /// Whether the menu has a frame to present.
    pub fn dirty(&self) -> bool {
        self.dirty
    }

    /// Applies an event for the menu's window.
    pub fn event(
        &mut self,
        shell: &mut dyn Shell,
        event: &ShellEvent,
        look: &Look<'_>,
    ) -> anyhow::Result<Outcome> {
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
                return Ok(Outcome::Dismissed);
            }
            ShellEvent::Key {
                key_code: Some(key),
                state: ButtonState::Pressed,
                ..
            } => match key {
                KeyCode::ArrowUp => self.moved(look, Menu::up),
                KeyCode::ArrowDown => self.moved(look, Menu::down),
                KeyCode::Enter | KeyCode::NumpadEnter => {
                    return Ok(Outcome::Chosen(self.menu.selected()));
                }
                // `Escape` comes back as a close request: crcbl's grab
                // queues one after the key (0.6.0, every backend).
                _ => {}
            },
            _ => {
                let presses = self.pointer.handle(event, self.scale);
                if let Some(&entry) = self.pointer.target(&self.regions) {
                    if self.menu.select(entry) {
                        self.redrawn(look);
                    }
                    if !presses.is_empty() {
                        return Ok(Outcome::Chosen(entry));
                    }
                }
            }
        }
        Ok(Outcome::Kept)
    }

    /// Moves the selection with `step` and redraws.
    fn moved(&mut self, look: &Look<'_>, step: fn(&mut Menu)) {
        step(&mut self.menu);
        self.redrawn(look);
    }

    /// Rebuilds the draw list for the current selection.
    fn redrawn(&mut self, look: &Look<'_>) {
        let (list, _, regions) = self.menu.draw(look.measure, look.line_height, look.theme);
        self.list = list;
        self.regions = regions;
        self.dirty = true;
    }

    /// Draws the menu if it has a frame to present.
    pub fn draw(&mut self, text: &Text) -> anyhow::Result<()> {
        if let (true, Some(surface)) = (self.dirty, &mut self.surface) {
            self.dirty = !surface.draw(&translate(&self.list, text))?;
        }
        Ok(())
    }

    /// Closes the menu: its surface, then its window unless the system has
    /// destroyed it already (`window_gone`).
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

#[cfg(test)]
mod tests;
