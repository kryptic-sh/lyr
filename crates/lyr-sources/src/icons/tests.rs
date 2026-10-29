use std::path::{Path, PathBuf};

use crcbl_image::{Rgba8Image, png};

use super::{Icons, MAX_FILE_BYTES, data_url, file_uri_path, icon_theme_setting, percent_decode};

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
fn a_webp_is_read_by_its_extension_or_its_contents() {
    // Two pixels, lossless: the colours come back exact.
    let two = include_bytes!("two.webp");
    let dir = tempfile::tempdir().expect("a temporary directory");
    let mut icons = Icons::new("lyr-test-theme".to_owned());
    for name in ["art.webp", "art-without-extension"] {
        let path = dir.path().join(name);
        std::fs::write(&path, two).expect("written");
        let picture = icons
            .uri_or_name(path.to_str().expect("UTF-8"), 16)
            .unwrap_or_else(|| panic!("{name} decoded"));
        assert_eq!((picture.width(), picture.height()), (2, 1), "{name}");
        assert_eq!(
            picture.pixels(),
            [10, 20, 30, 255, 200, 100, 50, 255],
            "{name}"
        );
    }
}

#[test]
fn an_svg_icon_with_text_is_drawn_in_the_system_s_fonts() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let path = dir.path().join("badge.svg");
    std::fs::write(
        &path,
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16"><text x="0" y="14" font-size="16" fill="#ff0000">W</text></svg>"##,
    )
    .expect("written");
    let picture = Icons::new("lyr-test-theme".to_owned())
        .uri_or_name(path.to_str().expect("UTF-8"), 16)
        .expect("drawn, not refused");
    assert!(
        picture.pixels().chunks(4).any(|pixel| pixel[3] > 0),
        "the letter is drawn"
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

#[test]
fn a_file_too_large_to_hold_a_picture_within_the_limits_is_not_read() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let path = dir.path().join("padded.png");
    // A PNG ends at its `IEND` chunk; what follows is not read by a decoder.
    let padded = |len: u64| {
        write_png(&path, [0, 0, 255, 255]);
        let file = std::fs::OpenOptions::new()
            .write(true)
            .open(&path)
            .expect("opened");
        file.set_len(len).expect("padded");
        super::read(&path, 16)
    };
    assert!(padded(MAX_FILE_BYTES).is_some(), "the largest file read");
    assert!(padded(MAX_FILE_BYTES + 1).is_none());
}

/// `bytes` as a `data:` URL of `media_type`, every byte percent-escaped.
pub fn data_url_of(media_type: &str, bytes: &[u8]) -> String {
    let escaped: String = bytes.iter().map(|byte| format!("%{byte:02X}")).collect();
    format!("data:{media_type},{escaped}")
}

#[test]
fn a_data_url_s_picture_is_decoded_by_its_media_type() {
    let image = Rgba8Image::new(1, 1, vec![4, 5, 6, 255]).expect("an image");
    let png = png::encode(&image).expect("encoded");
    let picture = data_url(&data_url_of("image/png", &png), 16).expect("decoded");
    assert_eq!(picture.pixels(), [4, 5, 6, 255]);
    let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="4" height="4"><rect width="4" height="4" fill="red"/></svg>"##;
    let drawn = data_url(&data_url_of("image/svg+xml", svg), 16).expect("drawn");
    assert_eq!(
        (drawn.width(), drawn.height()),
        (16, 16),
        "an SVG at the size asked"
    );
    assert!(
        data_url(&data_url_of("application/octet-stream", &png), 16).is_some(),
        "another type is read by its contents"
    );
    assert_eq!(data_url("data:image/png,not a picture", 16), None);
    assert_eq!(
        data_url("https://example.com/a.png", 16),
        None,
        "not fetched"
    );
    // A PNG ends at its `IEND` chunk; what follows is not read by a decoder.
    let padded = |len: u64| {
        let mut bytes = png.clone();
        bytes.resize(usize::try_from(len).expect("fits"), 0);
        data_url(&data_url_of("image/png", &bytes), 16)
    };
    assert!(padded(MAX_FILE_BYTES).is_some(), "the largest payload read");
    assert!(padded(MAX_FILE_BYTES + 1).is_none());
}

/// A desktop portal's settings answering `icon-theme` with `theme`; an old
/// one has only `Read`, which nests the value in another variant.
struct FakePortal {
    theme: &'static str,
    old: bool,
}

#[zbus::interface(name = "org.freedesktop.portal.Settings")]
impl FakePortal {
    fn read_one(&self, namespace: &str, key: &str) -> zbus::fdo::Result<zbus::OwnedVariant> {
        if self.old {
            return Err(zbus::fdo::Error::UnknownMethod("ReadOne".to_owned()));
        }
        self.value(namespace, key)
    }

    fn read(&self, namespace: &str, key: &str) -> zbus::fdo::Result<zbus::OwnedVariant> {
        self.value(namespace, key)?;
        let nested = zbus::Variant::Variant(Box::new(zbus::Variant::from(self.theme)));
        zbus::OwnedVariant::try_from(nested)
            .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))
    }
}

impl FakePortal {
    fn value(&self, namespace: &str, key: &str) -> zbus::fdo::Result<zbus::OwnedVariant> {
        if (namespace, key) != ("org.gnome.desktop.interface", "icon-theme") {
            return Err(zbus::fdo::Error::Failed(format!("{namespace} {key}")));
        }
        zbus::OwnedVariant::try_from(zbus::Variant::from(self.theme))
            .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))
    }
}

/// The portal read through a client connected to `portal`.
async fn read_through(portal: FakePortal) -> Option<String> {
    let (server, client) = std::os::unix::net::UnixStream::pair().expect("socket pair");
    let server = zbus::connection::Builder::unix_stream(server)
        .server(zbus::Guid::generate())
        .p2p()
        .serve_at("/org/freedesktop/portal/desktop", portal)
        .build();
    let client = zbus::connection::Builder::unix_stream(client).p2p().build();
    let (_server, client) = tokio::try_join!(server, client).expect("connected");
    super::portal_icon_theme(&client).await
}

#[tokio::test]
async fn gnome_s_icon_theme_is_read_through_the_portal() {
    let read = |theme, old| read_through(FakePortal { theme, old });
    assert_eq!(read("Papirus", false).await.as_deref(), Some("Papirus"));
    assert_eq!(
        read("Adwaita", true).await.as_deref(),
        Some("Adwaita"),
        "an older portal's Read"
    );
    assert_eq!(read("", false).await, None, "an empty name is none");
}
