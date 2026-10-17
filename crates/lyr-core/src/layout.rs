//! Placing module boxes in the bar's three zones
//! (`docs/plan/05-layout-and-config.md`).
//!
//! The centre zone is centred on the bar however wide the side zones are. The
//! left zone runs from the bar's left edge and the right zone ends at its right
//! edge, each up to the centre zone's edge. Where a side zone would cross that
//! edge, the centre wins: the side's modules nearest the centre lose their
//! text first, keeping only their icon, and are dropped when even the icon
//! does not fit. With an empty centre zone the edge is the bar's middle.
//!
//! Widths are logical pixels and include each box's margins, so boxes are
//! placed edge to edge.

/// How wide one module's box is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Slot {
    /// With its icon and text.
    pub full: f32,
    /// With its icon alone, or `None` when it has no shorter form (a module
    /// that is only text, or only an icon).
    pub compact: Option<f32>,
}

/// How a module is drawn after layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Form {
    Full,
    /// The icon alone: the text did not fit.
    Compact,
}

/// Where one module's box goes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placed {
    /// The box's left edge, from the bar's left edge.
    pub x: f32,
    pub width: f32,
    pub form: Form,
}

/// The layout of one bar. Each zone has one entry per module, in config
/// order; `None` is a module that did not fit and is not drawn.
#[derive(Clone, Debug, PartialEq)]
pub struct Zones {
    pub left: Vec<Option<Placed>>,
    pub center: Vec<Option<Placed>>,
    pub right: Vec<Option<Placed>>,
}

/// Lays out a bar `width` wide.
#[must_use]
pub fn layout(width: f32, left: &[Slot], center: &[Slot], right: &[Slot]) -> Zones {
    let center_width: f32 = center.iter().map(|slot| slot.full).sum();
    let center_start = (width - center_width) / 2.0;
    let center_end = center_start + center_width;

    let mut x = center_start;
    let center = center
        .iter()
        .map(|slot| {
            let placed = Placed {
                x,
                width: slot.full,
                form: Form::Full,
            };
            x += slot.full;
            Some(placed)
        })
        .collect();

    // The left zone grows rightwards from 0 to `center_start`.
    let left = fit(left.iter(), center_start.max(0.0), |start, _| start);
    // The right zone grows leftwards from `width` to `center_end`, so it is
    // fitted from its last module and then put back in config order.
    let mut right = fit(
        right.iter().rev(),
        (width - center_end.min(width)).max(0.0),
        |start, box_width| width - start - box_width,
    );
    right.reverse();

    Zones {
        left,
        center,
        right,
    }
}

/// Packs `slots`, outermost first, into `room` pixels. `x_of(start, width)`
/// turns a distance from the zone's outer edge into the box's left edge.
/// Once a module does not fit, those after it (nearer the centre) are
/// dropped too, so a zone never has a gap in it.
fn fit<'a>(
    slots: impl Iterator<Item = &'a Slot>,
    room: f32,
    x_of: impl Fn(f32, f32) -> f32,
) -> Vec<Option<Placed>> {
    let mut used = 0.0;
    let mut full = true;
    slots
        .map(|slot| {
            if !full {
                return None;
            }
            let (box_width, form) = if used + slot.full <= room {
                (slot.full, Form::Full)
            } else {
                full = false;
                match slot.compact {
                    Some(compact) if used + compact <= room => (compact, Form::Compact),
                    _ => return None,
                }
            };
            let placed = Placed {
                x: x_of(used, box_width),
                width: box_width,
                form,
            };
            used += box_width;
            Some(placed)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{Form, Placed, Slot, layout};

    const fn slot(full: f32) -> Slot {
        Slot {
            full,
            compact: None,
        }
    }

    const fn with_icon(full: f32, compact: f32) -> Slot {
        Slot {
            full,
            compact: Some(compact),
        }
    }

    fn xs(zone: &[Option<Placed>]) -> Vec<Option<f32>> {
        zone.iter().map(|placed| placed.map(|p| p.x)).collect()
    }

    #[test]
    fn zones_sit_at_the_edges_and_the_centre() {
        let zones = layout(
            1000.0,
            &[slot(100.0), slot(50.0)],
            &[slot(200.0)],
            &[slot(30.0), slot(70.0)],
        );
        assert_eq!(xs(&zones.left), [Some(0.0), Some(100.0)]);
        assert_eq!(xs(&zones.center), [Some(400.0)]);
        assert_eq!(xs(&zones.right), [Some(900.0), Some(930.0)]);
    }

    #[test]
    fn the_centre_stays_centred_however_wide_the_sides_are() {
        let narrow = layout(1000.0, &[slot(10.0)], &[slot(200.0)], &[]);
        let wide = layout(1000.0, &[slot(390.0)], &[slot(200.0)], &[slot(5.0)]);
        assert_eq!(xs(&narrow.center), xs(&wide.center));
    }

    #[test]
    fn a_side_crossing_the_centre_loses_text_then_modules() {
        // The left zone has 400 px before the centre starts.
        let zones = layout(
            1000.0,
            &[slot(300.0), with_icon(150.0, 60.0), with_icon(100.0, 20.0)],
            &[slot(200.0)],
            &[],
        );
        let forms: Vec<_> = zones.left.iter().map(|p| p.map(|p| p.form)).collect();
        assert_eq!(forms, [Some(Form::Full), Some(Form::Compact), None]);
        assert_eq!(zones.left[1].map(|p| (p.x, p.width)), Some((300.0, 60.0)));
    }

    #[test]
    fn a_module_with_no_short_form_is_dropped_whole() {
        let zones = layout(1000.0, &[slot(350.0), slot(100.0)], &[slot(200.0)], &[]);
        assert_eq!(xs(&zones.left), [Some(0.0), None]);
    }

    #[test]
    fn the_right_zone_is_clipped_nearest_the_centre_first() {
        // 400 px between the centre's end (600) and the right edge.
        let zones = layout(
            1000.0,
            &[],
            &[slot(200.0)],
            &[with_icon(200.0, 40.0), slot(150.0), slot(150.0)],
        );
        assert_eq!(xs(&zones.right), [Some(660.0), Some(700.0), Some(850.0)]);
        assert_eq!(zones.right[0].map(|p| p.form), Some(Form::Compact));
    }

    #[test]
    fn with_no_centre_each_side_gets_half() {
        let zones = layout(100.0, &[slot(60.0)], &[], &[slot(40.0)]);
        assert_eq!(xs(&zones.left), [None]);
        assert_eq!(xs(&zones.right), [Some(60.0)]);
    }

    #[test]
    fn a_centre_wider_than_the_bar_leaves_no_room_for_the_sides() {
        let zones = layout(100.0, &[slot(1.0)], &[slot(300.0)], &[slot(1.0)]);
        assert_eq!(xs(&zones.center), [Some(-100.0)]);
        assert_eq!(xs(&zones.left), [None]);
        assert_eq!(xs(&zones.right), [None]);
    }
}
