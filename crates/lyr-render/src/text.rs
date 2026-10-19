//! Measuring and drawing the bar's text in one font and size.

use crcbl_ui::DrawList;
use crcbl_ui::font::Font;
use crcbl_ui::font::layout::TextLayout;
use glam::Vec2;
use lyr_core::hit::Rect;

/// The bar's font at the theme's size.
#[derive(Clone, Debug)]
pub struct Text {
    font: Font,
    size: f32,
    line_height: f32,
}

impl Text {
    /// `font` at `size` logical pixels, with CSS `line-height: normal`.
    #[must_use]
    pub fn new(font: Font, size: f32) -> Self {
        let line_height = font.metrics().normal_line_height(size);
        Self {
            font,
            size,
            line_height,
        }
    }

    /// The single-line layout of `text`, shaped with the font's fallbacks.
    fn layout(&self, text: &str) -> TextLayout {
        TextLayout::new(&self.font, text, self.size, self.line_height, None)
    }

    /// The height of one line, in logical pixels.
    #[must_use]
    pub const fn line_height(&self) -> f32 {
        self.line_height
    }

    /// How wide `text` is on one line, in logical pixels.
    #[must_use]
    pub fn measure(&self, text: &str) -> f32 {
        self.layout(text).width()
    }

    /// Draws `text` from `rect`'s left edge, centred in its height, one glyph
    /// run per face so fallback glyphs (emoji) draw from their own font.
    pub fn draw(&self, list: &mut DrawList, rect: Rect, text: &str, colour: [f32; 4]) {
        let layout = self.layout(text);
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
    }
}

#[cfg(test)]
mod tests {
    use crcbl_ui::DrawList;
    use crcbl_ui::draw_list::DrawCommand;
    use crcbl_ui::font::Font;
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
        text.draw(&mut list, rect, "abc", [1.0; 4]);
        let origins: Vec<f32> = list
            .commands()
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Glyphs { origin, .. } => Some(origin.y),
                _ => None,
            })
            .collect();
        assert_eq!(origins.len(), 1, "one face, one run");
        let line = text.font.metrics().normal_line_height(16.0);
        assert!((origins[0] - (100.0 + (40.0 - line) / 2.0)).abs() < 0.01);
    }
}
