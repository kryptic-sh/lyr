use std::collections::HashMap;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::sync::{Arc, Mutex};

use crcbl_image::{Rgba8Image, png};
use zbus::connection::Builder;
use zbus::{Connection, Guid, OwnedVariant};

use super::{ART_SIZE, Art, NO_ART_ICON, PATH, control, read};
use crate::icons::Icons;
use lyr_core::media::{Artwork, Control, Status};

const NAME: &str = "org.mpris.MediaPlayer2.test";

/// A stand-in for a media player.
struct FakePlayer {
    status: &'static str,
    artists: Vec<String>,
    album_artists: Vec<String>,
    /// Text entries over the default title "A Song".
    texts: Vec<(&'static str, String)>,
    pressed: Arc<Mutex<Vec<&'static str>>>,
}

#[zbus::interface(name = "org.mpris.MediaPlayer2.Player")]
impl FakePlayer {
    fn previous(&self) {
        self.pressed.lock().expect("pressed").push("Previous");
    }
    fn play_pause(&self) {
        self.pressed.lock().expect("pressed").push("PlayPause");
    }
    fn next(&self) {
        self.pressed.lock().expect("pressed").push("Next");
    }

    #[zbus(property)]
    fn metadata(&self) -> HashMap<String, OwnedVariant> {
        let names = |names: &[String]| {
            OwnedVariant::try_from(zbus::Variant::from(names.to_vec())).expect("an array")
        };
        let mut metadata = HashMap::from([
            (
                "xesam:title".to_owned(),
                OwnedVariant::from(zbus::Str::from("A Song")),
            ),
            ("xesam:artist".to_owned(), names(&self.artists)),
            ("xesam:albumArtist".to_owned(), names(&self.album_artists)),
        ]);
        for (key, value) in &self.texts {
            metadata.insert(
                (*key).to_owned(),
                OwnedVariant::from(zbus::Str::from(value.clone())),
            );
        }
        metadata
    }

