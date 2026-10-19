use std::path::Path;

use super::{Colour, ConfigError, Item, Platform, Position, from_sources};

fn user(text: &str, platform: Platform) -> Result<super::Config, ConfigError> {
    from_sources(Some((Path::new("config.toml"), text)), platform)
}

fn names(items: &[Item]) -> Vec<&str> {
    items
        .iter()
        .map(|item| match item {
            Item::Named(name) => name.as_str(),
            Item::Spacer { .. } => "<spacer>",
        })
        .collect()
}

/// The text of an `Invalid` error, so a test can say which key it named.
fn invalid(result: Result<super::Config, ConfigError>) -> String {
    match result {
        Err(ConfigError::Invalid { message, .. }) => message,
        other => panic!("expected an invalid config, got {other:?}"),
    }
}

#[test]
fn the_defaults_load_on_every_platform() {
    for platform in [Platform::Linux, Platform::Windows, Platform::Macos] {
        from_sources(None, platform).unwrap_or_else(|e| panic!("{platform:?}: {e}"));
    }
}

#[test]
fn the_defaults_are_the_waybar_port() {
    let config = from_sources(None, Platform::Linux).expect("defaults");
    assert_eq!(config.bar.position, Position::Top);
    assert_eq!(config.bar.height, 50);
    assert_eq!(
        names(&config.layout.left),
        ["power", "battery", "power_profile", "network", "bluetooth"]
    );
    assert_eq!(names(&config.layout.center), ["workspaces"]);
    assert_eq!(
        names(&config.layout.right),
        ["tray", "volume", "mic", "clock", "notifications"]
    );
    assert_eq!(config.theme.font, "Hack Nerd Font Propo");
    assert_eq!(
        config.theme.border,
        Colour::parse("#2bc5e3").expect("colour")
    );
    assert_eq!(config.theme.bar_background.a, 0, "transparent bar");
    assert_eq!(config.clock.format, "%Y/%m/%d %H:%M");
}

#[test]
fn modules_a_platform_cannot_run_are_left_out_of_its_defaults() {
    let windows = from_sources(None, Platform::Windows).expect("defaults");
    assert_eq!(
        names(&windows.layout.right),
        ["volume", "mic", "clock", "notifications"]
    );
    let macos = from_sources(None, Platform::Macos).expect("defaults");
    assert_eq!(names(&macos.layout.right), ["volume", "mic", "clock"]);
}

#[test]
fn a_user_table_merges_and_a_user_list_replaces() {
    let config = user(
        "[bar]\nheight = 30\n[layout]\nright = [\"clock\"]\n",
        Platform::Linux,
    )
    .expect("valid");
    assert_eq!(config.bar.height, 30);
    assert_eq!(config.bar.position, Position::Top, "kept from the defaults");
    assert_eq!(names(&config.layout.right), ["clock"]);
    assert_eq!(names(&config.layout.center), ["workspaces"], "kept");
}

#[test]
fn a_user_platform_table_applies_only_on_that_platform() {
    let text = "[platform.windows.bar]\nheight = 40\n[platform.linux.bar]\nheight = 44\n";
    assert_eq!(user(text, Platform::Windows).expect("valid").bar.height, 40);
    assert_eq!(user(text, Platform::Linux).expect("valid").bar.height, 44);
    assert_eq!(user(text, Platform::Macos).expect("valid").bar.height, 50);
}

#[test]
fn the_user_file_wins_over_the_defaults_platform_table() {
    let config = user("[layout]\nright = [\"volume\"]\n", Platform::Macos).expect("valid");
    assert_eq!(names(&config.layout.right), ["volume"]);
}

#[test]
fn spacers_and_separators_are_layout_items() {
    let config = user(
        "[layout]\nleft = [\"power\", { spacer = 20 }, \"separator\"]\n",
        Platform::Linux,
    )
    .expect("valid");
    assert_eq!(config.layout.left[1], Item::Spacer { spacer: 20 });
    assert_eq!(
        names(&config.layout.left),
        ["power", "<spacer>", "separator"]
    );
}

#[test]
fn an_unknown_module_is_named_with_its_key() {
    let message = invalid(user(
        "[layout]\ncenter = [\"workspaces\", \"wifi\"]\n",
        Platform::Linux,
    ));
    assert!(
        message.starts_with("layout.center[1]: unknown module `wifi`"),
        "{message}"
    );
}

#[test]
fn a_module_the_platform_cannot_run_is_an_error() {
    let message = invalid(user("[layout]\nright = [\"tray\"]\n", Platform::Windows));
    assert_eq!(
        message,
        "layout.right[0]: the `tray` module is not supported on windows"
    );
    assert!(user("[layout]\nright = [\"tray\"]\n", Platform::Linux).is_ok());
}

#[test]
fn an_unknown_table_is_an_error() {
    let message = invalid(user("[clocks]\nformat = \"%H\"\n", Platform::Linux));
    assert_eq!(message, "clocks: unknown table");
}

#[test]
fn an_error_inside_a_table_names_its_key() {
    let message = invalid(user("[theme]\nborder = \"teal\"\n", Platform::Linux));
    assert!(
        message.starts_with("theme.border: `teal` is not a colour"),
        "{message}"
    );
    let message = invalid(user("[layout]\nleft = [\"power\", 3]\n", Platform::Linux));
    assert!(message.starts_with("layout.left[1]: "), "{message}");
    let message = invalid(user("[bar]\nheight = \"tall\"\n", Platform::Linux));
    assert!(message.starts_with("bar.height: "), "{message}");
}

