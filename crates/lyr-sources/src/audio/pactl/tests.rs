use super::{nodes, reading, shows, step_argument, streams};

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
   "properties":{"device.class":"monitor"},"active_port":null},
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
fn a_step_is_relative_with_no_ceiling() {
    assert_eq!(step_argument(5).as_deref(), Some("+5%"));
    assert_eq!(step_argument(-5).as_deref(), Some("-5%"));
    assert_eq!(step_argument(0), None);
}

/// `pactl --format=json list sink-inputs` with one stream, as `pacat`
/// playing silence showed it on the owner's machine (2026-10-20).
const SINK_INPUTS: &str = r#"[{"index":1469,"driver":"PipeWire","sink":71,"mute":false,
    "corked":false,"volume":{"front-left":{"value":65536},"front-right":{"value":32768}},
    "properties":{"application.name":"lyr-probe","media.name":"silence",
                  "application.process.binary":"pacat"}}]"#;

#[test]
fn the_menu_lists_streams_and_devices_without_monitors() {
    let listed = nodes(INFO, SINKS, SOURCES, SINK_INPUTS, "[]", CARDS).expect("read");
    let [stream] = &listed.playback[..] else {
        panic!("one stream: {:?}", listed.playback);
    };
    assert_eq!(stream.id, "1469");
    assert_eq!(stream.name, "lyr-probe", "its application");
    assert_eq!(stream.volume, 75, "the channels' mean");
    assert_eq!(
        stream.device.as_deref(),
        Some("alsa_output.usb-SteelSeries_Arctis_Nova_7-00.analog-stereo"),
        "the sink it plays to, by name"
    );
    assert!(!stream.default);
    assert!(listed.recording.is_empty());
    assert_eq!(listed.outputs.len(), 2);
    let defaults: Vec<_> = listed.outputs.iter().filter(|node| node.default).collect();
    assert_eq!(defaults.len(), 1);
    assert_eq!(defaults[0].name, "Arctis Nova 7 Analog Stereo");
    assert_eq!(
        defaults[0].id,
        "alsa_output.usb-SteelSeries_Arctis_Nova_7-00.analog-stereo"
    );
    let [input] = &listed.inputs[..] else {
        panic!("the monitor left out: {:?}", listed.inputs);
    };
    assert_eq!(input.name, "Arctis Nova 7 Mono");
    assert!(input.default && input.muted);
}

#[test]
fn a_stream_with_no_names_is_named_by_index() {
    let unnamed = r#"[{"index":9,"mute":true,"volume":{"mono":{"value":0}},"properties":{}}]"#;
    let listed = nodes(INFO, SINKS, SOURCES, "[]", unnamed, "[]").expect("read");
    assert_eq!(listed.recording[0].name, "Stream 9");
    assert_eq!(listed.recording[0].device, None, "no source named");
}

#[test]
fn stream_events_are_told_apart() {
    assert!(streams("Event 'new' on sink-input #1469"));
    assert!(streams("Event 'remove' on source-output #12"));
    assert!(!streams("Event 'change' on sink #71"));
}

/// `pactl --format=json list cards`, the headset's card on the owner's
/// machine cut to four profiles, one marked unavailable for the test.
const CARDS: &str = r#"[{"index":730,"name":"alsa_card.usb-SteelSeries_Arctis_Nova_7-00",
    "active_profile":"output:analog-stereo+input:mono-fallback",
    "properties":{"device.description":"Arctis Nova 7"},
    "profiles":{
      "off":{"description":"Off","available":true,"priority":0},
      "output:analog-stereo+input:mono-fallback":{"description":"Analog Stereo Output + Mono Input","available":true,"priority":6501},
      "output:analog-stereo":{"description":"Analog Stereo Output","available":true,"priority":6500},
      "output:iec958-stereo":{"description":"Digital Stereo (IEC958) Output","available":false,"priority":5500}}}]"#;

#[test]
fn cards_list_their_available_profiles_best_first() {
    let listed = nodes(INFO, SINKS, SOURCES, "[]", "[]", CARDS).expect("read");
    let cards = listed.settings.expect("cards on Linux");
    let [card] = &cards[..] else {
        panic!("one card: {cards:?}");
    };
    assert_eq!(card.id, "alsa_card.usb-SteelSeries_Arctis_Nova_7-00");
    assert_eq!(card.name, "Arctis Nova 7", "its description");
    let ids: Vec<_> = card.options.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "output:analog-stereo+input:mono-fallback",
            "output:analog-stereo",
            "off"
        ],
        "by priority, the unavailable one left out"
    );
    assert_eq!(card.kind, lyr_core::audio_menu::Kind::Profile);
    assert_eq!(card.chosen(), "Analog Stereo Output + Mono Input");
    assert_eq!(card.next(), Some("output:analog-stereo"));
}

#[test]
fn card_events_count() {
    assert!(shows("Event 'change' on card #730"));
}

#[test]
fn a_device_with_ports_to_choose_is_a_setting() {
    use lyr_core::audio_menu::Kind;

    // A laptop's analog sink, as `pactl` writes one with three ports.
    let sinks = r#"[{"index":5,"name":"alsa_output.pci.analog-stereo","description":"Built-in Audio",
        "mute":false,"volume":{"mono":{"value":65536}},"monitor_source":"alsa_output.pci.analog-stereo.monitor",
        "active_port":"analog-output-speaker","properties":{"device.class":"sound"},
        "ports":[
          {"name":"analog-output-speaker","description":"Speakers","availability":"availability unknown"},
          {"name":"analog-output-headphones","description":"Headphones","availability":"available"},
          {"name":"analog-output-lineout","description":"Line Out","availability":"not available"}]}]"#;
    let info =
        r#"{"default_sink_name":"alsa_output.pci.analog-stereo","default_source_name":"none"}"#;
    let listed = nodes(info, sinks, "[]", "[]", "[]", "[]").expect("read");
    let settings = listed.settings.expect("settings on Linux");
    let [port] = &settings[..] else {
        panic!("one port setting: {settings:?}");
    };
    assert_eq!(port.kind, Kind::OutputPort);
    assert_eq!(port.id, "alsa_output.pci.analog-stereo");
    assert_eq!(port.name, "Built-in Audio");
    assert_eq!(port.chosen(), "Speakers");
    let ids: Vec<_> = port.options.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(
        ids,
        ["analog-output-speaker", "analog-output-headphones"],
        "the unplugged line out left out"
    );
    assert_eq!(port.next(), Some("analog-output-headphones"));
    // A device with one port has nothing to choose.
    let single = nodes(INFO, SINKS, SOURCES, "[]", "[]", "[]").expect("read");
    assert_eq!(single.settings, Some(Vec::new()));
}
