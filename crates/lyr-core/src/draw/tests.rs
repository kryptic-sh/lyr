use lyr_config::{Colour, Platform, Theme, from_sources};

use super::{Bar, Button, Content, DrawList, Item, Module, Picture, Target, TextStyle, build};
use crate::hit::Rect;

/// Every character is 10 px wide.
fn measure(text: &str) -> f32 {
    text.chars().count() as f32 * 10.0
}

fn theme() -> Theme {
    from_sources(None, Platform::Linux).expect("defaults").theme
}

fn label(text: &str, target: &'static str) -> Module<&'static str> {
    Module {
        content: Content::Label {
            text: text.to_owned(),
            compact: None,
            colour: theme().foreground,
        },
        target,
    }
}

fn bar<'a>(
    left: &'a [Module<&'static str>],
    center: &'a [Module<&'static str>],
    right: &'a [Module<&'static str>],
) -> Bar<'a, &'static str> {
    Bar {
        width: 1000.0,
        height: 50.0,
        left,
        center,
        right,
    }
}

/// The module boxes, in drawing order.
fn boxes(list: &DrawList) -> Vec<Rect> {
    list.items
        .iter()
        .filter_map(|item| match item {
            Item::Group { items, .. } => items.iter().find_map(|item| match item {
                Item::Box { rect, .. } => Some(*rect),
                _ => None,
            }),
            _ => None,
        })
        .collect()
}

/// The text of every module, in drawing order.
fn texts(list: &DrawList) -> Vec<(String, Rect)> {
    list.items
        .iter()
        .filter_map(|item| match item {
            Item::Group { items, .. } => Some(items),
            _ => None,
        })
        .flatten()
        .filter_map(|item| match item {
            Item::Text { text, rect, .. } => Some((text.clone(), *rect)),
            _ => None,
        })
        .collect()
}

#[test]
fn a_module_box_has_the_themes_margins_and_padding() {
    let left = [label("abc", "power")];
    let (list, _) = build(&bar(&left, &[], &[]), &theme(), measure);
    // margin-left 10, margin-top 10, margin-bottom 2, and a 2 px border and
    // 12 px of padding either side of the 30 px text.
    assert_eq!(
        boxes(&list),
        [Rect {
            x: 10.0,
            y: 10.0,
            width: 58.0,
            height: 38.0
        }]
    );
    let (_, text) = &texts(&list)[0];
    assert!(
        (text.x - 24.0).abs() < f32::EPSILON,
        "text after the border and padding"
    );
}

#[test]
fn a_narrow_module_is_held_at_the_minimum_width_with_its_text_centred() {
    // The default minimum, 20 px, holds an empty label's content open.
    let left = [label("", "empty")];
    let (list, _) = build(&bar(&left, &[], &[]), &theme(), measure);
    assert!((boxes(&list)[0].width - 48.0).abs() < f32::EPSILON);

    let mut theme = theme();
    theme.module_min_width = 60;
    let left = [label("a", "x")];
    let (list, _) = build(&bar(&left, &[], &[]), &theme, measure);
    let rect = boxes(&list)[0];
    assert!(
        (rect.width - 88.0).abs() < f32::EPSILON,
        "60 + 2 * (12 + 2)"
    );
    let (_, text) = &texts(&list)[0];
    assert!((text.x - (rect.x + 39.0)).abs() < f32::EPSILON, "centred");
}

#[test]
fn the_last_module_keeps_its_right_margin() {
    let right = [label("ab", "clock"), label("cd", "bell")];
    let (list, _) = build(&bar(&[], &[], &right), &theme(), measure);
    let last = boxes(&list)[1];
    assert!((last.x + last.width - 990.0).abs() < f32::EPSILON);
}

#[test]
fn each_box_is_faded_as_a_group_over_the_bar_background() {
    let left = [label("a", "x")];
    let (list, _) = build(&bar(&left, &[], &[]), &theme(), measure);
    let Item::Box { fill, .. } = &list.items[0] else {
        panic!("the bar background comes first");
    };
    assert_eq!(fill.a, 0, "transparent");
    let Item::Group { opacity, .. } = &list.items[1] else {
        panic!("then the module");
    };
    assert!((opacity - 0.8).abs() < f32::EPSILON);
}

