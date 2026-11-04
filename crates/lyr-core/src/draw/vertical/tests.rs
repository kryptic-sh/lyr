use lyr_config::{Platform, Theme, from_sources};

use super::super::{
    Bar, Button, Content, DrawList, Entry, Item, Module, Orientation, Picture, Target, build,
};
use super::{cut, wrap};
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
            x: 10.0,
            y: 12.0,
            width: 30.0,
            height: 20.0
        }),
        "the whole row across the box, however narrow its face"
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
fn a_button_wider_than_the_bar_is_cut_as_a_label_is() {
    let left = [Entry::Module(Module {
        content: Content::Buttons {
            buttons: vec![Button {
                text: "mail".to_owned(),
                colour: theme().foreground,
                picture: None,
            }],
            padding: 8.0,
            gap: 4.0,
        },
        target: "workspaces",
    })];
    let (list, regions) = build(&bar(1000.0, &left, &[], &[]), &theme(), measure);
    // 26 px inside the box's border: a letter and the ellipsis, centred,
    // its padding cut to the box.
    assert_eq!(
        texts(&list),
        [(
            "m\u{2026}".to_owned(),
            Rect {
                x: 15.0,
                y: 12.0,
                width: 20.0,
                height: 20.0
            }
        )]
    );
    assert_eq!(
        regions.rect(&Target::Button("workspaces", 0)),
        Some(Rect {
            x: 10.0,
            y: 12.0,
            width: 30.0,
            height: 20.0
        }),
        "no wider than its box"
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

#[test]
fn a_label_too_wide_wraps_at_its_spaces_and_its_box_grows_a_line_each() {
    // 26 px inside the box's border: "ab cd" is 50 px, each word 20.
    let left = [label("ab cd", Some("x")), label("ef", None)];
    let (list, regions) = build(&bar(1000.0, &left, &[], &[]), &theme(), measure);
    let line = |text: &str, y| {
        (
            text.to_owned(),
            Rect {
                x: 15.0,
                width: 20.0,
                ..box_at(y)
            },
        )
    };
    assert_eq!(
        texts(&list),
        [line("ab", 10.0), line("cd", 30.0), line("ef", 66.0)],
        "all of it rather than its compact form, each line centred"
    );
    // Two 20 px lines in the border; the next box starts after it and the
    // margins between.
    assert_eq!(
        boxes(&list),
        [
            Rect {
                height: 44.0,
                ..box_at(10.0)
            },
            box_at(66.0)
        ]
    );
    assert_eq!(
        regions.hit(12.0, 52.0),
        Some(&Target::Module("label")),
        "the whole taller box is the module's"
    );
}

#[test]
fn a_word_too_wide_shows_the_compact_form_else_is_cut_on_its_line() {
    let left = [
        label("ab cdef", Some("x")),
        label("ab cdef", None),
        label("abcd ef", Some("wxyz")),
    ];
    let (list, _) = build(&bar(1000.0, &left, &[], &[]), &theme(), measure);
    let shown: Vec<String> = texts(&list).into_iter().map(|(text, _)| text).collect();
    assert_eq!(shown, ["x", "ab", "c\u{2026}", "w\u{2026}"]);
}

#[test]
fn a_picture_wider_than_the_bar_is_shrunk_to_it() {
    // 40 by 20 px, in the 26 px inside the box's border.
    let picture = Picture::new(2, 1, vec![0; 8]).expect("pixels").sized(20.0);
    let left = [Entry::Module(Module {
        content: Content::Buttons {
            buttons: vec![Button {
                text: "t".to_owned(),
                colour: theme().foreground,
                picture: Some(picture),
            }],
            padding: 0.0,
            gap: 4.0,
        },
        target: "tray",
    })];
    let (list, regions) = build(&bar(1000.0, &left, &[], &[]), &theme(), measure);
    let images: Vec<Rect> = list
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Group { items, .. } => Some(items),
            _ => None,
        })
        .flatten()
        .filter_map(|item| match item {
            Item::Image { rect, .. } => Some(*rect),
            _ => None,
        })
        .collect();
    // Its aspect kept, centred in its 20 px row.
    assert_eq!(
        images,
        [Rect {
            x: 12.0,
            y: 15.5,
            width: 26.0,
            height: 13.0
        }]
    );
    assert_eq!(
        regions.rect(&Target::Button("tray", 0)),
        Some(Rect {
            x: 10.0,
            y: 12.0,
            width: 30.0,
            height: 20.0
        })
    );
}

#[test]
fn a_wrap_fills_each_line_with_the_words_that_fit() {
    assert_eq!(wrap("a b  cdef g", 30.0, measure), ["a b", "cdef", "g"]);
    assert_eq!(wrap("  ", 30.0, measure), [""], "one empty line");
}
