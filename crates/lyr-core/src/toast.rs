//! A notification toast (`docs/plan/09-notifications.md#toasts`): its text
//! wrapped to the toast's width, and its draw list, in swaync's look from the
//! dotfiles' `config.json` and `style.css`.
//!
//! Drawn: the summary over the body, the body's markup, when it arrived,
//! the border coloured by urgency, the image and the app's icon, a button
//! per action, and the close button.

pub mod stack;

use std::ops::Range;
use std::time::Duration;

use lyr_config::Theme;

use crate::draw::{self, DrawList, Item, Size, TextStyle};
use crate::hit::Rect;
use crate::markup::{self, Span, Style};
use crate::notifications::{Incoming, Urgency};

/// swaync's `notification-window-width`.
pub const WIDTH: f32 = 500.0;
/// The space between stacked toasts: swaync's `.notification` margin.
pub const MARGIN: f32 = 18.0;
/// swaync's `.notification` border.
pub const BORDER: f32 = 2.0;
/// swaync's `.notification` corner radius.
pub const RADIUS: f32 = 8.0;
/// From the border to the text: the toast's 8 px padding and its content's
/// 7 px inset.
pub const INSET: f32 = 8.0 + 7.0;
/// Around an action button's label: swaync's 8 px padding.
pub const BUTTON_PADDING: f32 = 8.0;
/// Around an action button: swaync's 4 px margin.
pub const BUTTON_MARGIN: f32 = 4.0;
/// The close button's side: GTK's 16 px symbolic icon in the dotfiles' 2 px
/// `.close-button` padding.
pub const CLOSE_SIZE: f32 = 16.0 + 2.0 * 2.0;
/// From the toast's top and right edges to its close button. swaync 0.12.6
/// overlays the button on the box around `.notification`
/// (`data/ui/notification.blp`), so the dotfiles' 24 px `.close-button`
/// margin counts from outside the toast's own margin.
pub const CLOSE_INSET: f32 = 24.0 - MARGIN;
/// The close button's cross, a character every font has where swaync draws
/// its `swaync-close-symbolic` icon.
const CLOSE: &str = "\u{d7}";

/// The side of a notification's image, and of the app's icon over it:
/// GTK's default `-gtk-icon-size`, which the dotfiles' `* { all: unset }`
/// leaves swaync's `.image` and `.app-icon` at (seen in swaync 0.12.6).
pub const PICTURE_SIZE: f32 = 16.0;
/// Around the image: the dotfiles' `.notification-content overlay` margin.
pub const PICTURE_MARGIN: f32 = 4.0;
/// Right of the image: the dotfiles' `.image` `padding-right: 0.5rem`, at
/// their 14 px font. The app's icon sits at the far end of it, over the
/// image's right side, as swaync's overlay puts it (`halign: end`).
pub const PICTURE_PADDING: f32 = 7.0;

/// A part of a toast a click can land on, apart from the toast itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Area {
    /// An action button, by the action's index in the notification's
    /// actions.
    Action(usize),
    /// A link in the body, by its index among the body's
    /// ([`markup::links`]); a link that wraps has an area per line.
    Link(usize),
}

/// The action a click on the body invokes, which gets no button.
pub const DEFAULT_ACTION: &str = "default";

/// The least space between the summary and the time beside it, which cuts a
/// long summary short: lyr's own, since swaync ellipsizes instead.
pub const TIME_GAP: f32 = 8.0;

/// How a span in `style` is drawn: a link underlined.
#[must_use]
pub const fn text_style(style: Style) -> TextStyle {
    TextStyle {
        bold: style.bold,
        italic: style.italic,
        underline: style.underline || style.link.is_some(),
        ..TextStyle::PLAIN
    }
}

/// A stretch of one span on a line of a laid-out body: the bytes `text` of
/// span `span`'s text, `x` from the line's left and `width` wide. A space a
/// line breaks at belongs to no stretch, and the width of a line's last
/// stretch leaves out the spaces it ends with.
#[derive(Clone, Debug, PartialEq)]
pub struct Placed {
    pub span: usize,
    pub text: Range<usize>,
    pub x: f32,
    pub width: f32,
}

