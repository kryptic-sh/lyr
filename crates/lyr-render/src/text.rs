//! Measuring and drawing the bar's text in one family and size.

use crcbl_ui::DrawList;
use crcbl_ui::font::Font;
use crcbl_ui::font::face::{FontStyle, FontWeight, select_face};
use crcbl_ui::font::layout::TextLayout;
use glam::Vec2;
use lyr_core::draw::TextStyle;
use lyr_core::hit::Rect;

/// The bar's font family at the theme's size.
#[derive(Clone, Debug)]
pub struct Text {
    /// Its faces, the regular one first.
    faces: Vec<Font>,
    size: f32,
    line_height: f32,
}

impl Text {
    /// `font` at `size` logical pixels, with CSS `line-height: normal`.
    #[must_use]
    pub fn new(font: Font, size: f32) -> Self {
        let line_height = font.metrics().normal_line_height(size);
        Self {
            faces: vec![font],
            size,
            line_height,
        }
    }

    /// This text with the family's `faces` besides its regular one (bold,
    /// italic), to draw bold and italic text in; crcbl-ui synthesises a
    /// face the family lacks.
    #[must_use]
    pub fn with_faces(mut self, faces: Vec<Font>) -> Self {
        self.faces.extend(faces);
        self
    }

    /// The family's font for `style`: crcbl-ui's CSS face matching.
    fn font(&self, style: TextStyle) -> Font {
        let weight = if style.bold {
            FontWeight::BOLD
        } else {
            FontWeight::NORMAL
        };
        let slant = if style.italic {
            FontStyle::Italic
        } else {
            FontStyle::Normal
        };
        // There is always the regular face to choose from.
        select_face(&self.faces, weight, slant).unwrap_or_else(|| self.faces[0].clone())
    }

    /// The single-line layout of `text` in `font`, shaped with its
    /// fallbacks.
    fn layout(&self, text: &str, font: &Font) -> TextLayout {
        TextLayout::new(font, text, self.size, self.line_height, None)
    }

    /// The height of one line, in logical pixels.
    #[must_use]
    pub const fn line_height(&self) -> f32 {
        self.line_height
    }

    /// How wide `text` is on one line in the regular face, in logical
    /// pixels.
    #[must_use]
    pub fn measure(&self, text: &str) -> f32 {
        self.measure_in(text, TextStyle::PLAIN)
    }

    /// How wide `text` is on one line in `style`, in logical pixels:
    /// spaces at its end are not counted, those at its start are.
    #[must_use]
    pub fn measure_in(&self, text: &str, style: TextStyle) -> f32 {
        self.layout(text, &self.font(style)).width()
    }

    /// Draws `text` in `style` from `rect`'s left edge, centred in its
    /// height, one glyph run per face so fallback glyphs (emoji) draw from
    /// their own font.
    pub fn draw(
        &self,
        list: &mut DrawList,
        rect: Rect,
        text: &str,
        style: TextStyle,
        colour: [f32; 4],
    ) {
        let font = self.font(style);
        let layout = self.layout(text, &font);
        let origin = Vec2::new(rect.x, rect.y + (rect.height - layout.height()) / 2.0);
        for run in layout.face_runs() {
            list.text_glyphs(
                origin,
                run.font,
                self.size,
                colour,
                run.glyphs,
                &text[run.text],
            );
        }
        if style.underline
            && let Some(line) = layout.lines().first()
        {
            let (min, max) = underline(
                &font,
                self.size,
                origin.y + line.baseline,
                origin.x,
                layout.width(),
            );
            list.rect(min, max, colour);
        }
    }
}

/// How far below the baseline an underline's top is, in ems, for a font that
/// does not say.
const FALLBACK_UNDERLINE_OFFSET_EM: f32 = 0.1;
/// How thick that underline is, in ems.
const FALLBACK_UNDERLINE_THICKNESS_EM: f32 = 1.0 / 16.0;
/// The thinnest underline drawn, in pixels, so a small one covers a row of
/// pixels rather than falling between two.
const MIN_UNDERLINE_THICKNESS: f32 = 1.0;

