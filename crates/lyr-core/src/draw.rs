//! Building a bar's draw list (`docs/plan/04-rendering-and-theme.md`).
//!
//! lyr has its own [`DrawList`], which `lyr-render` translates into crcbl's UI
//! pass. Keeping the type here means a bar can be built and checked without a
//! GPU, and a crcbl API change touches one translation function.
//!
//! Text is measured by the caller (`measure`), since only the renderer has the
//! font. Lengths are logical pixels.

use lyr_config::{Colour, Theme};

use crate::hit::{Rect, Regions};
use crate::layout::{Form, Slot, layout};

/// How text is drawn: which of the font's faces, whether underlined, and
/// whether in the calendar's font.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TextStyle {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    /// In the theme's `calendar_font`, smaller, as waybar's `<tt><small>`.
    pub calendar: bool,
}

impl TextStyle {
    /// Upright, regular, not underlined, in the bar's font.
    pub const PLAIN: Self = Self {
        bold: false,
        italic: false,
        underline: false,
        calendar: false,
    };
}

/// Something to draw.
#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    /// A filled rectangle with rounded corners and an optional border drawn
    /// inside its edge.
    Box {
        rect: Rect,
        radius: f32,
        fill: Colour,
        border: Option<(f32, Colour)>,
    },
    /// A line of text starting at `rect`'s left edge and centred in its
    /// height, cut at `rect`'s edges where it runs past them.
    Text {
        rect: Rect,
        text: String,
        colour: Colour,
        style: TextStyle,
    },
    /// Items drawn together and then faded as one, like CSS `opacity`, so a
    /// box's text does not show its background through itself.
    Group { opacity: f32, items: Vec<Self> },
    /// Items cut at `rect`'s edges: a scrolled list inside a panel.
    Clip { rect: Rect, items: Vec<Self> },
}

impl Item {
    /// The item moved `dx` right and `dy` down, as when one list is drawn
    /// inside another.
    #[must_use]
    pub fn offset(self, dx: f32, dy: f32) -> Self {
        let moved = |rect: Rect| Rect {
            x: rect.x + dx,
            y: rect.y + dy,
            ..rect
        };
        match self {
            Self::Box {
                rect,
                radius,
                fill,
                border,
            } => Self::Box {
                rect: moved(rect),
                radius,
                fill,
                border,
            },
            Self::Text {
                rect,
                text,
                colour,
                style,
            } => Self::Text {
                rect: moved(rect),
                text,
                colour,
                style,
            },
            Self::Group { opacity, items } => Self::Group {
                opacity,
                items: items.into_iter().map(|item| item.offset(dx, dy)).collect(),
            },
            Self::Clip { rect, items } => Self::Clip {
                rect: moved(rect),
                items: items.into_iter().map(|item| item.offset(dx, dy)).collect(),
            },
        }
    }
}

/// Everything one bar draws, back to front.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DrawList {
    pub items: Vec<Item>,
}

/// What a module shows.
#[derive(Clone, Debug, PartialEq)]
pub enum Content {
    /// One line of text, with a shorter, icon-only form for when the bar runs
    /// out of room.
    Label {
        text: String,
        compact: Option<String>,
        colour: Colour,
    },
    /// A row of buttons, each with its own text and colour: the workspaces.
    Buttons(Vec<Button>),
}

/// One button in a [`Content::Buttons`] row.
#[derive(Clone, Debug, PartialEq)]
pub struct Button {
    pub text: String,
    pub colour: Colour,
}

/// A module to place: what it shows, and what a click on it stands for.
#[derive(Clone, Debug, PartialEq)]
pub struct Module<T> {
    pub content: Content,
    pub target: T,
}

/// What the pointer hit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target<T> {
    Module(T),
    /// A button inside a module, by its index in the row.
    Button(T, usize),
}

/// The modules of one bar, by zone.
#[derive(Clone, Copy, Debug)]
pub struct Bar<'a, T> {
    pub width: f32,
    pub height: f32,
    pub left: &'a [Module<T>],
    pub center: &'a [Module<T>],
    pub right: &'a [Module<T>],
}

