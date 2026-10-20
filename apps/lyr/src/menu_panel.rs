//! What the Bluetooth, Wi-Fi and audio menus do alike: a panel beside the
//! bar, under the module that opened it and kept inside the output
//! ([`Place`], where every menu and popup of the bar's opens), taking the
//! keyboard, drawn from a draw list and its input
//! regions, the arrow keys moving an outline through the regions in the
//! order drawn, and Escape, losing focus or the window system closing it.
//!
//! It is a panel rather than a popup so `lyr panel …` can open it: a
//! Wayland popup's grab needs the serial of the input that opened it. It
//! follows its content's size in place (`Shell::set_panel`).

use crcbl_shell::{
    ButtonState, CloseReply, LogicalSize, Margins, PanelAnchor, PanelDesc, PanelKeyboard,
    PanelLayer, PointerButton, Reserve, Shell, ShellEvent, TextPurpose, WindowDesc, WindowId,
    WindowRole,
};
use lyr_config::Position;
use lyr_core::draw::{DrawList, Item, Size};
use lyr_core::hit::{Rect, Regions};
use lyr_core::pointer::Press;
use lyr_render::surface::Surface;
use lyr_render::text::Text;
use lyr_render::translate::translate;
use lyr_shell::bars::NAMESPACE;

use crate::input::Pointer;
use crate::keys::{self, Key};
use crate::toasts::EDGE_MARGIN;

/// An open menu's panel.
pub struct MenuPanel<T> {
    window: WindowId,
    place: Place,
    /// The window's size: the content's, held to [`Place::max_height`].
    size: Size,
    /// What the menu holds, and how far it is scrolled when it holds more
    /// than the window shows.
    content: Size,
    scroll: f32,
    /// How far a wheel step scrolls it.
    scroll_step: f32,
    list: DrawList,
    regions: Regions<T>,
    /// The keyboard's place, by the target's index in the order drawn.
    focus: Option<usize>,
    pointer: Pointer,
    /// Where the last [`Event::Pick`] was clicked, in logical pixels; `None`
    /// when Enter chose it.
    picked_at: Option<(f32, f32)>,
    /// Which targets a drag picks again as the pointer moves, a slider's.
    drags: fn(&T) -> bool,
    /// The target the left button went down on, while it is held, when it
    /// is one [`Self::drags`] takes.
    dragging: Option<T>,
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

/// Where a menu, popup or panel the bar opens sits: `GAP` past the bar's
/// edge, its left edge on the module that opened it, kept inside the
/// output. Every one of them opens by this rule, so they line up alike.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Place {
    /// The module's left edge, from its output's left edge, in logical
    /// pixels.
    pub left: f64,
    /// The output's width.
    pub width: f64,
    /// The height beside the bar, from its edge to the output's far edge.
    pub room: f64,
    /// The bar's edge of the output, which the menu opens away from.
    pub edge: Position,
}

/// Lines a wheel step scrolls a menu taller than its room, as the clock
/// panel's centre scrolls.
const SCROLL_LINES: f32 = 3.0;

/// How far past the bar's edge a menu opens: the toasts' margin, so they
/// all keep one distance.
pub const GAP: f64 = EDGE_MARGIN;

impl Place {
    /// At the output's left edge, on an output of unknown size: where
    /// `lyr panel` opens a menu no bar shows the module of.
    pub const EDGE: Self = Self {
        left: EDGE_MARGIN,
        width: f64::INFINITY,
        room: f64::INFINITY,
        edge: Position::Top,
    };

    /// The left margin of a panel `size` big: `left`, moved left as far as
    /// it takes to end a margin from the output's right edge, but not past
    /// its left edge.
    fn left(self, size: Size) -> f64 {
        self.left
            .min(self.width - f64::from(size.width) - EDGE_MARGIN)
            .max(0.0)
    }

    /// The tallest a panel here can be and still end a margin short of the
    /// output's far edge.
    pub fn max_height(self) -> f32 {
        (self.room - 2.0 * GAP).max(0.0) as f32
    }
}

