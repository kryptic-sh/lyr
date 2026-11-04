//! Custom modules, waybar's `custom/<name>` (`waybar-custom(5)`,
//! `docs/plan/07-modules.md#custom`): a script's output, or static text.
//!
//! A zone names one `custom/<name>`, and its config is the `[custom.<name>]`
//! table, with waybar's keys in lyr's spelling.

use std::collections::BTreeMap;
use std::fmt;
use std::time::Duration;

use serde::Deserialize;
use serde::de::{self, Deserializer, Visitor};

use crate::actions::On;
use crate::colour::Colour;
use crate::format::Format;
use crate::schema::Icon;

/// The prefix a zone names a custom module with.
pub const PREFIX: &str = "custom/";

/// The placeholders a custom module's formats take.
pub const PLACEHOLDERS: [&str; 4] = ["text", "alt", "percentage", "icon"];

/// waybar's bare `{}`, which stands for `{text}` and cannot be combined
/// with any other placeholder.
const BARE: &str = "{}";

/// The shortest interval waybar takes; a shorter one is raised to it.
pub const SHORTEST: Duration = Duration::from_millis(1);

/// One `[custom.<name>]` table.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Custom {
    /// The table's name, which a zone names as `custom/<name>`.
    #[serde(skip)]
    pub name: String,
    /// The command whose output the module shows, run through the platform
    /// shell. Unset, the module is static: its `format` alone. A config file
    /// is code: the command runs as the user (SECURITY.md).
    #[serde(default)]
    pub exec: Option<String>,
    /// A command run before each run of `exec`, which runs only when this
    /// one exits with status 0.
    #[serde(default)]
    pub exec_if: Option<String>,
    /// How `exec`'s output is read.
    #[serde(default)]
    pub return_type: ReturnType,
    /// How often `exec` runs, or `"once"`. Unset, with no `signal` either,
    /// `exec` is a continuous script: each line it prints is an update.
    #[serde(default)]
    pub interval: Option<Interval>,
    /// How long after a continuous script exits it is run again. Unset, it
    /// is not.
    #[serde(default, deserialize_with = "some_seconds")]
    pub restart_interval: Option<Duration>,
    /// `lyr signal <N>` with this number runs `exec` again.
    #[serde(default)]
    pub signal: Option<u32>,
    /// `{text}` (or a bare `{}`), `{alt}`, `{percentage}` and `{icon}`.
    #[serde(default = "text_only", deserialize_with = "bare_format")]
    pub format: Format,
    /// What `{icon}` is: one icon, a ramp by percentage, or icons by `alt`
    /// (each one icon or a ramp), with `default` for an `alt` that has none.
    #[serde(default)]
    pub icons: Option<Icons>,
    /// Leaves the module out while the script's text is empty, even when
    /// the format has text of its own.
    #[serde(default)]
    pub hide_empty_text: bool,
    /// Runs `exec` again once an `{ exec = … }` binding's command started.
    #[serde(default = "yes")]
    pub exec_on_event: bool,
    /// The most characters shown, the rest cut with an ellipsis.
    #[serde(default)]
    pub max_length: Option<u32>,
    /// The fewest characters shown: a shorter text is padded with spaces.
    #[serde(default)]
    pub min_length: Option<u32>,
    /// Where a text shorter than `min_length` sits in its padding: 0 at the
    /// left, 1 at the right. Unset is the middle.
    #[serde(default)]
    pub align: Option<f32>,
    /// Whether the module has a tooltip.
    #[serde(default = "yes")]
    pub tooltip: bool,
    /// The tooltip, with the format's placeholders; it replaces the
    /// script's own.
    #[serde(default, deserialize_with = "some_bare_format")]
    pub tooltip_format: Option<Format>,
    /// Shows the script's text and alt as they are, rather than reading
    /// them as Pango markup.
    #[serde(default)]
    pub escape: bool,
    /// A command run after each update, through the platform shell.
    #[serde(default)]
    pub on_update: Option<String>,
    /// The label's colour by the script's class, where waybar would use
    /// CSS: the first of its classes listed here colours it.
    #[serde(default)]
    pub colours: BTreeMap<String, Colour>,
    /// What a click or a scroll does.
    #[serde(default)]
    pub on: On,
}