#[test]
fn buttons_keep_their_gap_apart_and_the_gap_is_the_modules() {
    let row = |gap| {
        [Module {
            content: Content::Buttons {
                buttons: vec![
                    Button {
                        text: "a".to_owned(),
                        colour: theme().foreground,
                        picture: None,
                    },
                    Button {
                        text: "b".to_owned(),
                        colour: theme().foreground,
                        picture: None,
                    },
                    Button {
                        text: "c".to_owned(),
                        colour: theme().foreground,
                        picture: None,
                    },
                ],
                padding: 10.0,
                gap,
            },
            target: "tray",
        }]
    };
    let (list, _) = build(&bar(&[], &row(0.0), &[]), &theme(), measure);
    let tight = boxes(&list)[0];
    let (list, regions) = build(&bar(&[], &row(10.0), &[]), &theme(), measure);
    let spaced = boxes(&list)[0];
    assert!(
        (spaced.width - tight.width - 20.0).abs() < f32::EPSILON,
        "two gaps between three buttons"
    );
    // Each button is 30 px (10 of text, 10 of padding each side) after the
    // 14 px of border and padding; the second starts a gap after the first.
    let second = spaced.x + 14.0 + 30.0 + 10.0;
    assert_eq!(
        regions.hit(second + 1.0, 20.0),
        Some(&Target::Button("tray", 1))
    );
    assert_eq!(
        regions.hit(second - 5.0, 20.0),
        Some(&Target::Module("tray")),
        "the gap belongs to the module"
    );
}

#[test]
fn a_click_on_a_workspace_button_names_the_button() {
    let active = Colour::parse("#2bc5e3").expect("colour");
    let center = [Module {
        content: Content::Buttons {
            buttons: vec![
                Button {
                    text: "1".to_owned(),
                    colour: active,
                    picture: None,
                },
                Button {
                    text: "2".to_owned(),
                    colour: theme().foreground,
                    picture: None,
                },
            ],
            padding: 10.0,
            gap: 0.0,
        },
        target: "workspaces",
    }];
    let (list, regions) = build(&bar(&[], &center, &[]), &theme(), measure);
    // Box: 2 + 12 + (10 + 10 + 10) * 2 + 12 + 2 = 88 wide.
    let rect = boxes(&list)[0];
    assert!((rect.width - 88.0).abs() < f32::EPSILON);
    let first = rect.x + 14.0 + 15.0;
    let second = rect.x + 14.0 + 30.0 + 15.0;
    assert_eq!(
        regions.hit(first, 20.0),
        Some(&Target::Button("workspaces", 0))
    );
    assert_eq!(
        regions.hit(second, 20.0),
        Some(&Target::Button("workspaces", 1))
    );
    assert_eq!(
        regions.hit(rect.x + 5.0, 20.0),
        Some(&Target::Module("workspaces")),
        "the padding belongs to the module"
    );
    assert_eq!(
        regions.hit(rect.x + 13.0, 20.0),
        Some(&Target::Module("workspaces")),
        "the first button starts after the border and the padding"
    );
    assert_eq!(
        regions.hit(rect.x + 14.0, 20.0),
        Some(&Target::Button("workspaces", 0))
    );
    assert_eq!(texts(&list)[0].0, "1");
}

#[test]
fn a_crowded_module_falls_back_to_its_icon() {
    let wide = Module {
        content: Content::Label {
            text: "x".repeat(60),
            compact: Some("i".to_owned()),
            colour: theme().foreground,
        },
        target: "network",
    };
    // The centre's 218 px slot starts at 391. Power takes 328 of that, which
    // leaves room for the network module's 58 px icon form, not its text.
    let left = [label(&"y".repeat(29), "power"), wide];
    let center = [label(&"z".repeat(18), "workspaces")];
    let (list, regions) = build(&bar(&left, &center, &[]), &theme(), measure);
    let shown: Vec<String> = texts(&list).into_iter().map(|(text, _)| text).collect();
    assert_eq!(shown[1], "i");
    assert_eq!(regions.hit(345.0, 20.0), Some(&Target::Module("network")));
}

#[test]
fn a_tooltip_pads_its_text_on_its_own_background() {
    let theme = theme();
    let (list, size) = super::tooltip("Power Button", measure, 20.0, &theme);
    assert_eq!((size.width, size.height), (136.0, 36.0));
    assert_eq!(
        list.items,
        [
            Item::Box {
                rect: Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 136.0,
                    height: 36.0,
                },
                radius: 0.0,
                fill: theme.tooltip_background,
                border: None,
            },
            Item::Text {
                rect: Rect {
                    x: 8.0,
                    y: 8.0,
                    width: 120.0,
                    height: 20.0,
                },
                text: "Power Button".to_owned(),
                colour: theme.tooltip_foreground,
                style: TextStyle::PLAIN,
            },
        ]
    );
}

#[test]
fn a_tooltip_of_several_lines_is_as_wide_as_its_widest() {
    let theme = theme();
    let (list, size) = super::tooltip("80%\nDischarging, 2h left", measure, 20.0, &theme);
    // "Discharging, 2h left" is 20 characters.
    assert_eq!((size.width, size.height), (216.0, 56.0));
    let lines: Vec<(&str, f32)> = list
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Text { text, rect, .. } => Some((text.as_str(), rect.y)),
            _ => None,
        })
        .collect();
    assert_eq!(lines, [("80%", 8.0), ("Discharging, 2h left", 28.0)]);
}

