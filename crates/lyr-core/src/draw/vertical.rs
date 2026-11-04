//! A side bar's layout (`docs/plan/17-bar-placement.md`): the horizontal
//! bar's boxes stacked down the bar.
//!
//! The zones run top to bottom, placed by [`layout`] along the bar's length
//! as a horizontal bar's are along its width: `left` is the top zone, `right`
//! the bottom one, and the centre is centred. Each box spans the bar's width
//! less the theme's left and right margins, and is one line of text tall
//! plus its border, with the theme's top and bottom margins around it. The
//! theme's padding is horizontal only (waybar's `padding: 0 12px`), so it
//! adds nothing to a box's height; nor is it kept clear inside a box whose
//! width the bar sets, so a narrow bar still has room for an icon.
//!
//! Text stays horizontal and is centred in its box. A label too wide for
//! its box shows its compact form, and is cut with an ellipsis when even
//! that does not fit; a button's name too wide for the box is cut the same
//! way. Giving a module its compact form saves no length, so
//! where zones meet, the modules nearest the centre are dropped.

use lyr_config::Theme;

use super::{
    Bar, Button, Content, DrawList, Entry, Item, Target, TextStyle, background, module_box,
    separator,
};
use crate::hit::{Rect, Regions};
use crate::layout::{Slot, layout};
use crate::window::ELLIPSIS;

/// Lays out and draws `bar` down a side, in lines `line_height` tall.
pub(super) fn build<T: Clone>(
    bar: &Bar<'_, T>,
    theme: &Theme,
    measure: impl Fn(&str) -> f32,
    line_height: f32,
) -> (DrawList, Regions<Target<T>>) {
    let px = |value: u32| value as f32;
    let border = px(theme.border_width);
    let margin_top = px(theme.module_margin_top);
    let margins = margin_top + px(theme.module_margin_bottom);
    let x = px(theme.module_margin_left);
    let box_width = (bar.width - x - px(theme.last_module_margin_right)).max(0.0);
    // What a label has to fit in: the box inside its border.
    let room = (box_width - 2.0 * border).max(0.0);

    // Each entry's length along the bar, margins included.
    let length = |entry: &Entry<T>| match entry {
        &Entry::Spacer(length) => length,
        Entry::Separator => border + margins,
        Entry::Module(module) => {
            content_height(&module.content, line_height) + 2.0 * border + margins
        }
    };
    let slots = |entries: &[Entry<T>]| -> Vec<Slot> {
        entries
            .iter()
            .map(|entry| Slot {
                full: length(entry),
                compact: None,
            })
            .collect()
    };
    let zones = layout(
        bar.height,
        &slots(bar.left),
        &slots(bar.center),
        &slots(bar.right),
    );

    let mut list = DrawList {
        items: vec![background(bar, theme)],
    };
    let mut regions = Regions::default();
    let placed_zones = [
        (bar.left, &zones.left),
        (bar.center, &zones.center),
        (bar.right, &zones.right),
    ];
    for (entries, placed) in placed_zones {
        for (entry, placed) in entries.iter().zip(placed) {
            let Some(placed) = placed else { continue };
            // `layout` places along one axis, here the bar's length.
            let rect = Rect {
                x,
                y: placed.x + margin_top,
                width: box_width,
                height: placed.width - margins,
            };
            let module = match entry {
                Entry::Module(module) => module,
                Entry::Spacer(_) => continue,
                Entry::Separator => {
                    list.items.push(separator(rect, theme));
                    continue;
                }
            };
            let mut items = vec![module_box(rect, theme)];
            regions.push(rect, Target::Module(module.target.clone()));
            match &module.content {
                Content::Label {
                    text,
                    compact,
                    colour,
                } => {
                    let text = fitted(text, compact.as_deref(), room, &measure);
                    let width = measure(&text);
                    items.push(Item::Text {
                        rect: Rect {
                            x: rect.x + (rect.width - width) / 2.0,
                            width,
                            ..rect
                        },
                        text,
                        colour: *colour,
                        style: TextStyle::PLAIN,
                    });
                }
                Content::Buttons {
                    buttons,
                    padding,
                    gap,
                } => {
                    let mut y = rect.y + border;
                    for (i, button) in buttons.iter().enumerate() {
                        // A name wider than the box is cut as a label is;
                        // the padding around it gives way first.
                        let text = cut(&button.text, room, &measure);
                        let face = button
                            .picture
                            .as_ref()
                            .map_or_else(|| measure(&text), |picture| picture.width);
                        let height = row_height(button, line_height);
                        let width = (face + 2.0 * padding).min(rect.width);
                        // One button a row, centred across the bar.
                        let button_rect = Rect {
                            x: rect.x + (rect.width - width) / 2.0,
                            y,
                            width,
                            height,
                        };
                        let face_x = rect.x + (rect.width - face) / 2.0;
                        items.push(match &button.picture {
                            Some(picture) => Item::Image {
                                rect: Rect {
                                    x: face_x,
                                    y: y + (height - picture.height) / 2.0,
                                    width: picture.width,
                                    height: picture.height,
                                },
                                picture: picture.picture.clone(),
                            },
                            None => Item::Text {
                                rect: Rect {
                                    x: face_x,
                                    width: face,
                                    ..button_rect
                                },
                                text,
                                colour: button.colour,
                                style: TextStyle::PLAIN,
                            },
                        });
                        regions.push(button_rect, Target::Button(module.target.clone(), i));
                        y += height + gap;
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

/// How tall `content` is inside its box's border: a line, or a column of
/// buttons `gap` apart.
fn content_height(content: &Content, line_height: f32) -> f32 {
    match content {
        Content::Label { .. } => line_height,
        Content::Buttons { buttons, gap, .. } => {
            buttons
                .iter()
                .map(|button| row_height(button, line_height))
                .sum::<f32>()
                + gap * buttons.len().saturating_sub(1) as f32
        }
    }
}

/// How tall `button`'s row is: a line, or its picture where that is taller.
fn row_height(button: &Button, line_height: f32) -> f32 {
    button
        .picture
        .as_ref()
        .map_or(line_height, |picture| picture.height.max(line_height))
}

/// What a label shows in `room` pixels: its text where that fits, else its
/// compact form, cut to fit; a label with no compact form cuts its text.
fn fitted(text: &str, compact: Option<&str>, room: f32, measure: impl Fn(&str) -> f32) -> String {
    if measure(text) <= room {
        return text.to_owned();
    }
    cut(compact.unwrap_or(text), room, measure)
}

/// `text` cut to `room` pixels as `measure` reads them: whole when it fits,
/// else as much of it as fits before an ellipsis, and empty when not even the
/// ellipsis does.
fn cut(text: &str, room: f32, measure: impl Fn(&str) -> f32) -> String {
    if measure(text) <= room {
        return text.to_owned();
    }
    let mut kept = String::new();
    for c in text.chars() {
        kept.push(c);
        if measure(&format!("{kept}{ELLIPSIS}")) > room {
            kept.pop();
            break;
        }
    }
    kept.push(ELLIPSIS);
    if measure(&kept) <= room {
        kept
    } else {
        String::new()
    }
}

#[cfg(test)]
mod tests;
