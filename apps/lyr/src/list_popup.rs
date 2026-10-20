//! A list menu's popup ([`lyr_core::list_menu`]), as the power menu
//! (`docs/plan/10-popups-and-menus.md#power-menu`) and a tray item's menu
//! open it: anchored to the module that opened it, taking the pointer and the
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
use lyr_core::list_menu::{Menu, Row};
use lyr_render::surface::Surface;
use lyr_render::text::Text;
use lyr_render::translate::translate;
use lyr_shell::bars::NAMESPACE;

use crate::input::Pointer;
use crate::tooltips::Look;

/// An open list menu, whose rows stand for `T`s.
pub struct ListPopup<T> {
    window: WindowId,
    /// The bar it opened from.
    bar: WindowId,
    menu: Menu<T>,
    list: DrawList,
    regions: Regions<T>,
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
pub enum Outcome<T> {
    /// Nothing the caller acts on.
    Kept,
    /// A row was chosen; the menu is to close and carry it out.
    Chosen(T),
    /// The window system is closing the menu's window, with nothing chosen:
    /// `Escape`, or a click outside it.
    Dismissed,
}

impl<T: Clone + PartialEq> ListPopup<T> {
    /// Opens a menu of `rows` anchored to `anchor` on the bar `bar`.
    /// `None` when there are no rows.
    pub fn open(
        shell: &mut dyn Shell,
        bar: WindowId,
        anchor: Rect,
        position: Position,
        rows: Vec<Row<T>>,
        look: &Look<'_>,
    ) -> anyhow::Result<Option<Self>> {
        let Some(menu) = Menu::new(rows) else {
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

    /// Its window.
    #[cfg(test)]
    pub const fn window(&self) -> WindowId {
        self.window
    }

    /// Whether the menu opened from the bar `bar`.
    pub fn opened_from(&self, bar: WindowId) -> bool {
        self.bar == bar
    }

    /// Reads one shell event for its surface's frame request.
    pub fn observe(&mut self, event: &ShellEvent) {
        if let Some(surface) = &mut self.surface {
            surface.observe(event);
        }
    }

    /// Whether it has a frame to present that it can present now; one whose
    /// last frame's answer has not come waits for it rather than for nothing.
    pub fn ready(&self) -> bool {
        self.dirty && self.surface.as_ref().is_none_or(Surface::frame_due)
    }

    /// Applies an event for the menu's window.
    pub fn event(
        &mut self,
        shell: &mut dyn Shell,
        event: &ShellEvent,
        look: &Look<'_>,
    ) -> anyhow::Result<Outcome<T>> {
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
                    if let Some(value) = self.menu.selected() {
                        return Ok(Outcome::Chosen(value.clone()));
                    }
                }
                // `Escape` comes back as a close request: crcbl's grab
                // queues one after the key (0.6.0, every backend).
                _ => {}
            },
            _ => {
                let presses = self.pointer.handle(event, self.scale);
                if let Some(value) = self.pointer.target(&self.regions).cloned() {
                    if self.menu.select(&value) {
                        self.redrawn(look);
                    }
                    if !presses.is_empty() {
                        return Ok(Outcome::Chosen(value));
                    }
                }
            }
        }
        Ok(Outcome::Kept)
    }

    /// Moves the selection with `step` and redraws.
    fn moved(&mut self, look: &Look<'_>, step: fn(&mut Menu<T>)) {
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
    pub fn draw(&mut self, shell: &mut dyn Shell, text: &Text) -> anyhow::Result<()> {
        if let (true, Some(surface)) = (self.dirty, &mut self.surface) {
            self.dirty = !surface.draw(shell, &translate(&self.list, text))?;
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
