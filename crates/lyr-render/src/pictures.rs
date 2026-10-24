//! A window's pictures on its image atlas: each drawn picture resized to the
//! device pixels it covers and placed once, and the ones a frame stopped
//! drawing given back.

use std::collections::{HashMap, HashSet};

use crcbl_image::{Limits, Rgba8Image, resize::resize};
use crcbl_ui::image::{AtlasImage, ImageAtlas};
use lyr_core::draw::Picture;

/// A picture at the size it is drawn, in device pixels.
type Key = (Picture, u32, u32);

/// The pictures one window has placed on its atlas.
#[derive(Debug, Default)]
pub struct Pictures {
    placed: HashMap<Key, AtlasImage>,
    /// What the frame being built has drawn.
    used: HashSet<Key>,
}

impl Pictures {
    /// Where `picture` sits on `atlas` at `width` × `height` device pixels,
    /// resized and placed there first if it was not; `None` when it cannot
    /// be (a zero size, or no room left on the page).
    pub fn image(
        &mut self,
        atlas: &mut ImageAtlas,
        picture: &Picture,
        width: u32,
        height: u32,
    ) -> Option<AtlasImage> {
        let key = (picture.clone(), width, height);
        let image = match self.placed.get(&key) {
            Some(image) => *image,
            None => {
                let image = place(atlas, picture, width, height)?;
                self.placed.insert(key.clone(), image);
                image
            }
        };
        self.used.insert(key);
        Some(image)
    }

    /// Ends a frame: what it did not draw leaves the atlas.
    pub fn sweep(&mut self, atlas: &mut ImageAtlas) {
        let used = std::mem::take(&mut self.used);
        self.placed.retain(|key, image| {
            let keep = used.contains(key);
            if !keep {
                atlas.unregister(image.id);
            }
            keep
        });
    }
}

/// `picture` resized to `width` × `height` and registered on `atlas`.
fn place(atlas: &mut ImageAtlas, picture: &Picture, width: u32, height: u32) -> Option<AtlasImage> {
    let source =
        Rgba8Image::new(picture.width(), picture.height(), picture.pixels().to_vec()).ok()?;
    let sized = resize(&source, width, height, &Limits::DEFAULT).ok()?;
    atlas.register(width, height, sized.pixels()).ok()
}

#[cfg(test)]
mod tests {
    use crcbl_ui::image::ImageAtlas;
    use lyr_core::draw::Picture;

    use super::Pictures;

    fn picture(value: u8) -> Picture {
        Picture::new(2, 2, vec![value; 16]).expect("a picture")
    }

    #[test]
    fn a_picture_is_placed_once_per_size_and_given_back_when_undrawn() {
        let mut atlas = ImageAtlas::new();
        let mut pictures = Pictures::default();
        let first = pictures
            .image(&mut atlas, &picture(1), 4, 4)
            .expect("placed");
        assert_eq!((first.width, first.height), (4, 4), "resized to its size");
        assert_eq!(
            pictures.image(&mut atlas, &picture(1), 4, 4),
            Some(first),
            "the same picture again is the same image"
        );
        let other = pictures
            .image(&mut atlas, &picture(2), 4, 4)
            .expect("placed");
        assert_ne!(other.id, first.id, "other pixels are another image");
        pictures.sweep(&mut atlas);
        assert_eq!(atlas.images().len(), 2, "both were drawn");

        pictures.image(&mut atlas, &picture(2), 4, 4);
        pictures.sweep(&mut atlas);
        assert_eq!(atlas.image(first.id), None, "undrawn, it left the atlas");
        assert_eq!(atlas.image(other.id), Some(other));
    }

    #[test]
    fn a_picture_that_cannot_be_drawn_is_none() {
        let mut atlas = ImageAtlas::new();
        assert_eq!(
            Pictures::default().image(&mut atlas, &picture(1), 0, 4),
            None
        );
    }
}