/// The anchor for a popup under `module`, on a bar `bar_height` tall at
/// `edge`: the module's box stretched `GAP` past the bar's edge, since a
/// popup opens against its anchor's edge, so it lands where a [`Place`]
/// puts a panel.
pub fn popup_anchor(module: Rect, bar_height: f32, edge: Position) -> Rect {
    let gap = GAP as f32;
    match edge {
        Position::Top => Rect {
            height: bar_height + gap - module.y,
            ..module
        },
        Position::Bottom => Rect {
            y: -gap,
            height: module.y + module.height + gap,
            ..module
        },
    }
}

/// The panel, `size` big at `place`: kept inside the output, so its left
/// margin moves with its width.
pub fn panel_desc(size: Size, place: Place) -> PanelDesc<'static> {
    let (edge, margins) = match place.edge {
        Position::Top => (
            PanelAnchor::TOP,
            Margins {
                top: GAP,
                right: 0.0,
                bottom: 0.0,
                left: place.left(size),
            },
        ),
        Position::Bottom => (
            PanelAnchor::BOTTOM,
            Margins {
                top: 0.0,
                right: 0.0,
                bottom: GAP,
                left: place.left(size),
            },
        ),
    };
    PanelDesc {
        namespace: NAMESPACE,
        monitor: None,
        layer: PanelLayer::Overlay,
        anchor: edge | PanelAnchor::LEFT,
        reserve: Reserve::None,
        margins,
        keyboard: PanelKeyboard::Exclusive,
    }
}

/// The window's size for `content` at `place`: as big, but no taller than
/// the room beside the bar.
fn held(content: Size, place: Place) -> Size {
    Size {
        width: content.width,
        height: content.height.min(place.max_height()),
    }
}

/// `size` as the window system's logical size.
pub fn logical(size: Size) -> LogicalSize {
    LogicalSize {
        width: f64::from(size.width),
        height: f64::from(size.height),
    }
}

/// The panel's window, `size` big, at `place`.
fn create(shell: &mut dyn Shell, size: Size, place: Place) -> anyhow::Result<WindowId> {
    Ok(shell.create_window(&WindowDesc {
        title: NAMESPACE,
        app_id: NAMESPACE,
        size: logical(size),
        transparent: true,
        role: WindowRole::Panel(panel_desc(size, place)),
        ..WindowDesc::default()
    })?)
}

impl<T: Copy> MenuPanel<T> {
    /// Opens a panel showing `list`, `size` big, with its input `regions`,
    /// at `place`; text in it is `line_height` a line.
    pub fn open(
        shell: &mut dyn Shell,
        place: Place,
        line_height: f32,
        (list, content, regions): (DrawList, Size, Regions<T>),
    ) -> anyhow::Result<Self> {
        let size = held(content, place);
        Ok(Self {
            window: create(shell, size, place)?,
            place,
            size,
            content,
            scroll: 0.0,
            scroll_step: SCROLL_LINES * line_height,
            list,
            regions,
            focus: None,
            picked_at: None,
            drags: |_| false,
            dragging: None,
            pointer: Pointer::default(),
            scale: 1.0,
            surface: None,
            dirty: false,
        })
    }

