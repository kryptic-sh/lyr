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
    let Some(program) = program else {
        return "Unknown".to_owned();
    };
    // Split on both separators, so the tests run the same everywhere.
    let file = program.rsplit(['\\', '/']).next().unwrap_or(program);
    let stem = file.len().checked_sub(".exe".len()).filter(|&end| {
        file.get(end..)
            .is_some_and(|extension| extension.eq_ignore_ascii_case(".exe"))
    });
    stem.map_or(file, |end| &file[..end]).to_owned()
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
    use super::{percent, port, session_name, stepped, stream_row};

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