impl Custom {
    /// How `exec` is run; `None` for a static module.
    #[must_use]
    pub fn runs(&self) -> Option<Runs> {
        self.exec.as_ref()?;
        Some(match (self.interval, self.signal) {
            (Some(Interval::Every(every)), _) => Runs::Every(every),
            (Some(Interval::Once), _) | (None, Some(_)) => Runs::Once,
            (None, None) => Runs::Continuous {
                restart: self.restart_interval,
            },
        })
    }

    /// The checks the types alone cannot make, under `key`
    /// (`custom.<name>`). Returns the first problem, naming its key.
    pub(crate) fn validate(&self, key: &str) -> Result<(), String> {
        if self.exec.is_none() {
            let needs_exec = [
                ("exec_if", self.exec_if.is_some()),
                ("return_type", self.return_type != ReturnType::Lines),
                ("interval", self.interval.is_some()),
                ("restart_interval", self.restart_interval.is_some()),
                ("signal", self.signal.is_some()),
                ("on_update", self.on_update.is_some()),
            ];
            if let Some((name, _)) = needs_exec.iter().find(|(_, set)| *set) {
                return Err(format!("{key}.{name}: needs `exec`"));
            }
        }
        for (name, command) in [
            ("exec", &self.exec),
            ("exec_if", &self.exec_if),
            ("on_update", &self.on_update),
        ] {
            if command
                .as_deref()
                .is_some_and(|command| command.trim().is_empty())
            {
                return Err(format!("{key}.{name}: is empty"));
            }
        }
        if self.restart_interval.is_some() && (self.interval.is_some() || self.signal.is_some()) {
            return Err(format!(
                "{key}.restart_interval: only a continuous script restarts, one with no \
                 `interval` and no `signal`"
            ));
        }
        if self.signal == Some(0) {
            return Err(format!("{key}.signal: must be more than 0"));
        }
        if self.max_length == Some(0) {
            return Err(format!("{key}.max_length: must be more than 0"));
        }
        if let Some(align) = self.align
            && !(0.0..=1.0).contains(&align)
        {
            return Err(format!("{key}.align: {align} is outside 0 – 1"));
        }
        Ok(())
    }
}

/// How a custom module's `exec` runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Runs {
    /// Again this long after the last run ended, so never two at once; a
    /// signal or a click runs it in between.
    Every(Duration),
    /// Once at the start, and then on a signal or a click.
    Once,
    /// It runs on, each line it prints an update; once it exits, it is run
    /// again after `restart`, when set.
    Continuous { restart: Option<Duration> },
}

/// How a script's output is read (waybar's `return-type`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ReturnType {
    /// i3blocks' lines: the text, then the tooltip, then the class. Any
    /// `return_type` other than `"json"` means this, as in waybar.
    #[default]
    Lines,
    /// One JSON object per line: `text`, `alt`, `tooltip`, `class` and
    /// `percentage`.
    Json,
}

impl<'de> Deserialize<'de> for ReturnType {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let name = String::deserialize(deserializer)?;
        Ok(if name == "json" {
            Self::Json
        } else {
            Self::Lines
        })
    }
}

/// waybar's `interval`: seconds, or `"once"`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Interval {
    Every(Duration),
    Once,
}

impl<'de> Deserialize<'de> for Interval {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(IntervalVisitor { once: true })
    }
}

/// Reads seconds, and `"once"` when `once` is set.
struct IntervalVisitor {
    once: bool,
}

impl IntervalVisitor {
    /// `seconds` as a duration, raised to [`SHORTEST`].
    fn seconds<E: de::Error>(seconds: f64) -> Result<Interval, E> {
        if !seconds.is_finite() || seconds < 0.0 {
            return Err(E::custom(format!("{seconds} is not a number of seconds")));
        }
        let every = Duration::try_from_secs_f64(seconds).map_err(E::custom)?;
        Ok(Interval::Every(every.max(SHORTEST)))
    }
}

