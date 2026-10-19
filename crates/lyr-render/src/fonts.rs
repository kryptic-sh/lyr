//! Finding the bar's font on the system (`docs/plan/04-rendering-and-theme.md#text`).

use std::path::PathBuf;

use crcbl_font::{FontQuery, SystemFonts};
use crcbl_ui::font::Font;

/// The family lyr falls back to when the configured one is not installed.
const FALLBACK_FAMILY: &str = "monospace";

/// The bar's font and what went wrong finding it, for the caller to log. lyr
/// never draws "tofu" without saying why.
#[derive(Clone, Debug)]
pub struct Loaded {
    /// The font, carrying the system's fallback faces for characters it lacks
    /// (emoji in a notification).
    pub font: Font,
    /// The configured family, when it was not found and [`FALLBACK_FAMILY`]
    /// was used instead.
    pub missing: Option<String>,
    /// Fallback faces that were found but could not be read or parsed, and why.
    pub unreadable: Vec<(PathBuf, String)>,
}

/// Why no font could be loaded at all.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("neither `{family}` nor `{FALLBACK_FAMILY}` is installed")]
    NoFont { family: String },
    #[error("the font file {path} could not be loaded: {reason}")]
    Unloadable { path: PathBuf, reason: String },
}

/// Loads `family` from the system fonts, with the system's fallback faces.
///
/// # Errors
///
/// When neither `family` nor a monospace font is installed, or the face found
/// cannot be read.
pub fn load(family: &str) -> Result<Loaded, Error> {
    let system = SystemFonts::new();
    let wanted = FontQuery::new(family);
    let (query, missing) = match system.find(&wanted) {
        Some(_) => (wanted, None),
        None => (FontQuery::new(FALLBACK_FAMILY), Some(family.to_owned())),
    };
    let face = system.find(&query).ok_or_else(|| Error::NoFont {
        family: family.to_owned(),
    })?;
    let font = Font::from_system(&face).map_err(|error| Error::Unloadable {
        path: face.path.clone(),
        reason: error.to_string(),
    })?;
    let mut fallbacks = Vec::new();
    let mut unreadable = Vec::new();
    for face in system.fallbacks(&query) {
        match Font::from_system(&face) {
            Ok(font) => fallbacks.push(font),
            Err(error) => unreadable.push((face.path, error.to_string())),
        }
    }
    Ok(Loaded {
        font: font.with_fallback(&fallbacks),
        missing,
        unreadable,
    })
}
