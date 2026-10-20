use super::{reading, shows, step_argument};

/// `pactl --format=json info`, cut to what is read.
const INFO: &str = r#"{"server_name":"PulseAudio (on PipeWire 1.6.8)",
    "default_sink_name":"alsa_output.usb-SteelSeries_Arctis_Nova_7-00.analog-stereo",
    "default_source_name":"alsa_input.usb-SteelSeries_Arctis_Nova_7-00.mono-fallback"}"#;

/// `pactl --format=json list sinks` on the owner's machine (2026-10-20),
/// cut to two sinks.
const SINKS: &str = r#"[
  {"index":56,"state":"SUSPENDED",
   "name":"alsa_output.pci-0000_00_1f.3-platform-skl_hda_dsp_generic.HiFi__Speaker__sink",
   "description":"Raptor Lake-P/U/H cAVS Speaker","mute":false,
   "volume":{"front-left":{"value":98304,"value_percent":"150%","db":"10.57 dB"},
             "front-right":{"value":98304,"value_percent":"150%","db":"10.57 dB"}},
   "monitor_source":"alsa_output.pci-0000_00_1f.3-platform-skl_hda_dsp_generic.HiFi__Speaker__sink.monitor",
   "properties":{"device.class":"sound"},"active_port":"[Out] Speaker"},
  {"index":71,"state":"SUSPENDED",
   "name":"alsa_output.usb-SteelSeries_Arctis_Nova_7-00.analog-stereo",
   "description":"Arctis Nova 7 Analog Stereo","mute":false,
   "volume":{"front-left":{"value":32696,"value_percent":"50%","db":"-18.12 dB"},
             "front-right":{"value":32696,"value_percent":"50%","db":"-18.12 dB"}},
   "monitor_source":"alsa_output.usb-SteelSeries_Arctis_Nova_7-00.analog-stereo.monitor",
   "properties":{"device.form_factor":"headset"},"active_port":"analog-output"}
]"#;

const SOURCES: &str = r#"[
  {"index":72,"name":"alsa_output.usb-SteelSeries_Arctis_Nova_7-00.analog-stereo.monitor",
   "description":"Monitor of Arctis Nova 7 Analog Stereo","mute":false,
   "volume":{"front-left":{"value":65536},"front-right":{"value":65536}},
   "properties":{},"active_port":null},
  {"index":73,"name":"alsa_input.usb-SteelSeries_Arctis_Nova_7-00.mono-fallback",
   "description":"Arctis Nova 7 Mono","mute":true,
   "volume":{"mono":{"value":19275,"value_percent":"29%","db":"-31.96 dB"}},
   "properties":{},"active_port":"analog-input-mic"}
]"#;

#[test]
fn the_defaults_are_read_as_waybar_reads_them() {
    let (sink, source) = reading(INFO, SINKS, SOURCES).expect("read");
    let sink = sink.expect("the default sink");
    assert_eq!(sink.volume, 50, "32696 of 65536, rounded");
    assert!(!sink.muted);
    assert_eq!(
        sink.name,
        "alsa_output.usb-SteelSeries_Arctis_Nova_7-00.analog-stereo"
    );
    assert_eq!(sink.description, "Arctis Nova 7 Analog Stereo");
    assert_eq!(sink.port, "analog-outputheadset", "port then form factor");
    assert_eq!(
        sink.source_name,
        "alsa_input.usb-SteelSeries_Arctis_Nova_7-00.mono-fallback"
    );
    assert!(!sink.bluetooth);
    let source = source.expect("the default source");
    assert_eq!(source.volume, 29, "19275 of 65536, rounded");
    assert!(source.muted);
    assert_eq!(source.description, "Arctis Nova 7 Mono");
}

#[test]
fn a_bluetooth_sink_is_told_by_its_monitor() {
    let info = INFO.replace(
        "alsa_output.usb-SteelSeries_Arctis_Nova_7-00.analog-stereo\"",
        "bluez_output.AA_BB.1\"",
    );
    let sinks = r#"[{"name":"bluez_output.AA_BB.1","description":"Buds","mute":false,
        "volume":{"front-left":{"value":65536},"front-right":{"value":0}},
        "monitor_source":"bluez_output.AA_BB.1.monitor","properties":{},"active_port":null}]"#;
    let (sink, _) = reading(&info, sinks, SOURCES).expect("read");
    let sink = sink.expect("the default sink");
    assert!(sink.bluetooth);
    assert_eq!(sink.volume, 50, "the channels' mean");
    assert_eq!(sink.port, "Unknown", "no active port, as waybar names it");
}

#[test]
fn no_default_is_no_reading_and_bad_json_an_error() {
    let info = r#"{"default_sink_name":"gone","default_source_name":"gone"}"#;
    assert_eq!(reading(info, SINKS, SOURCES).expect("read"), (None, None));
    assert!(reading("not json", SINKS, SOURCES).is_err());
}

#[test]
fn only_device_and_server_events_count() {
    assert!(shows("Event 'change' on sink #71"));
    assert!(shows("Event 'new' on source #73"));
    assert!(shows("Event 'change' on server #4294967295"));
    assert!(!shows("Event 'change' on sink-input #120"));
    assert!(!shows("Event 'change' on source-output #9"));
    assert!(!shows("Event 'change' on client #301"));
}

#[test]
fn a_step_stops_at_100_and_never_lowers_a_boost() {
    assert_eq!(step_argument(50, 5).as_deref(), Some("+5%"));
    assert_eq!(step_argument(97, 5).as_deref(), Some("100%"), "held to 100");
    assert_eq!(step_argument(150, 5), None, "a boost is left as it is");
    assert_eq!(step_argument(150, -5).as_deref(), Some("-5%"));
    assert_eq!(
        step_argument(3, -5).as_deref(),
        Some("-5%"),
        "pactl stops at 0"
    );
    assert_eq!(step_argument(50, 0), None);
}