#[test]
fn an_unknown_key_is_an_error() {
    let message = invalid(user("[bar]\nhieght = 30\n", Platform::Linux));
    assert!(
        message.starts_with("bar.hieght: unknown field `hieght`"),
        "{message}"
    );
    let message = invalid(user("bar = 3\n", Platform::Linux));
    assert!(message.starts_with("bar: invalid type"), "{message}");
}

#[test]
fn a_bad_colour_is_an_error() {
    let message = invalid(user("[theme]\nborder = \"teal\"\n", Platform::Linux));
    assert!(message.contains("`teal` is not a colour"), "{message}");
}

#[test]
fn values_out_of_range_are_errors() {
    for (text, key) in [
        ("[bar]\nheight = 0\n", "bar.height"),
        ("[theme]\nfont_size = 0\n", "theme.font_size"),
        ("[theme]\nmodule_opacity = 1.5\n", "theme.module_opacity"),
    ] {
        let message = invalid(user(text, Platform::Linux));
        assert!(message.starts_with(key), "{text:?}: {message}");
    }
}

#[test]
fn workspace_format_placeholders_are_checked() {
    assert!(user("[workspaces]\nformat = \"[{name}]\"\n", Platform::Linux).is_ok());
    for format in ["{id}", "{name", "name}", "{}"] {
        let message = invalid(user(
            &format!("[workspaces]\nformat = \"{format}\"\n"),
            Platform::Linux,
        ));
        assert!(
            message.starts_with("workspaces.format: "),
            "{format:?}: {message}"
        );
    }
}

#[test]
fn the_bell_format_takes_only_its_icon() {
    let message = invalid(user(
        "[notifications]\nformat = \"{count}\"\n",
        Platform::Linux,
    ));
    assert!(message.starts_with("notifications.format: "), "{message}");
}

#[test]
fn a_syntax_error_names_its_line_and_column() {
    let message = invalid(user("[bar]\nheight = = 3\n", Platform::Linux));
    assert!(message.contains("line 2, column 10"), "{message}");
}

#[test]
fn the_defaults_write_every_glyph_as_an_escape() {
    // Private-use glyphs are invisible in most editors and are dropped by
    // tools without a trace, so the defaults spell them `\uXXXX`.
    let private_use = |c: char| matches!(u32::from(c), 0xE000..=0xF8FF | 0xF_0000..=0x10_FFFF);
    let raw: Vec<(usize, &str)> = super::DEFAULTS
        .lines()
        .enumerate()
        .filter(|(_, line)| line.chars().any(private_use))
        .map(|(i, line)| (i + 1, line))
        .collect();
    assert!(raw.is_empty(), "raw glyphs in defaults.toml: {raw:?}");
}

#[test]
fn the_defaults_bind_waybars_clicks() {
    use super::{Binding, Builtin};
    let config = from_sources(None, Platform::Linux).expect("defaults");
    let builtin = |binding: &Option<Binding>| match binding {
        Some(Binding::Builtin(builtin)) => Some(*builtin),
        _ => None,
    };
    assert_eq!(
        builtin(&config.power.on.click),
        Some(Builtin::OpenPowerMenu)
    );
    assert_eq!(builtin(&config.volume.on.click), Some(Builtin::ToggleMute));
    assert_eq!(
        builtin(&config.volume.on.click_right),
        Some(Builtin::OpenAudioMenu)
    );
    assert_eq!(builtin(&config.mic.on.scroll_up), Some(Builtin::VolumeUp));
    assert_eq!(
        builtin(&config.notifications.on.click_right),
        Some(Builtin::ToggleDnd)
    );
    assert_eq!(config.volume.scroll_step, 5);
    assert_eq!(builtin(&config.battery.on.click), None, "battery has none");
}

#[test]
fn a_binding_can_run_a_command() {
    use super::Binding;
    let config = user(
        "[clock.on]\nclick = { exec = \"gnome-calendar\" }\n",
        Platform::Linux,
    )
    .expect("valid");
    assert_eq!(
        config.clock.on.click,
        Some(Binding::Exec {
            exec: "gnome-calendar".to_owned()
        })
    );
    assert_eq!(
        config.clock.on.click_right, None,
        "the user table replaces only the keys it names, and adds none"
    );
}

#[test]
fn a_builtin_that_does_nothing_on_its_module_is_an_error() {
    let message = invalid(user(
        "[clock.on]\nscroll_up = \"volume_up\"\n",
        Platform::Linux,
    ));
    assert_eq!(
        message,
        "clock.on.scroll_up: `volume_up` does nothing on the clock module"
    );
}

#[test]
fn an_empty_command_and_an_unknown_action_are_errors() {
    let message = invalid(user(
        "[clock.on]\nclick = { exec = \" \" }\n",
        Platform::Linux,
    ));
    assert_eq!(message, "clock.on.click: `exec` is empty");
    let message = invalid(user(
        "[clock.on]\nclick = \"open_calendar\"\n",
        Platform::Linux,
    ));
    assert!(message.starts_with("clock.on.click: "), "{message}");
}

#[test]
fn a_bad_clock_format_is_an_error() {
    let message = invalid(user("[clock]\nformat = \"%H:%Q\"\n", Platform::Linux));
    assert_eq!(message, "clock.format: `%H:%Q` is not a strftime format");
    assert!(user("[clock]\nformat = \"%H:%M:%S\"\n", Platform::Linux).is_ok());
}
