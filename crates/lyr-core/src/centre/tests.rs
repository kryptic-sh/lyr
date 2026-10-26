use std::time::Instant;

use lyr_config::{Platform, Theme, from_sources};

use super::{ART_MARGIN, ART_SIZE, ART_SPACING, GAP, Shown, Target, draw};
use crate::draw::{Item, Picture, TextStyle};
use crate::notifications::{Incoming, Store, Urgency};
use crate::toast::{BORDER, CLOSE_INSET, CLOSE_SIZE, INSET};

fn theme() -> Theme {
    from_sources(None, Platform::Linux)
        .expect("the defaults")
        .theme
}

/// Every character is 10 px wide.
fn measure(text: &str, _: TextStyle) -> f32 {
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
        &Shown {
            store: &Store::default(),
            players: &[],
            now: Instant::now(),
            hovered: None,
        },
        WIDTH,
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
        let (list, _, regions) = draw(
            &Shown {
                store,
                players: &[],
                now: Instant::now(),
                hovered: None,
            },
            WIDTH,
            &measure,
            LINE,
            &theme,
        );
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
    let (_, size, regions) = draw(
        &Shown {
            store: &store,
            players: &[],
            now,
            hovered: None,
        },
        WIDTH,
        &measure,
        LINE,
        &theme,
    );
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
        &Shown {
            store: &Store::default(),
            players: &[],
            now: Instant::now(),
            hovered: None,
        },
        WIDTH,
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
    let (_, _, regions) = draw(
        &Shown {
            store: &store,
            players: &[],
            now,
            hovered: None,
        },
        WIDTH,
        &measure,
        LINE,
        &theme,
    );
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
    let (_, _, regions) = draw(
        &Shown {
            store: &store,
            players: &[],
            now,
            hovered: None,
        },
        WIDTH,
        &measure,
        LINE,
        &theme,
    );
    let entry = 32.0 + GAP + LINE + GAP;
    let button_y = entry + BORDER + INSET + LINE + 4.0 + 1.0;
    assert_eq!(
        regions.hit(BORDER + INSET + 1.0, button_y),
        Some(&Target::Action(id, 0))
    );
    assert_eq!(regions.hit(1.0, entry), Some(&Target::Notification(id)));
}

#[test]
fn a_link_in_an_entry_is_its_own_target() {
    let theme = theme();
    let now = Instant::now();
    let mut store = Store::default();
    let id = store.notify(
        Incoming {
            summary: "x".to_owned(),
            body: "<a href=\"https://x\">there</a>".to_owned(),
            ..Incoming::default()
        },
        now,
    );
    let (_, _, regions) = draw(
        &Shown {
            store: &store,
            players: &[],
            now,
            hovered: None,
        },
        WIDTH,
        &measure,
        LINE,
        &theme,
    );
    let entry = 32.0 + GAP + LINE + GAP;
    let body_y = entry + BORDER + INSET + LINE + 1.0;
    assert_eq!(
        regions.hit(BORDER + INSET + 1.0, body_y),
        Some(&Target::Link(id, 0))
    );
    assert_eq!(
        regions.hit(WIDTH - BORDER - INSET - 1.0, body_y),
        Some(&Target::Notification(id)),
        "past the link's end"
    );
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
        subtitle: "Someone".to_owned(),
        status: Status::Playing,
        art: None,
    }];
    let (list, _, regions) = draw(
        &Shown {
            store: &store,
            players: &players,
            now,
            hovered: None,
        },
        WIDTH,
        &measure,
        LINE,
        &theme,
    );
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

#[test]
fn a_media_player_s_art_stands_left_of_its_text() {
    use crate::media::{Artwork, Player, Status};
    let theme = theme();
    let store = Store::default();
    // Twice as wide as tall, so it fills the square's width.
    let art = Picture::new(2, 1, vec![255; 8]).expect("a picture");
    let players = [Player {
        bus_name: "org.mpris.MediaPlayer2.test".to_owned(),
        title: "A Song".to_owned(),
        subtitle: "Someone".to_owned(),
        status: Status::Paused,
        art: Some(Artwork::Picture(art.clone())),
    }];
    let (list, size, _) = draw(
        &Shown {
            store: &store,
            players: &players,
            now: Instant::now(),
            hovered: None,
        },
        WIDTH,
        &measure,
        LINE,
        &theme,
    );
    let card_top = 32.0 + GAP + LINE + GAP;
    // The art is taller than the two lines, inside an 8 px padding and a
    // 2 px border.
    let inner = 2.0 + 8.0;
    assert_eq!(size.height, card_top + 2.0 * inner + ART_SIZE);
    let image = list.items.iter().find_map(|item| match item {
        Item::Image { rect, picture } if *picture == art => Some(*rect),
        _ => None,
    });
    let image = image.expect("the art drawn");
    assert_eq!((image.x, image.width), (inner + ART_MARGIN, ART_SIZE));
    // Its aspect kept, centred in its square.
    assert_eq!(image.height, ART_SIZE / 2.0);
    assert_eq!(image.y, card_top + inner + ART_SIZE / 4.0);
    let title = list.items.iter().find_map(|item| match item {
        Item::Text { text, rect, .. } if text == "A Song" => Some(*rect),
        _ => None,
    });
    let title = title.expect("the title drawn");
    assert_eq!(title.x, inner + 2.0 * ART_MARGIN + ART_SIZE + ART_SPACING);
    // The two lines centred on the art.
    assert_eq!(title.y, card_top + inner + (ART_SIZE - 2.0 * LINE) / 2.0);
}

