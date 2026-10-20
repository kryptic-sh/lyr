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

/// Whether an endpoint's device is on Bluetooth, by the bus enumerator its
/// property store names (`PKEY_Device_EnumeratorName`): Windows' Bluetooth
/// buses for classic audio, hands-free and LE. The store gives the device's
/// bus, not the endpoint's own `SWD`: `USB` for a USB headset on this
/// development machine, where no Bluetooth endpoint could be seen.
#[must_use]
pub fn bluetooth(enumerator: &str) -> bool {
    ["BTHENUM", "BTHHFENUM", "BTHLEDEVICE"]
        .iter()
        .any(|bus| bus.eq_ignore_ascii_case(enumerator))
}

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

/// A stream's name in the audio menu: "System sounds" for Windows' own,
/// else the name the session gives itself, else its program's file name
/// without `.exe`. A session's name can be an indirect resource string
/// (`@%SystemRoot%\System32\...`), which is not a name to show.
#[must_use]
pub fn session_name(system_sounds: bool, display: &str, program: Option<&str>) -> String {
    if system_sounds {
        return "System sounds".to_owned();
    }
    if !display.is_empty() && !display.starts_with('@') {
        return display.to_owned();
    }
    program.map_or_else(
        || "Unknown".to_owned(),
        |path| crate::program::name(path).to_owned(),
    )
}

/// The audio menu's row for a stream named `name`, on the default endpoint
/// (`elsewhere` `None`) or on the endpoint named `elsewhere`; `None` for
/// one it leaves out. Every endpoint has a session for Windows' own sounds,
/// which play on the default, so another endpoint's is left out; another
/// stream elsewhere names its endpoint, which tells the rows apart.
#[must_use]
pub fn stream_row(name: String, system_sounds: bool, elsewhere: Option<&str>) -> Option<String> {
    match elsewhere {
        None => Some(name),
        Some(_) if system_sounds => None,
        Some(device) => Some(format!("{name} ({device})")),
    }
}

#[cfg(test)]
mod tests {
    use super::{port, session_name, stream_row};

    #[test]
    fn a_stream_elsewhere_names_its_endpoint_and_its_system_sounds_go() {
        let row = |name: &str, system, elsewhere| stream_row(name.to_owned(), system, elsewhere);
        assert_eq!(row("Spotify", false, None).as_deref(), Some("Spotify"));
        assert_eq!(
            row("System sounds", true, None).as_deref(),
            Some("System sounds")
        );
        assert_eq!(
            row("Spotify", false, Some("Headphones")).as_deref(),
            Some("Spotify (Headphones)")
        );
        assert_eq!(row("System sounds", true, Some("Headphones")), None);
    }

    #[test]
    fn a_stream_is_named_by_itself_else_by_its_program() {
        assert_eq!(
            session_name(true, "@%SystemRoot%\\x.dll,-1", None),
            "System sounds"
        );
        assert_eq!(
            session_name(false, "Spotify", Some("C:\\x\\Spotify.exe")),
            "Spotify"
        );
        assert_eq!(
            session_name(
                false,
                "",
                Some("C:\\Program Files\\Mozilla Firefox\\firefox.EXE")
            ),
            "firefox"
        );
        assert_eq!(
            session_name(false, "@%SystemRoot%\\x.dll,-1", Some("C:\\x\\app.exe")),
            "app",
            "a resource string is not a name"
        );
        assert_eq!(session_name(false, "", Some("/opt/tool")), "tool");
        assert_eq!(
            session_name(false, "", Some("exe")),
            "exe",
            "shorter than the extension"
        );
        assert_eq!(session_name(false, "", None), "Unknown");
    }

    #[test]
    fn a_bluetooth_bus_marks_the_endpoint_bluetooth() {
        use super::bluetooth;
        assert!(bluetooth("BTHENUM"));
        assert!(bluetooth("BTHHFENUM"), "hands-free");
        assert!(bluetooth("BTHLEDEVICE"), "LE audio");
        assert!(bluetooth("bthenum"), "names are not case-sensitive");
        assert!(!bluetooth("USB"), "this machine's headset");
        assert!(!bluetooth("HDAUDIO"));
        assert!(!bluetooth(""));
    }

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
}
