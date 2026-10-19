use std::time::{Duration, Instant};

use lyr_config::{Platform, from_sources};

use super::{Part, Stack};
use crate::draw::Item;
use crate::notifications::{Expiry, Incoming, LOW_TIMEOUT, NORMAL_TIMEOUT, Store, Urgency};
use crate::toast::{BORDER, INSET, MARGIN};

const MS: Duration = Duration::from_millis(1);

fn notify(store: &mut Store, summary: &str, urgency: Urgency, now: Instant) -> u32 {
    store.notify(
        Incoming {
            summary: summary.to_owned(),
            urgency,
            ..Incoming::default()
        },
        now,
    )
}

#[test]
fn toasts_stack_newest_first_and_leave_when_their_time_is_up() {
    let start = Instant::now();
    let mut store = Store::default();
    let mut stack = Stack::default();
    let low = notify(&mut store, "low", Urgency::Low, start);
    stack.notified(&store, low, false, start);
    let normal = notify(&mut store, "normal", Urgency::Normal, start);
    stack.notified(&store, normal, false, start);
    let critical = notify(&mut store, "critical", Urgency::Critical, start);
    stack.notified(&store, critical, false, start);
    assert_eq!(stack.ids().collect::<Vec<_>>(), [critical, normal, low]);
    assert_eq!(stack.next_wake(), Some(start + LOW_TIMEOUT));
    assert!(stack.expire(start + LOW_TIMEOUT - MS).is_empty());
    assert_eq!(stack.expire(start + LOW_TIMEOUT), [low]);
    assert_eq!(stack.expire(start + NORMAL_TIMEOUT), [normal]);
    assert_eq!(stack.next_wake(), None, "a critical one stays");
    assert_eq!(stack.ids().collect::<Vec<_>>(), [critical]);
}

#[test]
fn a_replacement_goes_back_on_top_with_a_new_timeout() {
    let start = Instant::now();
    let mut store = Store::default();
    let mut stack = Stack::default();
    let first = notify(&mut store, "first", Urgency::Normal, start);
    stack.notified(&store, first, false, start);
    let second = notify(&mut store, "second", Urgency::Normal, start);
    stack.notified(&store, second, false, start);
    let later = start + Duration::from_secs(3);
    store.notify(
        Incoming {
            replaces_id: first,
            summary: "first again".to_owned(),
            ..Incoming::default()
        },
        later,
    );
    stack.notified(&store, first, false, later);
    assert_eq!(stack.ids().collect::<Vec<_>>(), [first, second]);
    assert_eq!(stack.expire(start + NORMAL_TIMEOUT), [second]);
    assert_eq!(stack.expire(later + NORMAL_TIMEOUT), [first]);
}

#[test]
fn an_app_s_own_timeout_and_never_are_kept() {
    let start = Instant::now();
    let mut store = Store::default();
    let mut stack = Stack::default();
    let quick = store.notify(
        Incoming {
            summary: "quick".to_owned(),
            expiry: Some(Expiry::After(Duration::from_secs(2))),
            ..Incoming::default()
        },
        start,
    );
    stack.notified(&store, quick, false, start);
    let forever = store.notify(
        Incoming {
            summary: "forever".to_owned(),
            expiry: Some(Expiry::Never),
            ..Incoming::default()
        },
        start,
    );
    stack.notified(&store, forever, false, start);
    assert_eq!(stack.expire(start + Duration::from_secs(2)), [quick]);
    assert_eq!(stack.next_wake(), None);
}

#[test]
fn hovering_pauses_a_toast_and_leaving_resumes_it() {
    let start = Instant::now();
    let mut store = Store::default();
    let mut stack = Stack::default();
    let id = notify(&mut store, "hello", Urgency::Normal, start);
    stack.notified(&store, id, false, start);
    let over = start + Duration::from_secs(4);
    stack.hover(Some(id), over);
    assert_eq!(stack.next_wake(), None, "paused");
    assert!(stack.expire(start + NORMAL_TIMEOUT * 2).is_empty());
    let off = start + Duration::from_secs(30);
    stack.hover(None, off);
    // Six of its ten seconds were left.
    assert_eq!(stack.next_wake(), Some(off + Duration::from_secs(6)));
    // A toast that arrives under the pointer starts paused.
    stack.hover(Some(id), off);
    store.notify(
        Incoming {
            replaces_id: id,
            summary: "again".to_owned(),
            ..Incoming::default()
        },
        off,
    );
    stack.notified(&store, id, false, off);
    assert_eq!(stack.next_wake(), None);
    stack.hover(None, off + MS);
    assert_eq!(stack.next_wake(), Some(off + MS + NORMAL_TIMEOUT));
}

