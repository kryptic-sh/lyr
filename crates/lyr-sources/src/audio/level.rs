//! A scalar volume, 0 – 1, as Windows' endpoints and CoreAudio's devices
//! report it, and the percent the views show, on every platform's tests.

/// A device's scalar volume, 0 – 1, as the percent the views show.
#[must_use]
pub fn percent(scalar: f32) -> u32 {
    // The clamp keeps a driver's stray value in range; `as` then cannot
    // wrap.
    (scalar.clamp(0.0, 1.0) * 100.0).round() as u32
}

/// The scalar volume, 0 – 1, a volume of `percent` is, held to 100 %:
/// neither Windows' endpoint volume nor CoreAudio's scalar volume boosts
/// past it.
#[must_use]
pub fn scalar(percent: u32) -> f32 {
    percent.min(100) as f32 / 100.0
}

/// The scalar volume a step of `delta` percent from `scalar` sets, held to
/// 0 – 1, as [`scalar`] holds it.
#[must_use]
pub fn stepped(scalar: f32, delta: i32) -> f32 {
    (scalar + delta as f32 / 100.0).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::{percent, scalar, stepped};

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
        assert!(scalar(0).abs() < f32::EPSILON);
        assert!((scalar(40) - 0.4).abs() < f32::EPSILON);
        assert!(
            (scalar(150) - 1.0).abs() < f32::EPSILON,
            "no boost past 100 %"
        );
        assert_eq!(percent(scalar(55)), 55, "a percent survives the round trip");
    }
}
