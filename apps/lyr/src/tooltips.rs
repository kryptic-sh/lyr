//! A bar's tooltip (`docs/plan/10-popups-and-menus.md#tooltips`): when it
//! shows ([`lyr_core::tooltip`]) and the popup window that shows it, anchored
//! to the module's box and taking no input, so the pointer stays on the bar.

use std::time::Instant;

use crcbl_shell::{
    LogicalPoint, LogicalRect, LogicalSize, PopupDesc, PopupInput, PopupPlacement, Shell,
    WindowDesc, WindowId, WindowRole,
};
use lyr_config::{Module, Position, Theme};
use lyr_core::draw::{self, DrawList, Target};
use lyr_core::hit::Regions;
use lyr_core::tooltip::Tooltip;
use lyr_render::surface::Surface;
use lyr_render::text::Text;
use lyr_render::translate::translate;
use lyr_shell::bars::NAMESPACE;

/// The popup showing a tooltip.
struct Popup {
    window: WindowId,
    module: Module,
    text: String,
    list: DrawList,
    /// Opened on the window's first resize.
    surface: Option<Surface>,
    /// Whether it must be drawn: it has not presented since it was sized.
    dirty: bool,
}

/// The bar a tooltip opens from: its window, the input regions of its last
/// frame (the module boxes), and the screen edge it is on.
pub struct Anchor<'a> {
    pub bar: WindowId,
    pub regions: &'a Regions<Target<Module>>,
    pub position: Position,
}

/// How a tooltip looks: the theme's colours, and the bar font's measure.
pub struct Look<'a> {
    pub theme: &'a Theme,
    pub measure: &'a dyn Fn(&str) -> f32,
    pub line_height: f32,
}

/// One bar's tooltip.
#[derive(Default)]
pub struct Tooltips {
    timer: Tooltip<Module>,
    popup: Option<Popup>,
}

impl Tooltips {
    /// The pointer is over `target`; workspace buttons have no tooltip.
    pub fn hover(&mut self, target: Option<&Target<Module>>, now: Instant) {
        let module = match target {
            Some(Target::Module(module)) => Some(*module),
            _ => None,
        };
        self.timer.hover(module, now);
    }

    /// A click, which hides the tooltip.
    pub fn press(&mut self, now: Instant) {
        self.timer.press(now);
    }

    /// When the tooltip next has something to do.
    pub fn next_wake(&self) -> Option<Instant> {
        self.timer.next_wake()
    }

    /// Whether `window` is this bar's tooltip.
    pub fn owns(&self, window: WindowId) -> bool {
        self.popup
            .as_ref()
            .is_some_and(|popup| popup.window == window)
    }

    /// Whether the tooltip still has a frame to present.
    pub fn dirty(&self) -> bool {
        self.popup.as_ref().is_some_and(|popup| popup.dirty)
    }

    /// Opens, closes or replaces the popup so it shows what should show now:
    /// the hovered module's tooltip (`tooltip_of`) once its delay is up,
    /// anchored to the module's box on the bar.
    pub fn sync(
        &mut self,
        shell: &mut dyn Shell,
        anchor: &Anchor<'_>,
        look: &Look<'_>,
        tooltip_of: impl Fn(Module) -> Option<String>,
        now: Instant,
    ) -> anyhow::Result<()> {
        self.timer.tick(now);
        let wanted = self
            .timer
            .shown()
            .and_then(|&module| Some((module, tooltip_of(module)?)));
        let current = self
            .popup
            .as_ref()
            .map(|popup| (popup.module, popup.text.as_str()));
        if current
            == wanted
                .as_ref()
                .map(|(module, text)| (*module, text.as_str()))
        {
            return Ok(());
        }
        self.close(shell)?;
        let Some((module, text)) = wanted else {
            return Ok(());
        };
        let Some(rect) = anchor.regions.rect(&Target::Module(module)) else {
            return Ok(());
        };
        let (list, size) =
            draw::tooltip(&text, (look.measure)(&text), look.line_height, look.theme);
        let window = shell.create_window(&WindowDesc {
            title: NAMESPACE,
            app_id: NAMESPACE,
            size: LogicalSize {
                width: f64::from(size.width),
                height: f64::from(size.height),
            },
            role: WindowRole::Popup(PopupDesc {
                parent: anchor.bar,
                anchor: LogicalRect {
                    origin: LogicalPoint {
                        x: f64::from(rect.x),
                        y: f64::from(rect.y),
                    },
                    size: LogicalSize {
                        width: f64::from(rect.width),
                        height: f64::from(rect.height),
                    },
                },
                // Away from the screen edge the bar is on.
                placement: match anchor.position {
                    Position::Top => PopupPlacement::Below,
                    Position::Bottom => PopupPlacement::Above,
                },
                input: PopupInput::Passthrough,
            }),
            ..WindowDesc::default()
        })?;
        self.popup = Some(Popup {
            window,
            module,
            text,
            list,
            surface: None,
            dirty: false,
        });
        Ok(())
    }

    /// The popup was sized: opens its surface, or resizes it, to draw at
    /// `device` pixels.
    pub fn resized(
        &mut self,
        shell: &dyn Shell,
        device: (u32, u32),
        scale: f32,
    ) -> anyhow::Result<()> {
        let Some(popup) = &mut self.popup else {
            return Ok(());
        };
        match &mut popup.surface {
            Some(surface) => surface.resize(device, scale)?,
            None => popup.surface = Some(Surface::open(shell, popup.window, device, scale)?),
        }
        popup.dirty = true;
        Ok(())
    }

    /// Draws the popup if it has a frame to present.
    pub fn draw(&mut self, text: &Text) -> anyhow::Result<()> {
        let Some(popup) = &mut self.popup else {
            return Ok(());
        };
        if let (true, Some(surface)) = (popup.dirty, &mut popup.surface) {
            popup.dirty = !surface.draw(&translate(&popup.list, text))?;
        }
        Ok(())
    }

    /// Closes the popup, if one is open.
    pub fn close(&mut self, shell: &mut dyn Shell) -> anyhow::Result<()> {
        if let Some(popup) = self.popup.take() {
            if let Some(surface) = popup.surface {
                surface.destroy()?;
            }
            shell.destroy_window(popup.window)?;
        }
        Ok(())
    }

    /// The system destroyed the popup's window (its bar went, say): drops
    /// what drew into it.
    pub fn destroyed(&mut self) -> anyhow::Result<()> {
        if let Some(surface) = self.popup.take().and_then(|popup| popup.surface) {
            surface.destroy()?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
