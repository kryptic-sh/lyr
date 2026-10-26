//! A tray item's icon as a picture: found in the order its sources come
//! ([`Item::icon_sources`]), through the user's icon theme for a name and
//! straight from the pixels for a pixmap, else the theme's missing-image
//! icon, as waybar draws it.

use crcbl_image::{raw, resize};
use lyr_core::draw::Picture;
use lyr_core::tray::{IconSource, Item, MISSING_ICON, Pixmap};

pub use crate::icons::{Icons, icon_theme_setting};
use crate::icons::{LIMITS, picture};

impl Icons {
    /// `item`'s icon for drawing about `size` pixels tall: its first source
    /// that gives one, else the theme's missing-image icon; `None` when not
    /// even that is found.
    pub fn picture(&mut self, item: &Item, size: u32) -> Option<Picture> {
        item.icon_sources()
            .into_iter()
            .find_map(|source| match source {
                IconSource::Name(name) => self.named(name, &item.icon_theme_path, size),
                IconSource::Pixmaps(pixmaps) => pixmap(pixmaps, size),
            })
            .or_else(|| self.named(MISSING_ICON, &item.icon_theme_path, size))
    }
}

/// The pixmap nearest `size` among those that hold their pixels, as a
/// picture.
fn pixmap(pixmaps: &[Pixmap], size: u32) -> Option<Picture> {
    let sized: Vec<(&Pixmap, (u32, u32))> = pixmaps
        .iter()
        .filter_map(|pixmap| {
            let width = u32::try_from(pixmap.width).ok()?;
            let height = u32::try_from(pixmap.height).ok()?;
            Some((pixmap, (width, height)))
        })
        .collect();
    let sizes: Vec<(u32, u32)> = sized.iter().map(|(_, size)| *size).collect();
    let (pixmap, (width, height)) = sized[resize::nearest_size(&sizes, size)?];
    raw::from_argb32_be(width, height, &pixmap.data, &LIMITS)
        .ok()
        .and_then(picture)
}

#[cfg(test)]
mod tests;
