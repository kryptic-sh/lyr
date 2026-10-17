//! Module format strings: text with `{name}` placeholders, as waybar writes
//! them (`{icon} {volume}%`).
//!
//! A format is parsed once, when the config loads, so a typo is a config error
//! rather than a module that draws a literal `{volme}`. There is no escape for
//! a literal brace: waybar's formats in the dotfiles never need one.

use std::fmt;

use serde::Deserialize;

/// A parsed format string.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(try_from = "String")]
pub struct Format {
    source: String,
    pieces: Vec<Piece>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Piece {
    Text(String),
    Placeholder(String),
}

impl Format {
    /// Parses `source`.
    ///
    /// # Errors
    ///
    /// When a brace is unmatched or a placeholder is empty.
    pub fn parse(source: &str) -> Result<Self, InvalidFormat> {
        let mut pieces = Vec::new();
        let mut rest = source;
        while let Some(open) = rest.find(['{', '}']) {
            if rest[open..].starts_with('}') {
                return Err(InvalidFormat::new(source, "`}` with no `{` before it"));
            }
            if open > 0 {
                pieces.push(Piece::Text(rest[..open].to_owned()));
            }
            let after = &rest[open + 1..];
            let close = after
                .find(['{', '}'])
                .filter(|&i| after[i..].starts_with('}'))
                .ok_or_else(|| InvalidFormat::new(source, "`{` is never closed"))?;
            let name = &after[..close];
            if name.is_empty() {
                return Err(InvalidFormat::new(source, "`{}` names no placeholder"));
            }
            pieces.push(Piece::Placeholder(name.to_owned()));
            rest = &after[close + 1..];
        }
        if !rest.is_empty() {
            pieces.push(Piece::Text(rest.to_owned()));
        }
        Ok(Self {
            source: source.to_owned(),
            pieces,
        })
    }

    /// The placeholder names, in order, repeats included.
    pub fn placeholders(&self) -> impl Iterator<Item = &str> {
        self.pieces.iter().filter_map(|piece| match piece {
            Piece::Placeholder(name) => Some(name.as_str()),
            Piece::Text(_) => None,
        })
    }

    /// The format as written in the config.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.source
    }

    /// Fills each placeholder with `value(name)`. A placeholder `value` has
    /// nothing for is left empty; config validation makes sure every name is
    /// one the module provides.
    #[must_use]
    pub fn render<'a>(&self, mut value: impl FnMut(&str) -> Option<&'a str>) -> String {
        let mut out = String::new();
        for piece in &self.pieces {
            match piece {
                Piece::Text(text) => out.push_str(text),
                Piece::Placeholder(name) => out.push_str(value(name).unwrap_or_default()),
            }
        }
        out
    }
}

impl TryFrom<String> for Format {
    type Error = InvalidFormat;

    fn try_from(source: String) -> Result<Self, Self::Error> {
        Self::parse(&source)
    }
}

/// A format string that does not parse.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InvalidFormat {
    source: String,
    reason: &'static str,
}

impl InvalidFormat {
    fn new(source: &str, reason: &'static str) -> Self {
        Self {
            source: source.to_owned(),
            reason,
        }
    }
}

impl fmt::Display for InvalidFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "format `{}`: {}", self.source, self.reason)
    }
}

impl std::error::Error for InvalidFormat {}

#[cfg(test)]
mod tests {
    use super::Format;

    fn render(format: &str, values: &[(&str, &'static str)]) -> String {
        Format::parse(format).expect("valid").render(|name| {
            values
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| *value)
        })
    }

    #[test]
    fn placeholders_are_filled_and_text_is_kept() {
        assert_eq!(
            render("{icon} {volume}%", &[("icon", "V"), ("volume", "40")]),
            "V 40%"
        );
        assert_eq!(render("{name}", &[("name", "web")]), "web");
        assert_eq!(render("plain", &[]), "plain");
        assert_eq!(render("{a}{a}", &[("a", "x")]), "xx");
    }

    #[test]
    fn placeholders_are_listed_in_order() {
        let format = Format::parse("{icon} {alias} {icon}").expect("valid");
        assert_eq!(
            format.placeholders().collect::<Vec<_>>(),
            ["icon", "alias", "icon"]
        );
    }

    #[test]
    fn unmatched_braces_and_empty_names_are_refused() {
        for format in ["{name", "name}", "{}", "{a{b}", "{a{b", "}{"] {
            assert!(Format::parse(format).is_err(), "{format:?} parsed");
        }
    }
}
