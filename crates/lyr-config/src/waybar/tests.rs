use std::path::Path;

use super::{ImportError, import, strip_jsonc};
use crate::actions::Binding;
use crate::{Format, Platform, from_sources};

fn format(text: &str) -> Format {
    Format::parse(text).expect("a format")
}

#[test]
fn comments_and_trailing_commas_go_but_strings_stay() {
    let jsonc = r#"{
        // a line comment, with a "quote"
        "a": "keep // this, and } this", /* a block
        comment */ "b": [1, 2,],
        "c": "an \"escaped\" quote, }",
    }"#;
    let json: serde_json::Value = serde_json::from_str(&strip_jsonc(jsonc)).expect("JSON");
    assert_eq!(json["a"], "keep // this, and } this");
    assert_eq!(json["b"], serde_json::json!([1, 2]));
    assert_eq!(json["c"], "an \"escaped\" quote, }");
}

/// The dotfiles' shape, cut down: every kind of module and key the converter
/// handles, and some it cannot carry.
const CONFIG: &str = r#"{
    "position": "bottom",
    "height": 40,
    "mod": "dock",
    "modules-left": ["custom/power", "battery", "cpu"],
    "modules-center": ["hyprland/workspaces"],
    "modules-right": ["tray", "pulseaudio", "pulseaudio#microphone", "clock", "custom/notification"],
    "custom/power": { "format": "P", "tooltip-format": "Power", "on-click": "krypt menu power" },
    "battery": {
        "format": "{icon} {capacity}%",
        "format-icons": ["a", "b"],
        "states": { "warning": 25, "critical": 5 },
        "tooltip": true,
    },
    "hyprland/workspaces": { "all-outputs": false, "format": "<{}>" },
    "tray": { "icon-size": 18, "spacing": 10 },
    "pulseaudio": { "format": "{volume}%", "scroll-step": 2 },
    "pulseaudio#microphone": {
        "format": "{format_source}",
        "format-source": "M {volume}%",
        "format-source-muted": "m {volume}%",
    },
    "clock": { "format": "{:%H:%M}", "tooltip-format": "<tt>{calendar}</tt>" },
    "custom/notification": {
        "format": "{icon}",
        "format-icons": { "none": "N", "inhibited-none": "I" },
        "exec": "swaync-client -swb",
        "tooltip": false,
    },
}"#;

#[test]
fn the_dotfiles_shape_converts_and_loads() {
    let imported = import(CONFIG).expect("converted");
    let config = from_sources(
        Some((Path::new("import.toml"), &imported.toml)),
        Platform::Linux,
    )
    .expect("the result loads");
    assert_eq!(config.bar.height, 40);
    assert_eq!(config.bar.position, crate::Position::Bottom);
    assert_eq!(
        config.layout.left,
        [
            crate::Item::Named("power".to_owned()),
            crate::Item::Named("battery".to_owned())
        ]
    );
    assert_eq!(
        config.layout.right,
        [
            crate::Item::Named("tray".to_owned()),
            crate::Item::Named("volume".to_owned()),
            crate::Item::Named("mic".to_owned()),
            crate::Item::Named("clock".to_owned()),
            crate::Item::Named("notifications".to_owned()),
        ]
    );
    assert_eq!(config.battery.icons, ["a", "b"]);
    assert_eq!((config.battery.warning, config.battery.critical), (25, 5));
    assert!(!config.workspaces.all_outputs);
    assert_eq!(config.workspaces.format, format("<{name}>"));
    assert_eq!(config.volume.scroll_step, 2);
    assert_eq!(config.mic.format, format("M {volume}%"));
    assert_eq!(config.mic.format_muted, format("m {volume}%"));
    assert_eq!(config.clock.format, "%H:%M");
    assert_eq!(
        config.notifications.icons.get("none"),
        Some(&crate::Icon::One("N".to_owned()))
    );
    assert_eq!(config.power.tooltip_format, format("Power"));
    assert_eq!(
        config.power.on.click,
        Some(Binding::Exec {
            exec: "krypt menu power".to_owned()
        })
    );
}

#[test]
fn what_lyr_cannot_carry_is_listed() {
    let imported = import(CONFIG).expect("converted");
    let skipped = imported.skipped.join("\n");
    for expected in [
        "`cpu` has no lyr module",
        "tray.icon-size",
        "clock.tooltip-format",
        "custom/notification.exec",
        "custom/notification.tooltip",
    ] {
        assert!(skipped.contains(expected), "{expected} in:\n{skipped}");
    }
    assert!(
        !skipped.contains("battery.tooltip"),
        "a true tooltip is lyr's own"
    );
    assert!(
        imported
            .skipped
            .iter()
            .any(|line| line == "mod: not carried"),
        "waybar's own bar keys are listed"
    );
    assert!(imported.toml.starts_with("# Converted from waybar"));
    assert!(imported.toml.contains("#   tray.icon-size"));
    assert!(
        !skipped.contains("tray.spacing"),
        "lyr's tray takes waybar's spacing"
    );
}

#[test]
fn sway_and_niri_modules_are_lyrs_workspaces_and_window() {
    for compositor in ["sway", "niri"] {
        let waybar = format!(
            r#"{{ "modules-center": ["{compositor}/workspaces", "{compositor}/window"],
                 "{compositor}/workspaces": {{ "all-outputs": false }} }}"#
        );
        let imported = import(&waybar).expect("converted");
        let config = from_sources(
            Some((Path::new("import.toml"), &imported.toml)),
            Platform::Linux,
        )
        .expect("the result loads");
        assert_eq!(
            config.layout.center,
            [
                crate::Item::Named("workspaces".to_owned()),
                crate::Item::Named("window".to_owned())
            ],
            "{compositor}"
        );
        assert!(!config.workspaces.all_outputs, "{compositor}");
    }
}

#[test]
fn a_second_module_of_one_kind_and_bad_json_are_refused() {
    let doubled = r#"{ "modules-left": ["battery", "battery#bat2"] }"#;
    let imported = import(doubled).expect("converted");
    assert!(
        imported
            .skipped
            .iter()
            .any(|line| line.contains("second `battery`")),
        "{:?}",
        imported.skipped
    );
    assert!(matches!(import("{ nope"), Err(ImportError::Json(_))));
    assert!(matches!(import("[1, 2]"), Err(ImportError::NotAnObject)));
}

#[test]
fn glyphs_are_written_as_escapes() {
    let imported = import(
        "{ \"modules-left\": [\"battery\"], \"battery\": { \"format\": \"\u{F240} \u{F140B} {capacity}%\" } }",
    )
    .expect("converted");
    assert!(
        imported
            .toml
            .contains(r#"format = "\uF240 \U000F140B {capacity}%""#),
        "{}",
        imported.toml
    );
    assert!(!imported.toml.contains('\u{F240}'));
    let config = from_sources(
        Some((Path::new("import.toml"), &imported.toml)),
        Platform::Linux,
    )
    .expect("loads");
    assert_eq!(
        config.battery.format,
        format("\u{F240} \u{F140B} {capacity}%")
    );
}
