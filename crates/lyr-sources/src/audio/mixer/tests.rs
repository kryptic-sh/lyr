use crcbl_mixer::{
    Card, CardId, Choice, Device, DeviceId, Direction, MixerState, Stream, StreamId,
};
use lyr_core::audio_menu::Kind;

use super::{nodes, reading};

const HEADSET: &str = "alsa_output.usb-SteelSeries_Arctis_Nova_7-00.analog-stereo";
const SPEAKER: &str =
    "alsa_output.pci-0000_00_1f.3-platform-skl_hda_dsp_generic.HiFi__Speaker__sink";
const MIC: &str = "alsa_input.usb-SteelSeries_Arctis_Nova_7-00.mono-fallback";

fn choice(name: &str, description: &str, available: bool) -> Choice {
    Choice {
        name: name.to_owned(),
        description: description.to_owned(),
        available,
    }
}

fn device(id: u32, direction: Direction, name: &str, description: &str) -> Device {
    Device {
        id: DeviceId(id),
        direction,
        name: name.to_owned(),
        description: description.to_owned(),
        volume: 100,
        muted: false,
        default: false,
        active_port: None,
        ports: Vec::new(),
        form_factor: None,
        bluetooth: false,
        card: None,
    }
}

/// The owner's machine as crcbl-mixer read it (2026-10-22), cut to the
/// headset, the speaker, the headset's mic and the headset's card with
/// four profiles, one marked unavailable for the test.
fn machine() -> MixerState {
    MixerState {
        devices: vec![
            Device {
                volume: 150,
                active_port: Some("[Out] Speaker".to_owned()),
                ports: vec![choice("[Out] Speaker", "Speaker", true)],
                ..device(
                    69,
                    Direction::Output,
                    SPEAKER,
                    "Raptor Lake-P/U/H cAVS Speaker",
                )
            },
            Device {
                volume: 50,
                default: true,
                active_port: Some("analog-output".to_owned()),
                ports: vec![choice("analog-output", "Analog Output", true)],
                form_factor: Some("headset".to_owned()),
                card: Some(CardId(48)),
                ..device(
                    61,
                    Direction::Output,
                    HEADSET,
                    "Arctis Nova 7 Analog Stereo",
                )
            },
            Device {
                volume: 29,
                muted: true,
                default: true,
                active_port: Some("analog-input-mic".to_owned()),
                ..device(62, Direction::Input, MIC, "Arctis Nova 7 Mono")
            },
        ],
        streams: vec![Stream {
            id: StreamId(1469),
            direction: Direction::Output,
            application: "lyr-probe".to_owned(),
            title: Some("silence".to_owned()),
            volume: 75,
            muted: false,
            device: Some(DeviceId(61)),
        }],
        cards: vec![Card {
            id: CardId(48),
            name: "alsa_card.usb-SteelSeries_Arctis_Nova_7-00".to_owned(),
            description: "Arctis Nova 7".to_owned(),
            active_profile: Some("output:analog-stereo+input:mono-fallback".to_owned()),
            profiles: vec![
                choice("off", "Off", true),
                choice(
                    "output:analog-stereo+input:mono-fallback",
                    "Analog Stereo Output + Mono Input",
                    true,
                ),
                choice("output:analog-stereo", "Analog Stereo Output", true),
                choice(
                    "output:iec958-stereo",
                    "Digital Stereo (IEC958) Output",
                    false,
                ),
            ],
        }],
    }
}

#[test]
fn the_defaults_are_read_as_waybar_reads_them() {
    let (sink, source) = reading(&machine());
    let sink = sink.expect("the default sink");
    assert_eq!(sink.volume, 50);
    assert!(!sink.muted);
    assert_eq!(sink.name, HEADSET);
    assert_eq!(sink.description, "Arctis Nova 7 Analog Stereo");
    assert_eq!(sink.port, "analog-outputheadset", "port then form factor");
    assert_eq!(sink.source_name, MIC);
    assert!(!sink.bluetooth);
    let source = source.expect("the default source");
    assert_eq!(source.volume, 29);
    assert!(source.muted);
    assert_eq!(source.description, "Arctis Nova 7 Mono");
}