    /// Has a drag that starts on a target `drags` takes pick it again at
    /// every move until the button is let go.
    #[must_use]
    pub fn with_drags(mut self, drags: fn(&T) -> bool) -> Self {
        self.drags = drags;
        self
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

    /// Where the last pick was clicked across `target`, 0 – 1 of its width;
    /// `None` when Enter chose it or it is not drawn.
    pub fn picked_across(&self, target: &T) -> Option<f32>
    where
        T: PartialEq,
    {
        let (x, _) = self.picked_at?;
        Some(self.regions.rect(target)?.across(x))
    }

    /// Turns the window system's text input on for `purpose`, or off.
    pub fn set_typing(
        &self,
        shell: &mut dyn Shell,
        purpose: Option<TextPurpose>,
    ) -> anyhow::Result<()> {
        Ok(shell.set_text_input(self.window, purpose)?)
    }

    /// Takes the keyboard's outline away, so Enter is the menu's own.
    pub const fn unfocus(&mut self) {
        self.focus = None;
    }

    /// The target the keyboard is on.
    pub fn focused(&self) -> Option<T> {
        self.regions.targets().nth(self.focus?).copied()
    }

    /// Shows `list`, `content` big, with its input `regions`, keeping the
    /// keyboard's place within them, and resizing the window in place if its
    /// size changed; the new size comes back as its next `Resized`.
    pub fn show(
        &mut self,
        shell: &mut dyn Shell,
        (list, content, regions): (DrawList, Size, Regions<T>),
    ) -> anyhow::Result<()> {
        let count = regions.targets().count();
        self.focus = self.focus.filter(|_| count > 0).map(|at| at.min(count - 1));
        let size = held(content, self.place);
        if size != self.size {
            shell.set_panel(self.window, logical(size), &panel_desc(size, self.place))?;
            self.size = size;
        }
        self.content = content;
        self.list = list;
        self.regions = regions;
        self.scroll_to(self.scroll);
        self.dirty = true;
        Ok(())
    }

    /// How far it can scroll: what it holds beyond what the window shows.
    fn scroll_max(&self) -> f32 {
        (self.content.height - self.size.height).max(0.0)
    }

    /// Scrolls to `scroll`, held to what there is.
    fn scroll_to(&mut self, scroll: f32) {
        self.scroll = scroll.clamp(0.0, self.scroll_max());
    }

    /// Scrolls the keyboard's place into view.
    fn reveal_focus(&mut self) {
        let Some(rect) = self.focus.and_then(|at| self.regions.rect_at(at)) else {
            return;
        };
        if rect.y < self.scroll {
            self.scroll_to(rect.y);
        } else if rect.y + rect.height > self.scroll + self.size.height {
            self.scroll_to(rect.y + rect.height - self.size.height);
        }
    }

    /// The target under the pointer, through the scroll.
    fn under_pointer(&self) -> Option<&T> {
        let (x, y) = self.pointer.position()?;
        self.regions.hit(x, y + self.scroll)
    }

    /// What the window shows: the list, or when it holds more than fits,
    /// its frame at the window's size and the rest scrolled inside it.
    fn visible(&self) -> DrawList {
        if self.scroll_max() <= 0.0 {
            return self.list.clone();
        }
        let mut items = self.list.items.clone();
        let whole = |rect: &lyr_core::hit::Rect| {
            rect.x == 0.0
                && rect.y == 0.0
                && rect.width == self.content.width
                && rect.height == self.content.height
        };
        // `lyr_core::menu::frame` puts the box first; anything else is
        // clipped and scrolled whole.
        let frame = match items.first() {
            Some(Item::Box { rect, .. }) if whole(rect) => Some(items.remove(0)),
            _ => None,
        };
        let view = lyr_core::hit::Rect {
            x: 0.0,
            y: 0.0,
            width: self.size.width,
            height: self.size.height,
        };
        let (mut shown, inset) = match frame {
            Some(Item::Box {
                radius,
                fill,
                border,
                ..
            }) => (
                vec![Item::Box {
                    rect: view,
                    radius,
                    fill,
                    border,
                }],
                border.map_or(0.0, |(width, _)| width),
            ),
            _ => (Vec::new(), 0.0),
        };
        shown.push(Item::Clip {
            rect: lyr_core::hit::Rect {
                x: inset,
                y: inset,
                width: (view.width - 2.0 * inset).max(0.0),
                height: (view.height - 2.0 * inset).max(0.0),
            },
            items: items
                .into_iter()
                .map(|item| item.offset(0.0, -self.scroll))
                .collect(),
        });
        DrawList { items: shown }
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
            ShellEvent::Focus { focused: false, .. } => return Ok(Event::Close),
            ShellEvent::Key { .. } if keys::pressed(event) == Some(Key::Escape) => {
                return Ok(Event::Close);
            }
            ShellEvent::Key { .. } => {
                let count = self.regions.targets().count();
                if count == 0 {
                    return Ok(Event::Kept);
                }
                match keys::pressed(event) {
                    Some(Key::Down) => {
                        self.focus = Some(self.focus.map_or(0, |at| (at + 1).min(count - 1)));
                        self.reveal_focus();
                        return Ok(Event::Redraw);
                    }
                    Some(Key::Up) => {
                        self.focus = Some(self.focus.map_or(0, |at| at.saturating_sub(1)));
                        self.reveal_focus();
                        return Ok(Event::Redraw);
                    }
                    Some(Key::Enter) => {
                        self.picked_at = None;
                        return Ok(self.focused().map_or(Event::Enter, Event::Pick));
                    }
                    _ => {}
                }
            }
            ShellEvent::Button {
                button: PointerButton::Left,
                state: ButtonState::Released,
                ..
            } => self.dragging = None,
            _ => {
                let presses = self.pointer.handle(event, self.scale);
                let scrolled = presses
                    .iter()
                    .fold(self.scroll, |scroll, press| match press {
                        Press::ScrollUp => scroll - self.scroll_step,
                        Press::ScrollDown => scroll + self.scroll_step,
                        _ => scroll,
                    });
                if scrolled != self.scroll {
                    let before = self.scroll;
                    self.scroll_to(scrolled);
                    if self.scroll != before {
                        self.dirty = true;
                    }
                }
                if presses.contains(&Press::Left)
                    && let Some(&target) = self.under_pointer()
                {
                    self.picked_at = self.pointer.position();
                    self.dragging = (self.drags)(&target).then_some(target);
                    return Ok(Event::Pick(target));
                }
                if let (ShellEvent::PointerMotion { .. }, Some(target)) = (event, self.dragging) {
                    self.picked_at = self.pointer.position();
                    return Ok(Event::Pick(target));
                }
            }
        }
        Ok(Event::Kept)
    }

