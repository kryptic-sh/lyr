//! The tray on Linux: the StatusNotifierWatcher, which lyr serves when no
//! other process does, as waybar does, and the host that reads the items
//! registered with whichever watcher there is. Names and types are from the
//! KDE StatusNotifierItem spec as waybar's `protocol/*.xml` carry it
//! (master `c15ec02`, read 2026-10-20).
//!
//! Items are few, so a change to any of them, or to the list, reads them
//! all again.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use futures_util::StreamExt;
use futures_util::stream::select;
use lyr_core::tray::menu::{self, Entry};
use lyr_core::tray::{Item, Registry, Status, item_address};
use tokio::sync::watch;
use zbus::fdo::{DBusProxy, PropertiesProxy, RequestNameFlags};
use zbus::message::{Header, Type};
use zbus::names::{BusName, InterfaceName};
use zbus::object_server::SignalEmitter;
use zbus::{Connection, MatchRule, MessageStream, OwnedObjectPath, OwnedVariant, Variant};

use crate::bus::Follow;

/// The watcher's well-known name, and its object.
pub const WATCHER: &str = "org.kde.StatusNotifierWatcher";
pub const WATCHER_PATH: &str = "/StatusNotifierWatcher";
/// An item's interface.
const ITEM: &str = "org.kde.StatusNotifierItem";
/// The spec's protocol version, which waybar's watcher reports too.
const PROTOCOL_VERSION: i32 = 0;

/// lyr's watcher: who registered, with the spec's signals.
pub struct Watcher {
    registry: Arc<Mutex<Registry>>,
}

impl Watcher {
    fn registry(&self) -> std::sync::MutexGuard<'_, Registry> {
        self.registry.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[zbus::interface(name = "org.kde.StatusNotifierWatcher")]
impl Watcher {
    async fn register_status_notifier_item(
        &self,
        service: &str,
        #[zbus(header)] header: Header<'_>,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> zbus::fdo::Result<()> {
        let sender = header.sender().map_or("", |sender| sender.as_str());
        let (bus_name, path) = item_address(service, sender);
        BusName::try_from(bus_name.as_str()).map_err(|_| {
            zbus::fdo::Error::InvalidArgs(format!("D-Bus bus name '{bus_name}' is not valid"))
        })?;
        if self.registry().register_item(&bus_name, &path) {
            self.registered_status_notifier_items_changed(&emitter)
                .await?;
            Self::status_notifier_item_registered(&emitter, &format!("{bus_name}{path}")).await?;
        }
        Ok(())
    }

    async fn register_status_notifier_host(
        &self,
        service: &str,
        #[zbus(header)] header: Header<'_>,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> zbus::fdo::Result<()> {
        // A path alone is on the caller's connection, as for items.
        let bus_name = if service.starts_with('/') {
            header.sender().map_or("", |sender| sender.as_str())
        } else {
            service
        };
        BusName::try_from(bus_name).map_err(|_| {
            zbus::fdo::Error::InvalidArgs(format!("D-Bus bus name '{bus_name}' is not valid"))
        })?;
        if self.registry().register_host(bus_name) {
            self.is_status_notifier_host_registered_changed(&emitter)
                .await?;
            Self::status_notifier_host_registered(&emitter).await?;
        }
        Ok(())
    }

    #[zbus(property)]
    fn registered_status_notifier_items(&self) -> Vec<String> {
        self.registry().items()
    }

    #[zbus(property)]
    fn is_status_notifier_host_registered(&self) -> bool {
        self.registry().has_host()
    }

    #[zbus(property)]
    fn protocol_version(&self) -> i32 {
        PROTOCOL_VERSION
    }