/// One module's box size, before placement.
struct Sized {
    /// The box's own width, without margins.
    full: f32,
    compact: Option<f32>,
    /// The margins outside the box: the left one always, and the right one
    /// for the bar's last module.
    margin_left: f32,
    margin_right: f32,
}

/// Lays out and draws `bar`, returning its draw list and input regions.
pub fn build<T: Clone>(
    bar: &Bar<'_, T>,
    theme: &Theme,
    measure: impl Fn(&str) -> f32,
) -> (DrawList, Regions<Target<T>>) {
    let px = |value: u32| value as f32;
    let padding = px(theme.module_padding_x);
    let button_padding = px(theme.workspace_padding_x);
    let min_width = px(theme.module_min_width);
    let border_width = px(theme.border_width);

    let size = |module: &Module<T>, last: bool| {
        // The CSS box model waybar's style.css is written for: `min-width`
        // bounds the content, and the padding and border sit outside it.
        let box_width =
            |content_width: f32| content_width.max(min_width) + 2.0 * (padding + border_width);
        let (full, compact) = match &module.content {
            Content::Label { text, compact, .. } => (
                box_width(measure(text)),
                compact.as_deref().map(|icon| box_width(measure(icon))),
            ),
            Content::Buttons(buttons) => (
                box_width(
                    buttons
                        .iter()
                        .map(|button| measure(&button.text) + 2.0 * button_padding)
                        .sum(),
                ),
                None,
            ),
        };
        Sized {
            full,
            compact,
            margin_left: px(theme.module_margin_left),
            margin_right: if last {
                px(theme.last_module_margin_right)
            } else {
                0.0
            },
        }
    };
    let sizes = |modules: &[Module<T>], is_last_zone: bool| -> Vec<Sized> {
        modules
            .iter()
            .enumerate()
            .map(|(i, module)| size(module, is_last_zone && i + 1 == modules.len()))
            .collect()
    };
    let (left, center, right) = (
        sizes(bar.left, false),
        sizes(bar.center, false),
        sizes(bar.right, true),
    );
    let slots = |sized: &[Sized]| -> Vec<Slot> {
        sized
            .iter()
            .map(|s| {
                let margins = s.margin_left + s.margin_right;
                Slot {
                    full: s.full + margins,
                    compact: s.compact.map(|compact| compact + margins),
                }
            })
            .collect()
    };
    let zones = layout(bar.width, &slots(&left), &slots(&center), &slots(&right));

    let top = px(theme.module_margin_top);
    let box_height = (bar.height - top - px(theme.module_margin_bottom)).max(0.0);
    let mut list = DrawList::default();
    let mut regions = Regions::default();
    list.items.push(Item::Box {
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: bar.width,
            height: bar.height,
        },
        radius: 0.0,
        fill: theme.bar_background,
        border: None,
    });
    let placed_zones = [
        (bar.left, &left, &zones.left),
        (bar.center, &center, &zones.center),
        (bar.right, &right, &zones.right),
    ];
    for (modules, sized, placed) in placed_zones {
        for ((module, size), placed) in modules.iter().zip(sized).zip(placed) {
            let Some(placed) = placed else { continue };
            let rect = Rect {
                x: placed.x + size.margin_left,
                y: top,
                width: placed.width - size.margin_left - size.margin_right,
                height: box_height,
            };
            let mut items = vec![Item::Box {
                rect,
                radius: px(theme.radius),
                fill: theme.module_background,
                border: Some((px(theme.border_width), theme.border)),
            }];
            regions.push(rect, Target::Module(module.target.clone()));
            match &module.content {
                Content::Label {
                    text,
                    compact,
                    colour,
                } => {
                    let text = match (placed.form, compact) {
                        (Form::Compact, Some(icon)) => icon,
                        _ => text,
                    };
                    let width = measure(text);
                    items.push(Item::Text {
                        // Centred in the box, which may be wider than the
                        // text when the minimum width applies.
                        rect: Rect {
                            x: rect.x + (rect.width - width) / 2.0,
                            width,
                            ..rect
                        },
                        text: text.clone(),
                        colour: *colour,
                        style: TextStyle::PLAIN,
                    });
                }
                Content::Buttons(buttons) => {
                    let mut x = rect.x + border_width + padding;
                    for (i, button) in buttons.iter().enumerate() {
                        let width = measure(&button.text) + 2.0 * button_padding;
                        let button_rect = Rect { x, width, ..rect };
                        items.push(Item::Text {
                            rect: Rect {
                                x: x + button_padding,
                                width: width - 2.0 * button_padding,
                                ..rect
                            },
                            text: button.text.clone(),
                            colour: button.colour,
                            style: TextStyle::PLAIN,
                        });
                        regions.push(button_rect, Target::Button(module.target.clone(), i));
                        x += width;
                    }
                }
            }
            list.items.push(Item::Group {
                opacity: theme.module_opacity,
                items,
            });
        }
    }
    (list, regions)
}

