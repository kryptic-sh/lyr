//! Icons as pictures, for the tray's items and the notifications' icons
//! and images: a name through the user's icon theme, or a file by its path
//! or its `file://` URI.

use std::collections::HashMap;
use std::ffi::OsString;
use std::io::Read;
use std::os::unix::ffi::OsStringExt;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;

use crcbl_image::theme::{FileVersion, HICOLOR, IconThemes, SearchPath};
use crcbl_image::{Limits, Rgba8Image, jpeg, png, svg, webp, xpm};
use lyr_core::draw::Picture;

/// The largest icon read, each way: more than a tray ever draws, and a stop
/// for an item or a file that claims gigapixels.
const MAX_SIDE: u32 = 1024;

pub const LIMITS: Limits = Limits {
    max_width: MAX_SIDE,
    max_height: MAX_SIDE,
    max_bytes: MAX_SIDE as usize * MAX_SIDE as usize * 4,
};

/// The largest file read for a picture: an image within [`LIMITS`] stored
/// uncompressed is a little over `max_bytes`, and twice that leaves room for
/// any format's overhead. A larger file holds a picture the decoders would
/// refuse, so it is not read at all: a wallpaper sent as a notification's
/// image would otherwise be read whole only to be refused.
const MAX_FILE_BYTES: u64 = 2 * LIMITS.max_bytes as u64;

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
    /// Lookups in the user's icon theme: GTK's `settings.ini`'s, else the
    /// one GNOME keeps in GSettings ([`learn_icon_theme`]), else `hicolor`.
    fn default() -> Self {
        Self::new(
            gtk_icon_theme()
                .or_else(|| GSETTINGS_THEME.get().cloned().flatten())
                .unwrap_or_else(|| HICOLOR.to_owned()),
        )
    }
}

/// The icon theme GNOME keeps in GSettings, once [`learn_icon_theme`] has
/// read it: `None` inside when there was none to read.
static GSETTINGS_THEME: OnceLock<Option<String>> = OnceLock::new();

/// How long the desktop portal is given to answer.
const PORTAL_TIMEOUT: Duration = Duration::from_secs(1);

/// Reads the icon theme GNOME keeps in GSettings (`org.gnome.desktop.interface`
/// `icon-theme`), where GTK's `settings.ini` names none, for every
/// [`Icons::default`] after it. GNOME's GTK applications take their theme
/// from there, and writes no `settings.ini`. It is read through the desktop
/// portal's `Settings` on the session bus, so no program is run; a desktop
/// without the portal keeps `hicolor`. Returns the theme it read, if any.
pub async fn learn_icon_theme() -> Option<String> {
    if gtk_icon_theme().is_some() {
        return None;
    }
    if let Some(theme) = GSETTINGS_THEME.get() {
        return theme.clone();
    }
    let connection = zbus::connection::Builder::session()
        .method_timeout(PORTAL_TIMEOUT)
        .build()
        .await;
    let theme = match connection {
        Ok(connection) => portal_icon_theme(&connection).await,
        Err(_) => None,
    };
    GSETTINGS_THEME.get_or_init(|| theme).clone()
}

/// The desktop portal's settings.
#[zbus::proxy(
    interface = "org.freedesktop.portal.Settings",
    default_service = "org.freedesktop.portal.Desktop",
    default_path = "/org/freedesktop/portal/desktop"
)]
trait PortalSettings {
    fn read_one(&self, namespace: &str, key: &str) -> zbus::Result<zbus::OwnedVariant>;
    /// The older form, before `ReadOne`: the value in a variant of its own.
    fn read(&self, namespace: &str, key: &str) -> zbus::Result<zbus::OwnedVariant>;
}

