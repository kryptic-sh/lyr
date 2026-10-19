//! lyr's draw list in crcbl-ui's commands
//! (`docs/plan/04-rendering-and-theme.md`).

use crcbl_ui::DrawList;
use crcbl_ui::draw_list::{Border, CornerRadii};
use glam::Vec2;
use lyr_core::draw::{DrawList as Bar, Item};

use crate::colour::linear;
use crate::text::Text;

/// `bar` as crcbl-ui draw commands, its text drawn in `text`. A group is
/// one of crcbl-ui's opacity groups, faded as one layer as CSS `opacity`
/// fades it.
#[must_use]
pub fn translate(bar: &Bar, text: &Text) -> DrawList {
    let mut list = DrawList::new();
    for item in &bar.items {
        push(&mut list, item, text);
    }
    list
}

fn push(list: &mut DrawList, item: &Item, text: &Text) {
    match item {
        Item::Box {
            rect,
            radius,
            fill,
            border,
        } => list.rounded_rect(
            Vec2::new(rect.x, rect.y),
            Vec2::new(rect.x + rect.width, rect.y + rect.height),
            CornerRadii::uniform(*radius),
            linear(*fill),
            border.map_or(Border::NONE, |(width, colour)| Border {
                width,
                color: linear(colour),
            }),
        ),
        Item::Text {
            rect,
            text: string,
            colour,
            style,
        } => {
            // Text that runs past its rect is cut there, not over its
            // neighbours.
            list.push_clip(
                Vec2::new(rect.x, rect.y),
                Vec2::new(rect.x + rect.width, rect.y + rect.height),
            );
            text.draw(list, *rect, string, *style, linear(*colour));
            list.pop_clip().expect("the clip pushed just above");
        }
        Item::Group { opacity, items } => {
            list.push_opacity(*opacity);
            for item in items {
                push(list, item, text);
            }
            list.pop_opacity().expect("the group pushed just above");
        }
        Item::Clip { rect, items } => {
            list.push_clip(
                Vec2::new(rect.x, rect.y),
                Vec2::new(rect.x + rect.width, rect.y + rect.height),
            );
            for item in items {
                push(list, item, text);
            }
            list.pop_clip().expect("the clip pushed just above");
        }
    }
}

#[cfg(test)]
mod tests {
    use crcbl_ui::draw_list::{ClipRect, DrawCommand};
    use crcbl_ui::font::Font;
    use lyr_config::Colour;
    use lyr_core::draw::{DrawList as Bar, Item, TextStyle};
    use lyr_core::hit::Rect;

    use super::translate;
    use crate::text::Text;

    const RECT: Rect = Rect {
        x: 10.0,
        y: 10.0,
        width: 50.0,
        height: 30.0,
    };

    fn colour(hex: &str) -> Colour {
        Colour::parse(hex).expect("colour")
    }