#[test]
fn an_offset_moves_every_item_a_group_holds() {
    let theme = theme();
    let rect = Rect {
        x: 1.0,
        y: 2.0,
        width: 3.0,
        height: 4.0,
    };
    let moved = Rect {
        x: 11.0,
        y: 22.0,
        ..rect
    };
    let item = Item::Group {
        opacity: 0.5,
        items: vec![
            Item::Box {
                rect,
                radius: 0.0,
                fill: theme.foreground,
                border: None,
            },
            Item::Text {
                rect,
                text: "a".to_owned(),
                colour: theme.foreground,
                style: TextStyle::PLAIN,
            },
        ],
    };
    assert_eq!(
        item.offset(10.0, 20.0),
        Item::Group {
            opacity: 0.5,
            items: vec![
                Item::Box {
                    rect: moved,
                    radius: 0.0,
                    fill: theme.foreground,
                    border: None,
                },
                Item::Text {
                    rect: moved,
                    text: "a".to_owned(),
                    colour: theme.foreground,
                    style: TextStyle::PLAIN,
                },
            ],
        }
    );
}

#[test]
fn an_offset_moves_a_clip_with_what_it_holds() {
    let theme = theme();
    let rect = Rect {
        x: 1.0,
        y: 2.0,
        width: 3.0,
        height: 4.0,
    };
    let moved = Rect {
        x: 6.0,
        y: 9.0,
        ..rect
    };
    let boxed = |rect| Item::Box {
        rect,
        radius: 0.0,
        fill: theme.foreground,
        border: None,
    };
    assert_eq!(
        Item::Clip {
            rect,
            items: vec![boxed(rect)],
        }
        .offset(5.0, 7.0),
        Item::Clip {
            rect: moved,
            items: vec![boxed(moved)],
        }
    );
}

#[test]
fn a_picture_button_is_as_wide_as_its_picture_and_centred() {
    let picture = Picture::new(2, 1, vec![0; 8]).expect("a picture");
    let row = [Module {
        content: Content::Buttons {
            buttons: vec![Button {
                text: "W".to_owned(),
                colour: theme().foreground,
                picture: Some(picture.clone().sized(18.0)),
            }],
            padding: 10.0,
            gap: 0.0,
        },
        target: "tray",
    }];
    let (list, _) = build(&bar(&[], &[], &row), &theme(), measure);
    let rect = boxes(&list)[0];
    // 36 px wide for a 2:1 picture 18 tall, plus 10 px of padding each side
    // and 14 of border and padding each side of the box.
    assert!((rect.width - (36.0 + 20.0 + 28.0)).abs() < f32::EPSILON);
    let image = list
        .items
        .iter()
        .find_map(|item| match item {
            Item::Group { items, .. } => items.iter().find_map(|item| match item {
                Item::Image { rect, picture } => Some((*rect, picture.clone())),
                _ => None,
            }),
            _ => None,
        })
        .expect("the picture is drawn");
    assert_eq!(image.1, picture);
    assert_eq!(
        image.0,
        Rect {
            x: rect.x + 14.0 + 10.0,
            y: rect.y + (rect.height - 18.0) / 2.0,
            width: 36.0,
            height: 18.0,
        }
    );
    assert!(texts(&list).is_empty(), "the letter is not drawn too");
}

#[test]
fn buttons_without_padding_are_their_gap_apart() {
    let button = Button {
        text: "W".to_owned(),
        colour: theme().foreground,
        picture: Some(
            Picture::new(1, 1, vec![0; 4])
                .expect("a picture")
                .sized(18.0),
        ),
    };
    let row = [Module {
        content: Content::Buttons {
            buttons: vec![button.clone(), button],
            padding: 0.0,
            gap: 10.0,
        },
        target: "tray",
    }];
    let (list, _) = build(&bar(&[], &[], &row), &theme(), measure);
    let rect = boxes(&list)[0];
    assert!(
        (rect.width - (18.0 + 10.0 + 18.0 + 28.0)).abs() < f32::EPSILON,
        "two icons and one gap, in the box's border and padding"
    );
    let xs: Vec<f32> = list
        .items
        .iter()
        .flat_map(|item| match item {
            Item::Group { items, .. } => items.clone(),
            _ => Vec::new(),
        })
        .filter_map(|item| match item {
            Item::Image { rect, .. } => Some(rect.x),
            _ => None,
        })
        .collect();
    assert_eq!(xs, [rect.x + 14.0, rect.x + 14.0 + 18.0 + 10.0]);
}

#[test]
fn a_picture_needs_its_pixels() {
    assert!(Picture::new(2, 2, vec![0; 16]).is_some());
    assert!(Picture::new(2, 2, vec![0; 15]).is_none());
    assert!(Picture::new(0, 2, Vec::new()).is_none());
}
