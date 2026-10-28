//! Building a bar's draw list (`docs/plan/04-rendering-and-theme.md`).
//!
//! lyr has its own [`DrawList`], which `lyr-render` translates into crcbl's UI
//! pass. Keeping the type here means a bar can be built and checked without a
//! GPU, and a crcbl API change touches one translation function.
//!
//! Text is measured by the caller (`measure`), since only the renderer has the
//! font. Lengths are logical pixels.

use std::sync::Arc;

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

/// A picture: `width` × `height` pixels of straight-alpha RGBA8, rows top
/// to bottom. Two pictures are equal when their pixels are, which is what
/// the renderer keys its copies by, so a tray item read again with the
/// same icon is not uploaded again.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Picture {
    width: u32,
    height: u32,
    pixels: Arc<[u8]>,
}

impl Picture {
    /// The picture of `pixels`; `None` when it is empty or `pixels` is not
    /// `width * height * 4` bytes.
    #[must_use]
    pub fn new(width: u32, height: u32, pixels: Vec<u8>) -> Option<Self> {
        let expected = (width as usize)
            .checked_mul(height as usize)?
            .checked_mul(4)?;
        (expected > 0 && pixels.len() == expected).then(|| Self {
            width,
            height,
            pixels: pixels.into(),
        })
    }

    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    #[must_use]
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// It in `colour` alone, each pixel keeping its coverage: how GTK draws
    /// a symbolic icon in the text's colour.
    #[must_use]
    pub fn tinted(&self, colour: Colour) -> Self {
        let pixels: Vec<u8> = self
            .pixels
            .as_chunks::<4>()
            .0
            .iter()
            .flat_map(|pixel| {
                let alpha = u16::from(pixel[3]) * u16::from(colour.a) / 255;
                [colour.r, colour.g, colour.b, alpha as u8]
            })
            .collect();
        Self {
            width: self.width,
            height: self.height,
            pixels: pixels.into(),
        }
    }

    /// It drawn `height` logical pixels tall, its aspect kept.
    #[must_use]
    pub fn sized(self, height: f32) -> SizedPicture {
        let width = height * self.width as f32 / self.height as f32;
        SizedPicture {
            picture: self,
            width,
            height,
        }
    }
}

/// A picture and the size it is drawn at, in logical pixels.
#[derive(Clone, Debug, PartialEq)]
pub struct SizedPicture {
    pub picture: Picture,
    pub width: f32,
    pub height: f32,
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
    /// A picture stretched to fill `rect`.
    Image { rect: Rect, picture: Picture },
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
            Self::Image { rect, picture } => Self::Image {
                rect: moved(rect),
                picture,
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
    /// A row of buttons, each with its own text and colour or a picture,
    /// `padding` logical pixels each side of its face and `gap` apart: the
    /// workspaces, the tray's items.
    Buttons {
        buttons: Vec<Button>,
        padding: f32,
        gap: f32,
    },
}

/// One button in a [`Content::Buttons`] row: its picture when it has one,
/// else its text.
#[derive(Clone, Debug, PartialEq)]
pub struct Button {
    pub text: String,
    pub colour: Colour,
    pub picture: Option<SizedPicture>,
}

impl Button {
    /// The width of what it shows, without its padding.
    fn face_width(&self, measure: impl Fn(&str) -> f32) -> f32 {
        self.picture
            .as_ref()
            .map_or_else(|| measure(&self.text), |picture| picture.width)
    }
}

/// A module to place: what it shows, and what a click on it stands for.
#[derive(Clone, Debug, PartialEq)]
pub struct Module<T> {
    pub content: Content,
    pub target: T,
}

/// What a zone holds, in order: a module, or the config's `{ spacer = N }`
/// and `"separator"`.
#[derive(Clone, Debug, PartialEq)]
pub enum Entry<T> {
    Module(Module<T>),
    /// An empty gap this many logical pixels wide.
    Spacer(f32),
    /// A thin rule: as wide as the module boxes' border, in its colour, as
    /// tall as the boxes, and a module's left margin from what is before it.
    Separator,
}

impl<T> From<Module<T>> for Entry<T> {
    fn from(module: Module<T>) -> Self {
        Self::Module(module)
    }
}

/// What the pointer hit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Target<T> {
    Module(T),
    /// A button inside a module, by its index in the row.
    Button(T, usize),
}

/// What one bar holds, by zone.
#[derive(Clone, Copy, Debug)]
pub struct Bar<'a, T> {
    pub width: f32,
    pub height: f32,
    pub left: &'a [Entry<T>],
    pub center: &'a [Entry<T>],
    pub right: &'a [Entry<T>],
}

