//! A bar's tooltip (`docs/plan/10-popups-and-menus.md#tooltips`): when it
//! shows ([`lyr_core::tooltip`]) and the popup window that shows it, anchored
//! to the module's box, or a tray item's button, and taking no input, so the
//! pointer stays on the bar.

use std::time::Instant;

use crcbl_shell::{
    LogicalPoint, LogicalRect, LogicalSize, PopupDesc, PopupInput, PopupPlacement, Shell,
    ShellCaps, ShellEvent, WindowDesc, WindowId, WindowRole,
};
use lyr_config::{Module, Position, Theme};
use lyr_core::draw::{self, DrawList, Size, Target, TextStyle};
use lyr_core::hit::{Rect, Regions};
use lyr_core::tooltip::Tooltip;
use lyr_render::surface::Surface;
use lyr_render::text::Text;
use lyr_render::translate::translate;
use lyr_shell::bars::NAMESPACE;

/// The popup showing a tooltip.
struct Popup {
    window: WindowId,
    /// The module or tray button it is for.
    target: Target<Module>,
    text: String,
    list: DrawList,
    /// The box it was placed against, and its own size.
    anchor: Rect,
    size: Size,
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
    /// The same in a style, for text that may be bold or italic
    /// (notification markup).
    pub measure_in: &'a dyn Fn(&str, TextStyle) -> f32,
    pub line_height: f32,
    /// The calendar's, in its own font.
    pub calendar_line_height: f32,
}

/// One bar's tooltip.
#[derive(Default)]
pub struct Tooltips {
    timer: Tooltip<Target<Module>>,
    popup: Option<Popup>,
}

impl Tooltips {
    /// The pointer is over `target`. A module has a tooltip, and so does a
    /// tray item's button; workspace buttons have none, as in waybar.
    pub fn hover(&mut self, target: Option<&Target<Module>>, now: Instant) {
        let target = target.filter(|target| match target {
            Target::Module(_) | Target::Button(Module::Tray, _) => true,
            Target::Button(..) => false,
        });
        self.timer.hover(target.cloned(), now);
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

    /// Reads one shell event for its surface's frame request.
    pub fn observe(&mut self, event: &ShellEvent) {
        if let Some(surface) = self.popup.as_mut().and_then(|popup| popup.surface.as_mut()) {
            surface.observe(event);
        }
    }

    /// Whether it has a frame to present that it can present now.
    pub fn ready(&self) -> bool {
        self.popup.as_ref().is_some_and(|popup| {
            popup.dirty && popup.surface.as_ref().is_none_or(Surface::frame_due)
        })
    }

    /// Opens, closes or replaces the popup so it shows what should show now:
    /// the hovered target's tooltip (`tooltip_of`) once its delay is up,
    /// anchored to its box on the bar.
    pub fn sync(
        &mut self,
        shell: &mut dyn Shell,
        anchor: &Anchor<'_>,
        look: &Look<'_>,
        tooltip_of: impl Fn(&Target<Module>) -> Option<String>,
        now: Instant,
    ) -> anyhow::Result<()> {
        self.timer.tick(now);
        let wanted = self
            .timer
            .shown()
            .and_then(|target| Some((target.clone(), tooltip_of(target)?)));
        let current = self
            .popup
            .as_ref()
            .map(|popup| (&popup.target, popup.text.as_str()));
        if current
            == wanted
                .as_ref()
                .map(|(target, text)| (target, text.as_str()))
            && !self.moved(shell, anchor.regions)?
        {
            return Ok(());
        }
        self.close(shell)?;
        let Some((target, text)) = wanted else {
            return Ok(());
        };
        let Some(rect) = anchor.regions.rect(&target) else {
            return Ok(());
        };
        let (list, size) = draw::tooltip(&text, look.measure, look.line_height, look.theme);
        let window = shell.create_window(&WindowDesc {
            title: NAMESPACE,
            app_id: NAMESPACE,
            size: logical_size(size),
            role: WindowRole::Popup(PopupDesc {
                parent: anchor.bar,
                anchor: logical_rect(rect),
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
            target,
            text,
            list,
            anchor: rect,
            size,
            surface: None,
            dirty: false,
        });
        Ok(())
    }

    /// Whether the open popup's box has moved and the popup cannot
    /// follow it, so it is to be opened again. It follows by being
    /// repositioned where the window system can.
    fn moved(
        &mut self,
        shell: &mut dyn Shell,
        regions: &Regions<Target<Module>>,
    ) -> anyhow::Result<bool> {
        let Some(popup) = &mut self.popup else {
            return Ok(false);
        };
        let Some(rect) = regions.rect(&popup.target) else {
            return Ok(false);
        };
        if rect == popup.anchor {
            return Ok(false);
        }
        if !shell.caps().contains(ShellCaps::POPUP_REPOSITION) {
            return Ok(true);
        }
        shell.reposition_popup(popup.window, logical_rect(rect), logical_size(popup.size))?;
        popup.anchor = rect;
        Ok(false)
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
    pub fn draw(&mut self, shell: &mut dyn Shell, text: &Text) -> anyhow::Result<()> {
        let Some(popup) = &mut self.popup else {
            return Ok(());
        };
        if let (true, Some(surface)) = (popup.dirty, &mut popup.surface) {
            popup.dirty = !surface.draw(shell, &translate(&popup.list, text))?;
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

/// `rect` in crcbl's logical units.
fn logical_rect(rect: Rect) -> LogicalRect {
    LogicalRect {
        origin: LogicalPoint {
            x: f64::from(rect.x),
            y: f64::from(rect.y),
        },
        size: logical_size(Size {
            width: rect.width,
            height: rect.height,
        }),
    }
}

/// `size` in crcbl's logical units.
fn logical_size(size: Size) -> LogicalSize {
    LogicalSize {
        width: f64::from(size.width),
        height: f64::from(size.height),
    }
}

#[cfg(test)]
mod tests;
