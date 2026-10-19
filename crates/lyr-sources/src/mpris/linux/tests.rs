use std::collections::HashMap;
use std::os::unix::net::UnixStream;
use std::sync::{Arc, Mutex};

use zbus::connection::Builder;
use zbus::{Connection, Guid, OwnedVariant};

use super::{PATH, control, read};
use lyr_core::media::{Control, Status};

const NAME: &str = "org.mpris.MediaPlayer2.test";

/// A stand-in for a media player.
struct FakePlayer {
    status: &'static str,
    artists: Vec<String>,
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
    let read = read(&client, NAME).await.expect("read");
    assert_eq!(read.bus_name, NAME);
    assert_eq!(read.title, "A Song");
    assert_eq!(read.artist, "One, Two");
    assert_eq!(read.status, Status::Playing);
}

#[tokio::test]
async fn a_player_with_no_artist_has_none() {
    let (_server, client, _) = player("Paused", &[]).await;
    let read = read(&client, NAME).await.expect("read");
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
