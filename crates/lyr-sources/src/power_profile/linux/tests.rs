use std::collections::HashMap;
use std::os::unix::net::UnixStream;
use std::time::Duration;

use tokio::sync::watch;
use zbus::OwnedVariant;
use zbus::connection::Builder;
use zbus::object_server::{InterfaceRef, SignalEmitter};
use zbus::{Connection, Guid};

use super::PowerProfiles;
use crate::power_profile::Profiles;

const OLD_PATH: &str = "/net/hadess/PowerProfiles";

/// A stand-in for an older power-profiles-daemon, serving only its
/// `net.hadess` name, so lyr has to fall back to it.
struct FakeDaemon {
    active: String,
}

#[zbus::interface(name = "net.hadess.PowerProfiles")]
impl FakeDaemon {
    #[zbus(property)]
    fn active_profile(&self) -> String {
        self.active.clone()
    }

    #[zbus(property)]
    async fn set_active_profile(
        &mut self,
        profile: String,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) {
        self.active = profile;
        let _ = self.active_profile_changed(&emitter).await;
    }

    #[zbus(property)]
    fn profiles(&self) -> Vec<HashMap<String, OwnedVariant>> {
        ["power-saver", "balanced", "performance"]
            .into_iter()
            .map(|profile| {
                HashMap::from([
                    (
                        "Profile".to_owned(),
                        OwnedVariant::from(zbus::Str::from(profile)),
                    ),
                    (
                        "Driver".to_owned(),
                        OwnedVariant::from(zbus::Str::from("amd_pstate")),
                    ),
                ])
            })
            .collect()
    }
}

async fn fake_daemon(active: &str) -> (Connection, Connection) {
    let (server, client) = UnixStream::pair().expect("socket pair");
    let server = Builder::unix_stream(server)
        .server(Guid::generate())
        .p2p()
        .serve_at(
            OLD_PATH,
            FakeDaemon {
                active: active.to_owned(),
            },
        )
        .build();
    let client = Builder::unix_stream(client).p2p().build();
    tokio::try_join!(server, client).expect("connected")
}

async fn next(states: &mut watch::Receiver<Option<Profiles>>) -> Option<Profiles> {
    tokio::time::timeout(Duration::from_secs(5), states.changed())
        .await
        .expect("a state within 5 s")
        .expect("the source is running");
    states.borrow_and_update().clone()
}

#[tokio::test]
async fn the_old_name_is_found_and_its_profiles_read() {
    let (_server, client) = fake_daemon("balanced").await;
    let daemon = PowerProfiles::on(&client)
        .await
        .expect("found under net.hadess");
    let profiles = daemon.profiles().await.expect("read").expect("listed");
    assert_eq!(profiles.active.name, "balanced");
    assert_eq!(profiles.active.driver, "amd_pstate");
    assert_eq!(
        profiles.available,
        ["power-saver", "balanced", "performance"]
    );
}

#[tokio::test]
async fn a_switch_comes_back_as_a_change() {
    let (server, client) = fake_daemon("balanced").await;
    let daemon = PowerProfiles::on(&client).await.expect("found");
    let (sender, mut states) = watch::channel(None);
    let watcher = daemon.clone();
    let source = tokio::spawn(async move { watcher.watch(&sender).await });
    assert_eq!(
        next(&mut states).await.map(|p| p.active.name).as_deref(),
        Some("balanced")
    );

    daemon.set("performance").await.expect("switched");
    assert_eq!(
        next(&mut states).await.map(|p| p.active.name).as_deref(),
        Some("performance")
    );
    let fake: InterfaceRef<FakeDaemon> = server
        .object_server()
        .interface(OLD_PATH)
        .await
        .expect("served");
    assert_eq!(fake.get().await.active, "performance");
    source.abort();
}
