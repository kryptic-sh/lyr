//! Icons as pictures, for the tray's items and the notifications' icons:
//! a name through the user's icon theme, or a file by its path, each read
//! again only once it changes.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crcbl_image::theme::{FileVersion, HICOLOR, IconThemes, SearchPath};
use crcbl_image::{Limits, Rgba8Image, png, svg, xpm};
use lyr_core::draw::Picture;

/// The largest icon read, each way: more than a tray ever draws, and a stop
/// for an item or a file that claims gigapixels.
const MAX_SIDE: u32 = 1024;

pub const LIMITS: Limits = Limits {
    max_width: MAX_SIDE,
    max_height: MAX_SIDE,
    max_bytes: MAX_SIDE as usize * MAX_SIDE as usize * 4,
};

/// How long a name that found no icon is remembered before it is looked up
/// again, so a theme installed while the bar runs is found.
const MISS_RETRY: Duration = Duration::from_secs(60);

/// The icon themes and the icon files read so far.
#[derive(Debug)]
pub struct Icons {
    /// The theme lookups, by the item's `IconThemePath` (empty for none),
    /// since each searches that path first.
    themes: HashMap<String, IconThemes>,
    /// The user's icon theme.
    theme: String,
    /// Each file read, with its version then.
    files: HashMap<PathBuf, (FileVersion, Option<Picture>)>,
}

impl Default for Icons {
    fn default() -> Self {
        Self::new(gtk_icon_theme().unwrap_or_else(|| HICOLOR.to_owned()))
    }
}

impl Icons {
    /// Lookups in `theme`, then what it inherits and `hicolor`.
    #[must_use]
    pub fn new(theme: String) -> Self {
        Self {
            themes: HashMap::new(),
            theme,
            files: HashMap::new(),
        }
    }

    /// The icon `name` (or the file it names) at `size`, looked for first
    /// under `theme_path` when there is one.
    pub fn named(&mut self, name: &str, theme_path: &str, size: u32) -> Option<Picture> {
        let theme = &self.theme;
        let path = self
            .themes
            .entry(theme_path.to_owned())
            .or_insert_with(|| {
                let search = SearchPath::from_environment()
                    .prepend((!theme_path.is_empty()).then(|| PathBuf::from(theme_path)));
                IconThemes::new(search, theme.clone(), MISS_RETRY)
            })
            .lookup(name, size, 1)?;
        self.file(&path, size)
    }

    /// The icon file at `path`, read again only once it changed.
    fn file(&mut self, path: &Path, size: u32) -> Option<Picture> {
        let version = FileVersion::of(path)?;
        if let Some((read, picture)) = self.files.get(path)
            && *read == version
        {
            return picture.clone();
        }
        let picture = std::fs::read(path)
            .ok()
            .and_then(|bytes| decode(path, &bytes, size));
        self.files
            .insert(path.to_owned(), (version, picture.clone()));
        picture
    }
}

/// An icon file's bytes as a picture, by its extension; an SVG is drawn
/// `size` pixels square.
fn decode(path: &Path, bytes: &[u8], size: u32) -> Option<Picture> {
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();
    let image = match extension.as_str() {
        "png" => png::decode(bytes, &LIMITS),
        "svg" => svg::decode(bytes, size, 1, &LIMITS),
        "xpm" => xpm::decode(bytes, &LIMITS),
        _ => return None,
    };
    image.ok().and_then(picture)
}

pub fn picture(image: Rgba8Image) -> Option<Picture> {
    let (width, height) = (image.width(), image.height());
    Picture::new(width, height, image.into_pixels())
}

/// The icon theme GTK applications use, from GTK 3's settings file, else
/// GTK 4's: what waybar's tray takes through GTK.
fn gtk_icon_theme() -> Option<String> {
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|dir| !dir.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
    ["gtk-3.0", "gtk-4.0"].into_iter().find_map(|gtk| {
        let text = std::fs::read_to_string(config.join(gtk).join("settings.ini")).ok()?;
        icon_theme_setting(&text)
    })
}

/// `gtk-icon-theme-name` in a GTK `settings.ini`'s `[Settings]` group.
pub fn icon_theme_setting(text: &str) -> Option<String> {
    let mut in_settings = false;
    for line in text.lines().map(str::trim) {
        if let Some(group) = line
            .strip_prefix('[')
            .and_then(|line| line.strip_suffix(']'))
        {
            in_settings = group == "Settings";
        } else if in_settings
            && let Some((key, value)) = line.split_once('=')
            && key.trim() == "gtk-icon-theme-name"
        {
            let value = value.trim().trim_matches('"');
            return (!value.is_empty()).then(|| value.to_owned());
        }
    }
    None
}

#[cfg(test)]
mod tests;
