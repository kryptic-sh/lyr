//! Finding the bar's font on the system (`docs/plan/04-rendering-and-theme.md#text`).

use std::path::PathBuf;

use crcbl_font::{BOLD_WEIGHT, FontQuery, FontStyle, NORMAL_WEIGHT, SystemFonts};
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
    /// The family's bold, italic and bold italic faces the system has apart
    /// from [`Self::font`], each carrying the same fallbacks.
    pub faces: Vec<Font>,
    /// The configured family, when it was not found and [`FALLBACK_FAMILY`]
    /// was used instead.
    pub missing: Option<String>,
    /// The family's other faces that were found but could not be read or
    /// parsed, and why. Fallback faces are read when first needed, and crcbl
    /// logs one that fails then.
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
    // Read only when text misses every face before them: most of the
    // system's fallbacks never are, and read up front they were most of the
    // bar's memory. One that cannot be read is logged by crcbl when it is.
    let fallbacks: Vec<Font> = system
        .fallbacks(&query)
        .into_iter()
        .map(Font::from_system_lazy)
        .collect();
    let mut unreadable = Vec::new();
    // The family's other faces, each once: the nearest the system has to
    // each may be the regular face again.
    let mut found = vec![face];
    let mut faces = Vec::new();
    for (weight, style) in [
        (BOLD_WEIGHT, FontStyle::Normal),
        (NORMAL_WEIGHT, FontStyle::Italic),
        (BOLD_WEIGHT, FontStyle::Italic),
    ] {
        let Some(face) = system.find(&query.clone().weight(weight).style(style)) else {
            continue;
        };
        if found.contains(&face) {
            continue;
        }
        match Font::from_system(&face) {
            Ok(font) => faces.push(font.with_fallback(&fallbacks)),
            Err(error) => unreadable.push((face.path.clone(), error.to_string())),
        }
        found.push(face);
    }
    Ok(Loaded {
        font: font.with_fallback(&fallbacks),
        faces,
        missing,
        unreadable,
    })
}

#[cfg(test)]
mod tests {
    use crcbl_ui::font::GlyphId;
    use lyr_config::{Platform, from_sources};

    #[test]
    #[ignore = "needs the defaults' font (Hack Nerd Font Propo) installed; CI's nerd-font job installs it"]
    fn the_bar_font_has_every_glyph_the_defaults_draw() {
        let family = from_sources(None, Platform::current())
            .expect("the defaults")
            .theme
            .font;
        let loaded = super::load(&family).expect("a font");
        assert_eq!(loaded.missing, None, "{family} is installed");
        let glyphs = lyr_config::default_glyphs();
        assert!(!glyphs.is_empty(), "the defaults draw glyphs");
        let missing: Vec<String> = glyphs
            .into_iter()
            .filter(|&glyph| loaded.font.glyph_id(glyph) == GlyphId::NOTDEF)
            .map(|glyph| format!("U+{:04X}", u32::from(glyph)))
            .collect();
        assert!(missing.is_empty(), "{family} lacks {missing:?}");
    }

    #[test]
    #[ignore = "needs the defaults' font (Hack Nerd Font Propo) installed; CI's nerd-font job installs it"]
    fn no_fallback_face_is_read_until_text_needs_it() {
        let family = from_sources(None, Platform::current())
            .expect("the defaults")
            .theme
            .font;
        let loaded = super::load(&family).expect("a font");
        let fallbacks = loaded.font.fallback();
        assert!(!fallbacks.is_empty(), "the system lists fallback faces");
        assert!(
            fallbacks.iter().all(|face| !face.is_loaded()),
            "a fallback face was read up front"
        );
    }

    #[test]
    #[ignore = "needs the defaults' font and a CJK font (Noto Sans CJK) installed"]
    fn a_character_only_a_fallback_has_is_found_in_it() {
        // U+4F60, a CJK ideograph that a Latin family lacks.
        const CJK: char = '\u{4F60}';
        let family = from_sources(None, Platform::current())
            .expect("the defaults")
            .theme
            .font;
        let loaded = super::load(&family).expect("a font");
        assert_eq!(
            loaded.font.glyph_id(CJK),
            GlyphId::NOTDEF,
            "{family} lacks it"
        );
        let face = loaded
            .font
            .fallback()
            .iter()
            .find(|face| face.glyph_id(CJK) != GlyphId::NOTDEF)
            .expect("a fallback face has it");
        assert!(face.is_loaded(), "read once asked");
    }
}
