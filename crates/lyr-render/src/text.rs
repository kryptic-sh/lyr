//! Measuring and drawing the bar's text in one family and size.

use crcbl_ui::DrawList;
use crcbl_ui::font::Font;
use crcbl_ui::font::face::{FontStyle, FontWeight, select_face};
use crcbl_ui::font::layout::TextLayout;
use crcbl_ui::font::rich::{RichLayout, RichRun};
use glam::Vec2;
use lyr_core::draw::TextStyle;
use lyr_core::hit::Rect;
use lyr_core::markup::Span;
use lyr_core::toast::{self, Placed};

use crate::fonts::Styles;

/// How much smaller the calendar is: Pango's `<small>`, one step down its
/// scale of 1.2 (`PANGO_SCALE_SMALL`).
pub const SMALL: f32 = 1.0 / 1.2;

/// The bar's font family at the theme's size, and the calendar's.
#[derive(Clone, Debug)]
pub struct Text {
    /// Its regular face.
    font: Font,
    /// Its other faces.
    styles: Styles,
    size: f32,
    line_height: f32,
    /// The calendar's, when it is set apart; otherwise the calendar is
    /// drawn in this.
    calendar: Option<Box<Self>>,
}

impl Text {
    /// `font` at `size` logical pixels, with CSS `line-height: normal`.
    #[must_use]
    pub fn new(font: Font, size: f32) -> Self {
        let line_height = font.metrics().normal_line_height(size);
        Self {
            font,
            styles: Styles::default(),
            size,
            line_height,
            calendar: None,
        }
    }

    /// This text with `calendar` for the calendar's.
    #[must_use]
    pub fn with_calendar(mut self, calendar: Self) -> Self {
        self.calendar = Some(Box::new(calendar));
        self
    }

    /// The text `style` is drawn in.
    fn in_style(&self, style: TextStyle) -> &Self {
        match &self.calendar {
            Some(calendar) if style.calendar => calendar,
            _ => self,
        }
    }

    /// The height of one of the calendar's lines, in logical pixels.
    #[must_use]
    pub fn calendar_line_height(&self) -> f32 {
        let calendar = TextStyle {
            calendar: true,
            ..TextStyle::PLAIN
        };
        self.in_style(calendar).line_height
    }

    /// This text with the family's `styles` besides its regular face, to
    /// draw bold and italic text in; crcbl-ui synthesises a face the family
    /// lacks.
    #[must_use]
    pub fn with_styles(mut self, styles: Styles) -> Self {
        self.styles = styles;
        self
    }