#[test]
fn no_default_is_no_reading() {
    let mut state = machine();
    for device in &mut state.devices {
        device.default = false;
    }
    assert_eq!(reading(&state), (None, None));
    let (sink, _) = reading(&MixerState {
        devices: vec![Device {
            default: true,
            bluetooth: true,
            ..device(7, Direction::Output, "bluez_output.x", "Earbuds")
        }],
        ..MixerState::default()
    });
    let sink = sink.expect("a default sink");
    assert!(sink.bluetooth);
    assert_eq!(sink.port, "Unknown", "no port and no form factor");
    assert_eq!(sink.source_name, "", "no default source");
}

#[test]
fn the_menu_lists_streams_and_devices() {
    let listed = nodes(&machine());
    let [stream] = &listed.playback[..] else {
        panic!("one stream: {:?}", listed.playback);
    };
    assert_eq!(stream.id, "1469");
    assert_eq!(stream.name, "lyr-probe", "its application");
    assert_eq!(stream.volume, 75);
    assert_eq!(
        stream.device.as_deref(),
        Some(HEADSET),
        "the sink it plays to, by name"
    );
    assert!(!stream.default);
    assert!(listed.recording.is_empty());
    assert_eq!(listed.outputs.len(), 2);
    let defaults: Vec<_> = listed.outputs.iter().filter(|node| node.default).collect();
    assert_eq!(defaults.len(), 1);
    assert_eq!(defaults[0].name, "Arctis Nova 7 Analog Stereo");
    assert_eq!(defaults[0].id, HEADSET);
    let [input] = &listed.inputs[..] else {
        panic!("one input: {:?}", listed.inputs);
    };
    assert_eq!(input.id, MIC);
    assert!(input.default && input.muted);
}

#[test]
fn a_stream_is_named_by_its_title_else_its_number() {
    let mut state = machine();
    state.streams[0].application.clear();
    assert_eq!(nodes(&state).playback[0].name, "silence");
    state.streams[0].title = None;
    state.streams[0].device = Some(DeviceId(999));
    let stream = &nodes(&state).playback[0];
    assert_eq!(stream.name, "Stream 1469");
    assert_eq!(stream.device, None, "a device the state does not have");
}

#[test]
fn cards_list_their_available_profiles_in_order() {
    let mut state = machine();
    state.devices.clear();
    let cards = nodes(&state).settings.expect("cards on Linux");
    let [card] = &cards[..] else {
        panic!("one card: {cards:?}");
    };
    assert_eq!(card.id, "alsa_card.usb-SteelSeries_Arctis_Nova_7-00");
    assert_eq!(card.name, "Arctis Nova 7", "its description");
    assert_eq!(card.kind, Kind::Profile);
    let ids: Vec<_> = card.options.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "off",
            "output:analog-stereo+input:mono-fallback",
            "output:analog-stereo"
        ],
        "the server's order, the unavailable one left out"
    );
    assert_eq!(card.chosen(), "Analog Stereo Output + Mono Input");
    // The active profile stays listed when marked unavailable.
    state.cards[0].active_profile = Some("output:iec958-stereo".to_owned());
    let cards = nodes(&state).settings.expect("cards on Linux");
    assert!(
        cards[0]
            .options
            .iter()
            .any(|(id, _)| id == "output:iec958-stereo")
    );
}

#[test]
fn a_device_with_ports_to_choose_is_a_setting() {
    let state = MixerState {
        devices: vec![Device {
            active_port: Some("analog-output-speaker".to_owned()),
            ports: vec![
                choice("analog-output-speaker", "Speakers", true),
                choice("analog-output-headphones", "Headphones", true),
                choice("analog-output-lineout", "Line Out", false),
            ],
            ..device(
                5,
                Direction::Output,
                "alsa_output.pci.analog-stereo",
                "Built-in Audio",
            )
        }],
        ..MixerState::default()
    };
    let settings = nodes(&state).settings.expect("settings on Linux");
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
    // A device with one port has nothing to choose, nor a card-less state.
    let mut single = machine();
    single.cards.clear();
    assert_eq!(nodes(&single).settings, Some(Vec::new()));
}
