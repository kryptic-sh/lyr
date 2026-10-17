//! Config for lyr: the TOML schema, the embedded defaults and loading
//! (`docs/plan/05-layout-and-config.md`).
//!
//! The defaults (`defaults.toml`, a port of a waybar setup) are the base. On
//! top of them come, in order: the defaults' `[platform.<os>]` table, the user
//! file, and the user file's own `[platform.<os>]` table, so one user file can
//! serve every platform. A table merges key by key and a list replaces the
//! whole list. The result is then checked as a whole before anything uses it.
//!
//! Only the bar, layout, workspaces, clock and theme tables exist so far. The
//! other modules' tables, reload and its diff come with the rest of milestone
//! M3 and with M4 (`docs/plan/15-milestones.md`).

mod colour;
mod format;
mod schema;

use std::path::{Path, PathBuf};

pub use colour::{Colour, InvalidColour};
pub use format::{Format, InvalidFormat};
pub use hjkl_config::{ConfigError, ConfigSource};
pub use schema::{
    Bar, Battery, Bluetooth, Clock, Config, Item, Layer, Layout, Module, NamedIcons, Position,
    PowerProfile, Theme, Workspaces,
};

/// The built-in config.
const DEFAULTS: &str = include_str!("defaults.toml");

/// The application name, which is the leaf of the config directory.
const APPLICATION: &str = "lyr";

/// The key that holds per-platform override tables.
const PLATFORM_KEY: &str = "platform";

/// The platform whose defaults and overrides apply.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    Linux,
    Windows,
    Macos,
}

impl Platform {
    /// The platform lyr was built for. Unix systems other than macOS take the
    /// Linux defaults.
    #[must_use]
    pub const fn current() -> Self {
        if cfg!(windows) {
            Self::Windows
        } else if cfg!(target_os = "macos") {
            Self::Macos
        } else {
            Self::Linux
        }
    }

    /// The `[platform.<key>]` table that applies here.
    const fn key(self) -> &'static str {
        match self {
            Self::Linux => "linux",
            Self::Windows => "windows",
            Self::Macos => "macos",
        }
    }

    /// Whether a module can work here (`docs/plan/11-platforms.md`).
    const fn supports(self, module: Module) -> bool {
        !matches!(
            (self, module),
            (Self::Windows | Self::Macos, Module::Tray) | (Self::Macos, Module::Notifications)
        )
    }
}

/// Loads the config for `platform`: from `path` when one is given (it must
/// exist), otherwise from `~/.config/lyr/config.toml` when that exists, and
/// from the defaults alone when it does not.
///
/// # Errors
///
/// When the file cannot be read, is not valid TOML, or does not describe a
/// valid config. The error names the file and, where it can, the key.
pub fn load(
    path: Option<&Path>,
    platform: Platform,
) -> Result<(Config, ConfigSource), ConfigError> {
    let path = match path {
        Some(path) => Some(path.to_owned()),
        None => match hjkl_config::config_dir(APPLICATION) {
            Ok(dir) => Some(dir.join("config.toml")).filter(|path| path.exists()),
            Err(ConfigError::NoHomeDir) => None,
            Err(error) => return Err(error),
        },
    };
    let Some(path) = path else {
        return Ok((from_sources(None, platform)?, ConfigSource::Defaults));
    };
    let text = std::fs::read_to_string(&path).map_err(|source| ConfigError::Io {
        path: path.clone(),
        source,
    })?;
    let config = from_sources(Some((&path, &text)), platform)?;
    Ok((config, ConfigSource::File(path)))
}

/// Builds the config from the defaults and an optional user file's path and
/// text, for `platform`.
///
/// # Errors
///
/// As [`load`], apart from reading the file.
pub fn from_sources(
    user: Option<(&Path, &str)>,
    platform: Platform,
) -> Result<Config, ConfigError> {
    let defaults_path = PathBuf::from("<built-in defaults>");
    let mut merged = layer(parse(&defaults_path, DEFAULTS)?, platform);
    let path = match user {
        Some((path, text)) => {
            merge(&mut merged, layer(parse(path, text)?, platform));
            path.to_owned()
        }
        None => defaults_path,
    };
    let invalid = |message: String| ConfigError::Invalid {
        path: path.clone(),
        message,
    };
    let config = typed(merged).map_err(invalid)?;
    validate(&config, platform).map_err(invalid)?;
    Ok(config)
}

/// Turns the merged table into a [`Config`] one table at a time, so an error
/// from inside a table names it.
fn typed(mut merged: toml::Table) -> Result<Config, String> {
    let config = Config {
        bar: section(&mut merged, "bar")?,
        layout: section(&mut merged, "layout")?,
        workspaces: section(&mut merged, "workspaces")?,
        clock: section(&mut merged, "clock")?,
        battery: section(&mut merged, "battery")?,
        power_profile: section(&mut merged, "power_profile")?,
        bluetooth: section(&mut merged, "bluetooth")?,
        theme: section(&mut merged, "theme")?,
    };
    match merged.keys().next() {
        Some(key) => Err(format!("{key}: unknown table")),
        None => Ok(config),
    }
}

