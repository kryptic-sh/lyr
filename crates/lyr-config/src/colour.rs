//! Colours written as CSS hex: `#rrggbb` or `#rrggbbaa`.

use std::fmt;

use serde::Deserialize;

/// A straight-alpha sRGB colour.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(try_from = "String")]
pub struct Colour {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Colour {
    /// Parses `#rrggbb` (opaque) or `#rrggbbaa`.
    ///
    /// # Errors
    ///
    /// When `text` is not `#` followed by six or eight hex digits.
    pub fn parse(text: &str) -> Result<Self, InvalidColour> {
        let invalid = || InvalidColour(text.to_owned());
        let hex = text.strip_prefix('#').ok_or_else(invalid)?;
        if !matches!(hex.len(), 6 | 8) || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(invalid());
        }
        let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|_| invalid());
        Ok(Self {
            r: byte(0)?,
            g: byte(2)?,
            b: byte(4)?,
            a: if hex.len() == 8 { byte(6)? } else { u8::MAX },
        })
    }
}

impl TryFrom<String> for Colour {
    type Error = InvalidColour;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        Self::parse(&text)
    }
}

/// A colour that is not `#rrggbb` or `#rrggbbaa`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InvalidColour(String);

impl fmt::Display for InvalidColour {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "`{}` is not a colour (#rrggbb or #rrggbbaa)", self.0)
    }
}

impl std::error::Error for InvalidColour {}

#[cfg(test)]
mod tests {
    use super::Colour;

    #[test]
    fn six_digits_are_opaque() {
        assert_eq!(
            Colour::parse("#2bc5e3"),
            Ok(Colour {
                r: 0x2b,
                g: 0xc5,
                b: 0xe3,
                a: 0xff
            })
        );
    }

    #[test]
    fn eight_digits_carry_alpha() {
        assert_eq!(Colour::parse("#00000000").map(|c| c.a), Ok(0));
        assert_eq!(
            Colour::parse("#1A1B26cc").map(|c| (c.r, c.a)),
            Ok((0x1a, 0xcc))
        );
    }

    #[test]
    fn anything_else_is_refused() {
        for text in [
            "2bc5e3", "#2bc5e", "#2bc5e3f", "#2bc5eg", "#+bc5e3", "#ééé", "",
        ] {
            assert!(Colour::parse(text).is_err(), "{text:?} parsed");
        }
    }
}
