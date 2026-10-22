use super::{Click, ITEM_PATH, Item, Registry, Status, click, faces, item_address, scroll};
use crate::pointer::Press;

#[test]
fn a_status_reads_its_three_values() {
    assert_eq!(Status::parse("Passive"), Status::Passive);
    assert_eq!(Status::parse("Active"), Status::Active);
    assert_eq!(Status::parse("NeedsAttention"), Status::NeedsAttention);
    assert_eq!(Status::parse("whatever"), Status::Active);
    let item = |status| Item {
        status,
        ..Item::default()
    };
    assert!(!item(Status::Passive).shown());
    assert!(item(Status::Active).shown() && item(Status::NeedsAttention).shown());
}

#[test]
fn an_item_stands_in_with_its_first_letter() {
    let item = |title: &str, id: &str| Item {
        title: title.to_owned(),
        id: id.to_owned(),
        ..Item::default()
    };
    assert_eq!(item("steam", "x").letter(), "S");
    assert_eq!(item("", "teams").letter(), "T", "the id without a title");
    assert_eq!(
        item("  -- ", "1password").letter(),
        "1",
        "no letter in the title"
    );
    assert_eq!(item("", "").letter(), "?");
}

#[test]
fn the_faces_are_the_shown_items_letters_and_statuses() {
    let item = |title: &str, status| Item {
        title: title.to_owned(),
        status,
        ..Item::default()
    };
    let items = [
        item("spotify", Status::Active),
        item("hidden", Status::Passive),
        item("discord", Status::NeedsAttention),
    ];
    assert_eq!(
        faces(&items),
        [
            ("S".to_owned(), Status::Active),
            ("D".to_owned(), Status::NeedsAttention)
        ]
    );
    // A new tooltip or icon leaves them alone.
    let mut quiet = items.clone();
    quiet[0].tooltip = "now playing".to_owned();
    quiet[0].icon_name = "spotify-playing".to_owned();
    assert_eq!(faces(&quiet), faces(&items));
}

#[test]
fn an_item_s_tooltip_falls_back_to_its_title() {
    let item = |tooltip: &str, title: &str| Item {
        tooltip: tooltip.to_owned(),
        title: title.to_owned(),
        ..Item::default()
    };
    assert_eq!(
        item("Steam - 2 updates", "Steam").tooltip_text(),
        Some("Steam - 2 updates")
    );
    assert_eq!(item("", "Steam").tooltip_text(), Some("Steam"));
    assert_eq!(item("", "").tooltip_text(), None);
}

#[test]
fn clicks_follow_waybar() {
    assert_eq!(click(Press::Left, false), Some(Click::Activate));
    assert_eq!(click(Press::Left, true), Some(Click::Menu));
    assert_eq!(click(Press::Right, false), Some(Click::Menu));
    assert_eq!(click(Press::Middle, true), Some(Click::SecondaryActivate));
    assert_eq!(scroll(true), (-1, "vertical"));
    assert_eq!(scroll(false), (1, "vertical"));
}

#[test]
fn an_item_is_found_from_how_it_registered() {
    let at = |service| item_address(service, ":1.42");
    assert_eq!(
        at("/org/ayatana/NotificationItem/steam"),
        (
            ":1.42".to_owned(),
            "/org/ayatana/NotificationItem/steam".to_owned()
        )
    );
    assert_eq!(
        at(":1.7/StatusNotifierItem/app"),
        (":1.7".to_owned(), "/StatusNotifierItem/app".to_owned())
    );
    assert_eq!(
        at("org.kde.StatusNotifierItem-12-1"),
        (
            "org.kde.StatusNotifierItem-12-1".to_owned(),
            ITEM_PATH.to_owned()
        )
    );
}

#[test]
fn the_registry_keeps_items_once_and_drops_a_vanished_name() {
    let mut registry = Registry::default();
    assert!(registry.register_item(":1.1", "/a"));
    assert!(!registry.register_item(":1.1", "/a"), "again");
    assert!(registry.register_item(":1.1", "/b"));
    assert!(registry.register_item(":1.2", "/a"));
    assert_eq!(registry.items(), [":1.1/a", ":1.1/b", ":1.2/a"]);

    assert!(!registry.has_host());
    assert!(registry.register_host(":1.9"), "the first host");
    assert!(!registry.register_host(":1.8"));
    assert!(!registry.register_host(":1.9"), "again: not a new first");
    assert!(registry.has_host());

    assert_eq!(
        registry.vanished(":1.1"),
        (vec![":1.1/a".to_owned(), ":1.1/b".to_owned()], false)
    );
    assert_eq!(registry.items(), [":1.2/a"]);
    assert_eq!(registry.vanished(":1.9"), (vec![], false), "a host is left");
    assert_eq!(registry.vanished(":1.8"), (vec![], true), "the last host");
    assert_eq!(registry.vanished(":1.8"), (vec![], false), "none to lose");
    assert!(!registry.has_host());
}

#[test]
fn the_only_host_registering_again_is_not_new() {
    let mut registry = Registry::default();
    assert!(registry.register_host(":1.5"));
    assert!(!registry.register_host(":1.5"));
}