    #[zbus(signal)]
    async fn status_notifier_item_registered(
        emitter: &SignalEmitter<'_>,
        service: &str,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn status_notifier_item_unregistered(
        emitter: &SignalEmitter<'_>,
        service: &str,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn status_notifier_host_registered(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn status_notifier_host_unregistered(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;
}

/// The watcher as lyr serves it.
#[derive(Clone, Debug)]
pub struct Served {
    connection: Connection,
    registry: Arc<Mutex<Registry>>,
}

impl Served {
    /// Serves a watcher at [`WATCHER_PATH`] on `connection`, without taking
    /// a name: a peer-to-peer connection has none to take.
    ///
    /// # Errors
    ///
    /// When the object cannot be served.
    pub async fn at(connection: &Connection) -> zbus::Result<Self> {
        let registry = Arc::new(Mutex::new(Registry::default()));
        connection
            .object_server()
            .at(
                WATCHER_PATH,
                Watcher {
                    registry: registry.clone(),
                },
            )
            .await?;
        Ok(Self {
            connection: connection.clone(),
            registry,
        })
    }

    /// Drops what `bus_name` registered, now that it is gone, with the
    /// signals the spec asks for.
    ///
    /// # Errors
    ///
    /// When a signal cannot be sent.
    pub async fn vanished(&self, bus_name: &str) -> zbus::Result<()> {
        let (items, last_host) = self
            .registry
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .vanished(bus_name);
        let watcher = self
            .connection
            .object_server()
            .interface::<_, Watcher>(WATCHER_PATH)
            .await?;
        let emitter = watcher.signal_emitter();
        if !items.is_empty() {
            watcher
                .get()
                .await
                .registered_status_notifier_items_changed(emitter)
                .await?;
        }
        for item in items {
            Watcher::status_notifier_item_unregistered(emitter, &item).await?;
        }
        if last_host {
            watcher
                .get()
                .await
                .is_status_notifier_host_registered_changed(emitter)
                .await?;
            Watcher::status_notifier_host_unregistered(emitter).await?;
        }
        Ok(())
    }

    /// Follows names leaving the bus, dropping what each registered, until
    /// the bus goes.
    ///
    /// # Errors
    ///
    /// When subscribing fails, or a signal cannot be sent.
    pub async fn follow_names(&self) -> zbus::Result<()> {
        let dbus = DBusProxy::new(&self.connection).await?;
        let mut owners = dbus.receive_name_owner_changed().await?;
        while let Some(change) = owners.next().await {
            let args = change.args()?;
            if args.new_owner().is_none() {
                self.vanished(args.name()).await?;
            }
        }
        Ok(())
    }
}

/// Serves lyr's watcher on the session bus `connection` and takes
/// [`WATCHER`], unless another process has it, which then stays the
/// watcher: `None`.
///
/// # Errors
///
/// When the bus does not answer.
pub async fn serve(connection: &Connection) -> zbus::Result<Option<Served>> {
    let served = Served::at(connection).await?;
    if crate::bus::claim(connection, WATCHER, RequestNameFlags::DO_NOT_QUEUE).await? {
        return Ok(Some(served));
    }
    connection
        .object_server()
        .remove::<Watcher, _>(WATCHER_PATH)
        .await?;
    Ok(None)
}

/// The tray on the session bus: lyr's watcher when no other process has
/// the name ([`serve`]), and the host on the same connection.
///
/// # Errors
///
/// When the session bus cannot be reached or does not answer.
pub async fn session() -> zbus::Result<(Host, Option<Served>)> {
    let connection = Connection::session().await?;
    let served = serve(&connection).await?;
    Ok((Host::on(&connection).await?, served))
}

/// How the host sees a watcher, lyr's or another's.
#[zbus::proxy(
    interface = "org.kde.StatusNotifierWatcher",
    default_service = "org.kde.StatusNotifierWatcher",
    default_path = "/StatusNotifierWatcher"
)]
trait WatcherClient {
    fn register_status_notifier_host(&self, service: &str) -> zbus::Result<()>;
    #[zbus(property)]
    fn registered_status_notifier_items(&self) -> zbus::Result<Vec<String>>;
}

/// An item's calls.
#[zbus::proxy(interface = "org.kde.StatusNotifierItem")]
trait ItemControl {
    fn activate(&self, x: i32, y: i32) -> zbus::Result<()>;
    fn secondary_activate(&self, x: i32, y: i32) -> zbus::Result<()>;
    fn context_menu(&self, x: i32, y: i32) -> zbus::Result<()>;
    fn scroll(&self, delta: i32, orientation: &str) -> zbus::Result<()>;
}

/// An item's menu (`com.canonical.dbusmenu`).
#[zbus::proxy(interface = "com.canonical.dbusmenu")]
trait DbusMenu {
    fn get_layout(
        &self,
        parent_id: i32,
        recursion_depth: i32,
        property_names: &[&str],
    ) -> zbus::Result<(u32, Layout)>;
    fn event(
        &self,
        id: i32,
        event_id: &str,
        data: &Variant<'_>,
        timestamp: u32,
    ) -> zbus::Result<()>;
    fn about_to_show(&self, id: i32) -> zbus::Result<bool>;
}

/// A menu entry as `GetLayout` lays it out: its id, its properties, and its
/// children, each in the same shape inside a variant.
pub(crate) type Layout = (i32, HashMap<String, OwnedVariant>, Vec<OwnedVariant>);

/// The entry `layout` describes, with its submenus. A child in another
/// shape is left out.
fn entry((id, properties, children): &Layout) -> Entry {
    let text = |name: &str| {
        properties
            .get(name)
            .and_then(|value| String::try_from(value.clone()).ok())
    };
    let flag = |name: &str| {
        properties
            .get(name)
            .and_then(|value| bool::try_from(value).ok())
    };
    let defaults = Entry::new(*id);
    Entry {
        label: text("label")
            .map(|raw| menu::label(&raw))
            .unwrap_or_default(),
        enabled: flag("enabled").unwrap_or(defaults.enabled),
        visible: flag("visible").unwrap_or(defaults.visible),
        separator: text("type").as_deref() == Some("separator"),
        toggle: menu::toggle(
            &text("toggle-type").unwrap_or_default(),
            properties
                .get("toggle-state")
                .and_then(|value| i32::try_from(value).ok())
                .unwrap_or(-1),
        ),
        children: children
            .iter()
            .filter_map(|child| Layout::try_from(child.clone()).ok())
            .map(|child| entry(&child))
            .collect(),
        ..defaults
    }
}

/// `ToolTip`: icon name, icon pixmaps, title and description.
pub(crate) type ToolTip = (String, Vec<(i32, i32, Vec<u8>)>, String, String);

/// An item from its properties, as `GetAll` gives them; what it does not
/// set is left empty.
fn item(bus_name: &str, path: &str, properties: &HashMap<String, OwnedVariant>) -> Item {
    let text = |name: &str| {
        properties
            .get(name)
            .and_then(|value| String::try_from(value.clone()).ok())
            .unwrap_or_default()
    };
    let menu = properties
        .get("Menu")
        .and_then(|value| OwnedObjectPath::try_from(value.clone()).ok())
        .filter(|menu| menu.as_str() != "/")
        .map(|menu| menu.to_string());
    let tooltip = properties
        .get("ToolTip")
        .and_then(|value| ToolTip::try_from(value.clone()).ok())
        .map(|(_, _, title, _)| title)
        .unwrap_or_default();
    Item {
        bus_name: bus_name.to_owned(),
        path: path.to_owned(),
        id: text("Id"),
        title: text("Title"),
        status: Status::parse(&text("Status")),
        icon_name: text("IconName"),
        attention_icon_name: text("AttentionIconName"),
        icon_theme_path: text("IconThemePath"),
        tooltip,
        menu,
        item_is_menu: properties
            .get("ItemIsMenu")
            .and_then(|value| bool::try_from(value).ok())
            .unwrap_or(false),
    }
}

/// The items a watcher lists: those read, in its order, and why the others
/// could not be.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Listing {
    pub items: Vec<Item>,
    pub unreadable: Vec<String>,
}

/// The tray's host on one connection.
#[derive(Clone, Debug)]
pub struct Host {
    connection: Connection,
    watcher: WatcherClientProxy<'static>,
}

impl Host {
    /// The host on `connection`, reading the items of the watcher there.
    ///
    /// # Errors
    ///
    /// When the proxy cannot be built.
    pub async fn on(connection: &Connection) -> zbus::Result<Self> {
        let watcher = WatcherClientProxy::builder(connection)
            .cache_properties(zbus::proxy::CacheProperties::No)
            .build()
            .await?;
        Ok(Self {
            connection: connection.clone(),
            watcher,
        })
    }

