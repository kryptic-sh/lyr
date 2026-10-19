//! `lyr import-waybar`: a waybar `config.jsonc` as a lyr config
//! (`docs/plan/05-layout-and-config.md#migration-from-waybar`).
//!
//! The bar's settings, the layout and each module lyr has are carried, their
//! keys renamed from waybar's kebab case. Whether lyr takes a key is lyr's
//! own schema's call: each one is tried by loading it, so a key lyr does not
//! know is reported, never guessed at. waybar's `style.css` is not read; the
//! theme stays lyr's.

use std::fmt::{self, Write as _};
use std::path::Path;

use serde_json::{Map, Value};
use toml::Table;

use crate::{Platform, from_sources};

/// The converted config, and what could not be carried.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Imported {
    /// A lyr config file's text, the skipped parts listed at its top.
    pub toml: String,
    /// What was not carried, and why, one line each.
    pub skipped: Vec<String>,
}

/// Why a file could not be converted.
#[derive(Debug)]
pub enum ImportError {
    /// Not JSON, with comments taken out.
    Json(serde_json::Error),
    /// The top of the file is not an object.
    NotAnObject,
    /// The converted config does not load.
    Invalid(String),
}

impl fmt::Display for ImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Json(error) => write!(f, "not waybar JSON: {error}"),
            Self::NotAnObject => write!(f, "the top of the file is not an object"),
            Self::Invalid(error) => write!(f, "the converted config does not load: {error}"),
        }
    }
}

impl std::error::Error for ImportError {}

impl From<serde_json::Error> for ImportError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

/// `text` with JSONC's comments (`//` to the end of the line, `/* … */`) and
/// trailing commas taken out, as JSON. Text inside strings is kept as it is.
#[must_use]
pub fn strip_jsonc(text: &str) -> String {
    trailing_commas(&comments(text))
}

/// Copies a string literal that `chars` is inside of, its opening quote
/// already copied, through its closing quote.
fn copy_string(chars: &mut impl Iterator<Item = char>, out: &mut String) {
    while let Some(c) = chars.next() {
        out.push(c);
        match c {
            '\\' => {
                if let Some(escaped) = chars.next() {
                    out.push(escaped);
                }
            }
            '"' => break,
            _ => {}
        }
    }
}

/// `text` without its comments.
fn comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                out.push(c);
                copy_string(&mut chars, &mut out);
            }
            '/' if chars.peek() == Some(&'/') => {
                for c in chars.by_ref() {
                    if c == '\n' {
                        out.push('\n');
                        break;
                    }
                }
            }
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                let mut last = '\0';
                for c in chars.by_ref() {
                    if last == '*' && c == '/' {
                        break;
                    }
                    last = c;
                }
            }
            _ => out.push(c),
        }
    }
    out
}

/// `text`, which has no comments, without its trailing commas: a comma whose
/// next character past white space closes an object or an array.
fn trailing_commas(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                out.push(c);
                copy_string(&mut chars, &mut out);
            }
            ',' => {
                let next = chars.clone().find(|c| !c.is_whitespace());
                if !matches!(next, Some('}' | ']')) {
                    out.push(c);
                }
            }
            _ => out.push(c),
        }
    }
    out
}

/// The lyr module a waybar module stands for, from its name and its table.
fn module(name: &str, body: Option<&Map<String, Value>>) -> Option<&'static str> {
    let text = |key: &str| {
        body.and_then(|body| body.get(key))
            .and_then(Value::as_str)
            .unwrap_or_default()
    };
    let base = name.split('#').next().unwrap_or(name);
    Some(match base {
        "battery" => "battery",
        "power-profiles-daemon" => "power_profile",
        "network" => "network",
        "bluetooth" => "bluetooth",
        "hyprland/workspaces" => "workspaces",
        "hyprland/window" => "window",
        "tray" => "tray",
        "clock" => "clock",
        // The microphone is a pulseaudio module shown by its source.
        "pulseaudio" if text("format").contains("{format_source}") => "mic",
        "pulseaudio" => "volume",
        // waybar has no power button or bell of its own: these are the
        // custom modules that stand in for them.
        "custom/power" => "power",
        _ if base.starts_with("custom/") && text("exec").contains("swaync-client") => {
            "notifications"
        }
        _ => return None,
    })
}

/// waybar's `on-*` keys and the binding each is in lyr's `[<module>.on]`.
const BINDINGS: [(&str, &str); 5] = [
    ("on-click", "click"),
    ("on-click-right", "click_right"),
    ("on-click-middle", "click_middle"),
    ("on-scroll-up", "scroll_up"),
    ("on-scroll-down", "scroll_down"),
];

