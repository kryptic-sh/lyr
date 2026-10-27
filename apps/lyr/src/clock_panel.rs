//! The clock panel (`docs/plan/10-popups-and-menus.md#clock-panel`): a panel
//! under the module that opened it, where every menu of the bar's opens
//! ([`Place`]), as big as what it shows: the calendar's view, the year or a
//! month ([`lyr_core::calendar`]), and while lyr is the notification daemon
//! the notification centre below it ([`lyr_core::centre`]), held short of
//! the output's far edge, past which the centre scrolls.
//!
//! It takes the keyboard. `Escape`, losing focus and the same click or
//! binding again close it. Over the calendar a right click switches between
//! year and month, and scrolling moves by one of either; a click in the
//! centre is the caller's to carry out.

use chrono::{NaiveDate, Weekday};
use crcbl_shell::{CloseReply, Shell, ShellEvent, WindowDesc, WindowId, WindowRole};
use lyr_config::Theme;
use std::time::Instant;

use lyr_core::calendar::{self, Mode};
use lyr_core::centre::{self, Target};
use lyr_core::draw::{DrawList, Item, Size};
use lyr_core::hit::Rect;
use lyr_core::hit::Regions;
use lyr_core::media::Player;
use lyr_core::notifications::Store;
use lyr_core::pointer::Press;
use lyr_render::surface::Surface;
use lyr_render::text::Text;
use lyr_shell::bars::NAMESPACE;

use crate::input::Pointer;
use crate::keys::{self, Key};
use crate::menu_panel::{Place, logical, panel_desc, pressed_outside};
use crate::tooltips::Look;

/// swaync's `control-center-width`, the narrowest the notification centre
/// is laid out; the calendar alone takes only its own width.
const CENTRE_WIDTH: f32 = 500.0;
/// swaync's `.control-center` padding.
const PADDING: f32 = 16.0;
/// swaync's `.control-center` border.
const BORDER: f32 = 2.0;
/// swaync's corner radius.
const RADIUS: f32 = 8.0;
/// The calendar's first weekday where the locale's cannot be read: ISO's.
const FIRST_WEEKDAY: Weekday = Weekday::Mon;
/// Lines the centre scrolls per wheel step.
const SCROLL_LINES: f32 = 3.0;

/// The open clock panel.
pub struct ClockPanel {
    window: WindowId,
    place: Place,
    /// The locale's first weekday, read when the panel opens.
    first: Weekday,
    /// The month the calendar shows, and how.
    shown: NaiveDate,
    mode: Mode,
    /// Logical pixels: what it shows needs, held to the output.
    size: Size,
    /// The size last asked of the window system, and one still to ask.
    requested: Size,
    resize_to: Option<Size>,
    scale: f32,
    list: DrawList,
    /// The calendar's height, below which the centre starts.
    calendar_bottom: f32,
    /// The centre's clicks, in its own coordinates, and where it sits.
    centre: Regions<Target>,
    centre_origin: (f32, f32),
    /// The part of the panel the centre shows in, below the calendar.
    centre_view: Rect,
    /// How far the centre is scrolled up, and how far it can go.
    scroll: f32,
    scroll_max: f32,
    /// The history entry under the pointer, which shows its close button.
    hovered: Option<u32>,
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
    /// A click in the notification centre, for the caller to carry out.
    Centre(Target, Press),
}

/// What the panel shows at a moment: today, and the notification store when
/// lyr is the daemon.
pub struct Contents<'a> {
    pub today: NaiveDate,
    pub store: Option<&'a Store>,
    /// The media players, drawn in the centre.
    pub players: &'a [Player],
    pub now: Instant,
}

impl ClockPanel {
    /// Opens the panel at `place` showing `today`'s year, as big as the
    /// year's calendar; the centre, if any, is sized in on its first draw.
    pub fn open(
        shell: &mut dyn Shell,
        place: Place,
        look: &Look<'_>,
        today: NaiveDate,
    ) -> anyhow::Result<Self> {
        let first = lyr_sources::locale::first_weekday()
            .and_then(|day| Weekday::try_from(day).ok())
            .unwrap_or(FIRST_WEEKDAY);
        let (_, calendar) = calendar::draw::draw(
            today,
            Mode::Year,
            first,
            today,
            look.measure_in,
            look.calendar_line_height,
            look.theme,
        );
        let inset = PADDING + BORDER;
        let size = Size {
            width: calendar.width + 2.0 * inset,
            height: (calendar.height + 2.0 * inset).min(place.max_height()),
        };
        let window = shell.create_window(&WindowDesc {
            title: NAMESPACE,
            app_id: NAMESPACE,
            size: logical(size),
            transparent: true,
            role: WindowRole::Panel(panel_desc(size, place)),
            ..WindowDesc::default()
        })?;
        Ok(Self {
            window,
            place,
            first,
            shown: today,
            mode: Mode::Year,
            size,
            requested: size,
            resize_to: None,
            scale: 1.0,
            list: DrawList::default(),
            calendar_bottom: 0.0,
            centre: Regions::default(),
            centre_origin: (0.0, 0.0),
            centre_view: Rect {
                x: 0.0,
                y: 0.0,
                width: 0.0,
                height: 0.0,
            },
            scroll: 0.0,
            scroll_max: 0.0,
            hovered: None,
            pointer: Pointer::default(),
            surface: None,
            dirty: false,
        })
    }

