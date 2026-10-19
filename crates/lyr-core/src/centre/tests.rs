use std::time::Instant;

use lyr_config::{Platform, Theme, from_sources};

use super::{GAP, Target, draw};
use crate::draw::Item;
use crate::notifications::{Incoming, Store, Urgency};
use crate::toast::{BORDER, INSET};

fn theme() -> Theme {
    from_sources(None, Platform::Linux)
        .expect("the defaults")
        .theme
}

/// Every character is 10 px wide.
fn measure(text: &str) -> f32 {
    text.chars().count() as f32 * 10.0
}

const LINE: f32 = 20.0;
const WIDTH: f32 = 400.0;

fn notify(store: &mut Store, app: &str, urgency: Urgency, now: Instant) -> u32 {
    store.notify(
        Incoming {
            app_name: app.to_owned(),
            summary: format!("from {app}"),
            urgency,
            ..Incoming::default()
        },
        now,
    )
}

#[test]
fn the_title_row_has_clear_all_at_the_right() {
    let theme = theme();
    let (list, _, regions) = draw(
        &Store::default(),
        &[],
        WIDTH,
        Instant::now(),
        &measure,
        LINE,
        &theme,
    );
    // "Clear All" is 90 px, with 8 px of padding and a 2 px border each side:
    // 110 px wide, and 20 + 2 * (4 + 2) = 32 px tall.
    assert_eq!(regions.hit(WIDTH - 110.0, 0.0), Some(&Target::ClearAll));
    assert_eq!(regions.hit(WIDTH - 111.0, 0.0), None);
    assert_eq!(regions.hit(WIDTH - 1.0, 31.0), Some(&Target::ClearAll));
    let title = list.items.iter().find_map(|item| match item {
        Item::Text { text, colour, .. } if text == "Notification Center" => Some(*colour),
        _ => None,
    });
    assert_eq!(title, Some(theme.centre_title));
}

#[test]
fn the_switch_shows_do_not_disturb() {
    let theme = theme();
    let knob = |store: &Store| {
        let (list, _, regions) = draw(store, &[], WIDTH, Instant::now(), &measure, LINE, &theme);
        // Below the 32 px title row and a gap.
        let switch_y = 32.0 + GAP;
        assert_eq!(regions.hit(WIDTH - 1.0, switch_y), Some(&Target::Dnd));
        let boxes: Vec<_> = list
            .items
            .iter()
            .filter_map(|item| match item {
                Item::Box { rect, fill, .. } if rect.y >= switch_y => Some((rect.x, *fill)),
                _ => None,
            })
            .collect();
        let [(_, track), (knob_x, _)] = boxes[..] else {
            panic!("a switch and its knob: {boxes:?}");
        };
        (track, knob_x)
    };
    let mut store = Store::default();
    // The 40 px track at the right, its 12 px knob 4 px in from an end.
    assert_eq!(knob(&store), (theme.toast_background, WIDTH - 40.0 + 4.0));
    store.dnd = true;
    assert_eq!(knob(&store), (theme.switch_on, WIDTH - 4.0 - 12.0));
}

#[test]
fn the_history_follows_the_switch_newest_first() {
    let theme = theme();
    let now = Instant::now();
    let mut store = Store::default();
    let older = notify(&mut store, "mail", Urgency::Normal, now);
    let newer = notify(&mut store, "chat", Urgency::Normal, now);
    let (_, size, regions) = draw(&store, &[], WIDTH, now, &measure, LINE, &theme);
    let first = 32.0 + GAP + LINE + GAP;
    let entry = 2.0 * (BORDER + INSET) + LINE;
    assert_eq!(regions.hit(1.0, first), Some(&Target::Notification(newer)));
    // The next group: the entries' 8 px and the groups' 8 px between.
    let second = first + entry + 16.0;
    assert_eq!(regions.hit(1.0, second), Some(&Target::Notification(older)));
    assert_eq!(regions.hit(1.0, second - 1.0), None, "between them");
    assert_eq!(size.height, second + entry);
}

