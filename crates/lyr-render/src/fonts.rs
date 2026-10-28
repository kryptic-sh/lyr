//! Finding the bar's font on the system (`docs/plan/04-rendering-and-theme.md#text`).

use std::path::PathBuf;

use crcbl_font::{BOLD_WEIGHT, FontMatch, FontQuery, FontStyle, NORMAL_WEIGHT, SystemFonts};
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
    /// The family's bold, italic and bold italic faces, each carrying the
    /// same fallbacks.
    pub styles: Styles,
    /// The configured family, when it was not found and [`FALLBACK_FAMILY`]
    /// was used instead.
    pub missing: Option<String>,
}

/// The face the system matched for each of bold, italic and bold italic, or
/// `None` where that was the regular face. Each is read the first time text
/// in its style is measured or drawn, not when the bar starts: a face is
/// read whole, megabytes for a Nerd Font, and most bars draw no bold or
/// italic text at all.
/// One that cannot be read then is logged by crcbl, and its text falls
/// through to the fallback faces.
#[derive(Clone, Debug, Default)]
pub struct Styles {
    pub bold: Option<Font>,
    pub italic: Option<Font>,
    pub bold_italic: Option<Font>,
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
    // The family's other faces, each read once however many styles the
    // system matched it for: the nearest it has to bold italic may be the
    // bold face, and to any of them the regular face again.
    let mut found: Vec<(FontMatch, Font)> = Vec::new();
    let mut style = |weight, slant| {
        let matched = system.find(&query.clone().weight(weight).style(slant))?;
        if matched == face {
            return None;
        }
        if let Some((_, font)) = found.iter().find(|(seen, _)| *seen == matched) {
            return Some(font.clone());
        }
        let font = Font::from_system_lazy(matched.clone()).with_fallback(&fallbacks);
        found.push((matched, font.clone()));
        Some(font)
    };
    let styles = Styles {
        bold: style(BOLD_WEIGHT, FontStyle::Normal),
        italic: style(NORMAL_WEIGHT, FontStyle::Italic),
        bold_italic: style(BOLD_WEIGHT, FontStyle::Italic),
    };
    Ok(Loaded {
        font: font.with_fallback(&fallbacks),
        styles,
        missing,
    })
}

#[cfg(test)]
mod tests {
    use crcbl_ui::font::GlyphId;
    use lyr_config::{Platform, from_sources};
    use lyr_core::draw::TextStyle;

    use crate::text::Text;

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
    #[ignore = "needs the defaults' font (Hack Nerd Font Propo) installed; CI's nerd-font job installs it"]
    fn a_style_s_face_is_read_when_text_in_that_style_is_first_measured() {
        let family = from_sources(None, Platform::current())
            .expect("the defaults")
            .theme
            .font;
        let loaded = super::load(&family).expect("a font");
        let styles = loaded.styles.clone();
        let (Some(bold), Some(italic), Some(bold_italic)) =
            (styles.bold, styles.italic, styles.bold_italic)
        else {
            panic!("{family} has all three faces: {:?}", loaded.styles);
        };
        let text = Text::new(loaded.font, 16.0).with_styles(loaded.styles);
        assert!(text.measure("abc") > 0.0);
        let read = || [&bold, &italic, &bold_italic].map(|face| face.is_loaded());
        assert_eq!(read(), [false; 3], "read for plain text");
        let bold_style = TextStyle {
            bold: true,
            ..TextStyle::PLAIN
        };
        assert!(text.measure_in("abc", bold_style) > 0.0);
        assert_eq!(read(), [true, false, false]);
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
