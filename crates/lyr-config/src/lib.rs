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
mod schema;

use std::path::{Path, PathBuf};

pub use colour::{Colour, InvalidColour};
pub use hjkl_config::{ConfigError, ConfigSource};
pub use schema::{Bar, Clock, Config, Item, Layer, Layout, Module, Position, Theme, Workspaces};

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
    let config: Config = toml::Value::Table(merged)
        .try_into()
        .map_err(|error: toml::de::Error| invalid(error.message().to_owned()))?;
    validate(&config, platform).map_err(invalid)?;
    Ok(config)
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
    check_placeholders("workspaces.format", &config.workspaces.format, &["name"])
}

/// Checks that every `{…}` in `format` names one of `allowed` and that every
/// brace is closed.
fn check_placeholders(key: &str, format: &str, allowed: &[&str]) -> Result<(), String> {
    let mut rest = format;
    while let Some(open) = rest.find(['{', '}']) {
        if rest[open..].starts_with('}') {
            return Err(format!("{key}: `}}` with no `{{` before it"));
        }
        let after = &rest[open + 1..];
        let Some(close) = after.find('}') else {
            return Err(format!("{key}: `{{` is never closed"));
        };
        let name = &after[..close];
        if !allowed.contains(&name) {
            return Err(format!(
                "{key}: unknown placeholder `{{{name}}}` (expected {})",
                allowed
                    .iter()
                    .map(|name| format!("`{{{name}}}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        rest = &after[close + 1..];
    }
    Ok(())
}

#[cfg(test)]
mod tests;
