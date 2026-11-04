//! One bar window's state.

use crcbl_shell::MonitorId;
use lyr_config::{Bar, Module, Position};
use lyr_core::draw::{DrawList, Size, Target};
use lyr_core::hit::{Rect, Regions};
use lyr_render::surface::Surface;

use crate::input::Pointer;
use crate::menu_panel::Place;
use crate::tooltips::Tooltips;

/// One bar window: its surface, its size, and what it drew last.
pub(super) struct Window {
    pub(super) surface: Surface,
    /// Logical pixels.
    pub(super) width: f32,
    pub(super) height: f32,
    /// Its output's size, in logical pixels.
    pub(super) output_width: f32,
    pub(super) output_height: f32,
    /// Device pixels per logical pixel.
    pub(super) scale: f32,
    /// The output it is on, by name, for a per-output workspace list.
    pub(super) monitor: Option<String>,
    /// The output it is on, for a menu opened from it.
    pub(super) output: Option<MonitorId>,
    pub(super) pointer: Pointer,
    /// The input regions of the last frame.
    pub(super) regions: Regions<Target<Module>>,
    /// The workspaces the last frame's buttons stand for.
    pub(super) workspaces: Vec<i64>,
    /// The workspace button the pointer was over when it was last drawn.
    pub(super) hovered: Option<usize>,
    /// Whether it must be drawn again: what it shows changed since.
    pub(super) dirty: bool,
    /// What is on screen, so a redraw that comes out the same presents
    /// nothing; `None` when the surface needs a frame whatever it shows.
    pub(super) shown: Option<DrawList>,
    pub(super) tooltips: Tooltips,
}

impl Window {
    /// Whether it has a frame to present that it can present now; one whose
    /// last frame's answer has not come waits for it rather than for nothing.
    pub(super) const fn ready(&self) -> bool {
        self.dirty && self.surface.frame_due()
    }
}

impl Window {
    /// The workspace button the pointer is over now, by its index in the row.
    pub(super) fn hovered(&self) -> Option<usize> {
        match self.pointer.target(&self.regions) {
            Some(Target::Button(Module::Workspaces, index)) => Some(*index),
            _ => None,
        }
    }

    /// Where a menu opens under `target` on this bar at `edge`, or `None`
    /// when the last frame did not draw it.
    pub(super) fn place_under(&self, target: &Target<Module>, edge: Position) -> Option<Place> {
        let size = |width, height| Size { width, height };
        Some(Place::beside(
            self.regions.rect(target)?,
            size(self.width, self.height),
            size(self.output_width, self.output_height),
            edge,
            self.output,
        ))
    }

    /// Its size across its edge, `edge`: its height, or a side bar's width.
    pub(super) fn thickness(&self, edge: Position) -> f32 {
        if edge.is_vertical() {
            self.width
        } else {
            self.height
        }
    }

    /// The anchor for a popup under `target`, or `None` when the last frame
    /// did not draw it.
    pub(super) fn popup_anchor(&self, target: &Target<Module>, edge: Position) -> Option<Rect> {
        let rect = self.regions.rect(target)?;
        Some(crate::menu_panel::popup_anchor(
            rect,
            self.thickness(edge),
            edge,
        ))
    }

    /// The size it is to get when the bar moves from `from` to `bar`'s edge,
    /// as [`size_on`] gives it for this window on its output.
    pub(super) fn size_on(&self, from: Position, bar: &Bar) -> Size {
        size_on(
            Size {
                width: self.width,
                height: self.height,
            },
            Size {
                width: self.output_width,
                height: self.output_height,
            },
            from,
            bar,
        )
    }
}

/// The size a bar window `window` big on an output `output` big is to get
/// when the bar moves from `from` to `bar`'s edge: `bar`'s thickness across
/// it, and along it its own length on the same axis, or its output's length
/// on the other. The window system may give it less (a work area's), but a
/// size other than its current one only waits for the size it does get,
/// which a move between axes always changes.
fn size_on(window: Size, output: Size, from: Position, bar: &Bar) -> Size {
    let thickness = bar.thickness() as f32;
    match (from.is_vertical(), bar.position.is_vertical()) {
        (false, false) => Size {
            width: window.width,
            height: thickness,
        },
        (true, false) => Size {
            width: output.width,
            height: thickness,
        },
        (true, true) => Size {
            width: thickness,
            height: window.height,
        },
        (false, true) => Size {
            width: thickness,
            height: output.height,
        },
    }
}

#[cfg(test)]
mod tests {
    use lyr_config::{Platform, Position, from_sources};
    use lyr_core::draw::Size;

    use super::size_on;

    #[test]
    fn a_moved_bar_keeps_its_length_on_its_axis_and_takes_the_outputs_across() {
        let mut bar = from_sources(None, Platform::current())
            .expect("the defaults")
            .bar;
        bar.height = 40;
        bar.width = 60;
        let size = |width, height| Size { width, height };
        // A top bar 1800 long in a 1920 by 1080 output, and a side bar 1000
        // long in it.
        let top = size(1800.0, 50.0);
        let side = size(50.0, 1000.0);
        let output = size(1920.0, 1080.0);
        for (window, from, to, expected) in [
            (top, Position::Top, Position::Bottom, size(1800.0, 40.0)),
            (side, Position::Left, Position::Top, size(1920.0, 40.0)),
            (side, Position::Left, Position::Right, size(60.0, 1000.0)),
            (top, Position::Top, Position::Left, size(60.0, 1080.0)),
        ] {
            bar.position = to;
            assert_eq!(
                size_on(window, output, from, &bar),
                expected,
                "{from:?} to {to:?}"
            );
        }
    }
}
