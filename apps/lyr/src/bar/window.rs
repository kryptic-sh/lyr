//! One bar window's state.

use crcbl_shell::MonitorId;
use lyr_config::{Module, Position};
use lyr_core::draw::{DrawList, Target};
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
    /// Its output's height, in logical pixels.
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
        let rect = self.regions.rect(target)?;
        Some(Place {
            left: f64::from(rect.x),
            width: f64::from(self.width),
            room: f64::from(self.output_height - self.height),
            edge,
            monitor: self.output,
        })
    }

    /// The anchor for a popup under `target`, or `None` when the last frame
    /// did not draw it.
    pub(super) fn popup_anchor(&self, target: &Target<Module>, edge: Position) -> Option<Rect> {
        let rect = self.regions.rect(target)?;
        Some(crate::menu_panel::popup_anchor(rect, self.height, edge))
    }
}
