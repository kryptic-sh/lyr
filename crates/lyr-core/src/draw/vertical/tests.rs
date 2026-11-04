use lyr_config::{Platform, Theme, from_sources};

use super::super::{
    Bar, Button, Content, DrawList, Entry, Item, Module, Orientation, Target, build,
};
use super::cut;
use crate::hit::Rect;

/// Every character is 10 px wide.
fn measure(text: &str) -> f32 {
    text.chars().count() as f32 * 10.0
}

const LINE_HEIGHT: f32 = 20.0;

fn theme() -> Theme {
    from_sources(None, Platform::Linux).expect("defaults").theme
}

fn label(text: &str, compact: Option<&str>) -> Entry<&'static str> {
    Entry::Module(Module {
        content: Content::Label {
            text: text.to_owned(),
            compact: compact.map(str::to_owned),
            colour: theme().foreground,
        },
        target: "label",
    })
}

/// A side bar 50 px wide and `height` long.
fn bar<'a>(
    height: f32,
    left: &'a [Entry<&'static str>],
    center: &'a [Entry<&'static str>],
    right: &'a [Entry<&'static str>],
) -> Bar<'a, &'static str> {
    Bar {
        width: 50.0,
        height,
        orientation: Orientation::Vertical {
            line_height: LINE_HEIGHT,
        },
        left,
        center,
        right,
    }
}

/// The boxes, separators included, in drawing order.
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

/// A box `y` down the default side bar: across it less the 10 px margins
/// each side, and a 20 px line inside a 2 px border tall.
const fn box_at(y: f32) -> Rect {
    Rect {
        x: 10.0,
        y,
        width: 30.0,
        height: 24.0,
    }
}

#[test]
fn zones_run_top_to_bottom_with_the_centre_centred() {
    let (list, _) = build(
        &bar(
            1000.0,
            &[label("a", None)],
            &[label("b", None)],
            &[label("c", None)],
        ),
        &theme(),
        measure,
    );
    // Each slot is 36 px long: a 10 px top margin, the box, a 2 px bottom
    // one. The centre's starts at (1000 - 36) / 2 and the bottom zone's
    // ends at the bar's end.
    assert_eq!(boxes(&list), [box_at(10.0), box_at(492.0), box_at(974.0)]);
}

#[test]
fn text_stays_level_and_centred_across_its_box() {
    let (list, _) = build(
        &bar(1000.0, &[label("ab", None)], &[], &[]),
        &theme(),
        measure,
    );
    let (text, rect) = &texts(&list)[0];
    assert_eq!(text, "ab");
    assert_eq!(
        *rect,
        Rect {
            x: 15.0,
            width: 20.0,
            ..box_at(10.0)
        }
    );
}

#[test]
fn a_label_too_wide_shows_its_compact_form_then_is_cut() {
    // 26 px inside the box's border.
    let left = [
        label("abcd", Some("x")),
        label("abcd", None),
        label("abcd", Some("xyz")),
        label("ab", Some("x")),
    ];
    let (list, _) = build(&bar(1000.0, &left, &[], &[]), &theme(), measure);
    let shown: Vec<String> = texts(&list).into_iter().map(|(text, _)| text).collect();
    assert_eq!(shown, ["x", "a\u{2026}", "x\u{2026}", "ab"]);
}

#[test]
fn where_zones_meet_the_modules_nearest_the_centre_are_dropped() {
    // The centre starts at (200 - 36) / 2 = 82: room for two 36 px slots.
    let left = [label("a", None), label("b", None), label("c", Some("c"))];
    let (list, _) = build(
        &bar(200.0, &left, &[label("m", None)], &[]),
        &theme(),
        measure,
    );
    let shown: Vec<String> = texts(&list).into_iter().map(|(text, _)| text).collect();
    assert_eq!(shown, ["a", "b", "m"], "a compact form saves no length");
}

#[test]
fn buttons_stack_one_a_row_centred_across_the_bar() {
    let button = |text: &str| Button {
        text: text.to_owned(),
        colour: theme().foreground,
        picture: None,
    };
    let left = [Entry::Module(Module {
        content: Content::Buttons {
            buttons: vec![button("1"), button("22")],
            padding: 5.0,
            gap: 4.0,
        },
        target: "workspaces",
    })];
    let (list, regions) = build(&bar(1000.0, &left, &[], &[]), &theme(), measure);
    // Two 20 px rows 4 px apart inside the 2 px border.
    assert_eq!(
        boxes(&list),
        [Rect {
            height: 48.0,
            ..box_at(10.0)
        }]
    );
    assert_eq!(
        regions.rect(&Target::Button("workspaces", 0)),
        Some(Rect {
            x: 15.0,
            y: 12.0,
            width: 20.0,
            height: 20.0
        })
    );
    assert_eq!(
        regions.rect(&Target::Button("workspaces", 1)),
        Some(Rect {
            x: 10.0,
            y: 36.0,
            width: 30.0,
            height: 20.0
        })
    );
}

#[test]
fn a_separator_lies_across_the_bar_and_a_spacer_is_a_vertical_gap() {
    let left = [
        Entry::Spacer(50.0),
        label("a", None),
        Entry::Separator,
        label("b", None),
    ];
    let (list, _) = build(&bar(1000.0, &left, &[], &[]), &theme(), measure);
    // The separator's slot is its 2 px rule between the module margins.
    assert_eq!(
        boxes(&list),
        [
            box_at(60.0),
            Rect {
                height: 2.0,
                ..box_at(96.0)
            },
            box_at(110.0),
        ]
    );
}

#[test]
fn a_cut_keeps_what_fits_before_its_ellipsis() {
    assert_eq!(cut("hello", 50.0, measure), "hello", "fits whole");
    assert_eq!(cut("hello", 49.0, measure), "hel\u{2026}");
    assert_eq!(cut("hello", 10.0, measure), "\u{2026}");
    assert_eq!(cut("hello", 9.0, measure), "", "not even the ellipsis");
}
