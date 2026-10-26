//! The notification centre under the clock panel's calendar
//! (`docs/plan/09-notifications.md`): swaync's `title`, `dnd`, `mpris` and
//! `notifications` widgets in its order, in the dotfiles' style.
//!
//! The title and the do-not-disturb label are drawn at the bar's text size,
//! not swaync's 2rem: lyr draws one size of text so far. A media player's art
//! is not drawn: lyr draws no images yet.

use std::time::Instant;

use lyr_config::Theme;

use crate::draw::{self, DrawList, Item, Size, TextStyle};
use crate::hit::{Rect, Regions};
use crate::media::{Control, Player, Status};
use crate::notifications::Store;
use crate::toast::{self, Area};

/// What a click in the centre lands on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    ClearAll,
    Dnd,
    Notification(u32),
    /// An action button, by the action's index in the notification's
    /// actions.
    Action(u32, usize),
    /// A link in an entry's body, by its index among the body's links.
    Link(u32, usize),
    /// A media player's control, by the player's index in the list drawn.
    Media(usize, Control),
    /// The close button of the entry under the pointer.
    Close(u32),
}

impl Target {
    /// The history entry it is part of, if any.
    #[must_use]
    pub const fn entry(self) -> Option<u32> {
        match self {
            Self::Notification(id) | Self::Action(id, _) | Self::Link(id, _) | Self::Close(id) => {
                Some(id)
            }
            Self::ClearAll | Self::Dnd | Self::Media(..) => None,
        }
    }
}

/// Above the do-not-disturb row and the history: swaync's `widget-dnd`
/// margin.
pub const GAP: f32 = 16.0;
/// Around the Clear All button's text: swaync's 4 px by 8 px.
const BUTTON_PADDING: (f32, f32) = (8.0, 4.0);
/// The buttons' and the switch's border.
const BORDER: f32 = 2.0;
/// swaync's corner radius.
const RADIUS: f32 = 8.0;
/// Between two history entries: 4 px above and below each.
const ENTRY_GAP: f32 = 8.0;
/// Between two of the history's groups, besides the entries' own gap.
const GROUP_GAP: f32 = 8.0;
/// Inside a media player's card.
const CARD_PADDING: f32 = 8.0;
/// Between a media player's control buttons: swaync's 5 px.
const CONTROL_GAP: f32 = 5.0;

/// A control's symbol: the standard media symbols, which most fonts carry.
const fn symbol(control: Control, status: Status) -> &'static str {
    match (control, status) {
        (Control::Previous, _) => "\u{23EE}",
        (Control::PlayPause, Status::Playing) => "\u{23F8}",
        (Control::PlayPause, _) => "\u{25B6}",
        (Control::Next, _) => "\u{23ED}",
    }
}

/// What the centre shows at a moment.
#[derive(Clone, Copy, Debug)]
pub struct Shown<'a> {
    pub store: &'a Store,
    pub players: &'a [Player],
    pub now: Instant,
    /// The history entry under the pointer, which shows its close button
    /// as a toast does.
    pub hovered: Option<u32>,
}

