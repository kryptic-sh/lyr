//! What a CoreAudio device reports, as the views' terms, on every
//! platform's tests. The transport types are `AudioHardware.h`'s
//! four-character codes, as `objc2-core-audio` 0.3.2 binds them, spelled
//! out here so this compiles everywhere.

/// `kAudioDeviceTransportType…`: HDMI, DisplayPort, Bluetooth and
/// Bluetooth LE.
const HDMI: u32 = u32::from_be_bytes(*b"hdmi");
const DISPLAY_PORT: u32 = u32::from_be_bytes(*b"dprt");
const BLUETOOTH: u32 = u32::from_be_bytes(*b"blue");
const BLUETOOTH_LE: u32 = u32::from_be_bytes(*b"blea");

/// The port kind waybar looks for (`lyr_core::audio`'s `PORTS`) that a
/// device's transport is; empty for one that says nothing about it, such as
/// the built-in output, whose speakers and headphone jack share one device.
#[must_use]
pub const fn port(transport: u32) -> &'static str {
    match transport {
        HDMI | DISPLAY_PORT => "hdmi",
        _ => "",
    }
}

/// Whether a device's transport is Bluetooth.
#[must_use]
pub const fn bluetooth(transport: u32) -> bool {
    matches!(transport, BLUETOOTH | BLUETOOTH_LE)
}

/// One volume for a device whose channels each have their own: their mean,
/// or `None` with no channel volume to read.
#[must_use]
pub fn mean(channels: &[f32]) -> Option<f32> {
    (!channels.is_empty()).then(|| channels.iter().sum::<f32>() / channels.len() as f32)
}

#[cfg(test)]
mod tests {
    use super::{bluetooth, mean, port};

    #[test]
    fn the_four_character_codes_are_coreaudios() {
        // kAudioDeviceTransportTypeHDMI and …Bluetooth, as the binding
        // writes them.
        assert_eq!(super::HDMI, 0x6864_6d69);
        assert_eq!(super::BLUETOOTH, 0x626c_7565);
    }

    #[test]
    fn a_display_is_hdmi_and_bluetooth_is_flagged() {
        assert_eq!(port(u32::from_be_bytes(*b"hdmi")), "hdmi");
        assert_eq!(port(u32::from_be_bytes(*b"dprt")), "hdmi");
        assert_eq!(port(u32::from_be_bytes(*b"bltn")), "", "built in");
        assert!(bluetooth(u32::from_be_bytes(*b"blue")));
        assert!(bluetooth(u32::from_be_bytes(*b"blea")));
        assert!(!bluetooth(u32::from_be_bytes(*b"usb ")));
    }

    #[test]
    fn channels_read_as_their_mean() {
        assert_eq!(mean(&[]), None);
        assert_eq!(mean(&[0.5]), Some(0.5));
        assert_eq!(mean(&[0.25, 0.75]), Some(0.5));
    }
}