#[test]
fn a_player_with_no_subtitle_has_its_title_centred() {
    use crate::media::{Player, Status};
    let theme = theme();
    let store = Store::default();
    let players = [Player {
        title: "A Song".to_owned(),
        status: Status::Paused,
        ..Player::default()
    }];
    let (list, _, _) = draw(
        &Shown {
            store: &store,
            players: &players,
            now: Instant::now(),
            hovered: None,
        },
        WIDTH,
        &measure,
        LINE,
        &theme,
    );
    let card_top = 32.0 + GAP + LINE + GAP;
    let inner = 2.0 + 8.0;
    let title = list.items.iter().find_map(|item| match item {
        Item::Text { text, rect, .. } if text == "A Song" => Some(rect.y),
        _ => None,
    });
    // Half a line down from where two lines would start.
    assert_eq!(title, Some(card_top + inner + LINE / 2.0));
    assert!(
        !list
            .items
            .iter()
            .any(|item| matches!(item, Item::Text { colour, .. } if *colour == theme.media_artist)),
        "no empty subtitle drawn"
    );
}

#[test]
fn a_symbolic_icon_is_drawn_in_the_text_s_colour() {
    use crate::media::{Artwork, Player, Status};
    let theme = theme();
    let store = Store::default();
    let icon = Picture::new(1, 1, vec![40, 40, 40, 128]).expect("a picture");
    let players = [Player {
        art: Some(Artwork::Symbolic(icon)),
        status: Status::Paused,
        ..Player::default()
    }];
    let (list, _, _) = draw(
        &Shown {
            store: &store,
            players: &players,
            now: Instant::now(),
            hovered: None,
        },
        WIDTH,
        &measure,
        LINE,
        &theme,
    );
    let drawn = list.items.iter().find_map(|item| match item {
        Item::Image { picture, .. } => Some(picture.pixels().to_vec()),
        _ => None,
    });
    let colour = theme.panel_foreground;
    let alpha = (128 * u16::from(colour.a) / 255) as u8;
    assert_eq!(drawn, Some(vec![colour.r, colour.g, colour.b, alpha]));
}

#[test]
fn the_entry_under_the_pointer_has_a_close_button() {
    let theme = theme();
    let now = Instant::now();
    let mut store = Store::default();
    let id = store.notify(
        Incoming {
            summary: "hello".to_owned(),
            ..Incoming::default()
        },
        now,
    );
    let (_, _, plain) = draw(
        &Shown {
            store: &store,
            players: &[],
            now,
            hovered: None,
        },
        WIDTH,
        &measure,
        LINE,
        &theme,
    );
    assert!(
        !plain
            .targets()
            .any(|target| matches!(target, Target::Close(_)))
    );
    let (_, _, regions) = draw(
        &Shown {
            store: &store,
            players: &[],
            now,
            hovered: Some(id),
        },
        WIDTH,
        &measure,
        LINE,
        &theme,
    );
    let entry = 32.0 + GAP + LINE + GAP;
    let x = WIDTH - CLOSE_INSET - CLOSE_SIZE + 1.0;
    assert_eq!(
        regions.hit(x, entry + CLOSE_INSET + 1.0),
        Some(&Target::Close(id))
    );
    assert_eq!(
        regions.hit(x - 2.0, entry + CLOSE_INSET + 1.0),
        Some(&Target::Notification(id))
    );
    assert_eq!(Target::Close(id).entry(), Some(id));
    assert_eq!(Target::Notification(id).entry(), Some(id));
    assert_eq!(Target::ClearAll.entry(), None);
}
