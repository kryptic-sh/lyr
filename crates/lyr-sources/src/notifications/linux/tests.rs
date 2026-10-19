use std::collections::HashMap;
use std::os::unix::net::UnixStream;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures_util::StreamExt;
use lyr_core::notifications::{CloseReason, Store, Urgency};
use tokio::sync::mpsc;
use zbus::connection::Builder;
use zbus::{Connection, Guid, OwnedVariant};

use super::{Change, Daemon, PATH, Served};

/// How an application sees the daemon.
#[zbus::proxy(
    interface = "org.freedesktop.Notifications",
    default_service = "org.freedesktop.Notifications",
    default_path = "/org/freedesktop/Notifications"
)]
trait Notifications {
    #[allow(clippy::too_many_arguments)]
    fn notify(
        &self,
        app_name: &str,
        replaces_id: u32,
        app_icon: &str,
        summary: &str,
        body: &str,
        actions: &[&str],
        hints: HashMap<&str, OwnedVariant>,
        expire_timeout: i32,
    ) -> zbus::Result<u32>;
    fn close_notification(&self, id: u32) -> zbus::Result<()>;
    fn get_capabilities(&self) -> zbus::Result<Vec<String>>;
    fn get_server_information(&self) -> zbus::Result<(String, String, String, String)>;
    #[zbus(signal)]
    fn notification_closed(&self, id: u32, reason: u32) -> zbus::Result<()>;
    #[zbus(signal)]
    fn action_invoked(&self, id: u32, action_key: String) -> zbus::Result<()>;
}

struct Running {
    store: Arc<Mutex<Store>>,
    changes: mpsc::UnboundedReceiver<Change>,
    client: NotificationsProxy<'static>,
    _server: Connection,
}

async fn daemon() -> Running {
    let store = Arc::new(Mutex::new(Store::default()));
    let (sender, changes) = mpsc::unbounded_channel();
    let (server, client) = UnixStream::pair().expect("socket pair");
    let server = Builder::unix_stream(server)
        .server(Guid::generate())
        .p2p()
        .serve_at(
            PATH,
            Daemon {
                store: store.clone(),
                changes: sender,
            },
        )
        .build();
    let client = Builder::unix_stream(client).p2p().build();
    let (server, client) = tokio::try_join!(server, client).expect("connected");
    let client = NotificationsProxy::new(&client).await.expect("proxy");
    Running {
        store,
        changes,
        client,
        _server: server,
    }
}

#[tokio::test]
async fn a_notification_is_stored_with_its_hints() {
    let mut running = daemon().await;
    let hints = HashMap::from([
        ("urgency", OwnedVariant::from(2_u8)),
        (
            "desktop-entry",
            OwnedVariant::from(zbus::Str::from("firefox")),
        ),
        ("transient", OwnedVariant::from(true)),
    ]);
    let id = running
        .client
        .notify(
            "Firefox",
            0,
            "",
            "Download done",
            "file.zip",
            &["default", "Open", "dismiss", "Dismiss"],
            hints,
            -1,
        )
        .await
        .expect("notified");
    assert_eq!(id, 1);
    assert_eq!(running.changes.recv().await, Some(Change::Notified(1)));
    let store = running.store.lock().expect("store");
    let held = store.get(1).expect("held");
    assert_eq!(held.content.summary, "Download done");
    assert_eq!(held.content.urgency, Urgency::Critical);
    assert_eq!(held.content.desktop_entry.as_deref(), Some("firefox"));
    assert!(held.content.transient);
    assert_eq!(
        held.content.actions,
        [
            ("default".to_owned(), "Open".to_owned()),
            ("dismiss".to_owned(), "Dismiss".to_owned()),
        ]
    );
}

#[tokio::test]
async fn closing_signals_the_reason_and_an_unknown_id_is_an_error() {
    let mut running = daemon().await;
    let mut closed = running
        .client
        .receive_notification_closed()
        .await
        .expect("subscribed");
    let id = running
        .client
        .notify("app", 0, "", "x", "", &[], HashMap::new(), 0)
        .await
        .expect("notified");
    running.client.close_notification(id).await.expect("closed");
    let signal = tokio::time::timeout(Duration::from_secs(5), closed.next())
        .await
        .expect("a signal within 5 s")
        .expect("open");
    let args = signal.args().expect("arguments");
    assert_eq!((args.id, args.reason), (id, CloseReason::Closed as u32));
    running.changes.recv().await;
    assert_eq!(
        running.changes.recv().await,
        Some(Change::Closed(id, CloseReason::Closed))
    );
    assert!(running.client.close_notification(id).await.is_err());
}

#[tokio::test]
async fn the_server_says_what_it_is_and_what_it_can_do() {
    let running = daemon().await;
    let (name, _, _, spec) = running
        .client
        .get_server_information()
        .await
        .expect("answered");
    assert_eq!((name.as_str(), spec.as_str()), ("lyr", "1.3"));
    assert_eq!(
        running.client.get_capabilities().await.expect("answered"),
        ["actions", "body", "body-markup", "persistence"],
        "not hyperlinks or icons, which lyr does not open or draw yet"
    );
}

