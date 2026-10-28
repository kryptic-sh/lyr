use std::time::{Duration, Instant};

use lyr_config::{Platform, from_sources};

use super::{ANIMATION, Part, Stack};
use crate::draw::Item;
use crate::notifications::{Expiry, Incoming, LOW_TIMEOUT, NORMAL_TIMEOUT, Store, Urgency};
use crate::toast::tests::lay_out;
use crate::toast::{BORDER, CLOSE_INSET, CLOSE_SIZE, INSET, MARGIN, WIDTH};

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
    stack.closed(critical, start);
    assert_eq!(stack.ids().count(), 0);
}

#[test]
fn the_stack_draws_each_toast_below_the_last() {
    let theme = from_sources(None, Platform::Linux)
        .expect("the defaults")
        .theme;
    let measure = |text: &str, _| text.chars().count() as f32 * 10.0;
    let start = Instant::now();
    let mut store = Store::default();
    let mut stack = Stack::default();
    let older = notify(&mut store, "older", Urgency::Normal, start);
    stack.notified(&store, older, false, start);
    let newer = notify(&mut store, "newer", Urgency::Normal, start);
    stack.notified(&store, newer, false, start);
    stack.settle(start + ANIMATION);
    let (list, size, regions) = stack.draw(start + ANIMATION, measure, &lay_out, 20.0, &theme);
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
    let empty = Stack::default().draw(start, measure, &lay_out, 20.0, &theme);
    assert!(empty.0.items.is_empty() && empty.1.height == 0.0);
}

#[test]
fn a_label_changes_a_whole_minute_after_its_toast_arrived() {
    let start = Instant::now();
    let mut store = Store::default();
    let mut stack = Stack::default();
    assert_eq!(stack.next_label_change(start), None);
    let id = notify(&mut store, "critical", Urgency::Critical, start);
    stack.notified(&store, id, false, start);
    let minute = Duration::from_secs(60);
    assert_eq!(stack.next_label_change(start), Some(start + minute));
    assert_eq!(
        stack.next_label_change(start + minute),
        Some(start + 2 * minute),
        "on the boundary, the next one"
    );
    assert_eq!(
        stack.next_label_change(start + minute * 3 / 2),
        Some(start + 2 * minute)
    );
}

#[test]
fn an_action_button_is_its_own_region_on_top_of_the_toast() {
    let theme = from_sources(None, Platform::Linux)
        .expect("the defaults")
        .theme;
    let measure = |text: &str, _| text.chars().count() as f32 * 10.0;
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
    stack.settle(start + ANIMATION);
    let (_, _, regions) = stack.draw(start + ANIMATION, measure, &lay_out, 20.0, &theme);
    let button_y = BORDER + INSET + 20.0 + 4.0 + 1.0;
    assert_eq!(
        regions.hit(BORDER + INSET + 1.0, button_y),
        Some(&Part::Action(id, 0))
    );
    assert_eq!(regions.hit(1.0, 1.0), Some(&Part::Body(id)));
}

