use std::time::{Duration, Instant};

use super::{Expiry, Incoming, LOW_TIMEOUT, NORMAL_TIMEOUT, Store, Urgency};

fn from(app: &str, summary: &str) -> Incoming {
    Incoming {
        app_name: app.to_owned(),
        summary: summary.to_owned(),
        ..Incoming::default()
    }
}

fn critical(app: &str, summary: &str) -> Incoming {
    Incoming {
        urgency: Urgency::Critical,
        ..from(app, summary)
    }
}

#[test]
fn ids_start_at_1_and_are_not_reused_while_held() {
    let mut store = Store::default();
    assert_eq!(store.notify(from("a", "1"), Instant::now()), 1);
    assert_eq!(store.notify(from("a", "2"), Instant::now()), 2);
    store.close(1);
    assert_eq!(
        store.notify(from("a", "3"), Instant::now()),
        3,
        "not 1 again straight away"
    );
}

#[test]
fn ids_wrap_past_the_top_and_skip_0_and_held_ids() {
    let mut store = Store::default();
    store.notify(from("a", "first"), Instant::now());
    store.last_id = u32::MAX;
    assert_eq!(
        store.notify(from("a", "wrapped"), Instant::now()),
        2,
        "1 is held, 0 is never used"
    );
}

#[test]
fn a_replacement_keeps_the_id_and_takes_the_place() {
    let mut store = Store::default();
    let id = store.notify(from("music", "Song A"), Instant::now());
    store.notify(from("mail", "New mail"), Instant::now());
    let replaced = store.notify(
        Incoming {
            replaces_id: id,
            ..from("music", "Song B")
        },
        Instant::now(),
    );
    assert_eq!(replaced, id);
    assert_eq!(
        store.get(id).map(|n| n.content.summary.as_str()),
        Some("Song B")
    );
    assert_eq!(store.history().len(), 2, "still two notifications");
}

#[test]
fn a_notification_with_a_held_one_s_synchronous_value_replaces_it() {
    let mut store = Store::default();
    let volume = |summary: &str, key: &str| Incoming {
        synchronous: Some(key.to_owned()),
        ..from("osd", summary)
    };
    let id = store.notify(volume("Volume 40%", "volume"), Instant::now());
    let other = store.notify(volume("Brightness 70%", "brightness"), Instant::now());
    assert_ne!(other, id, "another value is another notification");
    assert_eq!(
        store.notify(volume("Volume 45%", "volume"), Instant::now()),
        id
    );
    assert_eq!(
        store.get(id).map(|n| n.content.summary.as_str()),
        Some("Volume 45%")
    );
    assert_eq!(
        store.get(other).map(|n| n.content.summary.as_str()),
        Some("Brightness 70%"),
        "the other kept"
    );
    // An empty value is none, and replaces_id still wins.
    let empty = store.notify(volume("a", ""), Instant::now());
    assert_ne!(store.notify(volume("b", ""), Instant::now()), empty);
    assert_eq!(
        store.notify(
            Incoming {
                replaces_id: other,
                ..volume("Volume 50%", "volume")
            },
            Instant::now()
        ),
        other
    );
}

#[test]
fn replacing_an_unknown_id_uses_that_id_as_the_spec_says() {
    let mut store = Store::default();
    assert_eq!(
        store.notify(
            Incoming {
                replaces_id: 42,
                ..from("a", "x")
            },
            Instant::now()
        ),
        42
    );
}

#[test]
fn closing_an_unknown_id_says_so() {
    let mut store = Store::default();
    let id = store.notify(from("a", "x"), Instant::now());
    assert!(store.close(id).is_some());
    assert!(store.close(id).is_none());
}

#[test]
fn timeouts_follow_the_app_then_swaync_by_urgency() {
    let mut store = Store::default();
    let timeout = |store: &mut Store, incoming| {
        let id = store.notify(incoming, Instant::now());
        Store::timeout(store.get(id).expect("held"))
    };
    assert_eq!(timeout(&mut store, from("a", "x")), Some(NORMAL_TIMEOUT));
    let low = Incoming {
        urgency: Urgency::Low,
        ..from("a", "x")
    };
    assert_eq!(timeout(&mut store, low), Some(LOW_TIMEOUT));
    assert_eq!(timeout(&mut store, critical("a", "x")), None);
    let own = Incoming {
        expiry: Some(Expiry::from_millis(1500)),
        ..critical("a", "x")
    };
    assert_eq!(timeout(&mut store, own), Some(Duration::from_millis(1500)));
    let never = Incoming {
        expiry: Some(Expiry::from_millis(0)),
        ..from("a", "x")
    };
    assert_eq!(timeout(&mut store, never), None);
    assert_eq!(Expiry::from_millis(-1), Expiry::Default);
}

#[test]
fn do_not_disturb_hides_toasts_but_not_critical_ones() {
    let mut store = Store::default();
    let normal = store.notify(from("a", "x"), Instant::now());
    let urgent = store.notify(critical("a", "y"), Instant::now());
    store.dnd = true;
    let shows = |store: &Store, id, panel| store.shows_toast(store.get(id).expect("held"), panel);
    assert!(!shows(&store, normal, false));
    assert!(shows(&store, urgent, false));
    assert!(
        !shows(&store, urgent, true),
        "never while the panel is open"
    );
    store.dnd = false;
    assert!(shows(&store, normal, false));
}

#[test]
fn the_history_puts_critical_groups_first_then_the_newest() {
    let mut store = Store::default();
    // Chat arrives first, so only the newest-first rule puts mail ahead of it.
    store.notify(from("chat", "hi"), Instant::now());
    store.notify(from("mail", "old mail"), Instant::now());
    store.notify(critical("battery", "low"), Instant::now());
    store.notify(from("mail", "new mail"), Instant::now());
    store.notify(
        Incoming {
            transient: true,
            ..from("volume", "50%")
        },
        Instant::now(),
    );
    let history = store.history();
    let groups: Vec<&str> = history.iter().map(|(name, _)| *name).collect();
    assert_eq!(groups, ["battery", "mail", "chat"], "transient left out");
    let mail: Vec<&str> = history[1]
        .1
        .iter()
        .map(|n| n.content.summary.as_str())
        .collect();
    assert_eq!(mail, ["new mail", "old mail"]);
}

#[test]
fn the_desktop_entry_groups_ahead_of_the_app_name() {
    let mut store = Store::default();
    store.notify(
        Incoming {
            desktop_entry: Some("firefox".to_owned()),
            ..from("Firefox Nightly", "a")
        },
        Instant::now(),
    );
    store.notify(
        Incoming {
            desktop_entry: Some("firefox".to_owned()),
            ..from("Firefox", "b")
        },
        Instant::now(),
    );
    assert_eq!(store.history().len(), 1);
}

#[test]
fn clear_all_closes_everything() {
    let mut store = Store::default();
    store.notify(from("a", "x"), Instant::now());
    store.notify(from("b", "y"), Instant::now());
    assert_eq!(store.clear(), [1, 2]);
    assert!(store.history().is_empty());
}
