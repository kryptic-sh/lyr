use lyr_config::{Platform, Theme, from_sources};

use super::{DISABLED_OPACITY, Menu, PADDING, Row};
use crate::draw::Item;
use crate::power_menu::{Entry, entries, rows};

fn theme() -> Theme {
    from_sources(None, Platform::Linux)
        .expect("the defaults")
        .theme
}

/// Every character is 10 px wide.
fn measure(text: &str) -> f32 {
    text.chars().count() as f32 * 10.0
}

fn texts(items: &[Item]) -> Vec<(&str, f32)> {
    items
        .iter()
        .filter_map(|item| match item {
            Item::Text { text, rect, .. } => Some((text.as_str(), rect.y)),
            _ => None,
        })
        .collect()
}

fn menu(entries: &[Entry]) -> Menu<Entry> {
    Menu::new(rows(entries)).expect("a menu")
}

#[test]
fn the_selection_moves_and_stops_at_the_ends() {
    let mut menu = menu(&entries(Platform::Linux));
    assert_eq!(menu.selected(), Some(&Entry::Lock));
    menu.up();
    assert_eq!(menu.selected(), Some(&Entry::Lock), "already at the top");
    menu.down();
    menu.down();
    assert_eq!(menu.selected(), Some(&Entry::Hibernate));
    for _ in 0..10 {
        menu.down();
    }
    assert_eq!(menu.selected(), Some(&Entry::Reboot), "stops at the bottom");
    assert!(menu.select(&Entry::Sleep));
    assert!(!menu.select(&Entry::Sleep), "already selected");
    assert!(menu.select(&Entry::Hibernate));
    assert_eq!(menu.selected(), Some(&Entry::Hibernate));
    assert_eq!(Menu::<Entry>::new(Vec::new()), None);
}

#[test]
fn an_entry_the_menu_lacks_cannot_be_selected() {
    let mut menu = menu(&entries(Platform::Macos));
    assert!(!menu.select(&Entry::Hibernate));
    assert_eq!(menu.selected(), Some(&Entry::Lock));
}

#[test]
fn rows_stack_inside_the_padding_with_the_selected_one_coloured() {
    let theme = theme();
    let mut menu = menu(&[Entry::Lock, Entry::Hibernate]);
    menu.down();
    let (list, size, regions) = menu.draw(measure, 20.0, &theme);
    // The widest label, "Hibernate", is 90 px; rows are 12 px either side
    // and 6 px above and below; the menu 8 px around them.
    assert_eq!((size.width, size.height), (130.0, 80.0));
    assert_eq!(texts(&list.items), [("Lock", 14.0), ("Hibernate", 46.0)]);
    let colours: Vec<_> = list
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Text { colour, .. } => Some(*colour),
            _ => None,
        })
        .collect();
    assert_eq!(colours, [theme.foreground, theme.workspace_active]);
    assert!(matches!(
        list.items[0],
        Item::Box { fill, border: Some((width, colour)), .. }
            if fill == theme.module_background
                && colour == theme.border
                && width == theme.border_width as f32
    ));
    assert_eq!(
        regions.hit(PADDING + 1.0, PADDING + 1.0),
        Some(&Entry::Lock)
    );
    assert_eq!(regions.hit(PADDING + 1.0, 45.0), Some(&Entry::Hibernate));
    assert_eq!(regions.hit(1.0, 1.0), None, "the padding is no row");
}

/// "Open", a separator, a disabled "Quit", then "More".
fn mixed() -> Vec<Row<u8>> {
    vec![
        Row::new("Open", 1),
        Row {
            label: String::new(),
            value: None,
            separator: true,
        },
        Row {
            label: "Quit".to_owned(),
            value: None,
            separator: false,
        },
        Row::new("More", 4),
    ]
}

#[test]
fn the_selection_steps_over_separators_and_disabled_rows() {
    let mut menu = Menu::new(mixed()).expect("a menu");
    assert_eq!(menu.selected(), Some(&1));
    menu.down();
    assert_eq!(menu.selected(), Some(&4), "past the separator and Quit");
    menu.down();
    assert_eq!(menu.selected(), Some(&4));
    menu.up();
    assert_eq!(menu.selected(), Some(&1));
    let disabled = Menu::new(vec![Row::<u8> {
        label: "Quit".to_owned(),
        value: None,
        separator: false,
    }])
    .expect("a menu");
    assert_eq!(disabled.selected(), None, "nothing to choose");
    let mut disabled = disabled;
    disabled.down();
    disabled.up();
    assert_eq!(disabled.selected(), None);
}

#[test]
fn a_separator_is_a_line_and_a_disabled_row_is_faint_and_inert() {
    let theme = theme();
    let menu = Menu::new(mixed()).expect("a menu");
    let (list, size, regions) = menu.draw(measure, 20.0, &theme);
    // Three rows of 32 px and a separator of 12 px, inside 8 px.
    assert_eq!(size.height, 3.0 * 32.0 + 12.0 + 2.0 * PADDING);
    let line = list.items.iter().find_map(|item| match item {
        Item::Box { rect, fill, .. } if *fill == theme.border => Some(*rect),
        _ => None,
    });
    let line = line.expect("a line");
    let border = theme.border_width as f32;
    assert_eq!(line.y, PADDING + 32.0 + (12.0 - border) / 2.0);
    assert_eq!(line.height, border);
    assert!(list.items.iter().any(|item| matches!(
        item,
        Item::Group { opacity, items }
            if *opacity == DISABLED_OPACITY
                && matches!(&items[..], [Item::Text { text, .. }] if text == "Quit")
    )));
    let targets: Vec<u8> = regions.targets().copied().collect();
    assert_eq!(targets, [1, 4], "only the rows that can be chosen");
}

#[test]
fn a_menu_opens_on_its_first_row_that_can_be_chosen() {
    let mut rows = mixed();
    rows.remove(0);
    let menu = Menu::new(rows).expect("a menu");
    assert_eq!(menu.selected(), Some(&4), "past the separator and Quit");
}
