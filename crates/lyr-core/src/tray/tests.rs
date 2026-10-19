use super::{Click, ITEM_PATH, Item, Registry, Status, click, item_address, scroll};
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
