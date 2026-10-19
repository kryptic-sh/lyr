//! What a Windows audio endpoint reports, as the views' terms, on every
//! platform's tests. The form factors are `EndpointFormFactor`'s values from
//! Microsoft's reference, spelled out here, as plan 11 asks of Windows ABI
//! values, so this compiles everywhere.

/// `EndpointFormFactor`: speakers, headphones, a headset, a handset, and a
/// display's audio (HDMI or DisplayPort).
const SPEAKERS: u32 = 1;
const HEADPHONES: u32 = 3;
const HEADSET: u32 = 5;
const HANDSET: u32 = 6;
const DIGITAL_AUDIO_DISPLAY_DEVICE: u32 = 9;

/// The port kind waybar looks for (`lyr_core::audio`'s `PORTS`) that an
/// endpoint's form factor is; empty for one with none.
#[must_use]
pub const fn port(form_factor: u32) -> &'static str {
    match form_factor {
        SPEAKERS => "speaker",
        HEADPHONES => "headphone",
        HEADSET => "headset",
        HANDSET => "phone",
        DIGITAL_AUDIO_DISPLAY_DEVICE => "hdmi",
        _ => "",
    }
}

/// An endpoint's scalar volume, 0 – 1, as the percent the views show.
#[must_use]
pub fn percent(scalar: f32) -> u32 {
    // The clamp keeps a driver's stray value in range; `as` then cannot
    // wrap.
    (scalar.clamp(0.0, 1.0) * 100.0).round() as u32
}

/// The scalar volume a step of `delta` percent from `scalar` sets, held to
/// 0 – 1: Windows' endpoint volume does not boost past 100 %.
#[must_use]
pub fn stepped(scalar: f32, delta: i32) -> f32 {
    (scalar + delta as f32 / 100.0).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::{percent, port, stepped};

    #[test]
    fn a_form_factor_is_waybar_s_port_kind() {
        assert_eq!(port(1), "speaker");
        assert_eq!(port(3), "headphone");
        assert_eq!(port(5), "headset");
        assert_eq!(port(6), "phone");
        assert_eq!(port(9), "hdmi");
        assert_eq!(port(2), "", "line level");
        assert_eq!(port(10), "", "unknown");
    }

    #[test]
    fn a_scalar_is_a_percent_and_a_step_stays_in_range() {
        assert_eq!(percent(0.0), 0);
        assert_eq!(percent(0.554), 55);
        assert_eq!(percent(0.555), 56);
        assert_eq!(percent(1.0), 100);
        assert_eq!(percent(1.3), 100, "a stray value is held");
        assert_eq!(percent(-0.2), 0);
        assert!((stepped(0.5, 5) - 0.55).abs() < 1e-6);
        assert!((stepped(0.5, -5) - 0.45).abs() < 1e-6);
        assert!(
            (stepped(0.98, 5) - 1.0).abs() < f32::EPSILON,
            "no boost past 100 %"
        );
        assert!(stepped(0.02, -5).abs() < f32::EPSILON);
    }
}