/// The centre showing `shown`, `width` wide: its draw list, its size, and
/// what each click lands on, its text measured by `measure` in lines
/// `line_height` tall.
#[must_use]
pub fn draw(
    shown: &Shown<'_>,
    width: f32,
    measure: &dyn Fn(&str, TextStyle) -> f32,
    line_height: f32,
    theme: &Theme,
) -> (DrawList, Size, Regions<Target>) {
    let Shown {
        store,
        players,
        now,
        hovered,
    } = *shown;
    let plain = |text: &str| measure(text, TextStyle::PLAIN);
    let mut items = Vec::new();
    let mut regions = Regions::default();
    let label = |text: &str, x: f32, y: f32, colour| Item::Text {
        rect: Rect {
            x,
            y,
            width: plain(text),
            height: line_height,
        },
        text: text.to_owned(),
        colour,
        style: TextStyle::PLAIN,
    };

    // The title and Clear All.
    let (across, down) = BUTTON_PADDING;
    let button = Rect {
        x: width - (plain("Clear All") + 2.0 * (across + BORDER)),
        y: 0.0,
        width: plain("Clear All") + 2.0 * (across + BORDER),
        height: line_height + 2.0 * (down + BORDER),
    };
    let row = button.height;
    items.push(label(
        "Notification Center",
        0.0,
        (row - line_height) / 2.0,
        theme.centre_title,
    ));
    items.push(Item::Box {
        rect: button,
        radius: RADIUS,
        fill: theme.toast_background,
        border: Some((BORDER, theme.panel_border)),
    });
    items.push(label(
        "Clear All",
        button.x + BORDER + across,
        button.y + BORDER + down,
        theme.panel_foreground,
    ));
    regions.push(button, Target::ClearAll);
    let mut y = row + GAP;

    // Do Not Disturb and its switch.
    let switch = Rect {
        x: width - 2.0 * line_height,
        y,
        width: 2.0 * line_height,
        height: line_height,
    };
    items.push(label("Do Not Disturb", 0.0, y, theme.panel_foreground));
    items.extend(draw::switch(switch, store.dnd, theme));
    regions.push(switch, Target::Dnd);
    // The bottom of the last thing drawn.
    let mut bottom = switch.y + switch.height;
    y = bottom + GAP;

    // The media players, a card each.
    for (index, player) in players.iter().enumerate() {
        let card = Rect {
            x: 0.0,
            y,
            width,
            height: 2.0 * (BORDER + CARD_PADDING + line_height),
        };
        items.push(Item::Box {
            rect: card,
            radius: RADIUS,
            fill: theme.toast_background,
            border: Some((BORDER, theme.panel_border)),
        });
        let inner = BORDER + CARD_PADDING;
        let mut right = card.x + card.width - inner;
        // The controls, right to left, centred on the card's height.
        let middle = card.y + (card.height - line_height) / 2.0;
        for control in [Control::Next, Control::PlayPause, Control::Previous] {
            let text = symbol(control, player.status);
            let button = Rect {
                x: right - plain(text),
                y: middle,
                width: plain(text),
                height: line_height,
            };
            items.push(label(text, button.x, button.y, theme.panel_foreground));
            regions.push(button, Target::Media(index, control));
            right = button.x - CONTROL_GAP;
        }
        let text_width = (right - CONTROL_GAP - (card.x + inner)).max(0.0);
        for (line, (text, colour)) in [
            (player.title.as_str(), theme.panel_foreground),
            (player.artist.as_str(), theme.media_artist),
        ]
        .into_iter()
        .enumerate()
        {
            items.push(Item::Text {
                rect: Rect {
                    x: card.x + inner,
                    y: card.y + inner + line_height * line as f32,
                    width: text_width,
                    height: line_height,
                },
                text: text.to_owned(),
                colour,
                style: TextStyle::PLAIN,
            });
        }
        bottom = card.y + card.height;
        y = bottom + GAP;
    }

    // The history, swaync's order, each entry drawn as a toast.
    for (index, (_, members)) in store.history().into_iter().enumerate() {
        if index > 0 {
            y += GROUP_GAP;
        }
        for held in members {
            let elapsed = now.saturating_duration_since(held.received);
            let (entry, size, areas) =
                toast::draw(&held.content, elapsed, width, measure, line_height, theme);
            items.extend(entry.items.into_iter().map(|item| item.offset(0.0, y)));
            regions.push(
                Rect {
                    x: 0.0,
                    y,
                    width: size.width,
                    height: size.height,
                },
                Target::Notification(held.id),
            );
            for (area, rect) in areas {
                regions.push(
                    Rect {
                        y: rect.y + y,
                        ..rect
                    },
                    match area {
                        Area::Action(index) => Target::Action(held.id, index),
                        Area::Link(index) => Target::Link(held.id, index),
                    },
                );
            }
            if hovered == Some(held.id) {
                let (close, rect) = toast::close_button(size.width, measure, line_height, theme);
                items.extend(close.into_iter().map(|item| item.offset(0.0, y)));
                regions.push(
                    Rect {
                        y: rect.y + y,
                        ..rect
                    },
                    Target::Close(held.id),
                );
            }
            bottom = y + size.height;
            y = bottom + ENTRY_GAP;
        }
    }
    let size = Size {
        width,
        height: bottom,
    };
    (DrawList { items }, size, regions)
}

#[cfg(test)]
mod tests;