#[test]
fn an_empty_history_ends_at_the_switch() {
    let (_, size, _) = draw(
        &Store::default(),
        &[],
        WIDTH,
        Instant::now(),
        &measure,
        LINE,
        &theme(),
    );
    assert_eq!(size.height, 32.0 + GAP + LINE);
}

#[test]
fn entries_of_one_app_sit_closer_than_groups() {
    let theme = theme();
    let now = Instant::now();
    let mut store = Store::default();
    let first = notify(&mut store, "mail", Urgency::Normal, now);
    let second = notify(&mut store, "mail", Urgency::Normal, now);
    let (_, _, regions) = draw(&store, &[], WIDTH, now, &measure, LINE, &theme);
    let top = 32.0 + GAP + LINE + GAP;
    let entry = 2.0 * (BORDER + INSET) + LINE;
    assert_eq!(regions.hit(1.0, top), Some(&Target::Notification(second)));
    assert_eq!(
        regions.hit(1.0, top + entry + 8.0),
        Some(&Target::Notification(first))
    );
}

#[test]
fn an_entry_s_action_button_is_its_own_target() {
    let theme = theme();
    let now = Instant::now();
    let mut store = Store::default();
    let id = store.notify(
        Incoming {
            summary: "Meeting".to_owned(),
            actions: vec![("join".to_owned(), "Join".to_owned())],
            ..Incoming::default()
        },
        now,
    );
    let (_, _, regions) = draw(&store, &[], WIDTH, now, &measure, LINE, &theme);
    let entry = 32.0 + GAP + LINE + GAP;
    let button_y = entry + BORDER + INSET + LINE + 4.0 + 1.0;
    assert_eq!(
        regions.hit(BORDER + INSET + 1.0, button_y),
        Some(&Target::Action(id, 0))
    );
    assert_eq!(regions.hit(1.0, entry), Some(&Target::Notification(id)));
}

#[test]
fn a_media_player_is_a_card_with_its_controls_before_the_history() {
    use crate::media::{Control, Player, Status};
    let theme = theme();
    let now = Instant::now();
    let mut store = Store::default();
    let id = notify(&mut store, "mail", Urgency::Normal, now);
    let players = [Player {
        bus_name: "org.mpris.MediaPlayer2.test".to_owned(),
        title: "A Song".to_owned(),
        artist: "Someone".to_owned(),
        status: Status::Playing,
    }];
    let (list, _, regions) = draw(&store, &players, WIDTH, now, &measure, LINE, &theme);
    let card_top = 32.0 + GAP + LINE + GAP;
    // Two lines inside an 8 px padding and a 2 px border.
    let card = 2.0 * (2.0 + 8.0 + LINE);
    let middle = card_top + (card - LINE) / 2.0 + 1.0;
    // Each symbol is one character, 10 px; right to left, 5 px apart, inside
    // the card's padding and border.
    let next = WIDTH - 10.0 - 5.0;
    assert_eq!(
        regions.hit(next, middle),
        Some(&Target::Media(0, Control::Next))
    );
    assert_eq!(
        regions.hit(next - 15.0, middle),
        Some(&Target::Media(0, Control::PlayPause))
    );
    assert_eq!(
        regions.hit(next - 30.0, middle),
        Some(&Target::Media(0, Control::Previous))
    );
    let history = card_top + card + GAP;
    assert_eq!(regions.hit(1.0, history), Some(&Target::Notification(id)));
    let artist = list.items.iter().find_map(|item| match item {
        Item::Text { text, colour, .. } if text == "Someone" => Some(*colour),
        _ => None,
    });
    assert_eq!(artist, Some(theme.media_artist));
    let pause = char::from_u32(0x23F8).expect("a char").to_string();
    assert!(
        list.items
            .iter()
            .any(|item| matches!(item, Item::Text { text, .. } if *text == pause)),
        "a playing player offers pause"
    );
}