/// The daemon served through [`Served`], an application's proxy, and the
/// changes it reports.
async fn served() -> (
    Served,
    NotificationsProxy<'static>,
    Connection,
    mpsc::UnboundedReceiver<Change>,
) {
    let (sender, changes) = mpsc::unbounded_channel();
    let (server, client) = UnixStream::pair().expect("socket pair");
    let server = Builder::unix_stream(server)
        .server(Guid::generate())
        .p2p()
        .build();
    let client = Builder::unix_stream(client).p2p().build();
    let (server, client) = tokio::try_join!(server, client).expect("connected");
    let served = Served::at(
        &server,
        Daemon {
            store: Arc::new(Mutex::new(Store::default())),
            changes: sender,
        },
    )
    .await
    .expect("served");
    let proxy = NotificationsProxy::new(&client).await.expect("proxy");
    (served, proxy, server, changes)
}

#[tokio::test]
async fn lyr_closing_a_notification_signals_its_reason() {
    let (served, client, _server, mut changes) = served().await;
    let mut closed = client
        .receive_notification_closed()
        .await
        .expect("subscribed");
    let id = client
        .notify("app", 0, "", "x", "", &[], HashMap::new(), -1)
        .await
        .expect("notified");
    assert!(
        served
            .close(id, CloseReason::Expired)
            .await
            .expect("closed")
    );
    let mut next = async || {
        tokio::time::timeout(Duration::from_secs(5), changes.recv())
            .await
            .expect("a change within 5 s")
    };
    assert_eq!(next().await, Some(Change::Notified(id)));
    assert_eq!(
        next().await,
        Some(Change::Closed(id, CloseReason::Expired)),
        "lyr's own close is reported too"
    );
    let signal = tokio::time::timeout(Duration::from_secs(5), closed.next())
        .await
        .expect("a signal within 5 s")
        .expect("open");
    let args = signal.args().expect("arguments");
    assert_eq!((args.id, args.reason), (id, CloseReason::Expired as u32));
    assert!(
        !served
            .close(id, CloseReason::Expired)
            .await
            .expect("answered")
    );
    assert!(
        tokio::time::timeout(Duration::from_millis(200), closed.next())
            .await
            .is_err(),
        "no signal for an id lyr no longer holds"
    );
}

#[tokio::test]
async fn invoking_an_action_signals_it_and_closes_unless_resident() {
    let (served, client, _server, _changes) = served().await;
    let mut invoked = client.receive_action_invoked().await.expect("subscribed");
    let mut closed = client
        .receive_notification_closed()
        .await
        .expect("subscribed");
    let actions = ["default", "Open"];
    let resident = HashMap::from([("resident", OwnedVariant::from(true))]);
    let kept = client
        .notify("app", 0, "", "kept", "", &actions, resident, -1)
        .await
        .expect("notified");
    let gone = client
        .notify("app", 0, "", "gone", "", &actions, HashMap::new(), -1)
        .await
        .expect("notified");
    let within = Duration::from_secs(5);
    served.invoke(kept, "default").await.expect("invoked");
    served.invoke(gone, "default").await.expect("invoked");
    for id in [kept, gone] {
        let signal = tokio::time::timeout(within, invoked.next())
            .await
            .expect("a signal within 5 s")
            .expect("open");
        let args = signal.args().expect("arguments");
        assert_eq!((args.id, args.action_key.as_str()), (id, "default"));
    }
    let signal = tokio::time::timeout(within, closed.next())
        .await
        .expect("a signal within 5 s")
        .expect("open");
    let args = signal.args().expect("arguments");
    assert_eq!(
        (args.id, args.reason),
        (gone, CloseReason::Dismissed as u32)
    );
    assert!(
        !served
            .close(gone, CloseReason::Closed)
            .await
            .expect("answered")
    );
    assert!(
        served
            .close(kept, CloseReason::Closed)
            .await
            .expect("answered"),
        "resident"
    );
}

#[tokio::test]
async fn clear_all_dismisses_every_notification() {
    let (served, client, _server, _changes) = served().await;
    let mut closed = client
        .receive_notification_closed()
        .await
        .expect("subscribed");
    let mut ids = Vec::new();
    for summary in ["one", "two"] {
        ids.push(
            client
                .notify("app", 0, "", summary, "", &[], HashMap::new(), -1)
                .await
                .expect("notified"),
        );
    }
    let mut cleared = served.clear().await.expect("cleared");
    cleared.sort_unstable();
    assert_eq!(cleared, ids);
    for _ in &ids {
        let signal = tokio::time::timeout(Duration::from_secs(5), closed.next())
            .await
            .expect("a signal within 5 s")
            .expect("open");
        let args = signal.args().expect("arguments");
        assert!(ids.contains(&args.id));
        assert_eq!(args.reason, CloseReason::Dismissed as u32);
    }
    assert!(served.clear().await.expect("cleared").is_empty());
}
