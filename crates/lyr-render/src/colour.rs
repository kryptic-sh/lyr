//! lyr's colours, which are sRGB as CSS writes them, in the linear light
//! crcbl-ui's draw list takes.

use crcbl_ui::style::from_srgb8;
use lyr_config::Colour;

/// `colour` as linear-light RGBA, as crcbl-ui decodes a stylesheet's
/// `#rrggbbaa` (`crcbl_ui::style::from_srgb8`). Alpha is coverage, not a
/// colour, so it passes through.
#[must_use]
pub fn linear(colour: Colour) -> [f32; 4] {
    from_srgb8(colour.r, colour.g, colour.b, f32::from(colour.a) / 255.0)
}

#[cfg(test)]
mod tests {
    use lyr_config::Colour;

    use super::linear;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    #[test]
    fn mid_grey_is_crcbls_documented_value() {
        // crcbl-ui's style docs: "`#808080` is `0.2158`".
        let [r, g, b, a] = linear(Colour::parse("#808080").expect("colour"));
        assert!(
            close(r, 0.2158) && close(g, 0.2158) && close(b, 0.2158),
            "{r}"
        );
        assert!(close(a, 1.0));
    }

    #[test]
    fn the_ends_and_the_linear_segment_are_exact() {
        let [black, ..] = linear(Colour::parse("#000000").expect("colour"));
        let [white, ..] = linear(Colour::parse("#ffffff").expect("colour"));
        assert!(close(black, 0.0) && close(white, 1.0));
        // 0x0a / 255 = 0.0392, inside the linear segment: / 12.92.
        let [low, ..] = linear(Colour::parse("#0a0000").expect("colour"));
        // The two pieces of the curve nearly meet here (about 4e-6 apart), so
        // this one is held tighter than `close`.
        assert!((low - 10.0 / 255.0 / 12.92).abs() < 1e-7, "{low}");
    }

    #[test]
    fn alpha_passes_through() {
        let [r, _, _, a] = linear(Colour::parse("#ffffff80").expect("colour"));
        assert!(close(r, 1.0));
        assert!(close(a, 128.0 / 255.0));
    }
}