/// A JSON value as TOML; `None` for `null`, which TOML has no way to write.
fn to_toml(value: &Value) -> Option<toml::Value> {
    Some(match value {
        Value::Null => return None,
        Value::Bool(flag) => toml::Value::Boolean(*flag),
        Value::Number(number) => match number.as_i64() {
            Some(integer) => toml::Value::Integer(integer),
            None => toml::Value::Float(number.as_f64()?),
        },
        Value::String(text) => toml::Value::String(text.clone()),
        Value::Array(items) => toml::Value::Array(items.iter().filter_map(to_toml).collect()),
        Value::Object(map) => toml::Value::Table(
            map.iter()
                .filter_map(|(key, value)| Some((key.clone(), to_toml(value)?)))
                .collect(),
        ),
    })
}

/// A format with waybar's bare `{}` named as lyr names it.
fn named(format: &str, name: &str) -> String {
    format.replace("{}", &format!("{{{name}}}"))
}

/// One waybar module's keys as lyr keys for `lyr`, the lyr module, each
/// paired with the waybar key it came from; what cannot be carried at all
/// goes to `skipped`.
fn keys(
    lyr: &str,
    waybar: &str,
    body: &Map<String, Value>,
    skipped: &mut Vec<String>,
) -> Vec<(String, toml::Value, String)> {
    let mut keys = Vec::new();
    for (key, value) in body {
        let from = format!("{waybar}.{key}");
        if let Some((_, binding)) = BINDINGS.iter().find(|(on, _)| on == key) {
            if let Some(command) = value.as_str() {
                let mut exec = Table::new();
                exec.insert("exec".to_owned(), toml::Value::String(command.to_owned()));
                keys.push((format!("on.{binding}"), toml::Value::Table(exec), from));
            }
            continue;
        }
        let renamed = match (lyr, key.as_str()) {
            // lyr always has a tooltip.
            (_, "tooltip") if value.as_bool() == Some(true) => continue,
            (_, "format-icons") => "icons".to_owned(),
            ("battery", "states") => {
                for state in ["warning", "critical"] {
                    if let Some(level) = value.get(state).and_then(to_toml) {
                        keys.push((state.to_owned(), level, from.clone()));
                    }
                }
                continue;
            }
            // The microphone's text is its source's.
            ("mic", "format") if value.as_str() == Some("{format_source}") => continue,
            ("mic", "format-source") => "format".to_owned(),
            ("mic", "format-source-muted") => "format_muted".to_owned(),
            _ => key.replace('-', "_"),
        };
        let value = match (lyr, renamed.as_str(), value.as_str()) {
            ("workspaces", "format", Some(format)) => toml::Value::String(named(format, "name")),
            ("window", "format", Some(format)) => toml::Value::String(named(format, "title")),
            // waybar's clock format is `{:STRFTIME}`; lyr's is the strftime.
            ("clock", "format", Some(format)) => toml::Value::String(
                format
                    .strip_prefix("{:")
                    .and_then(|rest| rest.strip_suffix('}'))
                    .unwrap_or(format)
                    .to_owned(),
            ),
            _ => match to_toml(value) {
                Some(value) => value,
                None => {
                    skipped.push(format!("{from}: null"));
                    continue;
                }
            },
        };
        keys.push((renamed, value, from));
    }
    keys
}

/// Sets `key` (dotted, as `on.click`) in `table` to `value`.
fn set(table: &mut Table, key: &str, value: toml::Value) {
    match key.split_once('.') {
        Some((head, rest)) => {
            let inner = table
                .entry(head.to_owned())
                .or_insert_with(|| toml::Value::Table(Table::new()));
            if let toml::Value::Table(inner) = inner {
                set(inner, rest, value);
            }
        }
        None => {
            table.insert(key.to_owned(), value);
        }
    }
}

/// Whether lyr loads `config` (a whole config file's tables).
fn loads(config: &Table) -> Result<(), String> {
    let text = toml::to_string(config).map_err(|error| error.to_string())?;
    from_sources(Some((Path::new("import.toml"), &text)), Platform::Linux)
        .map(|_| ())
        .map_err(|error| error.to_string())
}

/// Whether `c` is in one of Unicode's private-use areas, where Nerd Font
/// glyphs live.
const fn private_use(c: char) -> bool {
    matches!(c as u32, 0xE000..=0xF8FF | 0xF_0000..=0xF_FFFD | 0x10_0000..=0x10_FFFD)
}