    /// Draws the panel if it has a frame to present.
    pub fn draw(&mut self, shell: &mut dyn Shell, text: &Text) -> anyhow::Result<()> {
        if self.dirty {
            let visible = self.visible();
            if let Some(surface) = &mut self.surface {
                self.dirty = !surface.draw(shell, &translate(&visible, text, surface.scale()))?;
            }
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

#[cfg(test)]
mod tests {
    use crcbl_shell::{
        ButtonState, HeadlessShell, KeyCode, PanelAnchor, PhysicalPoint, PointerButton,
        ScrollDelta, Shell, ShellEvent,
    };
    use lyr_config::{Platform, Position, from_sources};
    use lyr_core::draw::{Item, Size};
    use lyr_core::hit::{Rect, Regions};
    use lyr_core::menu;

    use super::{Event, GAP, MenuPanel, Place, held, panel_desc, popup_anchor};
    use crate::toasts::EDGE_MARGIN;

    #[test]
    fn a_panel_stays_inside_its_output() {
        let size = Size {
            width: 600.0,
            height: 200.0,
        };
        let place = |left| Place {
            left,
            width: 1920.0,
            ..Place::EDGE
        };
        assert!(
            (place(100.0).left(size) - 100.0).abs() < f64::EPSILON,
            "room"
        );
        assert!(
            (place(1500.0).left(size) - (1920.0 - 600.0 - EDGE_MARGIN)).abs() < f64::EPSILON,
            "moved left to end a margin from the right edge"
        );
        let narrow = Place {
            left: 100.0,
            width: 400.0,
            ..Place::EDGE
        };
        assert!(
            narrow.left(size).abs() < f64::EPSILON,
            "wider than the output"
        );
        assert!((Place::EDGE.left(size) - EDGE_MARGIN).abs() < f64::EPSILON);
    }

    #[test]
    fn panels_open_a_gap_past_the_bar_s_edge() {
        let size = Size {
            width: 300.0,
            height: 200.0,
        };
        let top = panel_desc(size, Place::EDGE);
        assert_eq!(top.anchor, PanelAnchor::TOP | PanelAnchor::LEFT);
        assert!((top.margins.top - GAP).abs() < f64::EPSILON);
        let bottom = panel_desc(
            size,
            Place {
                edge: Position::Bottom,
                ..Place::EDGE
            },
        );
        assert_eq!(bottom.anchor, PanelAnchor::BOTTOM | PanelAnchor::LEFT);
        assert!((bottom.margins.bottom - GAP).abs() < f64::EPSILON);
        assert!(bottom.margins.top.abs() < f64::EPSILON);
        let room = Place {
            room: 1000.0,
            ..Place::EDGE
        };
        assert!((room.max_height() - (1000.0 - 2.0 * GAP as f32)).abs() < f32::EPSILON);
    }

    /// Ten 60 px rows in a 600 px menu, with room for 200 px of it.
    fn tall() -> (MenuPanel<usize>, HeadlessShell) {
        let theme = from_sources(None, Platform::Linux)
            .expect("the defaults")
            .theme;
        let content = Size {
            width: 100.0,
            height: 600.0,
        };
        let mut regions = Regions::default();
        for row in 0..10_u8 {
            let rect = Rect {
                x: 0.0,
                y: f32::from(row) * 60.0,
                width: 100.0,
                height: 60.0,
            };
            regions.push(rect, usize::from(row));
        }
        let place = Place {
            room: 200.0 + 2.0 * GAP,
            ..Place::EDGE
        };
        let mut shell = HeadlessShell::new();
        let panel = MenuPanel::open(
            &mut shell,
            place,
            20.0,
            (
                menu::frame(content, Vec::new(), None, &theme),
                content,
                regions,
            ),
        )
        .expect("opened");
        (panel, shell)
    }

    /// Feeds the panel its queued events, sizing ones left out (they open
    /// a GPU surface).
    fn pump(panel: &mut MenuPanel<usize>, shell: &mut HeadlessShell) -> Vec<Event<usize>> {
        let mut events = Vec::new();
        shell.pump(&mut |event| events.push(event));
        events
            .iter()
            .filter(|event| !matches!(event, ShellEvent::Resized { .. }))
            .map(|event| panel.event(shell, event).expect("applied"))
            .collect()
    }

    #[test]
    fn a_menu_taller_than_its_room_scrolls() {
        let (mut panel, mut shell) = tall();
        let window = panel.window;
        assert!(
            (panel.size().height - 200.0).abs() < f32::EPSILON,
            "held to the room"
        );

        // The fifth row, 240 – 300, comes into view at the bottom.
        for _ in 0..5 {
            shell.key_press(window, KeyCode::ArrowDown).expect("a key");
        }
        pump(&mut panel, &mut shell);
        assert_eq!(panel.focused(), Some(4));
        assert!(
            (panel.scroll - 100.0).abs() < f32::EPSILON,
            "{}",
            panel.scroll
        );

        // Drawn as the frame at the window's size, the rows clipped in it.
        let visible = panel.visible();
        assert!(matches!(
            visible.items.first(),
            Some(Item::Box { rect, .. }) if (rect.height - 200.0).abs() < f32::EPSILON
        ));
        assert!(matches!(visible.items.get(1), Some(Item::Clip { .. })));

        // A click 10 px down hits the row scrolled under it, the second.
        let at = PhysicalPoint { x: 50.0, y: 10.0 };
        shell.move_pointer(window, at, (0.0, 0.0)).expect("moved");
        shell
            .button(window, PointerButton::Left, ButtonState::Pressed, Some(at))
            .expect("clicked");
        assert!(pump(&mut panel, &mut shell).contains(&Event::Pick(1)));

        // The wheel goes no further than the last row.
        for _ in 0..100 {
            shell
                .scroll(window, ScrollDelta::Lines { x: 0.0, y: -1.0 }, Some(at))
                .expect("scrolled");
        }
        pump(&mut panel, &mut shell);
        assert!(
            (panel.scroll - 400.0).abs() < f32::EPSILON,
            "{}",
            panel.scroll
        );
    }

    #[test]
    fn a_menu_that_fits_draws_as_it_is() {
        let (mut panel, _shell) = tall();
        panel.place.room = 1000.0;
        panel.size = held(panel.content, panel.place);
        assert_eq!(panel.visible(), panel.list);
    }

    #[test]
    fn a_popup_lands_where_a_panel_does() {
        // A 37 px module box 7 px down a 50 px bar.
        let module = Rect {
            x: 120.0,
            y: 7.0,
            width: 40.0,
            height: 37.0,
        };
        let gap = GAP as f32;
        let below = popup_anchor(module, 50.0, Position::Top);
        assert!((below.x - 120.0).abs() < f32::EPSILON, "left edges aligned");
        assert!((below.y + below.height - (50.0 + gap)).abs() < f32::EPSILON);
        let above = popup_anchor(module, 50.0, Position::Bottom);
        assert!((above.y + gap).abs() < f32::EPSILON, "a gap above the bar");
        assert!((above.y + above.height - (module.y + module.height)).abs() < f32::EPSILON);
    }
}
