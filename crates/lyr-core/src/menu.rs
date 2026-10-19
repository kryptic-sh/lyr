//! What the Bluetooth and Wi-Fi menus draw alike: rows of text and buttons
//! in the bar's module box, a field for typing, and the keyboard's outline.

use lyr_config::{Colour, Theme};

use crate::draw::{DrawList, Item, Size, TextStyle};
use crate::hit::Rect;

/// The menu's padding, around its rows.
pub const PADDING: f32 = 8.0;
/// The space between rows, and between a row's buttons.
pub const GAP: f32 = 6.0;
/// Around a button's label, across and down.
pub const BUTTON_PADDING: (f32, f32) = (8.0, 2.0);

/// The height of a row of text `line_height` tall, buttons included.
#[must_use]
pub fn row(line_height: f32) -> f32 {
    line_height + 2.0 * BUTTON_PADDING.1
}

/// The width of a button labelled `label`.
#[must_use]
pub fn button_width(label: &str, measure: &dyn Fn(&str) -> f32) -> f32 {
    measure(label) + 2.0 * BUTTON_PADDING.0
}

/// `text` at `x` in the row at `y`.
#[must_use]
pub fn text(
    text: String,
    x: f32,
    y: f32,
    measure: &dyn Fn(&str) -> f32,
    line_height: f32,
    colour: Colour,
) -> Item {
    Item::Text {
        rect: Rect {
            x,
            y: y + BUTTON_PADDING.1,
            width: measure(&text),
            height: line_height,
        },
        text,
        colour,
        style: TextStyle::PLAIN,
    }
}

/// A button labelled `label` filling `rect`.
#[must_use]
pub fn button(
    rect: Rect,
    label: &str,
    measure: &dyn Fn(&str) -> f32,
    line_height: f32,
    theme: &Theme,
) -> [Item; 2] {
    [
        Item::Box {
            rect,
            radius: theme.radius as f32,
            fill: theme.toast_background,
            border: Some((theme.border_width as f32, theme.panel_border)),
        },
        text(
            label.to_owned(),
            rect.x + BUTTON_PADDING.0,
            rect.y,
            measure,
            line_height,
            theme.foreground,
        ),
    ]
}

/// A field for typing filling `rect`, showing `shown`. Its end shows,
/// where the typing is, when it is too long for the field.
#[must_use]
pub fn field(
    rect: Rect,
    shown: &str,
    measure: &dyn Fn(&str) -> f32,
    line_height: f32,
    theme: &Theme,
) -> [Item; 2] {
    let across = BUTTON_PADDING.0;
    let inside = rect.width - 2.0 * across;
    let scrolled = (measure(shown) - inside).max(0.0);
    [
        Item::Box {
            rect,
            radius: theme.radius as f32,
            fill: theme.module_background,
            border: Some((theme.border_width as f32, theme.workspace_active)),
        },
        Item::Clip {
            rect: Rect {
                x: rect.x + across,
                width: inside,
                ..rect
            },
            items: vec![text(
                shown.to_owned(),
                rect.x + across - scrolled,
                rect.y,
                measure,
                line_height,
                theme.foreground,
            )],
        },
    ]
}

/// The menu: the bar's module box of `size` behind `items`, and the
/// keyboard's place, `focus`, outlined in the active colour.
#[must_use]
pub fn frame(size: Size, items: Vec<Item>, focus: Option<Rect>, theme: &Theme) -> DrawList {
    let mut list = DrawList {
        items: vec![Item::Box {
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: size.width,
                height: size.height,
            },
            radius: theme.radius as f32,
            fill: theme.module_background,
            border: Some((theme.border_width as f32, theme.border)),
        }],
    };
    list.items.extend(items);
    if let Some(rect) = focus {
        list.items.push(Item::Box {
            rect,
            radius: theme.radius as f32,
            fill: Colour::TRANSPARENT,
            border: Some((theme.border_width as f32, theme.workspace_active)),
        });
    }
    list
}