#[test]
fn no_toast_while_the_panel_is_open_or_under_do_not_disturb() {
    let start = Instant::now();
    let mut store = Store::default();
    let mut stack = Stack::default();
    let id = notify(&mut store, "hello", Urgency::Normal, start);
    stack.notified(&store, id, true, start);
    assert_eq!(stack.ids().count(), 0, "the panel lists it");
    store.dnd = true;
    stack.notified(&store, id, false, start);
    assert_eq!(stack.ids().count(), 0);
    let critical = notify(&mut store, "critical", Urgency::Critical, start);
    stack.notified(&store, critical, false, start);
    assert_eq!(stack.ids().collect::<Vec<_>>(), [critical]);
    stack.closed(critical);
    assert_eq!(stack.ids().count(), 0);
}

#[test]
fn the_stack_draws_each_toast_below_the_last() {
    let theme = from_sources(None, Platform::Linux)
        .expect("the defaults")
        .theme;
    let measure = |text: &str| text.chars().count() as f32 * 10.0;
    let start = Instant::now();
    let mut store = Store::default();
    let mut stack = Stack::default();
    let older = notify(&mut store, "older", Urgency::Normal, start);
    stack.notified(&store, older, false, start);
    let newer = notify(&mut store, "newer", Urgency::Normal, start);
    stack.notified(&store, newer, false, start);
    let (list, size, regions) = stack.draw(&store, start, measure, 20.0, &theme);
    let toast = 2.0 * (BORDER + INSET) + 20.0;
    assert_eq!(size.height, 2.0 * toast + MARGIN);
    let tops: Vec<(&str, f32)> = list
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Text { text, rect, .. } if text != "Now" => Some((text.as_str(), rect.y)),
            _ => None,
        })
        .collect();
    assert_eq!(
        tops,
        [
            ("newer", BORDER + INSET),
            ("older", toast + MARGIN + BORDER + INSET)
        ]
    );
    assert_eq!(regions.hit(10.0, 1.0), Some(&Part::Body(newer)));
    assert_eq!(
        regions.hit(10.0, toast + MARGIN + 1.0),
        Some(&Part::Body(older))
    );
    assert_eq!(regions.hit(10.0, toast + 1.0), None, "between them");
    let empty = Stack::default().draw(&store, start, measure, 20.0, &theme);
    assert!(empty.0.items.is_empty() && empty.1.height == 0.0);
}

#[test]
fn a_label_changes_a_whole_minute_after_its_toast_arrived() {
    let start = Instant::now();
    let mut store = Store::default();
    let mut stack = Stack::default();
    assert_eq!(stack.next_label_change(&store, start), None);
    let id = notify(&mut store, "critical", Urgency::Critical, start);
    stack.notified(&store, id, false, start);
    let minute = Duration::from_secs(60);
    assert_eq!(stack.next_label_change(&store, start), Some(start + minute));
    assert_eq!(
        stack.next_label_change(&store, start + minute),
        Some(start + 2 * minute),
        "on the boundary, the next one"
    );
    assert_eq!(
        stack.next_label_change(&store, start + minute * 3 / 2),
        Some(start + 2 * minute)
    );
}

#[test]
fn an_action_button_is_its_own_region_on_top_of_the_toast() {
    let theme = from_sources(None, Platform::Linux)
        .expect("the defaults")
        .theme;
    let measure = |text: &str| text.chars().count() as f32 * 10.0;
    let start = Instant::now();
    let mut store = Store::default();
    let mut stack = Stack::default();
    let id = store.notify(
        Incoming {
            summary: "Meeting".to_owned(),
            actions: vec![("join".to_owned(), "Join".to_owned())],
            ..Incoming::default()
        },
        start,
    );
    stack.notified(&store, id, false, start);
    let (_, _, regions) = stack.draw(&store, start, measure, 20.0, &theme);
    let button_y = BORDER + INSET + 20.0 + 4.0 + 1.0;
    assert_eq!(
        regions.hit(BORDER + INSET + 1.0, button_y),
        Some(&Part::Action(id, 0))
    );
    assert_eq!(regions.hit(1.0, 1.0), Some(&Part::Body(id)));
}
