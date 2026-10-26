//! An application's desktop entry, as the Desktop Entry Specification
//! places it: `<id>.desktop` under `applications` in each XDG data
//! directory, the user's first. Only what a notification or a media
//! player needs is read: the entry's icon and name.
//!
//! An id that the specification maps to a subdirectory (`kde-foo` as
//! `kde/foo.desktop`) is looked for only as the flat file.

use std::ffi::OsString;
use std::path::PathBuf;

/// The `Icon` of the first of `ids` with a desktop entry: a name in the
/// icon theme, or a path.
#[must_use]
pub fn icon<'a>(ids: impl IntoIterator<Item = &'a str>) -> Option<String> {
    icon_in(&dirs(), ids)
}

/// The data directories desktop entries are looked for under, from the
/// environment ([`data_dirs`]).
#[must_use]
pub fn dirs() -> Vec<PathBuf> {
    data_dirs(|name| std::env::var_os(name))
}

/// [`icon`], looking in `dirs`.
pub fn icon_in<'a>(dirs: &[PathBuf], ids: impl IntoIterator<Item = &'a str>) -> Option<String> {
    first_in(dirs, ids, &["Icon"])
}

/// The name the first of `ids` with a desktop entry shows, looking in
/// `dirs`: its `X-GNOME-FullName`, else its `Name`, as GLib's
/// `g_app_info_get_display_name` gives it (unlocalized).
pub fn name_in<'a>(dirs: &[PathBuf], ids: impl IntoIterator<Item = &'a str>) -> Option<String> {
    first_in(dirs, ids, &["X-GNOME-FullName", "Name"])
}

/// The first of `keys` set in the first of `ids`' entries under `dirs`
/// that sets one, not empty.
fn first_in<'a>(
    dirs: &[PathBuf],
    ids: impl IntoIterator<Item = &'a str>,
    keys: &[&str],
) -> Option<String> {
    ids.into_iter().filter(|id| is_id(id)).find_map(|id| {
        dirs.iter().find_map(|dir| {
            let path = dir.join("applications").join(format!("{id}.desktop"));
            let text = std::fs::read_to_string(path).ok()?;
            keys.iter().find_map(|key| {
                let value = crate::ini::value(&text, "Desktop Entry", key)?;
                (!value.is_empty()).then(|| value.to_owned())
            })
        })
    })
}

/// Whether `id` can only name a file in the directory it is joined to:
/// the ids come from other programs (a notification's `desktop-entry` and
/// `app_name`), so one such as `../../x` must not reach elsewhere.
fn is_id(id: &str) -> bool {
    !id.is_empty() && !id.starts_with('.') && !id.contains(['/', '\0'])
}

/// The Base Directory Specification's data directories, with `var`
/// reading the environment: `$XDG_DATA_HOME` (else `~/.local/share`), then
/// each of `$XDG_DATA_DIRS` (else `/usr/local/share` and `/usr/share`).
fn data_dirs(var: impl Fn(&str) -> Option<OsString>) -> Vec<PathBuf> {
    let set = |name: &str| var(name).filter(|value| !value.is_empty());
    let mut dirs = Vec::new();
    match set("XDG_DATA_HOME") {
        Some(home) => dirs.push(PathBuf::from(home)),
        None => dirs.extend(set("HOME").map(|home| PathBuf::from(home).join(".local/share"))),
    }
    // Colon-separated by the specification on every system, which is why
    // this is not `std::env::split_paths`.
    let shared = set("XDG_DATA_DIRS").unwrap_or_else(|| "/usr/local/share:/usr/share".into());
    dirs.extend(
        shared
            .to_string_lossy()
            .split(':')
            .filter(|dir| !dir.is_empty())
            .map(PathBuf::from),
    );
    dirs
}

#[cfg(test)]
mod tests;
