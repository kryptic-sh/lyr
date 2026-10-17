//! Choosing an icon from a ramp by a percentage.

/// The icon for `value` out of `max` from `icons`, ordered low to high.
///
/// This transcribes waybar 0.15.0's `ALabel::getIcon` (`src/ALabel.cpp`):
/// `clamp(value / ((max == 0 ? 100 : max) / len), 0, len - 1)` in integer
/// arithmetic, so the bar picks the same icon waybar did. A value over `max`
/// (a volume above 100 %) gets the top icon. `None` for an empty ramp.
#[must_use]
pub fn ramp<T>(icons: &[T], value: u32, max: u32) -> Option<&T> {
    let len = u32::try_from(icons.len()).ok().filter(|&len| len > 0)?;
    let max = if max == 0 { 100 } else { max };
    // `max / len` is 0 when the ramp is longer than `max`; waybar divides by
    // zero there, and lyr takes the top icon instead.
    let index = value
        .checked_div(max / len)
        .map_or(len - 1, |index| index.min(len - 1));
    icons.get(usize::try_from(index).ok()?)
}

#[cfg(test)]
mod tests {
    use super::ramp;

    const BATTERY: [char; 5] = ['0', '1', '2', '3', '4'];
    const VOLUME: [char; 3] = ['l', 'm', 'h'];

    #[test]
    fn a_five_step_ramp_changes_every_20_percent() {
        let at = |value| ramp(&BATTERY, value, 100).copied();
        assert_eq!(at(0), Some('0'));
        assert_eq!(at(19), Some('0'));
        assert_eq!(at(20), Some('1'));
        assert_eq!(at(79), Some('3'));
        assert_eq!(at(80), Some('4'));
        assert_eq!(at(100), Some('4'));
    }

    #[test]
    fn a_three_step_ramp_uses_waybars_integer_steps() {
        // 100 / 3 is 33 in integer arithmetic, so the steps are at 33 and 66.
        let at = |value| ramp(&VOLUME, value, 100).copied();
        assert_eq!(at(32), Some('l'));
        assert_eq!(at(33), Some('m'));
        assert_eq!(at(65), Some('m'));
        assert_eq!(at(66), Some('h'));
    }

    #[test]
    fn values_past_the_top_take_the_top_icon() {
        assert_eq!(ramp(&VOLUME, 150, 100).copied(), Some('h'));
    }

    #[test]
    fn a_zero_max_means_100() {
        assert_eq!(ramp(&BATTERY, 40, 0), ramp(&BATTERY, 40, 100));
    }

    #[test]
    fn an_empty_ramp_has_no_icon() {
        assert_eq!(ramp::<char>(&[], 50, 100), None);
    }

    #[test]
    fn a_ramp_longer_than_max_takes_the_top_icon() {
        assert_eq!(ramp(&BATTERY, 1, 3).copied(), Some('4'));
    }
}