    /// The family's font for `style`: the face the system matched for it,
    /// synthesised by crcbl-ui where it falls short. Only that face is asked,
    /// so a style's face is read when text in that style is first drawn.
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
        let matched = match (style.bold, style.italic) {
            (false, false) => None,
            (true, false) => self.styles.bold.as_ref(),
            (false, true) => self.styles.italic.as_ref(),
            (true, true) => self.styles.bold_italic.as_ref(),
        };
        let face = matched.unwrap_or(&self.font);
        // One face to choose from is always chosen.
        select_face(std::slice::from_ref(face), weight, slant).unwrap_or_else(|| face.clone())
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
        let within = self.in_style(style);
        within.layout(text, &within.font(style)).width()
    }

    /// `spans` laid out together in lines no wider than `width`, each span
    /// in its style's face ([`toast::text_style`]): crcbl-ui's
    /// [`RichLayout`], which breaks where UAX #14 allows across the spans and
    /// measures each stretch as [`Self::draw`] shapes it.
    #[must_use]
    pub fn lay_out(&self, spans: &[Span], width: f32) -> Vec<Vec<Placed>> {
        let fonts: Vec<Font> = spans
            .iter()
            .map(|span| self.font(toast::text_style(span.style)))
            .collect();
        let runs: Vec<RichRun<'_>> = spans
            .iter()
            .zip(&fonts)
            .map(|(span, font)| RichRun {
                text: &span.text,
                font,
                size: self.size,
            })
            .collect();
        let layout = RichLayout::new(&runs, self.line_height, Some(width));
        let mut lines: Vec<Vec<Placed>> = layout
            .lines()
            .iter()
            .map(|line| {
                layout
                    .line_segments(line)
                    .iter()
                    .map(|segment| Placed {
                        span: segment.run,
                        text: segment.text.clone(),
                        x: segment.x,
                        width: segment.width,
                    })
                    .collect()
            })
            .collect();
        // A final hard break ends the last line rather than starting another,
        // as `str::lines` has it.
        if lines.last().is_some_and(Vec::is_empty) {
            lines.pop();
        }
        lines
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
        let within = self.in_style(style);
        let font = within.font(style);
        let layout = within.layout(text, &font);
        let origin = Vec2::new(rect.x, rect.y + (rect.height - layout.height()) / 2.0);
        for run in layout.face_runs() {
            list.text_glyphs(
                origin,
                run.font,
                within.size,
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
                within.size,
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
        let line = text.font.metrics().normal_line_height(16.0);
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
        let font = &text.font;
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

    #[test]
    fn the_calendar_draws_in_its_own_smaller_text() {
        let text = text().with_calendar(Text::new(Font::sans().clone(), 16.0 * super::SMALL));
        let calendar = TextStyle {
            calendar: true,
            ..TextStyle::PLAIN
        };
        assert!(text.measure_in("abc", calendar) < text.measure("abc"));
        assert!(text.calendar_line_height() < text.line_height());
        let mut list = DrawList::new();
        text.draw(
            &mut list,
            Rect {
                x: 0.0,
                y: 0.0,
                width: 80.0,
                height: 40.0,
            },
            "abc",
            calendar,
            [1.0; 4],
        );
        let sizes: Vec<f32> = list
            .commands()
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Glyphs { size, .. } => Some(*size),
                _ => None,
            })
            .collect();
        assert_eq!(sizes, [16.0 * super::SMALL]);
        // Its underline is the small size's too.
        let mut list = DrawList::new();
        let rect = Rect {
            x: 0.0,
            y: 0.0,
            width: 80.0,
            height: 40.0,
        };
        let underlined = TextStyle {
            underline: true,
            ..calendar
        };
        text.draw(&mut list, rect, "abc", underlined, [1.0; 4]);
        let tops: Vec<f32> = list
            .commands()
            .iter()
            .filter_map(|command| match command {
                DrawCommand::Rect { min, .. } => Some(min.y),
                _ => None,
            })
            .collect();
        let small = text.calendar.as_deref().expect("a calendar text");
        let font = &small.font;
        let layout = small.layout("abc", font);
        let baseline = (40.0 - layout.height()) / 2.0 + layout.lines()[0].baseline;
        let line = font.underline().expect("the committed font says");
        let top = baseline - line.offset * font.metrics().scale(16.0 * super::SMALL);
        assert_eq!(tops.len(), 1);
        assert!((tops[0] - top).abs() < 0.001, "{tops:?} {top}");
        let alone = super::Text::new(Font::sans().clone(), 16.0);
        assert!(
            (alone.measure_in("abc", calendar) - alone.measure("abc")).abs() < f32::EPSILON,
            "without its own, the calendar is the bar's"
        );
    }

    /// `body`'s lines laid out `width` wide, each as its stretches' text.
    fn lines(body: &str, width: f32) -> Vec<Vec<String>> {
        let spans = lyr_core::markup::parse(body);
        text()
            .lay_out(&spans, width)
            .iter()
            .map(|line| {
                line.iter()
                    .map(|placed| spans[placed.span].text[placed.text.clone()].to_owned())
                    .collect()
            })
            .collect()
    }

    #[test]
    fn a_body_wraps_between_words_and_keeps_its_line_breaks() {
        assert_eq!(
            lines("one two three four five six seven", 100.0),
            [["one two three"], ["four five six"], ["seven"]]
        );
        assert_eq!(lines("a\n\nb", 100.0), [vec!["a"], vec![], vec!["b"]]);
        assert_eq!(
            lines("a\n", 100.0),
            [["a"]],
            "a final line break starts no line"
        );
        assert!(lines("", 100.0).is_empty());
        assert_eq!(
            lines("one\u{2028}two", 100.0),
            [["one"], ["two"]],
            "a line separator is a hard break too"
        );
        let hyphenated = text().measure("a well-") + 1.0;
        assert_eq!(
            lines("a well-known", hyphenated),
            [["a well-"], ["known"]],
            "a line may end after a hyphen"
        );
    }

    #[test]
    fn a_style_change_inside_a_word_does_not_break_it() {
        assert_eq!(
            lines("one hello<b>world</b>", 100.0),
            [vec!["one"], vec!["hello", "world"]]
        );
    }

    #[test]
    fn a_space_stays_in_its_own_span_and_bold_is_measured_bold() {
        let text = text();
        let spans = lyr_core::markup::parse("a <b>b</b> <a href=\"x\">link</a>");
        let line = &text.lay_out(&spans, 1000.0)[0];
        let texts: Vec<&str> = line
            .iter()
            .map(|placed| &spans[placed.span].text[placed.text.clone()])
            .collect();
        assert_eq!(texts, ["a ", "b", " ", "link"], "no space starts a link");
        for pair in line.windows(2) {
            assert!((pair[0].x + pair[0].width - pair[1].x).abs() < 0.01);
        }
        let bold = TextStyle {
            bold: true,
            ..TextStyle::PLAIN
        };
        assert!((line[1].width - text.measure_in("b", bold)).abs() < 0.01);
        assert!(line[1].width > text.measure("b"), "measured bold");
    }
}
