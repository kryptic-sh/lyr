//! The Bluetooth menu (`docs/plan/10-popups-and-menus.md#bluetooth-menu`): a
//! panel at the top left, under the Bluetooth module where a bar shows one,
//! taking the keyboard. A click on a switch or a device's button is the
//! caller's to carry out, as is Enter on the one the arrow keys have moved
//! to; `Escape`, losing focus and the same click or binding again close
//! it.
//!
//! It is a panel rather than a popup so `lyr panel bluetooth` can open it:
//! a Wayland popup's grab needs the serial of the input that opened it. It is
//! recreated when its size changes (a scan finding a device), since crcbl
//! 0.5.0 cannot resize a panel.

use crcbl_shell::{
    ButtonState, CloseReply, KeyCode, LogicalSize, Margins, PanelAnchor, PanelDesc, PanelKeyboard,
    PanelLayer, Reserve, Shell, ShellEvent, WindowDesc, WindowId, WindowRole,
};
use lyr_core::bluetooth_menu::{self, Adapter, Ask, Device, Target};
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
    pointer: Pointer,
    scale: f32,
    surface: Option<Surface>,
    dirty: bool,
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
    /// The keyboard's place moved: the menu is to be drawn again.
    Moved,
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
            ask,
            None,
            look.measure,
            look.line_height,
            look.theme,
        );
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
        let (list, size, regions) = bluetooth_menu::draw(
            adapter,
            devices,
            ask,
            self.focused(),
            look.measure,
            look.line_height,
            look.theme,
        );
        let count = regions.targets().count();
        self.focus = self.focus.filter(|_| count > 0).map(|at| at.min(count - 1));
        if size != self.size {
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
            ShellEvent::Focus { focused: false, .. }
            | ShellEvent::Key {
                key_code: Some(KeyCode::Escape),
                state: ButtonState::Pressed,
                ..
            } => return Ok(Outcome::Close),
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
                        return Ok(Outcome::Moved);
                    }
                    KeyCode::ArrowUp => {
                        self.focus = Some(self.focus.map_or(0, |at| at.saturating_sub(1)));
                        return Ok(Outcome::Moved);
                    }
                    KeyCode::Enter | KeyCode::NumpadEnter => {
                        if let Some(target) = self.focused() {
                            return Ok(Outcome::Act(target));
                        }
                    }
                    _ => {}
                }
            }
            _ => {
                let presses = self.pointer.handle(event, self.scale);
                if presses.contains(&Press::Left)
                    && let Some(&target) = self.pointer.target(&self.regions)
                {
                    return Ok(Outcome::Act(target));
                }
            }
        }
        Ok(Outcome::Kept)
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

#[cfg(test)]
mod tests;