    #[zbus(property)]
    fn playback_status(&self) -> String {
        self.status.to_owned()
    }
}

/// A stand-in for a player's root interface, naming its desktop entry.
struct FakeRoot;

#[zbus::interface(name = "org.mpris.MediaPlayer2")]
impl FakeRoot {
    #[zbus(property)]
    fn desktop_entry(&self) -> String {
        ENTRY.to_owned()
    }
}

/// The desktop entry the stand-in names.
const ENTRY: &str = "lyr-test-player";

async fn player(
    status: &'static str,
    artists: &[&str],
) -> (Connection, Connection, Arc<Mutex<Vec<&'static str>>>) {
    playing(FakePlayer {
        status,
        artists: names(artists),
        album_artists: Vec::new(),
        texts: Vec::new(),
        pressed: Arc::default(),
    })
    .await
}

fn names(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

async fn with_art(
    status: &'static str,
    artists: &[&str],
    art_url: &str,
) -> (Connection, Connection, Arc<Mutex<Vec<&'static str>>>) {
    with_texts(artists, &[("mpris:artUrl", art_url)], status).await
}

async fn with_texts(
    artists: &[&str],
    texts: &[(&'static str, &str)],
    status: &'static str,
) -> (Connection, Connection, Arc<Mutex<Vec<&'static str>>>) {
    playing(FakePlayer {
        status,
        artists: names(artists),
        album_artists: Vec::new(),
        texts: texts
            .iter()
            .map(|(key, value)| (*key, (*value).to_owned()))
            .collect(),
        pressed: Arc::default(),
    })
    .await
}

/// `fake` serving on one end of a socket pair, with [`FakeRoot`]: the
/// server, the client, and what was pressed.
async fn playing(fake: FakePlayer) -> (Connection, Connection, Arc<Mutex<Vec<&'static str>>>) {
    let pressed = fake.pressed.clone();
    let (server, client) = UnixStream::pair().expect("socket pair");
    let server = Builder::unix_stream(server)
        .server(Guid::generate())
        .p2p()
        .serve_at(PATH, fake)
        .serve_at(PATH, FakeRoot)
        .build();
    let client = Builder::unix_stream(client).p2p().build();
    let (server, client) = tokio::try_join!(server, client).expect("connected");
    (server, client, pressed)
}

#[tokio::test]
async fn a_player_is_read_with_its_title_artists_and_status() {
    let (_server, client, _) = player("Playing", &["One", "Two"]).await;
    let read = read(&client, NAME, &Mutex::default()).await.expect("read");
    assert_eq!(read.bus_name, NAME);
    assert_eq!(read.title, "A Song");
    assert_eq!(read.subtitle, "One", "the first artist, as swaync shows it");
    assert_eq!(read.status, Status::Playing);
}

/// A 1 × 1 PNG of `rgba` at `path`.
fn write_png(path: &Path, rgba: [u8; 4]) {
    let image = Rgba8Image::new(1, 1, rgba.to_vec()).expect("an image");
    std::fs::write(path, png::encode(&image).expect("encoded")).expect("written");
}

const GREEN: [u8; 4] = [0, 255, 0, 255];

/// Art whose desktop entries are under `dir`, where [`ENTRY`]'s icon is a
/// green PNG.
fn with_entry(dir: &Path) -> Mutex<Art> {
    let applications = dir.join("applications");
    std::fs::create_dir_all(&applications).expect("created");
    let icon = dir.join("player.png");
    write_png(&icon, GREEN);
    std::fs::write(
        applications.join(format!("{ENTRY}.desktop")),
        format!("[Desktop Entry]\nName=Player\nIcon={}\n", icon.display()),
    )
    .expect("written");
    Mutex::new(Art::new(
        Icons::new("lyr-test-theme".to_owned()),
        vec![dir.to_owned()],
    ))
}

#[tokio::test]
async fn a_player_s_art_is_read_from_its_file_uri() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let path = dir.path().join("cover art");
    write_png(&path, [0, 0, 255, 255]);
    let url = format!(
        "file://{}",
        path.to_str().expect("UTF-8").replace(' ', "%20")
    );
    let (_server, client, _) = with_art("Playing", &[], &url).await;
    let art = with_entry(dir.path());
    let read = read(&client, NAME, &art).await.expect("read");
    let Some(Artwork::Picture(art)) = read.art else {
        panic!("the art read: {:?}", read.art)
    };
    assert_eq!(art.pixels(), [0, 0, 255, 255], "before the entry's icon");
}

#[tokio::test]
async fn art_over_the_network_is_not_fetched_and_the_entry_s_icon_shows() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let (_server, client, _) = with_art("Playing", &[], "https://example.com/a.png").await;
    let read = read(&client, NAME, &with_entry(dir.path()))
        .await
        .expect("read");
    let Some(Artwork::Picture(icon)) = read.art else {
        panic!("the entry's icon: {:?}", read.art)
    };
    assert_eq!(icon.pixels(), GREEN);
}

#[test]
fn with_no_art_and_no_entry_it_is_the_generic_icon() {
    // What the generic icon is depends on the themes installed, so this
    // checks the lookup is made, and can only fail where one has it.
    let expected = Icons::default()
        .named(NO_ART_ICON, "", ART_SIZE)
        .map(Artwork::Symbolic);
    let mut art = Art::new(Icons::default(), Vec::new());
    assert_eq!(art.shown("", "lyr-no-such-entry"), expected);
}

#[test]
fn art_is_read_again_once_its_file_changed_and_swept_once_unasked() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let path = dir.path().join("art.png");
    let url = format!("file://{}", path.to_str().expect("UTF-8"));
    write_png(&path, [255, 0, 0, 255]);
    let mut art = Art::new(Icons::new("lyr-test-theme".to_owned()), Vec::new());
    let first = art.picture(&url).expect("read");
    assert_eq!(first.pixels(), [255, 0, 0, 255]);
    // A different length, so the file's version differs whatever the
    // filesystem's clock.
    let image = Rgba8Image::new(2, 1, [0, 255, 0, 255].repeat(2)).expect("an image");
    std::fs::write(&path, png::encode(&image).expect("encoded")).expect("written");
    let second = art.picture(&url).expect("read again");
    assert_eq!(second.width(), 2);
    art.sweep();
    assert!(
        art.read.contains_key(&url),
        "asked for since the last sweep"
    );
    art.sweep();
    assert!(art.read.is_empty(), "not asked for since");
}

#[tokio::test]
async fn a_player_with_no_artist_has_none() {
    let (_server, client, _) = player("Paused", &[]).await;
    let read = read(&client, NAME, &Mutex::default()).await.expect("read");
    assert_eq!((read.subtitle.as_str(), read.status), ("", Status::Paused));
}

#[tokio::test]
async fn each_control_presses_its_method() {
    let (_server, client, pressed) = player("Playing", &[]).await;
    for pressing in [Control::Previous, Control::PlayPause, Control::Next] {
        control(&client, NAME, pressing).await.expect("pressed");
    }
    assert_eq!(
        *pressed.lock().expect("pressed"),
        ["Previous", "PlayPause", "Next"]
    );
}

#[tokio::test]
async fn the_subtitle_is_the_artist_and_the_album() {
    let (_server, client, _) = with_texts(
        &["Miles Davis"],
        &[("xesam:album", "Kind of Blue")],
        "Playing",
    )
    .await;
    let read = read(&client, NAME, &Mutex::default()).await.expect("read");
    assert_eq!(read.subtitle, "Miles Davis - Kind of Blue");
}

#[tokio::test]
async fn with_no_artist_the_album_artist_is_taken() {
    let (_server, client, _) = playing(FakePlayer {
        status: "Playing",
        artists: Vec::new(),
        album_artists: names(&["Various"]),
        texts: Vec::new(),
        pressed: Arc::default(),
    })
    .await;
    let read = read(&client, NAME, &Mutex::default()).await.expect("read");
    assert_eq!(read.subtitle, "Various");
}

#[tokio::test]
async fn an_untitled_player_is_named_by_its_entry_else_media_player() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let (_server, client, _) = with_texts(&[], &[("xesam:title", "")], "Playing").await;
    let named = read(&client, NAME, &with_entry(dir.path()))
        .await
        .expect("read");
    assert_eq!(named.title, "Player");
    let unnamed = Mutex::new(Art::new(
        Icons::new("lyr-test-theme".to_owned()),
        Vec::new(),
    ));
    let read = read(&client, NAME, &unnamed).await.expect("read");
    assert_eq!(read.title, "Media Player");
}
