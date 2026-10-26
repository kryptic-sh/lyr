use std::path::Path;

use crcbl_image::{Rgba8Image, png};
use lyr_core::draw::Picture;
use lyr_core::tray::{Item, Pixmap, Status};

use super::{Icons, over_pixel, pixmap};

/// The theme the fixtures are in, named so no real theme can answer for it.
const THEME: &str = "lyr-test-theme";

/// A `side`-pixel square of one colour.
fn square(side: u32, rgba: [u8; 4]) -> Vec<u8> {
    rgba.repeat((side * side) as usize)
}

fn write_png(path: &Path, side: u32, rgba: [u8; 4]) {
    let image = Rgba8Image::new(side, side, square(side, rgba)).expect("an image");
    std::fs::write(path, png::encode(&image).expect("encoded")).expect("written");
}

/// A theme root holding [`THEME`] with `steam` in blue and the missing-image
/// icon in grey, both 16 px.
fn theme_root() -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("a temporary directory");
    let apps = root.path().join(THEME).join("16x16/apps");
    std::fs::create_dir_all(&apps).expect("created");
    std::fs::write(
        root.path().join(THEME).join("index.theme"),
        "[Icon Theme]\nName=lyr test\nDirectories=16x16/apps\n\n[16x16/apps]\nSize=16\nType=Fixed\n",
    )
    .expect("written");
    write_png(&apps.join("steam.png"), 16, BLUE);
    write_png(&apps.join("image-missing.png"), 16, GREY);
    root
}

const BLUE: [u8; 4] = [0, 0, 255, 255];
const GREY: [u8; 4] = [128, 128, 128, 255];

fn picture(side: u32, rgba: [u8; 4]) -> Picture {
    Picture::new(side, side, square(side, rgba)).expect("a picture")
}

/// One 1 px pixmap of ARGB `argb`.
fn pixmaps(argb: [u8; 4]) -> Vec<Pixmap> {
    vec![Pixmap {
        width: 1,
        height: 1,
        data: argb.to_vec(),
    }]
}

fn item(root: &Path) -> Item {
    Item {
        icon_theme_path: root.to_string_lossy().into_owned(),
        ..Item::default()
    }
}

#[test]
fn a_name_is_found_under_the_item_s_theme_path() {
    let root = theme_root();
    let steam = Item {
        icon_name: "steam".to_owned(),
        ..item(root.path())
    };
    assert_eq!(
        Icons::new(THEME.to_owned()).picture(&steam, 16),
        Some(picture(16, BLUE))
    );
}

#[test]
fn a_name_that_finds_nothing_gives_way_to_the_pixels() {
    let root = theme_root();
    let electron = Item {
        icon_name: "lyr-no-such-icon".to_owned(),
        icon_pixmap: pixmaps([255, 1, 2, 3]),
        ..item(root.path())
    };
    assert_eq!(
        Icons::new(THEME.to_owned()).picture(&electron, 16),
        Picture::new(1, 1, vec![1, 2, 3, 255]),
        "ARGB in, RGBA out"
    );
}

#[test]
fn the_attention_icon_comes_first_only_while_it_needs_attention() {
    let root = theme_root();
    let quiet = Item {
        icon_name: "steam".to_owned(),
        attention_icon_pixmap: pixmaps([255, 9, 9, 9]),
        ..item(root.path())
    };
    let mut icons = Icons::new(THEME.to_owned());
    assert_eq!(icons.picture(&quiet, 16), Some(picture(16, BLUE)));
    let alerting = Item {
        status: Status::NeedsAttention,
        ..quiet
    };
    assert_eq!(
        icons.picture(&alerting, 16),
        Picture::new(1, 1, vec![9, 9, 9, 255])
    );
}

#[test]
fn with_nothing_found_it_is_the_missing_image_icon() {
    let root = theme_root();
    let lost = Item {
        icon_name: "lyr-no-such-icon".to_owned(),
        ..item(root.path())
    };
    assert_eq!(
        Icons::new(THEME.to_owned()).picture(&lost, 16),
        Some(picture(16, GREY))
    );
}

