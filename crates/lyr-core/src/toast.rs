//! A notification toast (`docs/plan/09-notifications.md#toasts`): its text
//! wrapped to the toast's width, and its draw list, in swaync's look from the
//! dotfiles' `config.json` and `style.css`.
//!
//! Drawn so far: the summary over the body, the border coloured by urgency.
//! Not yet: the arrival time, the close button, action buttons, images, body
//! markup (lyr advertises no `body-markup`, so applications send plain text).

pub mod stack;

use lyr_config::Theme;

use crate::draw::{DrawList, Item, Size};
use crate::hit::Rect;
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

/// `text` broken into lines no wider than `width`, at spaces, keeping its own
/// line breaks. A word wider than `width` gets a line of its own and runs
/// past it.
#[must_use]
pub fn wrap(text: &str, width: f32, measure: impl Fn(&str) -> f32) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in text.lines() {
        let mut line = String::new();
        for word in paragraph.split_whitespace() {
            let candidate = if line.is_empty() {
                word.to_owned()
            } else {
                format!("{line} {word}")
            };
            if line.is_empty() || measure(&candidate) <= width {
                line = candidate;
            } else {
                lines.push(std::mem::replace(&mut line, word.to_owned()));
            }
        }
        lines.push(line);
    }
    lines
}

/// The toast for `content`: its draw list and its size, its text measured by
/// `measure` in lines `line_height` tall.
#[must_use]
pub fn draw(
    content: &Incoming,
    measure: impl Fn(&str) -> f32,
    line_height: f32,
    theme: &Theme,
) -> (DrawList, Size) {
    let text_width = WIDTH - 2.0 * (BORDER + INSET);
    let mut lines = vec![content.summary.clone()];
    lines.extend(wrap(&content.body, text_width, &measure));
    let size = Size {
        width: WIDTH,
        height: 2.0 * (BORDER + INSET) + line_height * lines.len() as f32,
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
    for (index, text) in lines.into_iter().enumerate() {
        items.push(Item::Text {
            rect: Rect {
                x: BORDER + INSET,
                y: BORDER + INSET + line_height * index as f32,
                width: text_width,
                height: line_height,
            },
            text,
            colour: theme.toast_foreground,
        });
    }
    (DrawList { items }, size)
}

#[cfg(test)]
mod tests;