/// Takes the table `key` out of `merged` and deserializes it.
fn section<T: serde::de::DeserializeOwned>(
    merged: &mut toml::Table,
    key: &str,
) -> Result<T, String> {
    let value = merged
        .remove(key)
        .ok_or_else(|| format!("{key}: missing"))?;
    value
        .try_into()
        .map_err(|error: toml::de::Error| format!("{key}: {}", error.message()))
}

/// Parses TOML text into a table. A syntax error's message is toml's own,
/// which carries the line, the column and the offending line.
fn parse(path: &Path, text: &str) -> Result<toml::Table, ConfigError> {
    toml::from_str(text).map_err(|error: toml::de::Error| ConfigError::Invalid {
        path: path.to_owned(),
        message: error.to_string(),
    })
}

/// Merges `from` over `into`: a table merges key by key, and anything else,
/// a list included, replaces what was there. The same rule as
/// `hjkl_config::load_layered`, whose merge is not public in hjkl-config
/// 0.42.2, so lyr cannot call it and layer the platform tables between files.
fn merge(into: &mut toml::Table, from: toml::Table) {
    for (key, value) in from {
        match (into.get_mut(&key), value) {
            (Some(toml::Value::Table(into)), toml::Value::Table(from)) => merge(into, from),
            (_, value) => {
                into.insert(key, value);
            }
        }
    }
}

/// Takes a file's `[platform]` table out and merges the part for `platform`
/// over the rest of the file. The other platforms' tables are dropped.
fn layer(mut table: toml::Table, platform: Platform) -> toml::Table {
    if let Some(toml::Value::Table(mut platforms)) = table.remove(PLATFORM_KEY)
        && let Some(toml::Value::Table(overrides)) = platforms.remove(platform.key())
    {
        merge(&mut table, overrides);
    }
    table
}

/// The checks the types alone cannot make. Returns the first problem, naming
/// its key.
fn validate(config: &Config, platform: Platform) -> Result<(), String> {
    for (key, item) in config.layout.items() {
        let Item::Named(name) = item else { continue };
        if name == "separator" {
            continue;
        }
        let Some(module) = Module::from_name(name) else {
            let known: Vec<&str> = Module::ALL.iter().map(|module| module.name()).collect();
            return Err(format!(
                "{key}: unknown module `{name}` (expected `separator`, \
                 `{{ spacer = N }}` or one of: {})",
                known.join(", ")
            ));
        };
        if !platform.supports(module) {
            return Err(format!(
                "{key}: the `{name}` module is not supported on {}",
                platform.key()
            ));
        }
    }
    if config.bar.height == 0 {
        return Err("bar.height: must be more than 0".to_owned());
    }
    if config.theme.font_size == 0 {
        return Err("theme.font_size: must be more than 0".to_owned());
    }
    if !(0.0..=1.0).contains(&config.theme.module_opacity) {
        return Err(format!(
            "theme.module_opacity: {} is outside 0 – 1",
            config.theme.module_opacity
        ));
    }
    check_placeholders("workspaces.format", &config.workspaces.format, &["name"])?;
    let battery = &config.battery;
    let bar_formats = [
        ("battery.format", Some(&battery.format)),
        ("battery.format_charging", battery.format_charging.as_ref()),
        ("battery.format_plugged", battery.format_plugged.as_ref()),
        ("battery.format_full", battery.format_full.as_ref()),
    ];
    for (key, format) in bar_formats {
        if let Some(format) = format {
            check_placeholders(key, format, &["icon", "capacity", "power", "time"])?;
        }
    }
    check_placeholders(
        "battery.tooltip_format",
        &battery.tooltip_format,
        &["timeTo", "capacity", "power", "time"],
    )?;
    let profile = &config.power_profile;
    for (key, format) in [
        ("power_profile.format", &profile.format),
        ("power_profile.tooltip_format", &profile.tooltip_format),
    ] {
        check_placeholders(key, format, &["icon", "profile", "driver"])?;
    }
    let bluetooth = &config.bluetooth;
    let bluetooth_formats = [
        ("bluetooth.format", Some(&bluetooth.format)),
        ("bluetooth.format_on", bluetooth.format_on.as_ref()),
        ("bluetooth.format_off", bluetooth.format_off.as_ref()),
        (
            "bluetooth.format_disabled",
            bluetooth.format_disabled.as_ref(),
        ),
        (
            "bluetooth.format_connected",
            bluetooth.format_connected.as_ref(),
        ),
        (
            "bluetooth.format_connected_battery",
            bluetooth.format_connected_battery.as_ref(),
        ),
        ("bluetooth.tooltip_format", Some(&bluetooth.tooltip_format)),
    ];
    for (key, format) in bluetooth_formats {
        if let Some(format) = format {
            check_placeholders(
                key,
                format,
                &[
                    "icon",
                    "status",
                    "num_connections",
                    "device_alias",
                    "device_battery_percentage",
                ],
            )?;
        }
    }
    Ok(())
}

/// Checks that every placeholder in `format` is one of `allowed`.
fn check_placeholders(key: &str, format: &Format, allowed: &[&str]) -> Result<(), String> {
    let Some(name) = format.placeholders().find(|name| !allowed.contains(name)) else {
        return Ok(());
    };
    let expected: Vec<String> = allowed.iter().map(|name| format!("`{{{name}}}`")).collect();
    Err(format!(
        "{key}: unknown placeholder `{{{name}}}` (expected {})",
        expected.join(", ")
    ))
}

#[cfg(test)]
mod tests;
