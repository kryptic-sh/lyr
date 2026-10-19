//! A notification toast (`docs/plan/09-notifications.md#toasts`): its text
//! wrapped to the toast's width, and its draw list, in swaync's look from the
//! dotfiles' `config.json` and `style.css`.
//!
//! Drawn so far: the summary over the body, the body's markup, when it
//! arrived, the border coloured by urgency, a button per action, and the
//! close button. Not yet: images.

pub mod stack;

use std::time::Duration;

use lyr_config::Theme;

use crate::draw::{DrawList, Item, Size, TextStyle};
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
        underline: style.underline || style.link,
    }
}

/// `spans` broken into lines no wider than `width`, at spaces, keeping the
/// text's own line breaks, each line in spans of one style. A word wider
/// than `width` gets a line of its own and runs past it. The space between
/// two words starts the second one's span: a span's width does not count a
/// space at its end.
#[must_use]
pub fn wrap(
    spans: &[Span],
    width: f32,
    measure: impl Fn(&str, TextStyle) -> f32,
) -> Vec<Vec<Span>> {
    let width_of = |line: &[Span]| -> f32 {
        line.iter()
            .map(|span| measure(&span.text, text_style(span.style)))
            .sum()
    };
    let mut lines = Vec::new();
    for paragraph in paragraphs(spans) {
        let mut line = Vec::new();
        for word in words(&paragraph) {
            if line.is_empty() {
                line = word;
                continue;
            }
            let mut candidate = line.clone();
            let mut spaced = word.clone();
            spaced[0].text.insert(0, ' ');
            append(&mut candidate, spaced);
            if width_of(&candidate) <= width {
                line = candidate;
            } else {
                lines.push(std::mem::replace(&mut line, word));
            }
        }
        lines.push(line);
    }
    lines
}

/// The text's lines, as `str::lines` splits them: a final line break ends
/// the last line rather than starting another.
fn paragraphs(spans: &[Span]) -> Vec<Vec<(char, Style)>> {
    let chars: Vec<(char, Style)> = spans
        .iter()
        .flat_map(|span| span.text.chars().map(move |c| (c, span.style)))
        .collect();
    // A `\r` before a line break is whitespace, which a line's words leave
    // out.
    let mut paragraphs: Vec<Vec<(char, Style)>> = chars
        .split(|&(c, _)| c == '\n')
        .map(<[(char, Style)]>::to_vec)
        .collect();
    if chars.last().is_none_or(|&(c, _)| c == '\n') {
        paragraphs.pop();
    }
    paragraphs
}

/// A line's words, each in spans of one style.
fn words(line: &[(char, Style)]) -> Vec<Vec<Span>> {
    line.split(|&(c, _)| c.is_whitespace())
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut spans = Vec::new();
            for &(c, style) in word {
                append(
                    &mut spans,
                    vec![Span {
                        text: String::from(c),
                        style,
                    }],
                );
            }
            spans
        })
        .collect()
}

/// Adds `more` to the end of `line`, joining spans of one style.
fn append(line: &mut Vec<Span>, more: Vec<Span>) {
    for span in more {
        match line.last_mut() {
            Some(last) if last.style == span.style => last.text.push_str(&span.text),
            _ => line.push(span),
        }
    }
}

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
/// its draw list, its size, and its action buttons' rects by the action's
/// index in `content.actions`, its text measured by `measure` in lines
/// `line_height` tall. The body is markup ([`markup`]).
///
/// The buttons share a row under the body, as swaync's do, at the bar's
/// text size: swaync's `font-size: 0.2rem` is one of the slips plan 09 asks
/// the owner about.
#[must_use]
pub fn draw(
    content: &Incoming,
    elapsed: Duration,
    width: f32,
    measure: impl Fn(&str, TextStyle) -> f32,
    line_height: f32,
    theme: &Theme,
) -> (DrawList, Size, Vec<(usize, Rect)>) {
    let plain = |text: &str| measure(text, TextStyle::PLAIN);
    let text_width = width - 2.0 * (BORDER + INSET);
    let body = wrap(&markup::parse(&content.body), text_width, &measure);
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
    // The summary's line, then the body's.
    let text_bottom = BORDER + INSET + line_height * (1 + body.len()) as f32;
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
            y: BORDER + INSET,
            width: time_width,
            height: line_height,
        },
        text: time,
        colour: theme.toast_time,
        style: TextStyle::PLAIN,
    });
    items.push(Item::Text {
        rect: Rect {
            x: BORDER + INSET,
            y: BORDER + INSET,
            width: (text_width - time_width - TIME_GAP).max(0.0),
            height: line_height,
        },
        text: content.summary.clone(),
        colour: theme.toast_foreground,
        style: TextStyle::PLAIN,
    });
    // Each span after the last, cut at the text's right edge.
    let right = BORDER + INSET + text_width;
    for (index, line) in body.into_iter().enumerate() {
        let y = BORDER + INSET + line_height * (index + 1) as f32;
        let mut x = BORDER + INSET;
        for span in line {
            let style = text_style(span.style);
            let span_width = measure(&span.text, style);
            items.push(Item::Text {
                rect: Rect {
                    x,
                    y,
                    width: span_width.min(right - x).max(0.0),
                    height: line_height,
                },
                text: span.text,
                colour: if span.style.link {
                    theme.toast_link
                } else {
                    theme.toast_foreground
                },
                style,
            });
            x += span_width;
        }
    }
    let mut buttons = Vec::new();
    if !actions.is_empty() {
        let count = actions.len() as f32;
        let button_width = (text_width - (count - 1.0) * 2.0 * BUTTON_MARGIN) / count;
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
            buttons.push((index, rect));
        }
    }
    (DrawList { items }, size, buttons)
}

#[cfg(test)]
mod tests;