/// `body`, a TOML file, with its private-use characters written as escapes,
/// as lyr's `defaults.toml` writes its glyphs: editors and tools drop them
/// without a trace. An escape means nothing in a literal string (`'…'`), so
/// the escaped file is kept only when it loads to the same config.
fn escape_glyphs(body: &str) -> String {
    let mut escaped = String::with_capacity(body.len());
    for c in body.chars() {
        match c as u32 {
            code if private_use(c) && code > 0xFFFF => {
                write!(escaped, "\\U{code:08X}").expect("a String takes any text");
            }
            code if private_use(c) => {
                write!(escaped, "\\u{code:04X}").expect("a String takes any text");
            }
            _ => escaped.push(c),
        }
    }
    let load =
        |text: &str| from_sources(Some((Path::new("import.toml"), text)), Platform::Linux).ok();
    if load(&escaped).is_some() && load(&escaped) == load(body) {
        escaped
    } else {
        body.to_owned()
    }
}

/// Sets `key` in `config` to `value` when lyr still loads it, and says why
/// in `skipped`, naming the waybar key it came `from`, when not. Tried one
/// at a time, so one key lyr refuses costs only itself.
fn try_set(
    config: &mut Table,
    skipped: &mut Vec<String>,
    key: String,
    value: toml::Value,
    from: String,
) {
    let mut tried = config.clone();
    set(&mut tried, &key, value);
    match loads(&tried) {
        Ok(()) => *config = tried,
        Err(error) => skipped.push(format!("{from}: {error}")),
    }
}

/// waybar's `config.jsonc` (`text`) as a lyr config.
///
/// # Errors
///
/// When `text` is not JSON with comments, its top is not an object, or the
/// result does not load as a lyr config.
pub fn import(text: &str) -> Result<Imported, ImportError> {
    let json: Value = serde_json::from_str(&strip_jsonc(text))?;
    let top = json.as_object().ok_or(ImportError::NotAnObject)?;
    let mut config = Table::new();
    let mut skipped = Vec::new();
    let mut layout_skipped = Vec::new();
    let mut modules: Vec<(String, &'static str)> = Vec::new();
    for (key, value) in top {
        let bar_key = match key.as_str() {
            "position" | "height" | "layer" | "output" => key.as_str(),
            "modules-left" | "modules-center" | "modules-right" => {
                let zone = key.trim_start_matches("modules-");
                let names: Vec<toml::Value> = value
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .filter_map(|name| {
                        let body = top.get(name).and_then(Value::as_object);
                        let Some(lyr) = module(name, body) else {
                            layout_skipped.push(format!("{key}: `{name}` has no lyr module"));
                            return None;
                        };
                        if modules.iter().any(|(_, taken)| *taken == lyr) {
                            layout_skipped.push(format!(
                                "{key}: `{name}` is a second `{lyr}`, which lyr has one of"
                            ));
                            return None;
                        }
                        modules.push((name.to_owned(), lyr));
                        Some(toml::Value::String(lyr.to_owned()))
                    })
                    .collect();
                try_set(
                    &mut config,
                    &mut skipped,
                    format!("layout.{zone}"),
                    toml::Value::Array(names),
                    key.clone(),
                );
                continue;
            }
            _ => continue,
        };
        match to_toml(value) {
            Some(value) => try_set(
                &mut config,
                &mut skipped,
                format!("bar.{bar_key}"),
                value,
                key.clone(),
            ),
            None => skipped.push(format!("{key}: null")),
        }
    }
    skipped.append(&mut layout_skipped);
    // Everything else at the top: waybar's own bar settings, and the tables
    // of modules not in the layout.
    for key in top.keys() {
        let carried = matches!(
            key.as_str(),
            "position"
                | "height"
                | "layer"
                | "output"
                | "modules-left"
                | "modules-center"
                | "modules-right"
        ) || modules.iter().any(|(name, _)| name == key);
        if !carried {
            skipped.push(format!("{key}: not carried"));
        }
    }
    for (waybar, lyr) in &modules {
        let Some(body) = top.get(waybar).and_then(Value::as_object) else {
            continue;
        };
        for (key, value, from) in keys(lyr, waybar, body, &mut skipped) {
            try_set(
                &mut config,
                &mut skipped,
                format!("{lyr}.{key}"),
                value,
                from,
            );
        }
    }
    loads(&config).map_err(ImportError::Invalid)?;
    let mut text = String::from(
        "# Converted from waybar's config.jsonc by `lyr import-waybar`. The theme\n\
         # is lyr's own: waybar's style.css is not read.\n",
    );
    if !skipped.is_empty() {
        text.push_str("#\n# Not carried:\n");
        for line in &skipped {
            text.push_str("#   ");
            text.push_str(line);
            text.push('\n');
        }
    }
    text.push('\n');
    let body = toml::to_string(&config).map_err(|error| ImportError::Invalid(error.to_string()))?;
    text.push_str(&escape_glyphs(&body));
    Ok(Imported {
        toml: text,
        skipped,
    })
}

#[cfg(test)]
mod tests;