/// The rectangle underlining `width` pixels of text from `left`, on the
/// baseline at `baseline`, in `font` at `size` pixels per em: where the
/// font's `post` table says, else the fallback above. Transcribed from
/// crcbl-ui 0.5.0's `tree::emit::underline`, which is private to it, so
/// lyr's underlines sit where crcbl-ui's own do.
fn underline(font: &Font, size: f32, baseline: f32, left: f32, width: f32) -> (Vec2, Vec2) {
    let (top, thickness) = match font.underline() {
        Some(line) => {
            let scale = font.metrics().scale(size);
            (baseline - line.offset * scale, line.thickness * scale)
        }
        None => (
            baseline + size * FALLBACK_UNDERLINE_OFFSET_EM,
            size * FALLBACK_UNDERLINE_THICKNESS_EM,
        ),
    };
    let thickness = thickness.max(MIN_UNDERLINE_THICKNESS);
    (
        Vec2::new(left, top),
        Vec2::new(left + width, top + thickness),
    )
}

#[cfg(test)]
mod tests {
    use crcbl_ui::DrawList;
    use crcbl_ui::draw_list::DrawCommand;
    use crcbl_ui::font::Font;
    use lyr_core::draw::TextStyle;
    use lyr_core::hit::Rect;

    use super::Text;

    fn text() -> Text {
        Text::new(Font::sans().clone(), 16.0)
    }

    #[test]
    fn longer_text_measures_wider_and_empty_text_measures_nothing() {
        let text = text();
        assert!(text.measure("") < f32::EPSILON);
        assert!(text.measure("abc") > 0.0);
        assert!(text.measure("abcdef") > text.measure("abc"));
    }

    #[test]
    fn drawn_text_is_vertically_centred_in_its_rect() {
        let text = text();
        let mut list = DrawList::new();
        let rect = Rect {
            x: 10.0,
            y: 100.0,
            width: 80.0,
            height: 40.0,
        };
        text.draw(&mut list, rect, "abc", TextStyle::PLAIN, [1.0; 4]);
        let origins: Vec<f32> = list
            .commands()
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Glyphs { origin, .. } => Some(origin.y),
                _ => None,
            })
            .collect();
        assert_eq!(origins.len(), 1, "one face, one run");
        let line = text.faces[0].metrics().normal_line_height(16.0);
        assert!((origins[0] - (100.0 + (40.0 - line) / 2.0)).abs() < 0.01);
    }

    #[test]
    fn a_leading_space_counts_and_a_trailing_one_does_not() {
        let text = text();
        assert!(text.measure(" b") > text.measure("b"));
        assert!((text.measure("b ") - text.measure("b")).abs() < f32::EPSILON);
    }

    #[test]
    fn underlined_text_has_a_line_under_it_where_the_font_says() {
        let text = text();
        let rect = Rect {
            x: 10.0,
            y: 100.0,
            width: 80.0,
            height: 40.0,
        };
        let underlined = TextStyle {
            underline: true,
            ..TextStyle::PLAIN
        };
        let rects = |style| {
            let mut list = DrawList::new();
            text.draw(&mut list, rect, "abc", style, [1.0; 4]);
            list.commands()
                .iter()
                .filter_map(|command| match command {
                    DrawCommand::Rect { min, max, .. } => Some((*min, *max)),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        assert!(rects(TextStyle::PLAIN).is_empty());
        let [(min, max)] = rects(underlined)[..] else {
            panic!("one line");
        };
        let font = &text.faces[0];
        let line = font.underline().expect("the committed font says");
        let scale = font.metrics().scale(16.0);
        let layout = text.layout("abc", font);
        let baseline = 100.0 + (40.0 - layout.height()) / 2.0 + layout.lines()[0].baseline;
        assert!(
            (min.y - (baseline - line.offset * scale)).abs() < 0.01,
            "{min:?}"
        );
        assert!((max.y - min.y - (line.thickness * scale).max(1.0)).abs() < 0.01);
        assert_eq!((min.x, max.x), (10.0, 10.0 + text.measure("abc")));
    }

    #[test]
    fn bold_text_is_wider_and_each_face_draws_in_its_own_font() {
        let text = text();
        let regular = text.measure("abc");
        let bold = TextStyle {
            bold: true,
            ..TextStyle::PLAIN
        };
        assert!(
            text.measure_in("abc", bold) > regular,
            "synthesised bold advances further"
        );
        let fonts: Vec<_> = [
            TextStyle::PLAIN,
            bold,
            TextStyle {
                italic: true,
                ..TextStyle::PLAIN
            },
        ]
        .into_iter()
        .map(|face| text.font(face).id())
        .collect();
        assert!(
            fonts[0] != fonts[1] && fonts[1] != fonts[2] && fonts[0] != fonts[2],
            "three faces: {fonts:?}"
        );
    }
}
