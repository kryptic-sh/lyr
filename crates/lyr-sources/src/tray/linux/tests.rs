use std::os::unix::net::UnixStream;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use futures_util::StreamExt;
use lyr_core::tray::{Click, Pixmap, Status};
use tokio::sync::watch;
use zbus::connection::Builder;
use zbus::object_server::{InterfaceRef, SignalEmitter};
use zbus::{Connection, Guid, OwnedObjectPath};

use super::{Host, Listing, Served, ToolTip, WATCHER_PATH, with_call_timeout};

/// Where the fake item says it is: p2p connections carry no sender, so it
/// registers with its name and path together.
const BUS: &str = "org.kde.StatusNotifierItem-7-1";
const PATH: &str = "/StatusNotifierItem/steam";

type Calls = Arc<Mutex<Vec<String>>>;

fn calls(calls: &Calls) -> Vec<String> {
    calls.lock().unwrap_or_else(PoisonError::into_inner).clone()
}

/// A stand-in for Steam's item, recording its calls.
struct FakeItem {
    calls: Calls,
    title: String,
    menu: &'static str,
}

#[zbus::interface(name = "org.kde.StatusNotifierItem")]
impl FakeItem {
    fn activate(&self, x: i32, y: i32) {
        self.record(format!("activate {x} {y}"));
    }
    fn secondary_activate(&self, x: i32, y: i32) {
        self.record(format!("secondary {x} {y}"));
    }
    fn context_menu(&self, x: i32, y: i32) {
        self.record(format!("menu {x} {y}"));
    }
    fn scroll(&self, delta: i32, orientation: &str) {
        self.record(format!("scroll {delta} {orientation}"));
    }
    #[zbus(property)]
    fn id(&self) -> String {
        "steam".to_owned()
    }
    #[zbus(property)]
    fn title(&self) -> String {
        self.title.clone()
    }
    #[zbus(property)]
    fn status(&self) -> String {
        "Passive".to_owned()
    }
    #[zbus(property)]
    fn icon_name(&self) -> String {
        "steam_tray_mono".to_owned()
    }
    /// Two sizes, as Electron's items send one and Qt's several.
    #[zbus(property)]
    fn icon_pixmap(&self) -> Vec<(i32, i32, Vec<u8>)> {
        vec![(1, 1, vec![255, 1, 2, 3]), (2, 1, vec![9; 8])]
    }
    #[zbus(property)]
    fn tool_tip(&self) -> ToolTip {
        (String::new(), Vec::new(), "Steam".to_owned(), String::new())
    }
    #[zbus(property)]
    fn menu(&self) -> OwnedObjectPath {
        OwnedObjectPath::try_from(self.menu).expect("a path")
    }
    #[zbus(property)]
    fn item_is_menu(&self) -> bool {
        true
    }
    #[zbus(signal)]
    async fn new_title(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;
}

impl FakeItem {
    fn record(&self, call: String) {
        self.calls
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(call);
    }
}

struct Running {
    server: Connection,
    client: Connection,
    served: Served,
    calls: Calls,
}

/// lyr's watcher and an item with its `menu`, served together.
async fn running(menu: &'static str) -> Running {
    let calls = Calls::default();
    let (server, client) = UnixStream::pair().expect("socket pair");
    let server = Builder::unix_stream(server)
        .server(Guid::generate())
        .p2p()
        .serve_at(
            PATH,
            FakeItem {
                calls: calls.clone(),
                title: "Steam".to_owned(),
                menu,
            },
        )
        .build();
    let client = Builder::unix_stream(client).p2p().build();
    let (server, client) = tokio::try_join!(server, client).expect("connected");
    let served = Served::at(&server).await.expect("served");
    Running {
        server,
        client,
        served,
        calls,
    }
}

/// The watcher, as an application sees it.
#[zbus::proxy(
    interface = "org.kde.StatusNotifierWatcher",
    default_service = "org.kde.StatusNotifierWatcher",
    default_path = "/StatusNotifierWatcher"
)]
trait Application {
    fn register_status_notifier_item(&self, service: &str) -> zbus::Result<()>;
    fn register_status_notifier_host(&self, service: &str) -> zbus::Result<()>;
    #[zbus(property)]
    fn registered_status_notifier_items(&self) -> zbus::Result<Vec<String>>;
    #[zbus(property)]
    fn is_status_notifier_host_registered(&self) -> zbus::Result<bool>;
    #[zbus(signal)]
    fn status_notifier_item_unregistered(&self, service: String) -> zbus::Result<()>;
}

async fn application(client: &Connection) -> ApplicationProxy<'static> {
    ApplicationProxy::builder(client)
        .cache_properties(zbus::proxy::CacheProperties::No)
        .build()
        .await
        .expect("proxy")
}