#[test]
fn the_toast_under_the_pointer_shows_its_close_button() {
    let theme = from_sources(None, Platform::Linux)
        .expect("the defaults")
        .theme;
    let measure = |text: &str, _| text.chars().count() as f32 * 10.0;
    let start = Instant::now();
    let mut store = Store::default();
    let mut stack = Stack::default();
    let first = notify(&mut store, "first", Urgency::Normal, start);
    stack.notified(&store, first, false, start);
    let second = notify(&mut store, "second", Urgency::Normal, start);
    stack.notified(&store, second, false, start);
    stack.settle(start + ANIMATION);
    let (plain, _, regions) = stack.draw(start + ANIMATION, measure, &lay_out, 20.0, &theme);
    assert!(!regions.targets().any(|part| matches!(part, Part::Close(_))));
    assert!(stack.hover(Some(first), start), "moved onto a toast");
    assert!(!stack.hover(Some(first), start), "still on it");
    stack.settle(start + ANIMATION);
    let (list, size, regions) = stack.draw(start + ANIMATION, measure, &lay_out, 20.0, &theme);
    // The first is now the lower toast.
    let top = (size.height + MARGIN) / 2.0;
    let x = WIDTH - CLOSE_INSET - CLOSE_SIZE + 1.0;
    assert_eq!(
        regions.hit(x, top + CLOSE_INSET + 1.0),
        Some(&Part::Close(first))
    );
    assert_eq!(
        regions.hit(x - 2.0, top + CLOSE_INSET + 1.0),
        Some(&Part::Body(first))
    );
    assert_eq!(regions.hit(x, CLOSE_INSET + 1.0), Some(&Part::Body(second)));
    assert_eq!(
        list.items.len(),
        plain.items.len() + 2,
        "a box and its cross"
    );
    assert!(list.items.iter().any(|item| matches!(
        item,
        Item::Box { rect, fill, .. } if *fill == theme.toast_close && rect.y == top + CLOSE_INSET
    )));
    assert_eq!(Part::Close(first).id(), first);
    assert_eq!(
        (CLOSE_INSET, CLOSE_SIZE),
        (6.0, 20.0),
        "swaync's 24 px margin outside the toast's 18, and a 16 px icon padded 2"
    );
}

#[test]
fn a_link_is_its_own_region_on_top_of_the_toast() {
    let theme = from_sources(None, Platform::Linux)
        .expect("the defaults")
        .theme;
    let measure = |text: &str, _| text.chars().count() as f32 * 10.0;
    let start = Instant::now();
    let mut store = Store::default();
    let mut stack = Stack::default();
    let id = store.notify(
        Incoming {
            summary: "x".to_owned(),
            body: "go <a href=\"https://x\">there</a>".to_owned(),
            ..Incoming::default()
        },
        start,
    );
    stack.notified(&store, id, false, start);
    stack.settle(start + ANIMATION);
    let (_, _, regions) = stack.draw(start + ANIMATION, measure, &lay_out, 20.0, &theme);
    let body_y = BORDER + INSET + 20.0 + 1.0;
    // "go " is 30 px; the link starts after it.
    assert_eq!(
        regions.hit(BORDER + INSET + 31.0, body_y),
        Some(&Part::Link(id, 0))
    );
    assert_eq!(
        regions.hit(BORDER + INSET + 1.0, body_y),
        Some(&Part::Body(id))
    );
}