    /// Takes `org.kde.StatusNotifierHost-<pid>` and registers it with the
    /// watcher, as the spec asks of a host.
    ///
    /// # Errors
    ///
    /// When the name cannot be taken or the watcher refuses.
    pub async fn register(&self) -> zbus::Result<()> {
        let name = format!("org.kde.StatusNotifierHost-{}", std::process::id());
        self.connection.request_name(name.as_str()).await?;
        self.watcher.register_status_notifier_host(&name).await
    }

    /// The items the watcher lists, in its order, and those that could not
    /// be read.
    ///
    /// # Errors
    ///
    /// When the watcher does not answer.
    pub async fn items(&self) -> zbus::Result<Listing> {
        let mut listing = Listing::default();
        for registered in self.watcher.registered_status_notifier_items().await? {
            let Some(slash) = registered.find('/') else {
                listing
                    .unreadable
                    .push(format!("{registered}: no object path"));
                continue;
            };
            let (bus_name, path) = registered.split_at(slash);
            match self.properties(bus_name, path).await {
                Ok(properties) => listing.items.push(item(bus_name, path, &properties)),
                Err(error) => listing.unreadable.push(format!("{registered}: {error}")),
            }
        }
        Ok(listing)
    }

    async fn properties(
        &self,
        bus_name: &str,
        path: &str,
    ) -> zbus::Result<HashMap<String, OwnedVariant>> {
        let properties = PropertiesProxy::builder(&self.connection)
            .destination(bus_name.to_owned())
            .path(path.to_owned())
            .cache_properties(zbus::proxy::CacheProperties::No)
            .build()
            .await?;
        Ok(properties.get_all(InterfaceName::try_from(ITEM)?).await?)
    }