/// GNOME's icon theme as the portal on `connection` reads it.
async fn portal_icon_theme(connection: &zbus::Connection) -> Option<String> {
    const NAMESPACE: &str = "org.gnome.desktop.interface";
    const KEY: &str = "icon-theme";
    let portal = PortalSettingsProxy::new(connection).await.ok()?;
    let value = match portal.read_one(NAMESPACE, KEY).await {
        Ok(value) => value,
        Err(_) => portal.read(NAMESPACE, KEY).await.ok()?,
    };
    // `Read` nests the value in one more variant.
    let value: &zbus::Variant<'_> = match &*value {
        zbus::Variant::Variant(inner) => inner,
        value => value,
    };
    let theme = String::try_from(value.try_clone().ok()?).ok()?;
    (!theme.is_empty()).then_some(theme)
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

    /// A notification's `app_icon` or `image-path` at `size`: a `file://`
    /// URI (the one scheme the spec allows), an absolute path, or a name in
    /// the icon theme, as plan 09 has it. A file is read every time rather
    /// than kept, since each notification can name a new one (a
    /// screenshot's, say) and the cache would only grow.
    pub fn uri_or_name(&mut self, value: &str, size: u32) -> Option<Picture> {
        if value.starts_with("file://") {
            return read(&file_uri(value)?, size);
        }
        if value.is_empty() {
            return None;
        }
        if Path::new(value).is_absolute() {
            return read(Path::new(value), size);
        }
        self.named(value, "", size)
    }

    /// The icon file at `path`, read again only once it changed.
    fn file(&mut self, path: &Path, size: u32) -> Option<Picture> {
        let version = FileVersion::of(path)?;
        if let Some((read, picture)) = self.files.get(path)
            && *read == version
        {
            return picture.clone();
        }
        let picture = read(path, size);
        self.files
            .insert(path.to_owned(), (version, picture.clone()));
        picture
    }
}

/// The picture in the file at `path`, an SVG drawn `size` pixels square;
/// `None` for a file larger than [`MAX_FILE_BYTES`].
pub fn read(path: &Path, size: u32) -> Option<Picture> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .ok()?
        .take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return None;
    }
    decode(path, &bytes, size)
}

/// The path the `file://` URI `uri` names; `None` for another scheme
/// ([`file_uri_path`]).
pub fn file_uri(uri: &str) -> Option<PathBuf> {
    file_uri_path(uri.strip_prefix("file://")?)
}

/// The path a `file://` URI names, given what follows `file://`: an empty
/// host or `localhost`, then the path, percent-decoded. `None` for another
/// host, a malformed escape, or a NUL, which no path can hold.
fn file_uri_path(uri: &str) -> Option<PathBuf> {
    let slash = uri.find('/')?;
    let (host, path) = uri.split_at(slash);
    if !(host.is_empty() || host.eq_ignore_ascii_case("localhost")) {
        return None;
    }
    let bytes = percent_decode(path)?;
    (!bytes.contains(&0)).then(|| PathBuf::from(OsString::from_vec(bytes)))
}

/// `text` with each `%` and two hex digits replaced by the byte they
/// spell (RFC 3986 section 2.1); `None` for a `%` without two hex digits.
fn percent_decode(text: &str) -> Option<Vec<u8>> {
    let mut bytes = Vec::with_capacity(text.len());
    let mut rest = text.as_bytes();
    while let Some((&first, tail)) = rest.split_first() {
        if first == b'%' {
            let hex = tail.get(..2)?;
            // `from_str_radix` would take a sign, as in `%+1`.
            if !hex.iter().all(u8::is_ascii_hexdigit) {
                return None;
            }
            let hex = std::str::from_utf8(hex).ok()?;
            bytes.push(u8::from_str_radix(hex, 16).ok()?);
            rest = &tail[2..];
        } else {
            bytes.push(first);
            rest = tail;
        }
    }
    Some(bytes)
}

/// An icon file's bytes as a picture, by its extension; an SVG is drawn
/// `size` pixels square, its text in the system's fonts. A file with any
/// other extension, or none, is tried as PNG, JPEG and then WebP, whose
/// decoders check the format's signature: browsers write a player's art to
/// files with no extension.
fn decode(path: &Path, bytes: &[u8], size: u32) -> Option<Picture> {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase);
    let image = match extension.as_deref() {
        Some("png") => png::decode(bytes, &LIMITS),
        Some("jpg" | "jpeg") => jpeg::decode(bytes, &LIMITS),
        Some("svg") => svg::decode(bytes, size, 1, &LIMITS),
        Some("xpm") => xpm::decode(bytes, &LIMITS),
        _ => png::decode(bytes, &LIMITS)
            .or_else(|_| jpeg::decode(bytes, &LIMITS))
            .or_else(|_| webp::decode(bytes, &LIMITS)),
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
    let value = crate::ini::value(text, "Settings", "gtk-icon-theme-name")?.trim_matches('"');
    (!value.is_empty()).then(|| value.to_owned())
}

#[cfg(test)]
mod tests;
