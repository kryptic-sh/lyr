//! The notification centre under the clock panel's calendar
//! (`docs/plan/09-notifications.md`): swaync's `title`, `dnd` and
//! `notifications` widgets in its order, in the dotfiles' style.
//!
//! The title and the do-not-disturb label are drawn at the bar's text size,
//! not swaync's 2rem: lyr draws one size of text so far. Media players (the
//! `mpris` widget) are not drawn.

use std::time::Instant;

use lyr_config::Theme;

use crate::draw::{DrawList, Item, Size};
use crate::hit::{Rect, Regions};
use crate::notifications::Store;
use crate::toast;

/// What a click in the centre lands on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    ClearAll,
    Dnd,
    Notification(u32),
}

/// Above the do-not-disturb row and the history: swaync's `widget-dnd`
/// margin.
pub const GAP: f32 = 16.0;
/// Around the Clear All button's text: swaync's 4 px by 8 px.
const BUTTON_PADDING: (f32, f32) = (8.0, 4.0);
/// The buttons' and the switch's border.
const BORDER: f32 = 2.0;
/// swaync's corner radius.
const RADIUS: f32 = 8.0;
/// Between the switch's edge and its knob.
const KNOB_INSET: f32 = 4.0;
/// Between two history entries: 4 px above and below each.
const ENTRY_GAP: f32 = 8.0;
/// Between two of the history's groups, besides the entries' own gap.
const GROUP_GAP: f32 = 8.0;

/// The centre for `store` at `now`, `width` wide: its draw list, its size,
/// and what each click lands on, its text measured by `measure` in lines
/// `line_height` tall.
#[must_use]
pub fn draw(
    store: &Store,
    width: f32,
    now: Instant,
    measure: &dyn Fn(&str) -> f32,
    line_height: f32,
    theme: &Theme,
) -> (DrawList, Size, Regions<Target>) {
    let mut items = Vec::new();
    let mut regions = Regions::default();
    let label = |text: &str, x: f32, y: f32, colour| Item::Text {
        rect: Rect {
            x,
            y,
            width: measure(text),
            height: line_height,
        },
        text: text.to_owned(),
        colour,
    };

    // The title and Clear All.
    let (across, down) = BUTTON_PADDING;
    let button = Rect {
        x: width - (measure("Clear All") + 2.0 * (across + BORDER)),
        y: 0.0,
        width: measure("Clear All") + 2.0 * (across + BORDER),
        height: line_height + 2.0 * (down + BORDER),
    };
    let row = button.height;
    items.push(label(
        "Notification Center",
        0.0,
        (row - line_height) / 2.0,
        theme.centre_title,
    ));
    items.push(Item::Box {
        rect: button,
        radius: RADIUS,
        fill: theme.toast_background,
        border: Some((BORDER, theme.panel_border)),
    });
    items.push(label(
        "Clear All",
        button.x + BORDER + across,
        button.y + BORDER + down,
        theme.panel_foreground,
    ));
    regions.push(button, Target::ClearAll);
    let mut y = row + GAP;

    // Do Not Disturb and its switch.
    let switch = Rect {
        x: width - 2.0 * line_height,
        y,
        width: 2.0 * line_height,
        height: line_height,
    };
    items.push(label("Do Not Disturb", 0.0, y, theme.panel_foreground));
    items.push(Item::Box {
        rect: switch,
        radius: switch.height / 2.0,
        fill: if store.dnd {
            theme.switch_on
        } else {
            theme.toast_background
        },
        border: Some((BORDER, theme.panel_border)),
    });
    let knob = switch.height - 2.0 * KNOB_INSET;
    items.push(Item::Box {
        rect: Rect {
            x: if store.dnd {
                switch.x + switch.width - KNOB_INSET - knob
            } else {
                switch.x + KNOB_INSET
            },
            y: switch.y + KNOB_INSET,
            width: knob,
            height: knob,
        },
        radius: knob / 2.0,
        fill: theme.panel_border,
        border: None,
    });
    regions.push(switch, Target::Dnd);
    // The bottom of the last thing drawn.
    let mut bottom = switch.y + switch.height;
    y = bottom + GAP;

    // The history, swaync's order, each entry drawn as a toast.
    for (index, (_, members)) in store.history().into_iter().enumerate() {
        if index > 0 {
            y += GROUP_GAP;
        }
        for held in members {
            let elapsed = now.saturating_duration_since(held.received);
            let (entry, size) =
                toast::draw(&held.content, elapsed, width, measure, line_height, theme);
            items.extend(entry.items.into_iter().map(|item| item.offset(0.0, y)));
            regions.push(
                Rect {
                    x: 0.0,
                    y,
                    width: size.width,
                    height: size.height,
                },
                Target::Notification(held.id),
            );
            bottom = y + size.height;
            y = bottom + ENTRY_GAP;
        }
    }
    let size = Size {
        width,
        height: bottom,
    };
    (DrawList { items }, size, regions)
}

#[cfg(test)]
mod tests;