/// The space around a tooltip's text: GTK 3's Adwaita pads both the
/// tooltip's box and its label by 4 px (`tooltip * { padding: 4px }`).
pub const TOOLTIP_PADDING: f32 = 8.0;

/// A tooltip showing `text`, one row per line, measured by `measure` in lines
/// `line_height` tall: its draw list and its size. Square and borderless, as
/// waybar's `* { border: none; border-radius: 0 }` leaves GTK's tooltip.
#[must_use]
pub fn tooltip(
    text: &str,
    measure: impl Fn(&str) -> f32,
    line_height: f32,
    theme: &Theme,
) -> (DrawList, Size) {
    let lines: Vec<&str> = text.lines().collect();
    let text_width = lines.iter().map(|line| measure(line)).fold(0.0, f32::max);
    let size = Size {
        width: text_width + 2.0 * TOOLTIP_PADDING,
        height: line_height * lines.len() as f32 + 2.0 * TOOLTIP_PADDING,
    };
    let mut items = vec![Item::Box {
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: size.width,
            height: size.height,
        },
        radius: 0.0,
        fill: theme.tooltip_background,
        border: None,
    }];
    items.extend(
        lines
            .into_iter()
            .enumerate()
            .map(|(index, line)| Item::Text {
                rect: Rect {
                    x: TOOLTIP_PADDING,
                    y: TOOLTIP_PADDING + line_height * index as f32,
                    width: text_width,
                    height: line_height,
                },
                text: line.to_owned(),
                colour: theme.tooltip_foreground,
                style: TextStyle::PLAIN,
            }),
    );
    (DrawList { items }, size)
}

/// Between a switch's edge and its knob: swaync's 4 px.
const KNOB_INSET: f32 = 4.0;
/// A switch's border.
const SWITCH_BORDER: f32 = 2.0;

/// An on/off switch filling `rect`, as swaync draws its do-not-disturb
/// switch: a pill orange while on, its knob at the end it is set to.
#[must_use]
pub fn switch(rect: Rect, on: bool, theme: &Theme) -> [Item; 2] {
    let knob = rect.height - 2.0 * KNOB_INSET;
    [
        Item::Box {
            rect,
            radius: rect.height / 2.0,
            fill: if on {
                theme.switch_on
            } else {
                theme.toast_background
            },
            border: Some((SWITCH_BORDER, theme.panel_border)),
        },
        Item::Box {
            rect: Rect {
                x: if on {
                    rect.x + rect.width - KNOB_INSET - knob
                } else {
                    rect.x + KNOB_INSET
                },
                y: rect.y + KNOB_INSET,
                width: knob,
                height: knob,
            },
            radius: knob / 2.0,
            fill: theme.panel_border,
            border: None,
        },
    ]
}

/// A width and height in logical pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

#[cfg(test)]
mod tests;
