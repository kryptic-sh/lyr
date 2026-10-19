use lyr_config::{Colour, Platform, Theme, from_sources};

use super::{Bar, Button, Content, DrawList, Item, Module, Target, build};
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
fn a_click_on_a_workspace_button_names_the_button() {
    let active = Colour::parse("#2bc5e3").expect("colour");
    let center = [Module {
        content: Content::Buttons(vec![
            Button {
                text: "1".to_owned(),
                colour: active,
            },
            Button {
                text: "2".to_owned(),
                colour: theme().foreground,
            },
        ]),
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
    let (list, size) = super::tooltip("Power Button", 120.0, 20.0, &theme);
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
            },
        ]
    );
}