#[test]
fn a_path_for_a_name_is_read_and_read_again_once_it_changes() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let file = dir.path().join("icon.png");
    write_png(&file, 2, BLUE);
    let named = Item {
        icon_name: file.to_string_lossy().into_owned(),
        ..Item::default()
    };
    let mut icons = Icons::new(THEME.to_owned());
    assert_eq!(icons.picture(&named, 16), Some(picture(2, BLUE)));
    // A different size, so its version differs even within one clock tick.
    write_png(&file, 3, GREY);
    assert_eq!(icons.picture(&named, 16), Some(picture(3, GREY)));
}

#[test]
fn the_pixmap_nearest_the_size_is_taken_and_a_broken_one_is_not() {
    let sized = |side: i32, value: u8| Pixmap {
        width: side,
        height: side,
        data: vec![value; 4 * (side * side) as usize],
    };
    let sizes = [sized(16, 1), sized(32, 2), sized(64, 3)];
    let chosen = pixmap(&sizes, 20).expect("a picture");
    assert_eq!(
        (chosen.width(), chosen.pixels()[0]),
        (32, 2),
        "the smallest at least as large"
    );
    let negative = Pixmap {
        width: -1,
        height: 1,
        data: vec![0; 4],
    };
    let short = Pixmap {
        width: 2,
        height: 2,
        data: vec![0; 4],
    };
    assert_eq!(pixmap(&[negative], 16), None);
    assert_eq!(pixmap(&[short], 16), None);
}

const RED: [u8; 4] = [255, 0, 0, 255];

/// The colour of the pixel at `x`, `y`.
fn at(picture: &Picture, x: u32, y: u32) -> [u8; 4] {
    let from = ((y * picture.width() + x) * 4) as usize;
    picture.pixels()[from..from + 4]
        .try_into()
        .expect("a pixel")
}

#[test]
fn an_overlay_fills_the_bottom_right_quarter() {
    let root = theme_root();
    let flagged = Item {
        icon_name: "steam".to_owned(),
        overlay_icon_name: "lyr-no-such-icon".to_owned(),
        overlay_icon_pixmap: pixmaps([255, 255, 0, 0]),
        ..item(root.path())
    };
    let picture = Icons::new(THEME.to_owned())
        .picture(&flagged, 16)
        .expect("a picture");
    assert_eq!((picture.width(), picture.height()), (16, 16));
    for (x, y) in [(8, 8), (15, 15), (8, 15), (15, 8)] {
        assert_eq!(at(&picture, x, y), RED, "{x}, {y} in the corner");
    }
    for (x, y) in [(7, 7), (7, 15), (15, 7), (0, 0)] {
        assert_eq!(at(&picture, x, y), BLUE, "{x}, {y} outside it");
    }
}

#[test]
fn a_wide_icon_is_drawn_its_height_and_its_overlay_square() {
    let wide = Item {
        icon_pixmap: vec![Pixmap {
            width: 2,
            height: 1,
            data: [255, 0, 0, 255].repeat(2),
        }],
        overlay_icon_pixmap: pixmaps([255, 255, 0, 0]),
        ..Item::default()
    };
    let picture = Icons::new(THEME.to_owned())
        .picture(&wide, 16)
        .expect("a picture");
    assert_eq!((picture.width(), picture.height()), (32, 16));
    assert_eq!(at(&picture, 24, 8), RED);
    assert_eq!(at(&picture, 23, 15), BLUE);
    assert_eq!(at(&picture, 24, 7), BLUE);
}

#[test]
fn an_item_with_no_overlay_keeps_its_own_size() {
    let plain = Item {
        icon_pixmap: pixmaps([255, 0, 0, 255]),
        ..Item::default()
    };
    assert_eq!(
        Icons::new(THEME.to_owned()).picture(&plain, 16),
        Some(picture(1, BLUE))
    );
}

#[test]
fn over_blends_by_the_source_s_alpha() {
    let mut target = BLUE;
    over_pixel(&mut target, &[255, 0, 0, 128]);
    // 128/255 of red over the rest of blue, opaque.
    assert_eq!(target, [128, 0, 127, 255]);
    let mut clear = [0, 0, 0, 0];
    over_pixel(&mut clear, &[10, 20, 30, 51]);
    assert_eq!(clear, [10, 20, 30, 51], "over nothing it is itself");
}
