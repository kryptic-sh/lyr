//! What the Bluetooth and Wi-Fi menus do alike: a panel at the top left of
//! the output, taking the keyboard, drawn from a draw list and its input
//! regions, the arrow keys moving an outline through the regions in the
//! order drawn, and Escape, losing focus or the window system closing it.
//!
//! It is a panel rather than a popup so `lyr panel …` can open it: a
//! Wayland popup's grab needs the serial of the input that opened it. It is
//! recreated when its size changes, since crcbl 0.5.0 cannot resize a panel.

use crcbl_shell::{
    ButtonState, CloseReply, KeyCode, LogicalSize, Margins, PanelAnchor, PanelDesc, PanelKeyboard,
    PanelLayer, Reserve, Shell, ShellEvent, TextPurpose, WindowDesc, WindowId, WindowRole,
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

/// An open menu's panel.
pub struct MenuPanel<T> {
    window: WindowId,
    /// From the output's left edge.
    left: f64,
    size: Size,
    list: DrawList,
    regions: Regions<T>,
    /// The keyboard's place, by the target's index in the order drawn.
    focus: Option<usize>,
    pointer: Pointer,
    scale: f32,
    surface: Option<Surface>,
    dirty: bool,
}

/// What an event did to the panel.
#[derive(Debug, PartialEq, Eq)]
pub enum Event<T> {
    Kept,
    /// It is to close.
    Close,
    /// The window system is closing its window.
    Closed,
    /// A click on a target, or Enter on the outlined one.
    Pick(T),
    /// Enter with nothing outlined.
    Enter,
    /// The keyboard moved its place: the menu is to be drawn again.
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

impl<T: Copy> MenuPanel<T> {
    /// Opens a panel showing `list`, `size` big, with its input `regions`,
    /// `left` from the output's left edge.
    pub fn open(
        shell: &mut dyn Shell,
        left: f64,
        (list, size, regions): (DrawList, Size, Regions<T>),
    ) -> anyhow::Result<Self> {
        Ok(Self {
            window: create(shell, size, left)?,
            left,
            size,
            list,
            regions,
            focus: None,
            pointer: Pointer::default(),
            scale: 1.0,
            surface: None,
            dirty: false,
        })
    }

    /// Whether `window` is the panel's.
    pub fn owns(&self, window: WindowId) -> bool {
        self.window == window
    }

    /// Its window, which a recreation replaces.
    #[cfg(test)]
    pub const fn window(&self) -> WindowId {
        self.window
    }

    /// Its size.
    #[cfg(test)]
    pub const fn size(&self) -> Size {
        self.size
    }

    /// Whether it has a frame to present.
    pub const fn dirty(&self) -> bool {
        self.dirty
    }

    /// Turns the window system's text input on for `purpose`, or off.
    pub fn set_typing(
        &self,
        shell: &mut dyn Shell,
        purpose: Option<TextPurpose>,
    ) -> anyhow::Result<()> {
        Ok(shell.set_text_input(self.window, purpose)?)
    }

    /// The target the keyboard is on.
    pub fn focused(&self) -> Option<T> {
        self.regions.targets().nth(self.focus?).copied()
    }

    /// Shows `list`, `size` big, with its input `regions`, keeping the
    /// keyboard's place within them, and recreating the window if its size
    /// changed. Returns whether it did, which turns its text input off.
    pub fn show(
        &mut self,
        shell: &mut dyn Shell,
        (list, size, regions): (DrawList, Size, Regions<T>),
    ) -> anyhow::Result<bool> {
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
        self.list = list;
        self.regions = regions;
        self.dirty = true;
        Ok(resized)
    }

    /// Applies an event for the panel's window. A menu that types takes
    /// its text, Backspace and its own Escape before this.
    pub fn event(&mut self, shell: &mut dyn Shell, event: &ShellEvent) -> anyhow::Result<Event<T>> {
        if event.window() != Some(self.window) {
            return Ok(Event::Kept);
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
                return Ok(Event::Closed);
            }
            ShellEvent::Focus { focused: false, .. }
            | ShellEvent::Key {
                key_code: Some(KeyCode::Escape),
                state: ButtonState::Pressed,
                ..
            } => return Ok(Event::Close),
            ShellEvent::Key {
                key_code: Some(key),
                state: ButtonState::Pressed,
                ..
            } => {
                let count = self.regions.targets().count();
                if count == 0 {
                    return Ok(Event::Kept);
                }
                match key {
                    KeyCode::ArrowDown => {
                        self.focus = Some(self.focus.map_or(0, |at| (at + 1).min(count - 1)));
                        return Ok(Event::Redraw);
                    }
                    KeyCode::ArrowUp => {
                        self.focus = Some(self.focus.map_or(0, |at| at.saturating_sub(1)));
                        return Ok(Event::Redraw);
                    }
                    KeyCode::Enter | KeyCode::NumpadEnter => {
                        return Ok(self.focused().map_or(Event::Enter, Event::Pick));
                    }
                    _ => {}
                }
            }
            _ => {
                let presses = self.pointer.handle(event, self.scale);
                if presses.contains(&Press::Left)
                    && let Some(&target) = self.pointer.target(&self.regions)
                {
                    return Ok(Event::Pick(target));
                }
            }
        }
        Ok(Event::Kept)
    }

    /// Draws the panel if it has a frame to present.
    pub fn draw(&mut self, text: &Text) -> anyhow::Result<()> {
        if let (true, Some(surface)) = (self.dirty, &mut self.surface) {
            self.dirty = !surface.draw(&translate(&self.list, text))?;
        }
        Ok(())
    }

    /// Closes the panel: its surface, then its window unless the window
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
