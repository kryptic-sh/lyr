use std::collections::HashMap;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::sync::{Arc, Mutex};

use crcbl_image::{Rgba8Image, png};
use zbus::connection::Builder;
use zbus::{Connection, Guid, OwnedVariant};

use super::{Art, PATH, control, read};
use lyr_core::media::{Control, Status};

const NAME: &str = "org.mpris.MediaPlayer2.test";

/// A stand-in for a media player.
struct FakePlayer {
    status: &'static str,
    artists: Vec<String>,
    art_url: String,
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
        HashMap::from([
            (
                "xesam:title".to_owned(),
                OwnedVariant::from(zbus::Str::from("A Song")),
            ),
            (
                "xesam:artist".to_owned(),
                OwnedVariant::try_from(zbus::Variant::from(self.artists.clone()))
                    .expect("an array"),
            ),
            (
                "mpris:artUrl".to_owned(),
                OwnedVariant::from(zbus::Str::from(self.art_url.clone())),
            ),
        ])
    }

    #[zbus(property)]
    fn playback_status(&self) -> String {
        self.status.to_owned()
    }
}

async fn player(
    status: &'static str,
    artists: &[&str],
) -> (Connection, Connection, Arc<Mutex<Vec<&'static str>>>) {
    with_art(status, artists, "").await
}

async fn with_art(
    status: &'static str,
    artists: &[&str],
    art_url: &str,
) -> (Connection, Connection, Arc<Mutex<Vec<&'static str>>>) {
    let pressed = Arc::new(Mutex::new(Vec::new()));
    let (server, client) = UnixStream::pair().expect("socket pair");
    let server = Builder::unix_stream(server)
        .server(Guid::generate())
        .p2p()
        .serve_at(
            PATH,
            FakePlayer {
                status,
                artists: artists.iter().map(|artist| (*artist).to_owned()).collect(),
                art_url: art_url.to_owned(),
                pressed: pressed.clone(),
            },
        )
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
    assert_eq!(read.artist, "One, Two");
    assert_eq!(read.status, Status::Playing);
    assert_eq!(read.art, None, "it names no art");
}

/// A 1 × 1 PNG of `rgba` at `path`.
fn write_png(path: &Path, rgba: [u8; 4]) {
    let image = Rgba8Image::new(1, 1, rgba.to_vec()).expect("an image");
    std::fs::write(path, png::encode(&image).expect("encoded")).expect("written");
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
    let read = read(&client, NAME, &Mutex::default()).await.expect("read");
    let art = read.art.expect("the art read");
    assert_eq!(art.pixels(), [0, 0, 255, 255]);
}

#[tokio::test]
async fn art_over_the_network_is_not_fetched() {
    let (_server, client, _) = with_art("Playing", &[], "https://example.com/a.png").await;
    let read = read(&client, NAME, &Mutex::default()).await.expect("read");
    assert_eq!(read.art, None);
}

#[test]
fn art_is_read_again_once_its_file_changed_and_swept_once_unasked() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let path = dir.path().join("art.png");
    let url = format!("file://{}", path.to_str().expect("UTF-8"));
    write_png(&path, [255, 0, 0, 255]);
    let mut art = Art::default();
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
    assert_eq!((read.artist.as_str(), read.status), ("", Status::Paused));
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
