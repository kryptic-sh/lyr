use lyr_config::Platform;

use super::{Entry, entries, rows};

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
fn each_entry_is_a_row_by_its_label() {
    let rows = rows(&[Entry::Lock, Entry::Reboot]);
    let shown: Vec<(&str, Option<Entry>, bool)> = rows
        .iter()
        .map(|row| (row.label.as_str(), row.value, row.separator))
        .collect();
    assert_eq!(
        shown,
        [
            ("Lock", Some(Entry::Lock), false),
            ("Reboot", Some(Entry::Reboot), false)
        ]
    );
}