/// Lays `spans` out in lines no wider than its `f32`, keeping the text's own
/// line breaks, each line its stretches left to right: what only the
/// renderer, which has the fonts, can do. lyr-render's lays them out as one
/// paragraph, breaking where Unicode's line breaking rules (UAX #14) allow
/// across the spans, so a style change inside a word does not break it.
pub type LayOut<'a> = dyn Fn(&[Span], f32) -> Vec<Vec<Placed>> + 'a;

/// The close button of a toast `width` wide, which swaync shows while the
/// pointer is over the toast (`NotificationCloseButton`): its items and its
/// rect, at the top right.
#[must_use]
pub fn close_button(
    width: f32,
    measure: impl Fn(&str, TextStyle) -> f32,
    line_height: f32,
    theme: &Theme,
) -> ([Item; 2], Rect) {
    let rect = Rect {
        x: width - CLOSE_INSET - CLOSE_SIZE,
        y: CLOSE_INSET,
        width: CLOSE_SIZE,
        height: CLOSE_SIZE,
    };
    let cross = measure(CLOSE, TextStyle::PLAIN);
    let items = [
        Item::Box {
            rect,
            radius: RADIUS,
            fill: theme.toast_close,
            border: None,
        },
        Item::Text {
            rect: Rect {
                x: rect.x + (CLOSE_SIZE - cross) / 2.0,
                y: rect.y + (CLOSE_SIZE - line_height) / 2.0,
                width: cross,
                height: line_height,
            },
            text: CLOSE.to_owned(),
            colour: theme.toast_close_foreground,
            style: TextStyle::PLAIN,
        },
    ];
    (items, rect)
}

/// How long ago a notification arrived, as swaync 0.12.6 words it with its
/// default `relative-timestamps` (`get_relative_time` in
/// `src/notification/notification.vala`): "Now" under a minute, then whole
/// minutes, hours and days, rounded down.
#[must_use]
pub fn relative_time(elapsed: Duration) -> String {
    let minutes = elapsed.as_secs() / 60;
    let (count, unit) = match minutes {
        0 => return "Now".to_owned(),
        1..60 => (minutes, "min"),
        60..1440 => (minutes / 60, "hour"),
        _ => (minutes / 1440, "day"),
    };
    let plural = if count > 1 { "s" } else { "" };
    format!("{count} {unit}{plural} ago")
}

