//! lyr's colours, which are sRGB as CSS writes them, in the linear light
//! crcbl-ui's draw list takes.

use lyr_config::Colour;

/// `colour` as linear-light RGBA, its alpha multiplied by `opacity`.
///
/// The sRGB transfer function is IEC 61966-2-1's, the one crcbl-ui's own CSS
/// colours go through (`srgb_to_linear` in its `style/property.rs`, which is
/// private in crcbl-ui 0.5.0). Alpha is not encoded, so it is only scaled.
#[must_use]
pub fn linear(colour: Colour, opacity: f32) -> [f32; 4] {
    let channel = |value: u8| srgb_to_linear(f32::from(value) / 255.0);
    [
        channel(colour.r),
        channel(colour.g),
        channel(colour.b),
        f32::from(colour.a) / 255.0 * opacity,
    ]
}

/// One sRGB-encoded channel in `0..=1` as linear light.
fn srgb_to_linear(encoded: f32) -> f32 {
    if encoded <= 0.040_45 {
        encoded / 12.92
    } else {
        ((encoded + 0.055) / 1.055).powf(2.4)
    }
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
        let [r, g, b, a] = linear(Colour::parse("#808080").expect("colour"), 1.0);
        assert!(
            close(r, 0.2158) && close(g, 0.2158) && close(b, 0.2158),
            "{r}"
        );
        assert!(close(a, 1.0));
    }

    #[test]
    fn the_ends_and_the_linear_segment_are_exact() {
        let [black, ..] = linear(Colour::parse("#000000").expect("colour"), 1.0);
        let [white, ..] = linear(Colour::parse("#ffffff").expect("colour"), 1.0);
        assert!(close(black, 0.0) && close(white, 1.0));
        // 0x0a / 255 = 0.0392, inside the linear segment: / 12.92.
        let [low, ..] = linear(Colour::parse("#0a0000").expect("colour"), 1.0);
        // The two pieces of the curve nearly meet here (about 4e-6 apart), so
        // this one is held tighter than `close`.
        assert!((low - 10.0 / 255.0 / 12.92).abs() < 1e-7, "{low}");
    }

    #[test]
    fn opacity_scales_alpha_only() {
        let [r, _, _, a] = linear(Colour::parse("#ffffff80").expect("colour"), 0.5);
        assert!(close(r, 1.0));
        assert!(close(a, 128.0 / 255.0 * 0.5));
    }
}
