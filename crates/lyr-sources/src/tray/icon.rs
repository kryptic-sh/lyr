//! A tray item's icon as a picture: found in the order its sources come
//! ([`Item::icon_sources`]), through the user's icon theme for a name and
//! straight from the pixels for a pixmap, else the theme's missing-image
//! icon, with its overlay icon in the corner, as waybar draws it.

use crcbl_image::{Rgba8Image, raw, resize};
use lyr_core::draw::Picture;
use lyr_core::tray::{IconSource, Item, MISSING_ICON, Pixmap};

pub use crate::icons::{Icons, icon_theme_setting};
use crate::icons::{LIMITS, picture};

impl Icons {
    /// `item`'s icon for drawing about `size` pixels tall: its first source
    /// that gives one, else the theme's missing-image icon; `None` when not
    /// even that is found.
    pub fn picture(&mut self, item: &Item, size: u32) -> Option<Picture> {
        let icon = self
            .first(item, item.icon_sources(), size)
            .or_else(|| self.named(MISSING_ICON, &item.icon_theme_path, size))?;
        match self.first(item, item.overlay_sources(), size) {
            Some(overlay) => overlaid(&icon, &overlay, size).or(Some(icon)),
            None => Some(icon),
        }
    }

    /// The first of `sources` that gives a picture.
    fn first(&mut self, item: &Item, sources: Vec<IconSource<'_>>, size: u32) -> Option<Picture> {
        sources.into_iter().find_map(|source| match source {
            IconSource::Name(name) => self.named(name, &item.icon_theme_path, size),
            IconSource::Pixmaps(pixmaps) => pixmap(pixmaps, size),
        })
    }
}

/// `icon` `size` pixels tall, its aspect kept, with `overlay` squeezed into
/// a square half its shorter side in the bottom right corner: waybar
/// master's `Item::updateImage` and `Item::overlayPixbufs` (`c15ec02`),
/// GdkPixbuf's `composite` being "over" on straight-alpha bytes.
fn overlaid(icon: &Picture, overlay: &Picture, size: u32) -> Option<Picture> {
    let image = |picture: &Picture| {
        Rgba8Image::new(picture.width(), picture.height(), picture.pixels().to_vec()).ok()
    };
    // waybar's integer arithmetic, so a wide icon keeps its width rounded
    // down as GdkPixbuf's is.
    let width = (u64::from(size) * u64::from(icon.width()) / u64::from(icon.height())).max(1);
    let icon = resize::resize(&image(icon)?, u32::try_from(width).ok()?, size, &LIMITS).ok()?;
    let side = (icon.width().min(icon.height()) / 2).max(1);
    let overlay = resize::resize(&image(overlay)?, side, side, &LIMITS).ok()?;
    let (width, height) = (icon.width(), icon.height());
    let mut pixels = icon.into_pixels();
    for y in 0..side {
        for x in 0..side {
            let from = ((y * side + x) * 4) as usize;
            let to = (((height - side + y) * width + width - side + x) * 4) as usize;
            let over = &overlay.pixels()[from..from + 4];
            over_pixel(&mut pixels[to..to + 4], over);
        }
    }
    picture(Rgba8Image::new(width, height, pixels).ok()?)
}

/// Porter and Duff's "over" of the straight-alpha RGBA8 pixel `source` onto
/// `target`.
fn over_pixel(target: &mut [u8], source: &[u8]) {
    let alpha = |byte: u8| f32::from(byte) / 255.0;
    let (source_alpha, target_alpha) = (alpha(source[3]), alpha(target[3]));
    let out_alpha = source_alpha + target_alpha * (1.0 - source_alpha);
    if out_alpha <= 0.0 {
        target.copy_from_slice(&[0; 4]);
        return;
    }
    for channel in 0..3 {
        let blended = (f32::from(source[channel]) * source_alpha
            + f32::from(target[channel]) * target_alpha * (1.0 - source_alpha))
            / out_alpha;
        target[channel] = blended.round().clamp(0.0, 255.0) as u8;
    }
    target[3] = (out_alpha * 255.0).round() as u8;
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
