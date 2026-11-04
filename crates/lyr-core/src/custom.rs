//! Custom modules, waybar's `custom/<name>` (`waybar-custom(5)`): reading a
//! script's output in either return type, and what the module shows from it.
//!
//! waybar draws the label and the tooltip as Pango markup. lyr's bar labels
//! and tooltips draw plain text, so markup's tags are dropped and its
//! entities decoded ([`markup::text_of`]): the words show, their styling
//! (colour, weight, size) does not. With `escape`, the script's text and alt
//! are escaped before the format takes them, as waybar escapes them, so they
//! show exactly as printed.

use std::fmt;

use lyr_config::{Colour, Custom, Icons, ReturnType};
use serde_json::Value;

use crate::icon::{pick, ramp};
use crate::markup;
use crate::window::cut;

/// What one update of a script says.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Output {
    pub text: String,
    /// What a `format-icons` table picks by, and `{alt}`.
    pub alt: String,
    /// The script's tooltip, `\r` breaking its lines; empty for none.
    pub tooltip: String,
    /// waybar's CSS classes, which pick the label's colour here.
    pub class: Vec<String>,
    /// What an icon ramp picks by, and `{percentage}`; 0 when not given.
    pub percentage: u32,
}

/// A JSON line that is not an object.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InvalidOutput(pub String);

impl fmt::Display for InvalidOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "not a JSON object: {}", self.0)
    }
}

impl std::error::Error for InvalidOutput {}

/// Reads `out`, one update's output: a whole run's, or one line of a
/// continuous script's. `None` when it is empty or only line endings, which
/// leaves the module out.
///
/// i3blocks' lines are the text, then the tooltip (the text when there is
/// no second line), then the class. JSON is the first line's object.
///
/// # Errors
///
/// With [`ReturnType::Json`], when the first line is not a JSON object.
pub fn parse(out: &str, return_type: ReturnType) -> Result<Option<Output>, InvalidOutput> {
    if out.chars().all(|c| matches!(c, '\r' | '\n')) {
        return Ok(None);
    }
    let mut lines = out.lines();
    let first = lines.next().unwrap_or_default();
    if return_type == ReturnType::Lines {
        let text = first.to_owned();
        let tooltip = lines.next().map_or_else(|| text.clone(), str::to_owned);
        let class = lines
            .next()
            .filter(|class| !class.is_empty())
            .map(str::to_owned)
            .into_iter()
            .collect();
        return Ok(Some(Output {
            text,
            tooltip,
            class,
            ..Output::default()
        }));
    }
    let invalid = || InvalidOutput(first.to_owned());
    let json: Value = serde_json::from_str(first).map_err(|_| invalid())?;
    let object = json.as_object().ok_or_else(invalid)?;
    let text = |key: &str| match object.get(key) {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Number(number)) => number.to_string(),
        _ => String::new(),
    };
    let class = match object.get("class") {
        Some(Value::String(class)) => vec![class.clone()],
        Some(Value::Array(classes)) => classes
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        _ => Vec::new(),
    };
    // Rounded, and 0 below it or when not a number, as waybar's.
    let percentage = object
        .get("percentage")
        .and_then(Value::as_f64)
        .map_or(0, |percentage| {
            percentage.round().clamp(0.0, f64::from(u32::MAX)) as u32
        });
    Ok(Some(Output {
        text: text("text"),
        alt: text("alt"),
        tooltip: text("tooltip"),
        class,
        percentage,
    }))
}

/// What the module draws.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct View {
    pub text: String,
    /// `None` for no tooltip.
    pub tooltip: Option<String>,
    /// From the first of the output's classes that `colours` has; `None`
    /// for the theme's foreground.
    pub colour: Option<Colour>,
}

/// The view of `output` under `config`, or `None` when the module is left
/// out: its text comes out empty, or `hide_empty_text` is set and the
/// script's text is empty. A static module (no `exec`) is viewed with an
/// empty [`Output`].
#[must_use]
pub fn view(output: &Output, config: &Custom) -> Option<View> {
    let (text, alt) = if config.escape {
        (markup::escape(&output.text), markup::escape(&output.alt))
    } else {
        (output.text.clone(), output.alt.clone())
    };
    let percentage = output.percentage.to_string();
    let icon = match &config.icons {
        None => "",
        Some(Icons::One(icon)) => icon,
        Some(Icons::Ramp(icons)) => ramp(icons, output.percentage, 100).map_or("", String::as_str),
        Some(Icons::Named(icons)) => pick(icons, &[&output.alt], output.percentage),
    };
    let fill = |format: &lyr_config::Format| {
        markup::text_of(&format.render(|name| match name {
            "text" => Some(text.as_str()),
            "alt" => Some(alt.as_str()),
            "percentage" => Some(percentage.as_str()),
            "icon" => Some(icon),
            _ => None,
        }))
    };
    let label = fill(&config.format);
    if label.is_empty() || (config.hide_empty_text && output.text.is_empty()) {
        return None;
    }
    let tooltip = if !config.tooltip {
        None
    } else if let Some(format) = &config.tooltip_format {
        Some(fill(format))
    } else if output.tooltip == output.text {
        // waybar shows the label, uncut, when the script's tooltip is its
        // text, as it is with a single i3blocks line.
        Some(label.clone())
    } else {
        Some(markup::text_of(&output.tooltip))
    }
    .map(|tooltip| tooltip.replace('\r', "\n"))
    .filter(|tooltip| !tooltip.is_empty());
    let colour = output
        .class
        .iter()
        .find_map(|class| config.colours.get(class))
        .copied();
    Some(View {
        text: pad(
            cut(label, config.max_length),
            config.min_length,
            config.align,
        ),
        tooltip,
        colour,
    })
}

/// `text` padded with spaces to `min` characters, `align` of the padding
/// before it (0 all after, 1 all before, unset half each). Text as long
/// already is kept. Spaces stand in for the character widths GTK's
/// `width-chars` reserves, so the width is close to waybar's, not equal.
fn pad(text: String, min: Option<u32>, align: Option<f32>) -> String {
    let count = text.chars().count();
    let Some(missing) = min
        .and_then(|min| usize::try_from(min).ok())
        .and_then(|min| min.checked_sub(count))
        .filter(|&missing| missing > 0)
    else {
        return text;
    };
    let before = (missing as f32 * align.unwrap_or(0.5)).round() as usize;
    let before = before.min(missing);
    format!(
        "{}{text}{}",
        " ".repeat(before),
        " ".repeat(missing - before)
    )
}

#[cfg(test)]
mod tests;