#[tokio::test]
async fn a_registered_item_is_listed_and_read() {
    let running = running("/MenuBar").await;
    let app = application(&running.client).await;
    app.register_status_notifier_item(&format!("{BUS}{PATH}"))
        .await
        .expect("registered");
    app.register_status_notifier_item(&format!("{BUS}{PATH}"))
        .await
        .expect("again");
    assert_eq!(
        app.registered_status_notifier_items().await.expect("read"),
        [format!("{BUS}{PATH}")],
        "listed once"
    );
    let listing = Host::on(&running.client)
        .await
        .expect("host")
        .items()
        .await
        .expect("read");
    assert!(listing.unreadable.is_empty(), "{:?}", listing.unreadable);
    let [item] = &listing.items[..] else {
        panic!("one item: {listing:?}");
    };
    assert_eq!((item.bus_name.as_str(), item.path.as_str()), (BUS, PATH));
    assert_eq!(item.id, "steam");
    assert_eq!(item.title, "Steam");
    assert_eq!(item.status, Status::Passive);
    assert_eq!(item.icon_name, "steam_tray_mono");
    assert_eq!(item.tooltip, "Steam");
    assert_eq!(item.menu.as_deref(), Some("/MenuBar"));
    assert!(item.item_is_menu);
    assert!(item.attention_icon_name.is_empty(), "not set");
    assert_eq!(
        item.icon_pixmap,
        [
            Pixmap {
                width: 1,
                height: 1,
                data: vec![255, 1, 2, 3]
            },
            Pixmap {
                width: 2,
                height: 1,
                data: vec![9; 8]
            }
        ]
    );
    assert!(item.attention_icon_pixmap.is_empty(), "not set");
}

#[tokio::test]
async fn a_root_menu_path_is_no_menu_and_a_bad_name_is_refused() {
    let running = running("/").await;
    let app = application(&running.client).await;
    assert!(
        app.register_status_notifier_item("not a bus name")
            .await
            .is_err()
    );
    app.register_status_notifier_item(&format!("{BUS}{PATH}"))
        .await
        .expect("registered");
    let listing = Host::on(&running.client)
        .await
        .expect("host")
        .items()
        .await
        .expect("read");
    assert_eq!(listing.items[0].menu, None);
}

#[tokio::test]
async fn an_item_that_does_not_answer_is_named() {
    let running = running("/MenuBar").await;
    let app = application(&running.client).await;
    app.register_status_notifier_item("org.example.Gone")
        .await
        .expect("registered");
    let Listing { items, unreadable } = Host::on(&running.client)
        .await
        .expect("host")
        .items()
        .await
        .expect("read");
    assert!(items.is_empty());
    assert_eq!(unreadable.len(), 1);
    assert!(unreadable[0].starts_with("org.example.Gone/StatusNotifierItem: "));
}

#[tokio::test]
async fn a_vanished_name_takes_its_items_and_the_last_host() {
    let running = running("/MenuBar").await;
    let app = application(&running.client).await;
    let mut unregistered = app
        .receive_status_notifier_item_unregistered()
        .await
        .expect("subscribed");
    app.register_status_notifier_item(&format!("{BUS}{PATH}"))
        .await
        .expect("registered");
    app.register_status_notifier_host("org.kde.StatusNotifierHost-9")
        .await
        .expect("registered");
    assert!(
        app.is_status_notifier_host_registered()
            .await
            .expect("read")
    );

    running.served.vanished(BUS).await.expect("dropped");
    assert!(
        app.registered_status_notifier_items()
            .await
            .expect("read")
            .is_empty()
    );
    let signal = tokio::time::timeout(Duration::from_secs(5), unregistered.next())
        .await
        .expect("a signal within 5 s")
        .expect("a signal");
    assert_eq!(signal.args().expect("args").service, format!("{BUS}{PATH}"));
    assert!(
        app.is_status_notifier_host_registered()
            .await
            .expect("read")
    );
    running
        .served
        .vanished("org.kde.StatusNotifierHost-9")
        .await
        .expect("dropped");
    assert!(
        !app.is_status_notifier_host_registered()
            .await
            .expect("read")
    );
}

#[tokio::test]
async fn clicks_and_steps_reach_the_item() {
    let running = running("/MenuBar").await;
    let host = Host::on(&running.client).await.expect("host");
    host.click(BUS, PATH, Click::Activate, (10, 20))
        .await
        .expect("clicked");
    host.click(BUS, PATH, Click::SecondaryActivate, (1, 2))
        .await
        .expect("clicked");
    host.click(BUS, PATH, Click::Menu, (3, 4))
        .await
        .expect("clicked");
    host.scroll(BUS, PATH, true).await.expect("stepped");
    assert_eq!(
        calls(&running.calls),
        [
            "activate 10 20",
            "secondary 1 2",
            "menu 3 4",
            "scroll -1 vertical"
        ]
    );
}

