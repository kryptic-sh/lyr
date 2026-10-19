//! One bar window's state.

use lyr_config::Module;
use lyr_core::draw::Target;
use lyr_core::hit::Regions;
use lyr_render::surface::Surface;

use crate::input::Pointer;
use crate::tooltips::Tooltips;

/// One bar window: its surface, its size, and what it drew last.
pub(super) struct Window {
    pub(super) surface: Surface,
    /// Logical pixels.
    pub(super) width: f32,
    /// Device pixels per logical pixel.
    pub(super) scale: f32,
    /// The output it is on, by name, for a per-output workspace list.
    pub(super) monitor: Option<String>,
    pub(super) pointer: Pointer,
    /// The input regions of the last frame.
    pub(super) regions: Regions<Target<Module>>,
    /// The workspaces the last frame's buttons stand for.
    pub(super) workspaces: Vec<i64>,
    /// The workspace button the pointer was over when it was last drawn.
    pub(super) hovered: Option<usize>,
    /// Whether it must be drawn again: what it shows changed since.
    pub(super) dirty: bool,
    pub(super) tooltips: Tooltips,
}

impl Window {
    /// The workspace button the pointer is over now, by its index in the row.
    pub(super) fn hovered(&self) -> Option<usize> {
        match self.pointer.target(&self.regions) {
            Some(Target::Button(Module::Workspaces, index)) => Some(*index),
            _ => None,
        }
    }
}