/// The toast for `content`, which arrived `elapsed` ago, `width` wide (a
/// toast is [`WIDTH`]; the clock panel's history is as wide as the panel):
/// its draw list, its size, and the rects of its action buttons and links
/// ([`Area`]), its text measured by `measure` and its body laid out by
/// `lay_out`, in lines `line_height` tall. The body is markup ([`markup`]).
///
/// The image, else the app's icon, stands left of the text, with the
/// app's icon over an image ([`PICTURE_SIZE`]). The buttons share a row
/// under both, as swaync's do, at the bar's text size: swaync's
/// `font-size: 0.2rem` is one of the slips plan 09 asks the owner about.
#[must_use]
pub fn draw(
    content: &Incoming,
    elapsed: Duration,
    width: f32,
    measure: impl Fn(&str, TextStyle) -> f32,
    lay_out: &LayOut<'_>,
    line_height: f32,
    theme: &Theme,
) -> (DrawList, Size, Vec<(Area, Rect)>) {
    let plain = |text: &str| measure(text, TextStyle::PLAIN);
    let mut areas = Vec::new();
    // swaync's `set_icon`: the image, else the app's icon, and the app's
    // icon over an image too.
    let (main, over) = match (&content.image, &content.icon) {
        (Some(image), icon) => (Some(image), icon.as_ref()),
        (None, icon) => (icon.as_ref(), None),
    };
    let picture_column = if main.is_some() {
        2.0 * PICTURE_MARGIN + PICTURE_SIZE + PICTURE_PADDING
    } else {
        0.0
    };
    let content_width = width - 2.0 * (BORDER + INSET);
    let text_width = content_width - picture_column;
    let spans = markup::parse(&content.body);
    let body = lay_out(&spans, text_width);
    let actions: Vec<(usize, &str)> = content
        .actions
        .iter()
        .enumerate()
        .filter(|(_, (key, _))| key != DEFAULT_ACTION)
        .map(|(index, (_, label))| (index, label.as_str()))
        .collect();
    let button_height = line_height + 2.0 * (BUTTON_PADDING + BORDER);
    let buttons_height = if actions.is_empty() {
        0.0
    } else {
        button_height + 2.0 * BUTTON_MARGIN
    };
    // The summary's line, then the body's, centred on the picture when
    // the picture is the taller.
    let text_height = line_height * (1 + body.len()) as f32;
    let row_height = if main.is_some() {
        text_height.max(PICTURE_SIZE + 2.0 * PICTURE_MARGIN)
    } else {
        text_height
    };
    let top = BORDER + INSET;
    let text_top = top + (row_height - text_height) / 2.0;
    let left = BORDER + INSET + picture_column;
    let text_bottom = top + row_height;
    let size = Size {
        width,
        height: text_bottom + buttons_height + BORDER + INSET,
    };
    let border = match content.urgency {
        Urgency::Low => theme.toast_border_low,
        Urgency::Normal => theme.toast_border,
        Urgency::Critical => theme.toast_border_critical,
    };
    let mut items = vec![Item::Box {
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: size.width,
            height: size.height,
        },
        radius: RADIUS,
        fill: theme.toast_background,
        border: Some((BORDER, border)),
    }];
    // On the summary's line, at the right, as swaync places it.
    let time = relative_time(elapsed);
    let time_width = plain(&time);
    items.push(Item::Text {
        rect: Rect {
            x: width - BORDER - INSET - time_width,
            y: text_top,
            width: time_width,
            height: line_height,
        },
        text: time,
        colour: theme.toast_time,
        style: TextStyle::PLAIN,
    });
    items.push(Item::Text {
        rect: Rect {
            x: left,
            y: text_top,
            width: (text_width - time_width - TIME_GAP).max(0.0),
            height: line_height,
        },
        text: content.summary.clone(),
        colour: theme.toast_foreground,
        style: TextStyle::PLAIN,
    });
    // Each stretch where the layout puts it, cut at the text's right edge.
    let right = left + text_width;
    for (index, line) in body.into_iter().enumerate() {
        let y = text_top + line_height * (index + 1) as f32;
        for placed in line {
            let span = &spans[placed.span];
            let x = left + placed.x;
            let rect = Rect {
                x,
                y,
                width: placed.width.min(right - x).max(0.0),
                height: line_height,
            };
            if let Some(link) = span.style.link {
                areas.push((Area::Link(link as usize), rect));
            }
            items.push(Item::Text {
                rect,
                text: span.text[placed.text].to_owned(),
                colour: if span.style.link.is_some() {
                    theme.toast_link
                } else {
                    theme.toast_foreground
                },
                style: text_style(span.style),
            });
        }
    }
    if !actions.is_empty() {
        let count = actions.len() as f32;
        let button_width = (content_width - (count - 1.0) * 2.0 * BUTTON_MARGIN) / count;
        let y = text_bottom + BUTTON_MARGIN;
        for (column, (index, label)) in actions.into_iter().enumerate() {
            let rect = Rect {
                x: BORDER + INSET + column as f32 * (button_width + 2.0 * BUTTON_MARGIN),
                y,
                width: button_width,
                height: button_height,
            };
            items.push(Item::Box {
                rect,
                radius: RADIUS,
                fill: theme.toast_background,
                border: Some((BORDER, theme.toast_border)),
            });
            let label_width = plain(label).min(rect.width - 2.0 * (BUTTON_PADDING + BORDER));
            items.push(Item::Text {
                rect: Rect {
                    x: rect.x + (rect.width - label_width) / 2.0,
                    y: rect.y + BORDER + BUTTON_PADDING,
                    width: label_width,
                    height: line_height,
                },
                text: label.to_owned(),
                colour: theme.toast_foreground,
                style: TextStyle::PLAIN,
            });
            areas.push((Area::Action(index), rect));
        }
    }
    if let Some(picture) = main {
        let x = BORDER + INSET + PICTURE_MARGIN;
        let y = top + (row_height - PICTURE_SIZE) / 2.0;
        items.push(draw::fitted(picture, x, y, PICTURE_SIZE));
        if let Some(icon) = over {
            items.push(draw::fitted(icon, x + PICTURE_PADDING, y, PICTURE_SIZE));
        }
    }
    (DrawList { items }, size, areas)
}

#[cfg(test)]
pub(crate) mod tests;