    /// The calls of the item at `bus_name` and `path`.
    async fn control(&self, bus_name: &str, path: &str) -> zbus::Result<ItemControlProxy<'static>> {
        ItemControlProxy::builder(&self.connection)
            .destination(bus_name.to_owned())
            .path(path.to_owned())
            .build()
            .await
    }

    /// The menu at `menu` of the item at `bus_name`, read whole after
    /// telling it the menu is about to show.
    ///
    /// # Errors
    ///
    /// When the item serves no such menu, as some that name one do not
    /// (Steam, by waybar's `Item::handleClick`): the caller falls back to
    /// `ContextMenu`.
    pub async fn menu(&self, bus_name: &str, menu: &str) -> zbus::Result<Entry> {
        let proxy = self.dbus_menu(bus_name, menu).await?;
        proxy.about_to_show(0).await?;
        let (_revision, layout) = proxy.get_layout(0, -1, &[]).await?;
        Ok(entry(&layout))
    }

    /// Tells the menu at `menu` of the item at `bus_name` that its entry
    /// `id` was clicked.
    ///
    /// # Errors
    ///
    /// When the item refuses or does not answer.
    pub async fn menu_clicked(&self, bus_name: &str, menu: &str, id: i32) -> zbus::Result<()> {
        // The spec gives a click no data; libdbusmenu sends an int 0.
        self.dbus_menu(bus_name, menu)
            .await?
            .event(id, "clicked", &Variant::from(0_i32), 0)
            .await
    }

    async fn dbus_menu(&self, bus_name: &str, menu: &str) -> zbus::Result<DbusMenuProxy<'static>> {
        DbusMenuProxy::builder(&self.connection)
            .destination(bus_name.to_owned())
            .path(menu.to_owned())
            .cache_properties(zbus::proxy::CacheProperties::No)
            .build()
            .await
    }

    /// Asks the item at `bus_name` and `path` to act on a click at `x`,
    /// `y` on the screen: `Activate`, `SecondaryActivate` or `ContextMenu`.
    ///
    /// # Errors
    ///
    /// When the item refuses or does not answer.
    pub async fn click(
        &self,
        bus_name: &str,
        path: &str,
        click: lyr_core::tray::Click,
        (x, y): (i32, i32),
    ) -> zbus::Result<()> {
        use lyr_core::tray::Click;
        let item = self.control(bus_name, path).await?;
        match click {
            Click::Activate => item.activate(x, y).await,
            Click::SecondaryActivate => item.secondary_activate(x, y).await,
            Click::Menu => item.context_menu(x, y).await,
        }
    }

    /// A wheel step on the item at `bus_name` and `path`.
    ///
    /// # Errors
    ///
    /// When the item refuses or does not answer.
    pub async fn scroll(&self, bus_name: &str, path: &str, up: bool) -> zbus::Result<()> {
        let (delta, orientation) = lyr_core::tray::scroll(up);
        self.control(bus_name, path)
            .await?
            .scroll(delta, orientation)
            .await
    }

    /// Sends the items to `items` now and after every change: one
    /// registering or going, or an item's `New…` signal, until the bus goes
    /// or every receiver is dropped.
    ///
    /// # Errors
    ///
    /// When subscribing or reading the list fails.
    pub async fn watch(&self, items: &watch::Sender<Listing>) -> zbus::Result<()> {
        let listed = MatchRule::builder()
            .msg_type(Type::Signal)
            .interface(WATCHER)
            .build()?;
        let changed = MatchRule::builder()
            .msg_type(Type::Signal)
            .interface(ITEM)
            .build()?;
        // Subscribed before the first read, so a change in between is not lost.
        let mut changes = Follow::new(select(
            MessageStream::for_match_rule(listed, &self.connection, None).await?,
            MessageStream::for_match_rule(changed, &self.connection, None).await?,
        ));
        let wanted = |_| Ok(true);
        items.send_replace(changes.during(self.items(), wanted).await??);
        while changes.changed(wanted).await? {
            if !crate::bus::publish(items, changes.during(self.items(), wanted).await??) {
                break;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
