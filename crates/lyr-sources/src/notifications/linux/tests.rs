use std::collections::HashMap;
use std::os::unix::net::UnixStream;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use futures_util::StreamExt;
use lyr_core::notifications::{CloseReason, Store, Urgency};
use tokio::sync::mpsc;
use zbus::connection::Builder;
use zbus::{Connection, Guid, OwnedVariant};

use super::{Change, Daemon, NotificationsProxy, PATH, Served, entry_ids, notify};

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
        .serve_at(PATH, Daemon::new(store.clone(), sender))
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

/// An `image-data` hint: one RGBA pixel, or one RGB pixel without alpha.
fn image_data(pixel: &[u8]) -> OwnedVariant {
    let alpha = pixel.len() == 4;
    let channels = i32::try_from(pixel.len()).expect("a channel count");
    zbus::Variant::from((
        1_i32,
        1_i32,
        channels,
        alpha,
        8_i32,
        channels,
        pixel.to_vec(),
    ))
    .try_into()
    .expect("owned")
}

/// A 1 px PNG of `rgba` at `path`.
fn write_png(path: &std::path::Path, rgba: [u8; 4]) {
    let image = crcbl_image::Rgba8Image::new(1, 1, rgba.to_vec()).expect("an image");
    std::fs::write(path, crcbl_image::png::encode(&image).expect("encoded")).expect("written");
}

#[tokio::test]
async fn the_image_comes_from_its_hints_in_the_spec_s_order_and_the_icon_from_app_icon() {
    let mut running = daemon().await;
    let dir = tempfile::tempdir().expect("a temporary directory");
    let (icon, shot) = (dir.path().join("icon.png"), dir.path().join("shot one.png"));
    write_png(&icon, [1, 1, 1, 255]);
    write_png(&shot, [2, 2, 2, 255]);
    let shot_uri = format!(
        "file://{}",
        shot.to_str().expect("UTF-8").replace(' ', "%20")
    );
    let path_hint = || OwnedVariant::from(zbus::Str::from(shot_uri.clone()));
    let cases = [
        (
            HashMap::from([
                ("image-data", image_data(&[3, 3, 3, 255])),
                ("image-path", path_hint()),
            ]),
            [3, 3, 3, 255],
            "image-data first",
        ),
        (
            HashMap::from([("image_data", image_data(&[4, 4, 4]))]),
            [4, 4, 4, 255],
            "under its deprecated name, and without alpha",
        ),
        (
            HashMap::from([
                ("image-path", path_hint()),
                ("icon_data", image_data(&[5, 5, 5, 255])),
            ]),
            [2, 2, 2, 255],
            "then image-path, as a file URI",
        ),
        (
            HashMap::from([("icon_data", image_data(&[5, 5, 5, 255]))]),
            [5, 5, 5, 255],
            "then icon_data",
        ),
    ];
    for (hints, pixel, why) in cases {
        let id = running
            .client
            .notify(
                "app",
                0,
                icon.to_str().expect("UTF-8"),
                "x",
                "",
                &[],
                hints,
                0,
            )
            .await
            .expect("notified");
        running.changes.recv().await;
        let store = running.store.lock().expect("store");
        let content = &store.get(id).expect("held").content;
        assert_eq!(
            content.image.as_ref().map(lyr_core::draw::Picture::pixels),
            Some(&pixel[..]),
            "{why}"
        );
        assert_eq!(
            content.icon.as_ref().map(lyr_core::draw::Picture::pixels),
            Some(&[1, 1, 1, 255][..]),
            "app_icon, by its path"
        );
    }
    let broken = HashMap::from([("image-data", image_data(&[1, 2]))]);
    let id = running
        .client
        .notify("app", 0, "", "x", "", &[], broken, 0)
        .await
        .expect("still notified");
    running.changes.recv().await;
    let store = running.store.lock().expect("store");
    let content = &store.get(id).expect("held").content;
    assert_eq!((&content.image, &content.icon), (&None, &None), "left out");
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
        [
            "actions",
            "body",
            "body-hyperlinks",
            "body-markup",
            "icon-static",
            "persistence"
        ]
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
        Daemon::new(Arc::new(Mutex::new(Store::default())), sender),
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

#[tokio::test]
async fn lyr_s_own_notice_reaches_the_daemon() {
    let running = daemon().await;
    let id = notify(running.client.inner().connection(), "Wi-Fi", "a &amp; b")
        .await
        .expect("notified");
    let store = running.store.lock().expect("a store");
    let held = store.get(id).expect("held");
    assert_eq!(held.content.app_name, "lyr");
    assert_eq!(held.content.summary, "Wi-Fi");
    assert_eq!(held.content.body, "a &amp; b");
}

fn fresh_daemon() -> Daemon {
    let (sender, _changes) = mpsc::unbounded_channel();
    Daemon::new(Arc::new(Mutex::new(Store::default())), sender)
}

#[tokio::test]
async fn a_name_another_daemon_holds_is_left_to_it_and_named() {
    let bus = crate::bus::test::Bus::start();
    let theirs = bus.connect().await;
    let ours = bus.connect().await;
    super::serve(&theirs, fresh_daemon(), false)
        .await
        .expect("the first daemon takes the name");

    let refused = super::serve(&ours, fresh_daemon(), false).await;
    let Err(super::ServeError::Owned { owner, pid }) = refused else {
        panic!("expected the name to be owned, got {refused:?}");
    };
    assert_eq!(owner, theirs.unique_name().expect("a bus name").as_str());
    assert_eq!(pid, Some(std::process::id()));
    // lyr took its object down again.
    let interface = ours.object_server().interface::<_, Daemon>(PATH).await;
    assert!(interface.is_err());
}

#[tokio::test]
async fn replacing_takes_the_name_from_the_daemon_holding_it() {
    let bus = crate::bus::test::Bus::start();
    let theirs = bus.connect().await;
    let ours = bus.connect().await;
    super::serve(&theirs, fresh_daemon(), false)
        .await
        .expect("the first daemon takes the name");

    super::serve(&ours, fresh_daemon(), true)
        .await
        .expect("replacing takes the name");
}

#[test]
fn the_app_s_desktop_entry_is_looked_for_as_swaync_does() {
    assert_eq!(
        entry_ids(Some("org.mozilla.Firefox.desktop"), "Firefox"),
        [
            "org.mozilla.Firefox",
            "org.mozilla.firefox",
            "Firefox",
            "firefox"
        ]
    );
    assert_eq!(entry_ids(None, "discord"), ["discord", "discord"]);
}
