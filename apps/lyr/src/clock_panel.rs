//! The clock panel (`docs/plan/10-popups-and-menus.md#clock-panel`): a panel
//! at the top right of the output the user is on, as swaync's control centre
//! sits, running the output's height. It holds the calendar
//! ([`lyr_core::calendar`]) so far; the notification centre below it is not
//! built.
//!
//! It takes the keyboard. `Escape`, losing focus and the same click or
//! binding again close it; a right click switches the calendar between year
//! and month, and scrolling moves it by one of either.

use chrono::{NaiveDate, Weekday};
use crcbl_shell::{
    ButtonState, CloseReply, KeyCode, LogicalSize, Margins, PanelAnchor, PanelDesc, PanelKeyboard,
    PanelLayer, Reserve, Shell, ShellEvent, WindowDesc, WindowId, WindowRole,
};
use lyr_config::Theme;
use lyr_core::calendar::{self, Mode};
use lyr_core::draw::{DrawList, Item, Size};
use lyr_core::hit::Rect;
use lyr_core::pointer::Press;
use lyr_render::surface::Surface;
use lyr_render::text::Text;
use lyr_render::translate::translate;
use lyr_shell::bars::NAMESPACE;

use crate::input::Pointer;
use crate::toasts::EDGE_MARGIN;
use crate::tooltips::Look;

/// swaync's `control-center-width`.
const WIDTH: f32 = 500.0;
/// swaync's `.control-center` padding.
const PADDING: f32 = 16.0;
/// swaync's `.control-center` border.
const BORDER: f32 = 2.0;
/// swaync's corner radius.
const RADIUS: f32 = 8.0;
/// Until the locale is read (plan 16, 19).
const FIRST_WEEKDAY: Weekday = Weekday::Mon;

/// The open clock panel.
pub struct ClockPanel {
    window: WindowId,
    /// The month the calendar shows, and how.
    shown: NaiveDate,
    mode: Mode,
    /// Logical pixels, from its last resize.
    size: Size,
    scale: f32,
    list: DrawList,
    pointer: Pointer,
    surface: Option<Surface>,
    dirty: bool,
}

/// What an event did to the panel.
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    Kept,
    /// It is to close.
    Close,
    /// The window system is closing its window.
    Closed,
}

impl ClockPanel {
    /// Opens the panel showing `today`'s year, wide enough for the year's
    /// calendar.
    pub fn open(shell: &mut dyn Shell, look: &Look<'_>, today: NaiveDate) -> anyhow::Result<Self> {
        let (_, calendar) = calendar::draw::draw(
            today,
            Mode::Year,
            FIRST_WEEKDAY,
            today,
            look.measure,
            look.line_height,
            look.theme,
        );
        let width = WIDTH.max(calendar.width + 2.0 * (PADDING + BORDER));
        let window = shell.create_window(&WindowDesc {
            title: NAMESPACE,
            app_id: NAMESPACE,
            // Its height is the output's: it is held against the top and the
            // bottom.
            size: LogicalSize {
                width: f64::from(width),
                height: 0.0,
            },
            transparent: true,
            role: WindowRole::Panel(PanelDesc {
                namespace: NAMESPACE,
                monitor: None,
                layer: PanelLayer::Overlay,
                anchor: PanelAnchor::TOP | PanelAnchor::BOTTOM | PanelAnchor::RIGHT,
                reserve: Reserve::None,
                margins: Margins {
                    top: EDGE_MARGIN,
                    right: EDGE_MARGIN,
                    bottom: EDGE_MARGIN,
                    left: 0.0,
                },
                keyboard: PanelKeyboard::Exclusive,
            }),
            ..WindowDesc::default()
        })?;
        Ok(Self {
            window,
            shown: today,
            mode: Mode::Year,
            size: Size { width, height: 0.0 },
            scale: 1.0,
            list: DrawList::default(),
            pointer: Pointer::default(),
            surface: None,
            dirty: false,
        })
    }

    /// Whether `window` is the panel.
    pub fn owns(&self, window: WindowId) -> bool {
        self.window == window
    }

    /// Whether it has a frame to present.
    pub fn dirty(&self) -> bool {
        self.dirty
    }

    /// Applies an event for the panel's window.
    pub fn event(
        &mut self,
        shell: &mut dyn Shell,
        event: &ShellEvent,
        look: &Look<'_>,
        today: NaiveDate,
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
                self.size = Size {
                    width: size.width as f32 / self.scale,
                    height: size.height as f32 / self.scale,
                };
                match &mut self.surface {
                    Some(surface) => surface.resize(device, self.scale)?,
                    None => {
                        self.surface = Some(Surface::open(shell, self.window, device, self.scale)?);
                    }
                }
                self.rebuild(look, today);
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
            _ => {
                for press in self.pointer.handle(event, self.scale) {
                    let moved = match press {
                        Press::Right => {
                            self.mode = match self.mode {
                                Mode::Year => Mode::Month,
                                Mode::Month => Mode::Year,
                            };
                            true
                        }
                        Press::ScrollUp => self.shift(1),
                        Press::ScrollDown => self.shift(-1),
                        Press::Left | Press::Middle => false,
                    };
                    if moved {
                        self.rebuild(look, today);
                    }
                }
            }
        }
        Ok(Outcome::Kept)
    }

    /// Moves the calendar by `steps` of its mode's unit. Returns whether it
    /// moved.
    fn shift(&mut self, steps: i32) -> bool {
        match calendar::shift(self.shown, self.mode, steps) {
            Some(shown) => {
                self.shown = shown;
                true
            }
            None => false,
        }
    }

    /// Draws the panel's frame again for its size, mode and month.
    fn rebuild(&mut self, look: &Look<'_>, today: NaiveDate) {
        self.list = frame(self.size, look.theme);
        let (calendar, _) = calendar::draw::draw(
            self.shown,
            self.mode,
            FIRST_WEEKDAY,
            today,
            look.measure,
            look.line_height,
            look.theme,
        );
        let inset = BORDER + PADDING;
        self.list.items.extend(
            calendar
                .items
                .into_iter()
                .map(|item| item.offset(inset, inset)),
        );
        self.dirty = true;
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

/// The panel's background and border at `size`.
fn frame(size: Size, theme: &Theme) -> DrawList {
    DrawList {
        items: vec![Item::Box {
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: size.width,
                height: size.height,
            },
            radius: RADIUS,
            fill: theme.panel_background,
            border: Some((BORDER, theme.panel_border)),
        }],
    }
}

#[cfg(test)]
mod tests;
