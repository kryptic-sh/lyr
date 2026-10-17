use lyr_config::{Config, Format, Icon, Platform, from_sources};

use super::{Sink, Source, mic, volume};

fn defaults() -> Config {
    from_sources(None, Platform::Linux).expect("defaults")
}

fn speakers(volume: u32) -> Sink {
    Sink {
        volume,
        muted: false,
        name: "alsa_output.analog-stereo".to_owned(),
        source_name: "alsa_input.analog-stereo".to_owned(),
        port: "analog-output-speaker".to_owned(),
        description: "Built-in Audio".to_owned(),
        bluetooth: false,
    }
}

#[test]
fn speakers_take_the_volume_ramp() {
    let config = defaults().volume;
    assert_eq!(volume(&speakers(10), &config).text, "\u{F026} 10%");
    assert_eq!(volume(&speakers(50), &config).text, "\u{F027} 50%");
    assert_eq!(volume(&speakers(90), &config).text, "\u{F028} 90%");
    assert_eq!(volume(&speakers(150), &config).text, "\u{F028} 150%");
}

#[test]
fn a_port_kind_in_the_port_name_picks_its_icon() {
    let config = defaults().volume;
    let mut headphones = speakers(40);
    headphones.port = "analog-output-Headphones".to_owned();
    assert_eq!(volume(&headphones, &config).text, "\u{F025} 40%");
    let mut car = speakers(40);
    car.port = "headset-output".to_owned();
    car.port.push_str("car");
    assert_eq!(
        volume(&car, &config).text,
        "\u{F025} 40%",
        "headset comes before car in waybar's order"
    );
}

#[test]
fn muted_shows_the_muted_format() {
    let config = defaults().volume;
    let mut muted = speakers(40);
    muted.muted = true;
    assert_eq!(volume(&muted, &config).text, "\u{EEE8} 40%");
}

#[test]
fn a_muted_icon_name_wins_while_muted() {
    let mut config = defaults().volume;
    config.format_muted = Format::parse("{icon}").expect("valid");
    config
        .icons
        .insert("speaker-muted".to_owned(), Icon::One("x".to_owned()));
    let mut muted = speakers(40);
    assert_eq!(volume(&muted, &config).text, "\u{F027} 40%");
    muted.muted = true;
    assert_eq!(volume(&muted, &config).text, "x");
}

#[test]
fn a_muted_bluetooth_sink_falls_back_to_the_plain_muted_format() {
    let mut config = defaults().volume;
    config.format_bluetooth = Some(Format::parse("bt {volume}").expect("valid"));
    let mut sink = speakers(40);
    sink.bluetooth = true;
    assert_eq!(volume(&sink, &config).text, "bt 40");
    sink.muted = true;
    assert_eq!(volume(&sink, &config).text, "\u{EEE8} 40%");
}

#[test]
fn the_tooltip_is_the_description_unless_a_format_is_set() {
    let mut config = defaults().volume;
    assert_eq!(volume(&speakers(40), &config).tooltip, "Built-in Audio");
    config.tooltip_format = Some(Format::parse("{desc}: {volume}%").expect("valid"));
    assert_eq!(
        volume(&speakers(40), &config).tooltip,
        "Built-in Audio: 40%"
    );
}

#[test]
fn the_mic_shows_its_own_icon_and_description() {
    let config = defaults().mic;
    let mut source = Source {
        volume: 65,
        muted: false,
        description: "USB Microphone".to_owned(),
    };
    assert_eq!(mic(&source, &config).text, "\u{F130} 65%");
    assert_eq!(mic(&source, &config).tooltip, "USB Microphone");
    source.muted = true;
    assert_eq!(mic(&source, &config).text, "\u{F131} 65%");
}
