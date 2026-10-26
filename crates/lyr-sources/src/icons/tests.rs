use std::path::{Path, PathBuf};

use crcbl_image::{Rgba8Image, png};

use super::{Icons, file_uri_path, icon_theme_setting, percent_decode};

fn write_png(path: &Path, rgba: [u8; 4]) {
    let image = Rgba8Image::new(1, 1, rgba.to_vec()).expect("an image");
    std::fs::write(path, png::encode(&image).expect("encoded")).expect("written");
}

#[test]
fn the_icon_theme_is_gtk_s_setting() {
    assert_eq!(
        icon_theme_setting(
            "[Settings]\ngtk-theme-name=Adwaita\ngtk-icon-theme-name = Papirus-Dark\n"
        )
        .as_deref(),
        Some("Papirus-Dark")
    );
    assert_eq!(
        icon_theme_setting("[Settings]\ngtk-icon-theme-name=\"Papirus\"\n").as_deref(),
        Some("Papirus"),
        "quotes are not part of the name"
    );
    assert_eq!(
        icon_theme_setting("[Other]\ngtk-icon-theme-name=Papirus\n"),
        None,
        "only the Settings group"
    );
    assert_eq!(
        icon_theme_setting("[Settings]\ngtk-icon-theme-name=\n"),
        None
    );
}

#[test]
fn percent_escapes_decode_as_rfc_3986_has_them() {
    assert_eq!(percent_decode("a%20b").as_deref(), Some(&b"a b"[..]));
    assert_eq!(
        percent_decode("%2f%2F").as_deref(),
        Some(&b"//"[..]),
        "either case"
    );
    assert_eq!(
        percent_decode("%E2%82%AC").as_deref(),
        Some("\u{20ac}".as_bytes()),
        "a UTF-8 sequence, byte by byte"
    );
    assert_eq!(percent_decode("plain").as_deref(), Some(&b"plain"[..]));
    for malformed in ["%", "%4", "a%zz", "%+1", "%-1"] {
        assert_eq!(percent_decode(malformed), None, "{malformed}");
    }
}

#[test]
fn a_file_uri_names_a_local_path() {
    assert_eq!(
        file_uri_path("/tmp/a%20b.png"),
        Some(PathBuf::from("/tmp/a b.png"))
    );
    assert_eq!(
        file_uri_path("localhost/tmp/x.png"),
        Some(PathBuf::from("/tmp/x.png"))
    );
    assert_eq!(file_uri_path("example.com/tmp/x.png"), None, "another host");
    assert_eq!(file_uri_path("/tmp/a%00b"), None, "a NUL");
    assert_eq!(file_uri_path("tmp"), None, "no path");
}

#[test]
fn a_notification_names_its_icon_by_uri_path_or_name() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let path = dir.path().join("an image.png");
    write_png(&path, [1, 2, 3, 255]);
    let mut icons = Icons::new("lyr-test-theme".to_owned());
    let uri = format!(
        "file://{}",
        path.to_str().expect("UTF-8").replace(' ', "%20")
    );
    let picture = icons.uri_or_name(&uri, 16).expect("read through the URI");
    assert_eq!(picture.pixels(), [1, 2, 3, 255]);
    assert_eq!(
        icons.uri_or_name(path.to_str().expect("UTF-8"), 16),
        Some(picture),
        "and through the path"
    );
    write_png(&path, [9, 9, 9, 255]);
    assert_eq!(
        icons.uri_or_name(&uri, 16).expect("read again").pixels(),
        [9, 9, 9, 255],
        "a file is read each time"
    );
    assert!(icons.files.is_empty(), "and none is kept");
    assert_eq!(icons.uri_or_name("", 16), None);
}

#[test]
fn a_jpeg_is_read_too() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let path = dir.path().join("art.JPG");
    std::fs::write(&path, include_bytes!("red.jpg")).expect("written");
    let picture = Icons::new("lyr-test-theme".to_owned())
        .uri_or_name(path.to_str().expect("UTF-8"), 16)
        .expect("decoded");
    assert_eq!((picture.width(), picture.height()), (8, 8));
    let [r, g, b, a] = picture.pixels()[..4] else {
        panic!("a pixel")
    };
    assert!(
        r > 250 && g < 5 && b < 5 && a == 255,
        "red: {r} {g} {b} {a}"
    );
}

#[test]
fn a_file_with_no_extension_is_read_by_its_contents() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let png_path = dir.path().join("art");
    write_png(&png_path, [0, 255, 0, 255]);
    let jpeg_path = dir.path().join("art.tmp");
    std::fs::write(&jpeg_path, include_bytes!("red.jpg")).expect("written");
    let mut icons = Icons::new("lyr-test-theme".to_owned());
    let png = icons
        .uri_or_name(png_path.to_str().expect("UTF-8"), 16)
        .expect("decoded as PNG");
    assert_eq!(png.pixels(), [0, 255, 0, 255]);
    let jpeg = icons
        .uri_or_name(jpeg_path.to_str().expect("UTF-8"), 16)
        .expect("decoded as JPEG");
    assert_eq!((jpeg.width(), jpeg.height()), (8, 8));
}
