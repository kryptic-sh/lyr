use std::os::unix::net::UnixStream;
use std::time::Duration;

use tokio::sync::watch;
use zbus::connection::Builder;
use zbus::object_server::{InterfaceRef, SignalEmitter};
use zbus::{Connection, Guid};

use super::TimeDate;

const PATH: &str = "/org/freedesktop/timedate1";

/// A stand-in for timedated, with `SetTimezone` as `timedatectl` calls it.
struct FakeTimeDate {
    zone: String,
}

#[zbus::interface(name = "org.freedesktop.timedate1")]
impl FakeTimeDate {
    #[zbus(property)]
    fn timezone(&self) -> String {
        self.zone.clone()
    }

    async fn set_timezone(
        &mut self,
        zone: String,
        _interactive: bool,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> zbus::fdo::Result<()> {
        self.zone = zone;
        self.timezone_changed(&emitter).await?;
        Ok(())
    }
}

async fn fake(zone: &str) -> (Connection, Connection) {
    let (server, client) = UnixStream::pair().expect("socket pair");
    let server = Builder::unix_stream(server)
        .server(Guid::generate())
        .p2p()
        .serve_at(
            PATH,
            FakeTimeDate {
                zone: zone.to_owned(),
            },
        )
        .build();
    let client = Builder::unix_stream(client).p2p().build();
    tokio::try_join!(server, client).expect("connected")
}

async fn next(zones: &mut watch::Receiver<Option<String>>) -> Option<String> {
    tokio::time::timeout(Duration::from_secs(5), zones.changed())
        .await
        .expect("a zone within 5 s")
        .expect("the source is running");
    zones.borrow_and_update().clone()
}

#[tokio::test]
async fn the_zone_is_read_and_followed() {
    let (server, client) = fake("Asia/Manila").await;
    let timedate = TimeDate::on(&client).await.expect("timedate1");
    let (sender, mut zones) = watch::channel(None);
    let watcher = timedate.clone();
    let source = tokio::spawn(async move { watcher.watch(&sender).await });
    assert_eq!(next(&mut zones).await.as_deref(), Some("Asia/Manila"));

    let fake: InterfaceRef<FakeTimeDate> = server
        .object_server()
        .interface(PATH)
        .await
        .expect("served");
    fake.get_mut()
        .await
        .set_timezone(
            "Europe/Lisbon".to_owned(),
            false,
            fake.signal_emitter().clone(),
        )
        .await
        .expect("set");
    assert_eq!(next(&mut zones).await.as_deref(), Some("Europe/Lisbon"));
    source.abort();
}

#[tokio::test]
async fn an_unset_zone_is_none() {
    let (_server, client) = fake("").await;
    let timedate = TimeDate::on(&client).await.expect("timedate1");
    assert_eq!(timedate.zone().await.expect("read"), None);
}