impl Visitor<'_> for IntervalVisitor {
    type Value = Interval;

    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.once {
            f.write_str("a number of seconds or \"once\"")
        } else {
            f.write_str("a number of seconds")
        }
    }

    fn visit_i64<E: de::Error>(self, seconds: i64) -> Result<Interval, E> {
        Self::seconds(seconds as f64)
    }

    fn visit_u64<E: de::Error>(self, seconds: u64) -> Result<Interval, E> {
        Self::seconds(seconds as f64)
    }

    fn visit_f64<E: de::Error>(self, seconds: f64) -> Result<Interval, E> {
        Self::seconds(seconds)
    }

    fn visit_str<E: de::Error>(self, text: &str) -> Result<Interval, E> {
        if self.once && text == "once" {
            Ok(Interval::Once)
        } else {
            Err(E::invalid_value(de::Unexpected::Str(text), &self))
        }
    }
}

/// `restart_interval`: seconds, raised to [`SHORTEST`].
fn some_seconds<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<Duration>, D::Error> {
    match deserializer.deserialize_any(IntervalVisitor { once: false })? {
        Interval::Every(every) => Ok(Some(every)),
        Interval::Once => Err(de::Error::custom("\"once\" is not a number of seconds")),
    }
}

/// What `{icon}` is (waybar's `format-icons`).
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum Icons {
    /// The same icon whatever the output.
    One(String),
    /// Chosen by the output's percentage, lowest first.
    Ramp(Vec<String>),
    /// Chosen by the output's `alt`, then `default`; a ramp among them is
    /// chosen by the percentage.
    Named(BTreeMap<String, Icon>),
}

/// The default format: the script's text.
fn text_only() -> Format {
    Format::parse("{text}").expect("`{text}` is a format")
}

/// For a `#[serde(default)]` that is `true`.
const fn yes() -> bool {
    true
}

/// Parses `source` as a custom module's format: waybar's bare `{}` is
/// `{text}`, and on its own, as waybar cannot combine it with another
/// placeholder.
///
/// # Errors
///
/// When `source` is not a format, or `{}` is combined with another
/// placeholder.
pub fn parse_format(source: &str) -> Result<Format, String> {
    if !source.contains(BARE) {
        return Format::parse(source).map_err(|error| error.to_string());
    }
    let format =
        Format::parse(&source.replace(BARE, "{text}")).map_err(|error| error.to_string())?;
    let bare = source.matches(BARE).count();
    if format.placeholders().count() > bare {
        return Err(format!(
            "format `{source}`: `{{}}` cannot be combined with other placeholders; \
             write `{{text}}` instead"
        ));
    }
    Ok(format)
}

/// `format`, through [`parse_format`].
fn bare_format<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Format, D::Error> {
    let source = String::deserialize(deserializer)?;
    parse_format(&source).map_err(de::Error::custom)
}

/// `tooltip_format`, through [`parse_format`].
fn some_bare_format<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Format>, D::Error> {
    bare_format(deserializer).map(Some)
}

#[cfg(test)]
mod tests {
    use super::parse_format;

    #[test]
    fn a_bare_placeholder_is_the_text_and_stands_alone() {
        let format = parse_format("<{}>").expect("valid");
        assert_eq!(format.placeholders().collect::<Vec<_>>(), ["text"]);
        assert_eq!(
            parse_format("{} {}").expect("valid").placeholders().count(),
            2
        );
        for mixed in ["{} {icon}", "{icon}{}", "{text} {}"] {
            let error = parse_format(mixed).expect_err(mixed);
            assert!(error.contains("cannot be combined"), "{mixed}: {error}");
        }
        assert!(parse_format("{icon} {text}").is_ok());
        assert!(parse_format("{text").is_err());
    }
}