    /// Whether `window` is the panel.
    pub fn owns(&self, window: WindowId) -> bool {
        self.window == window
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

    /// Applies an event for the panel's window.
    pub fn event(
        &mut self,
        shell: &mut dyn Shell,
        event: &ShellEvent,
        look: &Look<'_>,
        contents: &Contents<'_>,
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
                self.rebuild(look, contents);
            }
            ShellEvent::CloseRequested { window } => {
                shell.reply_close_request(window, CloseReply::Close)?;
                return Ok(Outcome::Closed);
            }
            ShellEvent::Focus { focused: false, .. } => return Ok(Outcome::Close),
            ShellEvent::Button { .. } if pressed_outside(shell, self.window, event) => {
                return Ok(Outcome::Close);
            }
            ShellEvent::Key { .. } if keys::pressed(event) == Some(Key::Escape) => {
                return Ok(Outcome::Close);
            }
            _ => {
                for press in self.pointer.handle(event, self.scale) {
                    let Some((_, y)) = self.pointer.position() else {
                        continue;
                    };
                    if y >= self.calendar_bottom {
                        let step = SCROLL_LINES * look.line_height;
                        let scrolled = match press {
                            Press::ScrollUp => Some(self.scroll - step),
                            Press::ScrollDown => Some(self.scroll + step),
                            _ => None,
                        };
                        if let Some(scroll) = scrolled {
                            self.scroll = scroll;
                            self.rebuild(look, contents);
                            continue;
                        }
                        if let Some(target) = self.centre_target() {
                            return Ok(Outcome::Centre(target, press));
                        }
                        continue;
                    }
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
                        self.rebuild(look, contents);
                    }
                }
                let hovered = self.centre_target().and_then(Target::entry);
                if hovered != self.hovered {
                    self.hovered = hovered;
                    self.rebuild(look, contents);
                }
            }
        }
        Ok(Outcome::Kept)
    }

    /// What in the centre is under the pointer.
    fn centre_target(&self) -> Option<Target> {
        let (x, y) = self.pointer.position()?;
        let (ox, oy) = self.centre_origin;
        if !self.centre_view.contains(x, y) {
            return None;
        }
        self.centre.hit(x - ox, y - oy).copied()
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

    /// Draws the panel again for what it shows now: after the notifications
    /// or do-not-disturb changed.
    pub fn refresh(&mut self, look: &Look<'_>, contents: &Contents<'_>) {
        self.rebuild(look, contents);
    }

    /// Lays the panel out again for its mode and month and the centre
    /// below the calendar, at the size they need, and asks the window
    /// system for that size when it changed.
    fn rebuild(&mut self, look: &Look<'_>, contents: &Contents<'_>) {
        let (calendar, calendar_size) = calendar::draw::draw(
            self.shown,
            self.mode,
            self.first,
            contents.today,
            look.measure_in,
            look.calendar_line_height,
            look.theme,
        );
        let inset = BORDER + PADDING;
        self.calendar_bottom = inset + calendar_size.height;
        let mut width = calendar_size.width + 2.0 * inset;
        let mut height = self.calendar_bottom + inset;
        let centre = contents.store.map(|store| {
            width = width.max(CENTRE_WIDTH);
            let top = self.calendar_bottom + centre::GAP;
            let shown = centre::Shown {
                store,
                players: contents.players,
                now: contents.now,
                hovered: self.hovered,
            };
            let drawn = centre::draw(
                &shown,
                width - 2.0 * inset,
                look.measure_in,
                look.lay_out,
                look.line_height,
                look.theme,
            );
            height = top + drawn.1.height + inset;
            (top, drawn)
        });
        self.size = Size {
            width,
            height: height.min(self.place.max_height()),
        };
        if self.size != self.requested {
            self.requested = self.size;
            self.resize_to = Some(self.size);
        }
        self.list = frame(self.size, look.theme);
        self.list.items.extend(
            calendar
                .items
                .into_iter()
                .map(|item| item.offset(inset, inset)),
        );
        self.centre = Regions::default();
        if let Some((top, (list, centre_size, regions))) = centre {
            self.centre_view = Rect {
                x: inset,
                y: top,
                width: self.size.width - 2.0 * inset,
                height: (self.size.height - inset - top).max(0.0),
            };
            self.scroll_max = (centre_size.height - self.centre_view.height).max(0.0);
            self.scroll = self.scroll.clamp(0.0, self.scroll_max);
            let origin = (inset, top - self.scroll);
            self.list.items.push(Item::Clip {
                rect: self.centre_view,
                items: list
                    .items
                    .into_iter()
                    .map(|item| item.offset(origin.0, origin.1))
                    .collect(),
            });
            self.centre = regions;
            self.centre_origin = origin;
        }
        self.dirty = true;
    }

    /// Asks the window system for the size the panel needs, if that
    /// changed; the new size comes back as its next `Resized`.
    fn ask_size(&mut self, shell: &mut dyn Shell) -> anyhow::Result<()> {
        if let Some(size) = self.resize_to.take() {
            shell.set_panel(self.window, logical(size), &panel_desc(size, self.place))?;
        }
        Ok(())
    }

    /// Draws the panel if it has a frame to present, at the size it asked
    /// for.
    pub fn draw(&mut self, shell: &mut dyn Shell, text: &Text) -> anyhow::Result<()> {
        self.ask_size(shell)?;
        if let (true, Some(surface)) = (self.dirty, &mut self.surface) {
            self.dirty = !surface.draw_bar(shell, &self.list, text)?;
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