#[tokio::test]
async fn a_new_title_is_read_again() {
    let running = running("/MenuBar").await;
    application(&running.client)
        .await
        .register_status_notifier_item(&format!("{BUS}{PATH}"))
        .await
        .expect("registered");
    let host = Host::on(&running.client).await.expect("host");
    let (sender, mut listings) = watch::channel(Listing::default());
    let source = tokio::spawn(async move { host.watch(&sender).await });
    let next = async |listings: &mut watch::Receiver<Listing>| {
        tokio::time::timeout(Duration::from_secs(5), listings.changed())
            .await
            .expect("a listing within 5 s")
            .expect("the source is running");
        listings.borrow_and_update().items[0].title.clone()
    };
    assert_eq!(next(&mut listings).await, "Steam");
    let item: InterfaceRef<FakeItem> = running
        .server
        .object_server()
        .interface(PATH)
        .await
        .expect("served");
    item.get_mut().await.title = "Steam (2 updates)".to_owned();
    FakeItem::new_title(item.signal_emitter())
        .await
        .expect("signalled");
    assert_eq!(next(&mut listings).await, "Steam (2 updates)");
    assert_eq!(WATCHER_PATH, "/StatusNotifierWatcher");
    source.abort();
}

#[tokio::test]
async fn the_watcher_signals_each_change() {
    use zbus::message::Type;
    use zbus::{MatchRule, MessageStream};

    let running = running("/MenuBar").await;
    let rule = MatchRule::builder()
        .msg_type(Type::Signal)
        .path(WATCHER_PATH)
        .build()
        .expect("a rule");
    let mut signals = MessageStream::for_match_rule(rule, &running.client, None)
        .await
        .expect("subscribed");
    // Each signal by name, a property change by the property it names.
    let mut next = async || {
        let message = tokio::time::timeout(Duration::from_secs(5), signals.next())
            .await
            .expect("a signal within 5 s")
            .expect("a signal")
            .expect("a message");
        let header = message.header();
        let member = header.member().expect("a member").to_string();
        if member != "PropertiesChanged" {
            return member;
        }
        let (_, changed, _): (
            String,
            std::collections::HashMap<String, zbus::OwnedVariant>,
            Vec<String>,
        ) = message.body().deserialize().expect("its arguments");
        let mut names: Vec<String> = changed.into_keys().collect();
        names.sort();
        names.join(",")
    };
    let app = application(&running.client).await;
    app.register_status_notifier_item(&format!("{BUS}{PATH}"))
        .await
        .expect("registered");
    assert_eq!(next().await, "RegisteredStatusNotifierItems");
    assert_eq!(next().await, "StatusNotifierItemRegistered");
    app.register_status_notifier_host("org.kde.StatusNotifierHost-9")
        .await
        .expect("registered");
    assert_eq!(next().await, "IsStatusNotifierHostRegistered");
    assert_eq!(next().await, "StatusNotifierHostRegistered");
    running.served.vanished(BUS).await.expect("dropped");
    assert_eq!(next().await, "RegisteredStatusNotifierItems");
    assert_eq!(next().await, "StatusNotifierItemUnregistered");
    running
        .served
        .vanished("org.kde.StatusNotifierHost-9")
        .await
        .expect("dropped");
    assert_eq!(next().await, "IsStatusNotifierHostRegistered");
    assert_eq!(next().await, "StatusNotifierHostUnregistered");
}

/// A menu: "_Open" (id 1), a separator (2), a disabled "Quit" (3), a
/// submenu "More" (4) holding a checked "Sync" (5), and a hidden entry (6).
struct FakeMenu {
    calls: Calls,
}

