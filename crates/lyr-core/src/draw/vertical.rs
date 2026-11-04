//! A side bar's layout (`docs/plan/05-layout-and-config.md#side-bars`): the horizontal
//! bar's boxes stacked down the bar.
//!
//! The zones run top to bottom, placed by [`layout`] along the bar's length
//! as a horizontal bar's are along its width: `left` is the top zone, `right`
//! the bottom one, and the centre is centred. Each box spans the bar's width
//! less the theme's left and right margins, and is as many lines of text
//! tall as it shows plus its border, with the theme's top and bottom margins
//! around it. The theme's padding is horizontal only (waybar's
//! `padding: 0 12px`), so it adds nothing to a box's height; nor is it kept
//! clear inside a box whose width the bar sets, so a narrow bar still has
//! room for an icon.
//!
//! Text stays horizontal and is centred in its box. A label too wide for one
//! line is wrapped at its spaces, each line centred under the one before;
//! `fitted` says what shows when even a word is too wide. A button's name
//! too wide for the box is cut with an ellipsis, and a picture too wide is
//! shrunk to it. Giving a module its compact form saves no length, so where
//! zones meet, the modules nearest the centre are dropped.

use lyr_config::Theme;

use super::{
    Bar, Button, Content, DrawList, Entry, Item, Module, SizedPicture, Target, TextStyle,
    background, module_box, separator,
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

    // Each label's lines, worked out before layout: how many there are sets
    // its box's length. Other entries have none.
    let lines = |entries: &[Entry<T>]| -> Vec<Vec<String>> {
        entries
            .iter()
            .map(|entry| match entry {
                Entry::Module(Module {
                    content: Content::Label { text, compact, .. },
                    ..
                }) => fitted(text, compact.as_deref(), room, &measure),
                Entry::Module(_) | Entry::Spacer(_) | Entry::Separator => Vec::new(),
            })
            .collect()
    };
    let lines = [lines(bar.left), lines(bar.center), lines(bar.right)];

    // Each entry's length along the bar, margins included.
    let length = |entry: &Entry<T>, lines: &[String]| match entry {
        &Entry::Spacer(length) => length,
        Entry::Separator => border + margins,
        Entry::Module(module) => {
            content_height(&module.content, lines.len(), line_height, room) + 2.0 * border + margins
        }
    };
    let slots = |entries: &[Entry<T>], lines: &[Vec<String>]| -> Vec<Slot> {
        entries
            .iter()
            .zip(lines)
            .map(|(entry, lines)| Slot {
                full: length(entry, lines),
                compact: None,
            })
            .collect()
    };
    let zones = layout(
        bar.height,
        &slots(bar.left, &lines[0]),
        &slots(bar.center, &lines[1]),
        &slots(bar.right, &lines[2]),
    );

    let mut list = DrawList {
        items: vec![background(bar, theme)],
    };
    let mut regions = Regions::default();
    let placed_zones = [
        (bar.left, &lines[0], &zones.left),
        (bar.center, &lines[1], &zones.center),
        (bar.right, &lines[2], &zones.right),
    ];
    for (entries, lines, placed) in placed_zones {
        for ((entry, lines), placed) in entries.iter().zip(lines).zip(placed) {
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
                Content::Label { colour, .. } => {
                    // Each line is centred in a line's height inside the
                    // border, the first level with the box's top.
                    let mut y = rect.y;
                    for line in lines {
                        let width = measure(line);
                        items.push(Item::Text {
                            rect: Rect {
                                x: rect.x + (rect.width - width) / 2.0,
                                y,
                                width,
                                height: line_height + 2.0 * border,
                            },
                            text: line.clone(),
                            colour: *colour,
                            style: TextStyle::PLAIN,
                        });
                        y += line_height;
                    }
                }
                Content::Buttons { buttons, gap, .. } => {
                    let mut y = rect.y + border;
                    for (i, button) in buttons.iter().enumerate() {
                        let height = row_height(button, line_height, room);
                        // One button a row: the whole row across the box is
                        // the button's, its face centred in it.
                        let row = Rect { y, height, ..rect };
                        items.push(match &button.picture {
                            Some(picture) => {
                                let (width, picture_height) = picture_size(picture, room);
                                Item::Image {
                                    rect: Rect {
                                        x: rect.x + (rect.width - width) / 2.0,
                                        y: y + (height - picture_height) / 2.0,
                                        width,
                                        height: picture_height,
                                    },
                                    picture: picture.picture.clone(),
                                }
                            }
                            None => {
                                // A name wider than the box is cut.
                                let text = cut(&button.text, room, &measure);
                                let width = measure(&text);
                                Item::Text {
                                    rect: Rect {
                                        x: rect.x + (rect.width - width) / 2.0,
                                        width,
                                        ..row
                                    },
                                    text,
                                    colour: button.colour,
                                    style: TextStyle::PLAIN,
                                }
                            }
                        });
                        regions.push(row, Target::Button(module.target.clone(), i));
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

/// How tall `content` is inside its box's border, `room` pixels across: a
/// label's `lines`, or a column of buttons `gap` apart.
fn content_height(content: &Content, lines: usize, line_height: f32, room: f32) -> f32 {
    match content {
        Content::Label { .. } => lines as f32 * line_height,
        Content::Buttons { buttons, gap, .. } => {
            buttons
                .iter()
                .map(|button| row_height(button, line_height, room))
                .sum::<f32>()
                + gap * buttons.len().saturating_sub(1) as f32
        }
    }
}

/// How tall `button`'s row is in a box `room` pixels across: a line, or its
/// picture as drawn there where that is taller.
fn row_height(button: &Button, line_height: f32, room: f32) -> f32 {
    button.picture.as_ref().map_or(line_height, |picture| {
        picture_size(picture, room).1.max(line_height)
    })
}

/// The width and height `picture` is drawn at in `room` pixels across: its
/// own size, or shrunk to `room` wide, its aspect kept, where it is wider.
fn picture_size(picture: &SizedPicture, room: f32) -> (f32, f32) {
    if picture.width <= room {
        (picture.width, picture.height)
    } else {
        (room, picture.height * room / picture.width)
    }
}

/// The lines a label shows in `room` pixels, the most readable that fits:
/// its text on one line; else its text wrapped at its spaces, where every
/// line then fits, as that still shows all of it; else its compact form,
/// whose icon reads whole where cut text loses part of what it says; else
/// its text wrapped with each line too wide cut, so a cut costs that word
/// rather than everything after it, unless no line keeps a whole word: then
/// more lines would show nothing more, so the text is cut on one. A compact
/// form too wide is cut.
fn fitted(
    text: &str,
    compact: Option<&str>,
    room: f32,
    measure: impl Fn(&str) -> f32,
) -> Vec<String> {
    if measure(text) <= room {
        return vec![text.to_owned()];
    }
    let lines = wrap(text, room, &measure);
    if lines.iter().all(|line| measure(line) <= room) {
        return lines;
    }
    match compact {
        Some(compact) => vec![cut(compact, room, &measure)],
        None if lines.iter().any(|line| measure(line) <= room) => {
            lines.iter().map(|line| cut(line, room, &measure)).collect()
        }
        None => vec![cut(text, room, &measure)],
    }
}

/// `text` broken at its spaces into lines of as many words as fit in `room`
/// pixels as `measure` reads them, a word too wide on a line of its own.
/// A run of spaces is one break; text with no words is one empty line.
fn wrap(text: &str, room: f32, measure: impl Fn(&str) -> f32) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for word in text.split(' ').filter(|word| !word.is_empty()) {
        if let Some(line) = lines.last_mut() {
            let joined = format!("{line} {word}");
            if measure(&joined) <= room {
                *line = joined;
                continue;
            }
        }
        lines.push(word.to_owned());
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
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
