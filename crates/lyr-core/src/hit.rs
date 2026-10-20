//! Finding what is under the pointer.
//!
//! Each layout records the rectangle of everything that takes input (a module
//! box, a workspace button inside it) with what it stands for. A pointer
//! event is then a lookup against the last layout, so a click acts on what
//! was drawn, never on state that changed since.

/// A rectangle in a bar's logical pixels. It contains its left and top edges
/// but not its right and bottom ones, so two boxes placed edge to edge never
/// both claim the pixel between them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    /// Whether the point `(x, y)` is inside.
    #[must_use]
    pub fn contains(&self, x: f32, y: f32) -> bool {
        (self.x..self.x + self.width).contains(&x) && (self.y..self.y + self.height).contains(&y)
    }

    /// Where `x` is across it, from 0 at its left edge to 1 at its right,
    /// held to that range; 0 for a rectangle with no width.
    #[must_use]
    pub fn across(&self, x: f32) -> f32 {
        if self.width > 0.0 {
            ((x - self.x) / self.width).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }
}

/// The input regions of one layout, each standing for a `T` (a module, a
/// workspace button).
#[derive(Clone, Debug)]
pub struct Regions<T> {
    regions: Vec<(Rect, T)>,
}

impl<T> Default for Regions<T> {
    fn default() -> Self {
        Self {
            regions: Vec::new(),
        }
    }
}

impl<T> Regions<T> {
    /// Adds a region. One added later is on top of one added earlier, so a
    /// button inside a module box is added after the box.
    pub fn push(&mut self, rect: Rect, target: T) {
        self.regions.push((rect, target));
    }

    /// The rectangle of the topmost region standing for `target`.
    #[must_use]
    pub fn rect(&self, target: &T) -> Option<Rect>
    where
        T: PartialEq,
    {
        self.regions
            .iter()
            .rev()
            .find(|(_, held)| held == target)
            .map(|(rect, _)| *rect)
    }

    /// Every region's target, in the order they were added.
    pub fn targets(&self) -> impl Iterator<Item = &T> {
        self.regions.iter().map(|(_, target)| target)
    }

    /// The topmost region containing `(x, y)`.
    #[must_use]
    pub fn hit(&self, x: f32, y: f32) -> Option<&T> {
        self.regions
            .iter()
            .rev()
            .find(|(rect, _)| rect.contains(x, y))
            .map(|(_, target)| target)
    }
}

#[cfg(test)]
mod tests {
    use super::{Rect, Regions};

    #[test]
    fn across_is_the_share_of_the_width_held_to_its_edges() {
        let rect = rect(100.0, 200.0);
        assert!(rect.across(100.0).abs() < f32::EPSILON);
        assert!((rect.across(150.0) - 0.25).abs() < f32::EPSILON);
        assert!((rect.across(300.0) - 1.0).abs() < f32::EPSILON);
        assert!(rect.across(40.0).abs() < f32::EPSILON, "left of it");
        assert!(
            (rect.across(900.0) - 1.0).abs() < f32::EPSILON,
            "right of it"
        );
        assert!(super::Rect { width: 0.0, ..rect }.across(100.0).abs() < f32::EPSILON);
    }

    const fn rect(x: f32, width: f32) -> Rect {
        Rect {
            x,
            y: 10.0,
            width,
            height: 30.0,
        }
    }

    #[test]
    fn a_point_hits_the_box_it_is_in() {
        let mut regions = Regions::default();
        regions.push(rect(0.0, 100.0), "power");
        regions.push(rect(100.0, 50.0), "battery");
        assert_eq!(regions.hit(20.0, 20.0), Some(&"power"));
        assert_eq!(regions.hit(120.0, 20.0), Some(&"battery"));
        assert_eq!(regions.hit(200.0, 20.0), None);
        assert_eq!(regions.hit(20.0, 5.0), None, "above the box, in its margin");
    }

    #[test]
    fn the_shared_edge_belongs_to_the_right_hand_box() {
        let mut regions = Regions::default();
        regions.push(rect(0.0, 100.0), "power");
        regions.push(rect(100.0, 50.0), "battery");
        assert_eq!(regions.hit(100.0, 20.0), Some(&"battery"));
        assert_eq!(regions.hit(150.0, 20.0), None);
        assert_eq!(regions.hit(20.0, 40.0), None, "the bottom edge is outside");
    }

    #[test]
    fn a_region_added_later_is_on_top() {
        let mut regions = Regions::default();
        regions.push(rect(0.0, 300.0), "workspaces");
        regions.push(rect(10.0, 40.0), "workspace 1");
        assert_eq!(regions.hit(20.0, 20.0), Some(&"workspace 1"));
        assert_eq!(regions.hit(200.0, 20.0), Some(&"workspaces"));
    }

    #[test]
    fn a_target_s_rect_is_found_by_what_it_stands_for() {
        let mut regions = Regions::default();
        regions.push(rect(0.0, 100.0), "power");
        regions.push(rect(100.0, 50.0), "battery");
        regions.push(rect(100.0, 20.0), "battery");
        assert_eq!(regions.rect(&"power"), Some(rect(0.0, 100.0)));
        assert_eq!(regions.rect(&"battery"), Some(rect(100.0, 20.0)), "topmost");
        assert_eq!(regions.rect(&"clock"), None);
    }
}