fn layout(
    id: i32,
    properties: &[(&str, zbus::Variant<'static>)],
    children: Vec<super::Layout>,
) -> super::Layout {
    (
        id,
        properties
            .iter()
            .map(|(name, value)| {
                (
                    (*name).to_owned(),
                    zbus::OwnedVariant::try_from(value.clone()).expect("owned"),
                )
            })
            .collect(),
        children
            .into_iter()
            .map(|child| zbus::OwnedVariant::try_from(zbus::Variant::from(child)).expect("owned"))
            .collect(),
    )
}

#[zbus::interface(name = "com.canonical.dbusmenu")]
impl FakeMenu {
    fn get_layout(
        &self,
        parent_id: i32,
        recursion_depth: i32,
        _property_names: Vec<String>,
    ) -> (u32, super::Layout) {
        self.record(format!("layout {parent_id} {recursion_depth}"));
        let text = |value: &'static str| zbus::Variant::from(value);
        let more = layout(
            4,
            &[
                ("label", text("More")),
                ("children-display", text("submenu")),
            ],
            vec![layout(
                5,
                &[
                    ("label", text("Sync")),
                    ("toggle-type", text("checkmark")),
                    ("toggle-state", zbus::Variant::from(1_i32)),
                ],
                vec![],
            )],
        );
        let root = layout(
            0,
            &[],
            vec![
                layout(1, &[("label", text("_Open"))], vec![]),
                layout(2, &[("type", text("separator"))], vec![]),
                layout(
                    3,
                    &[
                        ("label", text("Quit")),
                        ("enabled", zbus::Variant::from(false)),
                    ],
                    vec![],
                ),
                more,
                layout(6, &[("visible", zbus::Variant::from(false))], vec![]),
            ],
        );
        (7, root)
    }

    fn event(&self, id: i32, event_id: &str, _data: zbus::OwnedVariant, _timestamp: u32) {
        self.record(format!("event {id} {event_id}"));
    }

    fn about_to_show(&self, id: i32) -> bool {
        self.record(format!("about to show {id}"));
        false
    }
}

impl FakeMenu {
    fn record(&self, call: String) {
        self.calls
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(call);
    }
}

#[tokio::test]
async fn a_menu_is_read_whole_and_a_click_is_sent() {
    let running = running("/MenuBar").await;
    running
        .server
        .object_server()
        .at(
            "/MenuBar",
            FakeMenu {
                calls: running.calls.clone(),
            },
        )
        .await
        .expect("served");
    let host = Host::on(&running.client).await.expect("host");
    let root = host.menu(BUS, "/MenuBar").await.expect("read");
    let shown: Vec<(i32, &str, bool, bool)> = root
        .shown()
        .map(|entry| {
            (
                entry.id,
                entry.label.as_str(),
                entry.enabled,
                entry.separator,
            )
        })
        .collect();
    assert_eq!(
        shown,
        [
            (1, "Open", true, false),
            (2, "", true, true),
            (3, "Quit", false, false),
            (4, "More", true, false),
        ]
    );
    let sync = root.find(5).expect("in the submenu");
    assert_eq!(
        sync.toggle,
        Some(lyr_core::tray::menu::Toggle {
            radio: false,
            on: Some(true)
        })
    );
    assert!(root.find(1).expect("open").toggle.is_none());
    host.menu_clicked(BUS, "/MenuBar", 1)
        .await
        .expect("clicked");
    assert_eq!(
        calls(&running.calls),
        ["about to show 0", "layout 0 -1", "event 1 clicked"]
    );
}

#[tokio::test]
async fn an_item_without_its_menu_is_an_error() {
    let running = running("/MenuBar").await;
    let host = Host::on(&running.client).await.expect("host");
    assert!(host.menu(BUS, "/MenuBar").await.is_err());
}

#[tokio::test]
async fn a_watcher_another_process_has_is_left_to_it() {
    let bus = crate::bus::test::Bus::start();
    let theirs = bus.connect().await;
    let ours = bus.connect().await;

    assert!(super::serve(&theirs).await.expect("served").is_some());
    assert!(
        super::serve(&ours)
            .await
            .expect("left to the other")
            .is_none()
    );
}

/// An item whose `GetAll` never answers, as Spotify's was seen doing.
struct HungItem;

#[zbus::interface(name = "org.kde.StatusNotifierItem")]
impl HungItem {
    #[zbus(property)]
    async fn id(&self) -> String {
        std::future::pending().await
    }
}

#[tokio::test]
async fn an_item_that_hangs_does_not_hold_up_the_others() {
    const HUNG: &str = "/StatusNotifierItem/hung";
    let calls = Calls::default();
    let (server, client) = UnixStream::pair().expect("socket pair");
    let server = Builder::unix_stream(server)
        .server(Guid::generate())
        .p2p()
        .serve_at(HUNG, HungItem)
        .serve_at(
            PATH,
            FakeItem {
                calls,
                title: "Steam".to_owned(),
                menu: "/MenuBar",
            },
        )
        .build();
    let client = with_call_timeout(Builder::unix_stream(client).p2p()).build();
    let (server, client) = tokio::try_join!(server, client).expect("connected");
    let _served = Served::at(&server).await.expect("served");
    let app = application(&client).await;
    for path in [HUNG, PATH] {
        app.register_status_notifier_item(&format!("{BUS}{path}"))
            .await
            .expect("registered");
    }
    let host = Host::on(&client).await.expect("host");
    let read = tokio::time::timeout(Duration::from_secs(10), host.items())
        .await
        .expect("the hung item was given up on")
        .expect("read");
    assert_eq!(read.items.len(), 1, "the other item is read");
    assert_eq!(read.items[0].title, "Steam");
    assert_eq!(read.unreadable.len(), 1);
    assert!(read.unreadable[0].starts_with(&format!("{BUS}{HUNG}: ")));
}