/// The clips a moving toast is drawn in, with each one's opacity and how
/// far right its first item sits.
fn moving(list: &crate::draw::DrawList) -> Vec<(crate::hit::Rect, f32, f32)> {
    list.items
        .iter()
        .filter_map(|item| match item {
            Item::Clip { rect, items } => match &items[..] {
                [Item::Group { opacity, items }] => match items.first() {
                    Some(Item::Box { rect: first, .. }) => Some((*rect, *opacity, first.x)),
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        })
        .collect()
}

#[test]
fn a_toast_grows_in_from_the_right_and_shrinks_away() {
    let theme = from_sources(None, Platform::Linux)
        .expect("the defaults")
        .theme;
    let measure = |text: &str, _| text.chars().count() as f32 * 10.0;
    let start = Instant::now();
    let mut store = Store::default();
    let mut stack = Stack::default();
    let id = notify(&mut store, "hello", Urgency::Normal, start);
    stack.notified(&store, id, false, start);
    let toast = 2.0 * (BORDER + INSET) + 20.0;
    let draw = |stack: &Stack, at| stack.draw(at, measure, &lay_out, 20.0, &theme);

    assert!(stack.moving());
    let (list, size, regions) = draw(&stack, start);
    assert_eq!(size.height, toast, "the window takes its full size at once");
    assert_eq!(moving(&list)[0].0.height, 0.0, "no room yet");
    assert_eq!(regions.hit(10.0, 1.0), None, "nothing drawn to click yet");

    // Halfway, eased out quintically: 1 - 0.5^5 of the way.
    let half = 1.0 - 0.5_f32.powi(5);
    let (list, _, _) = draw(&stack, start + ANIMATION / 2);
    let [(clip, opacity, x)] = moving(&list)[..] else {
        panic!("one moving toast: {:?}", list.items);
    };
    assert!((clip.height - toast * half).abs() < 0.01, "{clip:?}");
    assert!((opacity - half).abs() < 0.001);
    assert!(
        (x - WIDTH * (1.0 - half)).abs() < 0.01,
        "sliding in from the right"
    );
    let (_, _, regions) = draw(&stack, start + ANIMATION / 2);
    assert_eq!(
        regions.hit(x + 1.0, 1.0),
        Some(&Part::Body(id)),
        "clickable where it is drawn"
    );
    assert_eq!(
        regions.hit(x - 1.0, 1.0),
        None,
        "not where it has yet to slide"
    );

    assert!(!stack.settle(start + ANIMATION - MS), "not yet");
    assert!(stack.settle(start + ANIMATION), "it arrived");
    assert!(!stack.moving());
    let (list, _, regions) = draw(&stack, start + ANIMATION);
    assert!(moving(&list).is_empty(), "drawn as it is");
    assert_eq!(regions.hit(10.0, 1.0), Some(&Part::Body(id)));

    // It closes: gone from the ids at once, drawn while it shrinks away.
    let left = start + ANIMATION * 2;
    store.close(id);
    stack.closed(id, left);
    assert_eq!(stack.ids().count(), 0);
    let (list, size, regions) = draw(&stack, left + ANIMATION / 2);
    let [(clip, opacity, x)] = moving(&list)[..] else {
        panic!("one leaving toast: {:?}", list.items);
    };
    assert_eq!(
        regions.hit(10.0, 1.0),
        None,
        "a leaving toast takes no clicks"
    );
    assert!(
        (clip.height - toast * (1.0 - half)).abs() < 0.01,
        "{clip:?}"
    );
    assert!((opacity - (1.0 - half)).abs() < 0.001);
    assert_eq!(x, 0.0, "fading where it was");
    assert_eq!(size.height, toast, "the window keeps its size meanwhile");
    assert!(stack.settle(left + ANIMATION), "it left");
    let (list, size, _) = draw(&stack, left + ANIMATION);
    assert!(list.items.is_empty() && size.height == 0.0);
}

#[test]
fn a_replaced_toast_does_not_come_up_again() {
    let start = Instant::now();
    let mut store = Store::default();
    let mut stack = Stack::default();
    let id = notify(&mut store, "40%", Urgency::Normal, start);
    stack.notified(&store, id, false, start);
    stack.settle(start + ANIMATION);
    store.notify(
        Incoming {
            replaces_id: id,
            summary: "45%".to_owned(),
            ..Incoming::default()
        },
        start + ANIMATION,
    );
    stack.notified(&store, id, false, start + ANIMATION);
    assert!(!stack.moving(), "replaced in place");
}

#[test]
fn an_arriving_toast_pushes_the_others_down_as_it_grows() {
    let theme = from_sources(None, Platform::Linux)
        .expect("the defaults")
        .theme;
    let measure = |text: &str, _| text.chars().count() as f32 * 10.0;
    let start = Instant::now();
    let mut store = Store::default();
    let mut stack = Stack::default();
    let older = notify(&mut store, "older", Urgency::Normal, start);
    stack.notified(&store, older, false, start);
    stack.settle(start + ANIMATION);
    let later = start + ANIMATION;
    let newer = notify(&mut store, "newer", Urgency::Normal, later);
    stack.notified(&store, newer, false, later);
    let toast = 2.0 * (BORDER + INSET) + 20.0;
    let half = 1.0 - 0.5_f32.powi(5);
    let (_, size, regions) = stack.draw(later + ANIMATION / 2, measure, &lay_out, 20.0, &theme);
    assert_eq!(size.height, 2.0 * toast + MARGIN, "both at full size");
    let older_top = (toast + MARGIN) * half;
    assert_eq!(
        regions.hit(10.0, older_top + 1.0),
        Some(&Part::Body(older)),
        "below the newer one's place so far"
    );
    assert_eq!(regions.hit(10.0, older_top - 1.0), None);
}
