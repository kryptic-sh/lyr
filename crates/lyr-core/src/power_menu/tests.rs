use lyr_config::{Platform, Theme, from_sources};

use super::{Entry, Menu, PADDING, entries};
use crate::draw::Item;

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

#[test]
fn the_entries_are_menu_power_s_in_its_order() {
    let labels =
        |platform| -> Vec<&str> { entries(platform).into_iter().map(Entry::label).collect() };
    assert_eq!(
        labels(Platform::Linux),
        ["Lock", "Sleep", "Hibernate", "Shutdown", "Logout", "Reboot"]
    );
    assert_eq!(labels(Platform::Windows), labels(Platform::Linux));
    assert_eq!(
        labels(Platform::Macos),
        ["Lock", "Sleep", "Shutdown", "Logout", "Reboot"]
    );
}

#[test]
fn the_selection_moves_and_stops_at_the_ends() {
    let mut menu = Menu::new(entries(Platform::Linux)).expect("a menu");
    assert_eq!(menu.selected(), Entry::Lock);
    menu.up();
    assert_eq!(menu.selected(), Entry::Lock, "already at the top");
    menu.down();
    menu.down();
    assert_eq!(menu.selected(), Entry::Hibernate);
    for _ in 0..10 {
        menu.down();
    }
    assert_eq!(menu.selected(), Entry::Reboot, "stops at the bottom");
    assert!(menu.select(Entry::Sleep));
    assert!(!menu.select(Entry::Sleep), "already selected");
    assert!(menu.select(Entry::Hibernate));
    assert_eq!(menu.selected(), Entry::Hibernate);
    assert_eq!(Menu::new(Vec::new()), None);
}

#[test]
fn an_entry_the_menu_lacks_cannot_be_selected() {
    let mut menu = Menu::new(entries(Platform::Macos)).expect("a menu");
    assert!(!menu.select(Entry::Hibernate));
    assert_eq!(menu.selected(), Entry::Lock);
}

#[test]
fn rows_stack_inside_the_padding_with_the_selected_one_coloured() {
    let theme = theme();
    let mut menu = Menu::new(vec![Entry::Lock, Entry::Hibernate]).expect("a menu");
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