/// One entry's size, before placement.
struct ModuleSize {
    /// The entry's own width, without margins.
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
    let min_width = px(theme.module_min_width);
    let border_width = px(theme.border_width);

    let size = |entry: &Entry<T>, last: bool| {
        let margin_left = px(theme.module_margin_left);
        let margin_right = if last {
            px(theme.last_module_margin_right)
        } else {
            0.0
        };
        let module = match entry {
            Entry::Module(module) => module,
            &Entry::Spacer(width) => {
                return ModuleSize {
                    full: width,
                    compact: None,
                    margin_left: 0.0,
                    margin_right: 0.0,
                };
            }
            Entry::Separator => {
                return ModuleSize {
                    full: border_width,
                    compact: None,
                    margin_left,
                    margin_right,
                };
            }
        };
        // The CSS box model waybar's style.css is written for: `min-width`
        // bounds the content, and the padding and border sit outside it.
        let box_width =
            |content_width: f32| content_width.max(min_width) + 2.0 * (padding + border_width);
        let (full, compact) = match &module.content {
            Content::Label { text, compact, .. } => (
                box_width(measure(text)),
                compact.as_deref().map(|icon| box_width(measure(icon))),
            ),
            Content::Buttons {
                buttons,
                padding: button_padding,
                gap,
            } => (
                box_width(
                    buttons
                        .iter()
                        .map(|button| button.face_width(&measure) + 2.0 * button_padding)
                        .sum::<f32>()
                        + gap * buttons.len().saturating_sub(1) as f32,
                ),
                None,
            ),
        };
        ModuleSize {
            full,
            compact,
            margin_left,
            margin_right,
        }
    };
    let sizes = |entries: &[Entry<T>], is_last_zone: bool| -> Vec<ModuleSize> {
        entries
            .iter()
            .enumerate()
            .map(|(i, entry)| size(entry, is_last_zone && i + 1 == entries.len()))
            .collect()
    };
    let (left, center, right) = (
        sizes(bar.left, false),
        sizes(bar.center, false),
        sizes(bar.right, true),
    );
    let slots = |sized: &[ModuleSize]| -> Vec<Slot> {
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
    for (entries, sized, placed) in placed_zones {
        for ((entry, size), placed) in entries.iter().zip(sized).zip(placed) {
            let Some(placed) = placed else { continue };
            let rect = Rect {
                x: placed.x + size.margin_left,
                y: top,
                width: placed.width - size.margin_left - size.margin_right,
                height: box_height,
            };
            let module = match entry {
                Entry::Module(module) => module,
                Entry::Spacer(_) => continue,
                Entry::Separator => {
                    list.items.push(Item::Group {
                        opacity: theme.module_opacity,
                        items: vec![Item::Box {
                            rect,
                            radius: 0.0,
                            fill: theme.border,
                            border: None,
                        }],
                    });
                    continue;
                }
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
                Content::Buttons {
                    buttons,
                    padding: button_padding,
                    gap,
                } => {
                    let button_padding = *button_padding;
                    let mut x = rect.x + border_width + padding;
                    for (i, button) in buttons.iter().enumerate() {
                        let width = button.face_width(&measure) + 2.0 * button_padding;
                        let button_rect = Rect { x, width, ..rect };
                        items.push(match &button.picture {
                            // Centred in the box's height, as GTK centres
                            // waybar's tray images.
                            Some(picture) => Item::Image {
                                rect: Rect {
                                    x: x + button_padding,
                                    y: rect.y + (rect.height - picture.height) / 2.0,
                                    width: picture.width,
                                    height: picture.height,
                                },
                                picture: picture.picture.clone(),
                            },
                            None => Item::Text {
                                rect: Rect {
                                    x: x + button_padding,
                                    width: width - 2.0 * button_padding,
                                    ..rect
                                },
                                text: button.text.clone(),
                                colour: button.colour,
                                style: TextStyle::PLAIN,
                            },
                        });
                        regions.push(button_rect, Target::Button(module.target.clone(), i));
                        x += width + gap;
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

/// `picture` drawn in the `side`-pixel square at `x`, `y`: as large as
/// fits, its aspect kept, centred.
#[must_use]
pub fn fitted(picture: &Picture, x: f32, y: f32, side: f32) -> Item {
    let (width, height) = (picture.width() as f32, picture.height() as f32);
    let scale = side / width.max(height);
    let (width, height) = (width * scale, height * scale);
    Item::Image {
        rect: Rect {
            x: x + (side - width) / 2.0,
            y: y + (side - height) / 2.0,
            width,
            height,
        },
        picture: picture.clone(),
    }
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