    #[test]
    fn a_box_becomes_a_rounded_rect_with_its_border_inside() {
        let bar = Bar {
            items: vec![Item::Box {
                rect: RECT,
                radius: 8.0,
                fill: colour("#ffffff"),
                border: Some((2.0, colour("#000000"))),
            }],
        };
        let list = translate(&bar, &Text::new(Font::sans().clone(), 16.0));
        let [
            DrawCommand::RoundedRect {
                min,
                max,
                radii,
                color,
                border,
            },
        ] = list.commands()
        else {
            panic!("one rounded rect: {:?}", list.commands());
        };
        assert_eq!((min.x, min.y, max.x, max.y), (10.0, 10.0, 60.0, 40.0));
        assert!((radii.top_left - 8.0).abs() < f32::EPSILON);
        assert!((color[3] - 1.0).abs() < f32::EPSILON);
        assert!((border.width - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn a_group_is_an_opacity_layer_around_its_items() {
        let bar = Bar {
            items: vec![
                Item::Group {
                    opacity: 0.5,
                    items: vec![
                        Item::Box {
                            rect: RECT,
                            radius: 0.0,
                            fill: colour("#ffffff"),
                            border: None,
                        },
                        Item::Group {
                            opacity: 0.5,
                            items: vec![Item::Text {
                                rect: RECT,
                                text: "a".to_owned(),
                                colour: colour("#ffffff"),
                                style: TextStyle::PLAIN,
                            }],
                        },
                    ],
                },
                // After the group, and outside it.
                Item::Box {
                    rect: RECT,
                    radius: 0.0,
                    fill: colour("#ffffff"),
                    border: None,
                },
            ],
        };
        let list = translate(&bar, &Text::new(Font::sans().clone(), 16.0));
        let count = list.commands().len();
        let groups: Vec<(std::ops::Range<usize>, f32)> = list
            .opacity_groups()
            .iter()
            .map(|group| (group.commands.clone(), group.opacity))
            .collect();
        assert_eq!(
            groups,
            [(0..count - 1, 0.5), (1..count - 1, 0.5)],
            "the outer group holds the box and the inner one, which holds the text"
        );
        assert!(
            list.commands().iter().all(|command| match command {
                DrawCommand::RoundedRect { color, .. } | DrawCommand::Glyphs { color, .. } =>
                    (color[3] - 1.0).abs() < f32::EPSILON,
                _ => true,
            }),
            "the items keep their own alpha"
        );
    }

    #[test]
    fn text_is_clipped_to_its_rect_and_boxes_are_not() {
        let bar = Bar {
            items: vec![
                Item::Text {
                    rect: RECT,
                    text: "a long line of text".to_owned(),
                    colour: colour("#ffffff"),
                    style: TextStyle::PLAIN,
                },
                Item::Box {
                    rect: RECT,
                    radius: 0.0,
                    fill: colour("#ffffff"),
                    border: None,
                },
            ],
        };
        let list = translate(&bar, &Text::new(Font::sans().clone(), 16.0));
        let (glyphs, boxes): (Vec<_>, Vec<_>) = list
            .commands()
            .iter()
            .zip(list.clips())
            .partition(|(command, _)| matches!(command, DrawCommand::Glyphs { .. }));
        assert!(!glyphs.is_empty(), "the text drew");
        for (_, clip) in glyphs {
            assert_eq!(
                (clip.min.x, clip.min.y, clip.max.x, clip.max.y),
                (10.0, 10.0, 60.0, 40.0)
            );
        }
        let [(_, clip)] = boxes.as_slice() else {
            panic!("one box");
        };
        assert_eq!(**clip, ClipRect::NONE);
    }

    #[test]
    fn a_clip_cuts_what_it_holds() {
        let bar = Bar {
            items: vec![Item::Clip {
                rect: RECT,
                items: vec![Item::Box {
                    rect: Rect {
                        x: 0.0,
                        y: 0.0,
                        width: 100.0,
                        height: 100.0,
                    },
                    radius: 0.0,
                    fill: colour("#ffffff"),
                    border: None,
                }],
            }],
        };
        let list = translate(&bar, &Text::new(Font::sans().clone(), 16.0));
        let [clip] = list.clips() else {
            panic!("one command");
        };
        assert_eq!(
            (clip.min.x, clip.min.y, clip.max.x, clip.max.y),
            (10.0, 10.0, 60.0, 40.0)
        );
    }

    #[test]
    fn text_draws_in_its_style() {
        let item = |style| Item::Text {
            rect: RECT,
            text: "a".to_owned(),
            colour: colour("#ffffff"),
            style,
        };
        let bold = TextStyle {
            bold: true,
            ..TextStyle::PLAIN
        };
        let bar = Bar {
            items: vec![item(TextStyle::PLAIN), item(bold)],
        };
        let list = translate(&bar, &Text::new(Font::sans().clone(), 16.0));
        let fonts: Vec<_> = list
            .commands()
            .iter()
            .filter_map(|command| match command {
                crcbl_ui::draw_list::DrawCommand::Glyphs { font, .. } => Some(font.synthesis()),
                _ => None,
            })
            .collect();
        assert_eq!(fonts.len(), 2);
        assert!(!fonts[0].bold && fonts[1].bold, "{fonts:?}");
    }
}
