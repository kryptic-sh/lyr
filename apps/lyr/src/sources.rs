//! Running the module sources (`docs/plan/02-architecture.md`): each on a
//! tokio runtime of its own thread, handing its state to the main loop over a
//! channel and waking it, so the main loop never blocks on a source and no
//! source is polled.
//!
//! Linux has the battery (UPower), the power profile
//! (power-profiles-daemon), the network (NetworkManager), bluetooth (BlueZ), the time zone (timedate1) and
//! the workspaces (Hyprland). Other platforms have no sources yet, and their
//! modules are left out of the bar.

use std::sync::mpsc;

use crcbl_shell::ShellWaker;

/// A source's new state. Empty on a platform with no sources.
#[derive(Debug)]
pub enum Update {
    /// The battery, or `None` when there is none to show.
    #[cfg(target_os = "linux")]
    Battery(Option<lyr_core::battery::Reading>),
    /// The power profiles, or `None` when the daemon lists none.
    #[cfg(target_os = "linux")]
    PowerProfile(Option<lyr_sources::power_profile::Profiles>),
    /// NetworkManager's reading, or `None` before the first.
    #[cfg(target_os = "linux")]
    Network(Option<lyr_core::network::Reading>),
    /// What the Wi-Fi menu shows.
    #[cfg(target_os = "linux")]
    Wifi(lyr_core::wifi_menu::Wifi),
    /// How a join the Wi-Fi menu asked for ended.
    #[cfg(target_os = "linux")]
    WifiJoined(crate::wifi_menu::Joined),
    /// BlueZ: the module's reading and the menu's adapter and devices.
    #[cfg(target_os = "linux")]
    Bluetooth(lyr_sources::bluetooth::Snapshot),
    /// The pairing agent asks, about the device at this path, under this
    /// id.
    #[cfg(target_os = "linux")]
    BluetoothAsk(u64, String, lyr_core::bluetooth_menu::Question),
    /// The pairing agent's question is no longer asked.
    #[cfg(target_os = "linux")]
    BluetoothCancel,
    /// Hyprland's workspaces.
    #[cfg(target_os = "linux")]
    Workspaces(lyr_wm::hyprland::State),
    /// The system time zone's name, or `None` when none is set.
    #[cfg(target_os = "linux")]
    Zone(Option<String>),
    /// lyr is the notification daemon, with this store.
    #[cfg(target_os = "linux")]
    Notifications(std::sync::Arc<std::sync::Mutex<lyr_core::notifications::Store>>),
    /// A notification arrived or an application closed one.
    #[cfg(target_os = "linux")]
    Notification(lyr_sources::notifications::linux::Change),
    /// The media players on the session bus.
    #[cfg(target_os = "linux")]
    Players(Vec<lyr_core::media::Player>),
    /// The tray's items, in the watcher's order.
    #[cfg(target_os = "linux")]
    Tray(Vec<lyr_core::tray::Item>),
    /// A tray item's menu, read for the menu the bar asked to open.
    #[cfg(target_os = "linux")]
    TrayMenu(crate::tray_menu::Read),
}

/// What the bar asks of a tray item, by its bus name and object path.
#[cfg(target_os = "linux")]
#[derive(Debug)]
pub enum TrayRequest {
    /// A click, at this point on the bar.
    Click {
        bus_name: String,
        path: String,
        click: lyr_core::tray::Click,
        at: (i32, i32),
    },
    /// A wheel step, away from the user when `up`.
    Scroll {
        bus_name: String,
        path: String,
        up: bool,
    },
    /// Its menu, to open under its button; `ContextMenu` when it serves
    /// none.
    Menu {
        path: String,
        at: (i32, i32),
        read: crate::tray_menu::Read,
    },
    /// The entry `id` of the menu at `menu` was clicked.
    MenuClicked {
        bus_name: String,
        menu: String,
        id: i32,
    },
}

/// What the Bluetooth menu asks of BlueZ, by object path.
#[cfg(target_os = "linux")]
#[derive(Debug)]
pub enum BluetoothRequest {
    Switch(String, lyr_core::bluetooth_menu::Switch, bool),
    Scan(String, bool),
    Device(String, lyr_core::bluetooth_menu::DeviceAction),
    /// The adapter's path, then the device's.
    Forget(String, String),
    /// The device's path, then its new name.
    Rename(String, String),
    /// The answer to the pairing agent's question with this id.
    Answer(u64, lyr_core::bluetooth_menu::Reply),
}

/// What the Wi-Fi menu asks of NetworkManager.
#[cfg(target_os = "linux")]
#[derive(Debug)]
pub enum WifiRequest {
    Radio(bool),
    /// A scan on the device at this path.
    Scan(String),
    Join(JoinRequest),
    /// The saved profiles at these paths are to go.
    Forget(Vec<String>),
}

/// A join the Wi-Fi menu asked for.
#[cfg(target_os = "linux")]
#[derive(Debug)]
pub struct JoinRequest {
    /// The device's path.
    pub device: String,
    pub join: lyr_sources::network::Join,
    /// The network's name and security, for what a failure means.
    pub ssid: String,
    pub security: lyr_core::wifi_menu::Security,
    /// A new password for the saved profile it joins with, set first.
    pub new_password: Option<String>,
}

/// What the bar asks of the notification daemon.
#[cfg(target_os = "linux")]
#[derive(Debug)]
enum NotificationRequest {
    Close(u32, lyr_core::notifications::CloseReason),
    Invoke(u32, String),
    Clear,
}

/// Where the bar sends what it asks of a source. A source that has stopped
/// drops what it is sent; it already logged why it stopped.
#[derive(Debug)]
pub struct Commands {
    #[cfg(target_os = "linux")]
    focus: tokio::sync::mpsc::UnboundedSender<lyr_wm::hyprland::Workspace>,
    #[cfg(target_os = "linux")]
    profile: tokio::sync::mpsc::UnboundedSender<String>,
    #[cfg(target_os = "linux")]
    power: tokio::sync::mpsc::UnboundedSender<lyr_core::power_menu::Entry>,
    #[cfg(target_os = "linux")]
    notifications: tokio::sync::mpsc::UnboundedSender<NotificationRequest>,
    #[cfg(target_os = "linux")]
    media: tokio::sync::mpsc::UnboundedSender<(String, lyr_core::media::Control)>,
    #[cfg(target_os = "linux")]
    bluetooth: tokio::sync::mpsc::UnboundedSender<BluetoothRequest>,
    #[cfg(target_os = "linux")]
    wifi: tokio::sync::mpsc::UnboundedSender<WifiRequest>,
    #[cfg(target_os = "linux")]
    notices: tokio::sync::mpsc::UnboundedSender<(String, String)>,
    #[cfg(target_os = "linux")]
    tray: tokio::sync::mpsc::UnboundedSender<TrayRequest>,
}

#[cfg(target_os = "linux")]
impl Commands {
    /// Switches to `workspace`.
    pub fn focus(&self, workspace: lyr_wm::hyprland::Workspace) {
        let _stopped = self.focus.send(workspace);
    }

    /// Switches to the power profile `name`.
    pub fn set_profile(&self, name: String) {
        let _stopped = self.profile.send(name);
    }

    /// Carries out a power menu entry.
    pub fn power(&self, entry: lyr_core::power_menu::Entry) {
        let _stopped = self.power.send(entry);
    }

    /// Closes the notification `id` for `reason`.
    pub fn close_notification(&self, id: u32, reason: lyr_core::notifications::CloseReason) {
        let _stopped = self
            .notifications
            .send(NotificationRequest::Close(id, reason));
    }

    /// Sends a notice from lyr itself: `summary`, and `body` in markup.
    pub fn notify(&self, summary: String, body: String) {
        let _stopped = self.notices.send((summary, body));
    }

    /// Asks a tray item what the bar wants of it.
    pub fn tray(&self, request: TrayRequest) {
        let _stopped = self.tray.send(request);
    }

    /// Asks NetworkManager what the Wi-Fi menu wants.
    pub fn wifi(&self, request: WifiRequest) {
        let _stopped = self.wifi.send(request);
    }

    /// Presses `control` on the media player at `bus_name`.
    pub fn media(&self, bus_name: String, control: lyr_core::media::Control) {
        let _stopped = self.media.send((bus_name, control));
    }

    /// Asks BlueZ for what the Bluetooth menu's click wants.
    pub fn bluetooth(&self, request: BluetoothRequest) {
        let _stopped = self.bluetooth.send(request);
    }

    /// Closes every notification, as Clear All does.
    pub fn clear_notifications(&self) {
        let _stopped = self.notifications.send(NotificationRequest::Clear);
    }

    /// Invokes the action `key` of the notification `id`.
    pub fn invoke(&self, id: u32, key: String) {
        let _stopped = self
            .notifications
            .send(NotificationRequest::Invoke(id, key));
    }
}

#[cfg(not(target_os = "linux"))]
impl Commands {
    /// Windows: carries out a power menu entry through Win32, on a thread of
    /// its own, since `SetSuspendState` returns only once the machine wakes.
    #[cfg(windows)]
    pub fn power(&self, entry: lyr_core::power_menu::Entry) {
        let spawned = std::thread::Builder::new()
            .name("lyr power".to_owned())
            .spawn(move || {
                if let Err(error) = lyr_sources::power::windows::carry_out(entry) {
                    tracing::warn!("power menu: {}: {error}", entry.label());
                }
            });
        if let Err(error) = spawned {
            tracing::warn!("power menu: {}: no thread: {error}", entry.label());
        }
    }

    /// macOS: the power menu's actions are not built yet (M10).
    #[cfg(target_os = "macos")]
    pub fn power(&self, entry: lyr_core::power_menu::Entry) {
        tracing::warn!(
            "power menu: {} is not built on this platform yet",
            entry.label()
        );
    }

    /// Windows and macOS: no media players are read there yet, so there is
    /// none to control.
    pub fn media(&self, _bus_name: String, _control: lyr_core::media::Control) {}

    /// Windows and macOS: lyr is not the notification daemon there, so it
    /// holds no notification to close.
    pub fn close_notification(&self, _id: u32, _reason: lyr_core::notifications::CloseReason) {}
}

/// Starts every source this platform has. A source that cannot start, or
/// stops, logs why and its module stays out of the bar. With
/// `replace_notifications`, lyr takes the notification daemon's name from
/// whoever has it.
#[cfg_attr(not(target_os = "linux"), allow(unused_variables))]
pub fn start(
    waker: Option<ShellWaker>,
    replace_notifications: bool,
) -> std::io::Result<(mpsc::Receiver<Update>, Commands)> {
    let (sender, receiver) = mpsc::channel();
    #[cfg(target_os = "linux")]
    {
        let (focus, focus_requests) = tokio::sync::mpsc::unbounded_channel();
        let (profile, profile_requests) = tokio::sync::mpsc::unbounded_channel();
        let (power, power_requests) = tokio::sync::mpsc::unbounded_channel();
        let (notifications, notification_requests) = tokio::sync::mpsc::unbounded_channel();
        let (media, media_requests) = tokio::sync::mpsc::unbounded_channel();
        let (bluetooth, bluetooth_requests) = tokio::sync::mpsc::unbounded_channel();
        let (wifi, wifi_requests) = tokio::sync::mpsc::unbounded_channel();
        let (notices, notice_requests) = tokio::sync::mpsc::unbounded_channel();
        let (tray, tray_requests) = tokio::sync::mpsc::unbounded_channel();
        linux::start(
            sender,
            linux::Requests {
                focus: focus_requests,
                profile: profile_requests,
                power: power_requests,
                notifications: notification_requests,
                media: media_requests,
                bluetooth: bluetooth_requests,
                wifi: wifi_requests,
                notices: notice_requests,
                tray: tray_requests,
            },
            replace_notifications,
            waker,
        )?;
        Ok((
            receiver,
            Commands {
                focus,
                profile,
                power,
                notifications,
                media,
                bluetooth,
                wifi,
                notices,
                tray,
            },
        ))
    }
    #[cfg(not(target_os = "linux"))]
    {
        drop((sender, waker));
        Ok((receiver, Commands {}))
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use std::sync::mpsc;

    use crcbl_shell::ShellWaker;
    use lyr_core::media::Control;
    use lyr_core::power_menu::Entry;
    use lyr_core::wifi_menu::Wifi;
    use lyr_sources::battery::linux::UPower;
    use lyr_sources::bluetooth::Snapshot;
    use lyr_sources::bluetooth::linux::BlueZ;
    use lyr_sources::bluetooth::linux::agent::{Asked, Questions};
    use lyr_sources::clock::linux::TimeDate;
    use lyr_sources::mpris::linux::Players;
    use lyr_sources::network::linux::NetworkManager;
    use lyr_sources::power::linux::Logind;
    use lyr_sources::power_profile::linux::PowerProfiles;
    use lyr_wm::hyprland::{Hyprland, Workspace};
    use tokio::sync::mpsc::UnboundedReceiver;
    use tokio::sync::watch;

    use std::sync::{Arc, Mutex};

    use lyr_core::notifications::Store;
    use lyr_sources::notifications::linux::{Daemon, serve_session};

    use super::{
        BluetoothRequest, JoinRequest, NotificationRequest, TrayRequest, Update, WifiRequest,
    };
    use crate::wifi_menu::Joined;

    /// Hands `update` to the main loop and wakes it. `false` once the main
    /// loop has gone.
    fn deliver(sender: &mpsc::Sender<Update>, waker: Option<&ShellWaker>, update: Update) -> bool {
        let delivered = sender.send(update).is_ok();
        if let Some(waker) = waker {
            waker.wake();
        }
        delivered
    }

    /// Where each task reads what the bar asks of it.
    pub(super) struct Requests {
        pub(super) focus: UnboundedReceiver<Workspace>,
        pub(super) profile: UnboundedReceiver<String>,
        pub(super) power: UnboundedReceiver<Entry>,
        pub(super) notifications: UnboundedReceiver<NotificationRequest>,
        pub(super) media: UnboundedReceiver<(String, Control)>,
        pub(super) bluetooth: UnboundedReceiver<BluetoothRequest>,
        pub(super) wifi: UnboundedReceiver<WifiRequest>,
        pub(super) notices: UnboundedReceiver<(String, String)>,
        pub(super) tray: UnboundedReceiver<TrayRequest>,
    }

    pub(super) fn start(
        sender: mpsc::Sender<Update>,
        requests: Requests,
        replace_notifications: bool,
        waker: Option<ShellWaker>,
    ) -> std::io::Result<()> {
        let Requests {
            focus,
            profile,
            power,
            notifications: notification_requests,
            media,
            bluetooth: bluetooth_requests,
            wifi,
            notices: notice_requests,
            tray: tray_requests,
        } = requests;
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        std::thread::Builder::new()
            .name("lyr sources".to_owned())
            .spawn(move || {
                runtime.block_on(async move {
                    tokio::join!(
                        battery(sender.clone(), waker.clone()),
                        network(sender.clone(), wifi, waker.clone()),
                        power_profile(sender.clone(), profile, waker.clone()),
                        bluetooth(sender.clone(), bluetooth_requests, waker.clone()),
                        clock(sender.clone(), waker.clone()),
                        players(sender.clone(), media, waker.clone()),
                        notifications(
                            sender.clone(),
                            notification_requests,
                            replace_notifications,
                            waker.clone(),
                        ),
                        tray(sender.clone(), tray_requests, waker.clone()),
                        workspaces(sender, focus, waker),
                        power_menu(power),
                        notices(notice_requests),
                    );
                });
            })?;
        Ok(())
    }

    async fn battery(sender: mpsc::Sender<Update>, waker: Option<ShellWaker>) {
        let upower = match UPower::system().await {
            Ok(upower) => upower,
            Err(error) => {
                tracing::warn!("battery: UPower is not reachable: {error}");
                return;
            }
        };
        let (readings, mut changed) = watch::channel(None);
        let forward = async {
            while changed.changed().await.is_ok() {
                let reading = *changed.borrow_and_update();
                if !deliver(&sender, waker.as_ref(), Update::Battery(reading)) {
                    break;
                }
            }
        };
        let (watched, ()) = tokio::join!(upower.watch(&readings), forward);
        if let Err(error) = watched {
            tracing::warn!("battery: UPower stopped answering: {error}");
        }
    }

    async fn network(
        sender: mpsc::Sender<Update>,
        mut requests: UnboundedReceiver<WifiRequest>,
        waker: Option<ShellWaker>,
    ) {
        let network = match NetworkManager::system().await {
            Ok(network) => network,
            Err(error) => {
                tracing::warn!("network: NetworkManager is not reachable: {error}");
                return;
            }
        };
        let (readings, mut changed) = watch::channel(None);
        let forward = async {
            while changed.changed().await.is_ok() {
                let reading = changed.borrow_and_update().clone();
                if !deliver(&sender, waker.as_ref(), Update::Network(reading)) {
                    break;
                }
            }
        };
        let (states, mut wifi_changed) = watch::channel(Wifi::default());
        let forward_wifi = async {
            while wifi_changed.changed().await.is_ok() {
                let state = wifi_changed.borrow_and_update().clone();
                if !deliver(&sender, waker.as_ref(), Update::Wifi(state)) {
                    break;
                }
            }
        };
        // One at a time: a join holds the next request until it ends.
        let act = async {
            while let Some(request) = requests.recv().await {
                // What changes comes back through the watch.
                let done = match request {
                    WifiRequest::Radio(on) => network.set_radio(on).await,
                    WifiRequest::Scan(device) => network.scan(&device).await,
                    WifiRequest::Forget(profiles) => network.forget(&profiles).await,
                    WifiRequest::Join(request) => {
                        let joined = join(&network, request).await;
                        if !deliver(&sender, waker.as_ref(), Update::WifiJoined(joined)) {
                            break;
                        }
                        Ok(())
                    }
                };
                if let Err(error) = done {
                    tracing::warn!("wi-fi: {error}");
                }
            }
        };
        let (watched, watched_wifi, (), (), ()) = tokio::join!(
            network.watch(&readings),
            network.watch_wifi(&states),
            forward,
            forward_wifi,
            act
        );
        for watched in [watched, watched_wifi] {
            if let Err(error) = watched {
                tracing::warn!("network: NetworkManager stopped answering: {error}");
            }
        }
    }

    /// Hosts the tray: serves the watcher when no other process does,
    /// registers as a host, sends the items on every change, and carries out
    /// what the bar asks of them.
    async fn tray(
        sender: mpsc::Sender<Update>,
        mut requests: UnboundedReceiver<TrayRequest>,
        waker: Option<ShellWaker>,
    ) {
        use lyr_core::tray::Click;
        use lyr_sources::tray::linux::{Listing, session};

        let (host, served) = match session().await {
            Ok(tray) => tray,
            Err(error) => {
                tracing::warn!("tray: the session bus is not reachable: {error}");
                return;
            }
        };
        match &served {
            Some(_) => tracing::info!("tray: lyr is the StatusNotifierWatcher"),
            None => tracing::info!("tray: another process is the StatusNotifierWatcher"),
        }
        if let Err(error) = host.register().await {
            tracing::warn!("tray: registering as a host: {error}");
        }
        let follow = async {
            if let Some(served) = &served
                && let Err(error) = served.follow_names().await
            {
                tracing::warn!("tray: following names stopped: {error}");
            }
        };
        let (listings, mut changed) = watch::channel(Listing::default());
        let forward = async {
            let mut reported = Vec::new();
            while changed.changed().await.is_ok() {
                let listing = changed.borrow_and_update().clone();
                // Each item that cannot be read is said once, not on every
                // change.
                for unreadable in &listing.unreadable {
                    if !reported.contains(unreadable) {
                        tracing::warn!("tray: {unreadable}");
                    }
                }
                reported = listing.unreadable;
                if !deliver(&sender, waker.as_ref(), Update::Tray(listing.items)) {
                    break;
                }
            }
        };
        let act = async {
            while let Some(request) = requests.recv().await {
                let done = match request {
                    TrayRequest::Click {
                        bus_name,
                        path,
                        click,
                        at,
                    } => host.click(&bus_name, &path, click, at).await,
                    TrayRequest::Scroll { bus_name, path, up } => {
                        host.scroll(&bus_name, &path, up).await
                    }
                    TrayRequest::MenuClicked { bus_name, menu, id } => {
                        host.menu_clicked(&bus_name, &menu, id).await
                    }
                    TrayRequest::Menu { path, at, mut read } => {
                        match host.menu(&read.bus_name, &read.menu).await {
                            Ok(root) if root.shown().next().is_some() => {
                                read.root = root;
                                if !deliver(&sender, waker.as_ref(), Update::TrayMenu(read)) {
                                    break;
                                }
                                Ok(())
                            }
                            // No menu after all (Steam names one it does not
                            // serve), or an empty one: the item's own.
                            outcome => {
                                if let Err(error) = outcome {
                                    tracing::info!(
                                        "tray: {} has no menu at {}: {error}",
                                        read.bus_name,
                                        read.menu
                                    );
                                }
                                host.click(&read.bus_name, &path, Click::Menu, at).await
                            }
                        }
                    }
                };
                if let Err(error) = done {
                    tracing::warn!("tray: {error}");
                }
            }
        };
        let ((), (), (), watched) = tokio::join!(follow, forward, act, host.watch(&listings));
        if let Err(error) = watched {
            tracing::warn!("tray: the watcher stopped answering: {error}");
        }
    }

    /// Sends lyr's own notices to whichever notification daemon is running.
    async fn notices(mut requests: UnboundedReceiver<(String, String)>) {
        while let Some((summary, body)) = requests.recv().await {
            let sent = lyr_sources::notifications::linux::notify_session(&summary, &body).await;
            if let Err(error) = sent {
                tracing::warn!("notice `{summary}` not sent: {error}");
            }
        }
    }

    /// Carries out `request` and says how it ended: a failed profile is
    /// kept, deleted or offered a new password as
    /// [`lyr_core::wifi_menu::recovery`] says.
    async fn join(network: &NetworkManager, request: JoinRequest) -> Joined {
        use lyr_core::wifi_menu::{Recovery, failure_text, recovery};
        use lyr_sources::network::Join;

        let JoinRequest {
            device,
            join,
            ssid,
            security,
            new_password,
        } = request;
        if let (Some(password), Join::Saved { profile, .. }) = (&new_password, &join)
            && let Err(error) = network.replace_password(profile, password).await
        {
            return Joined::Failed(format!("Saving the password for {ssid} failed: {error}"));
        }
        let saved = matches!(join, Join::Saved { .. });
        match network.join(&device, &join).await {
            Ok(Ok(())) => Joined::Up,
            Ok(Err(failed)) => {
                let recovery = recovery(saved, security, failed.reason);
                if recovery == Recovery::Delete
                    && let Some(profile) = failed.profile
                    && let Err(error) = network.forget(&[profile]).await
                {
                    tracing::warn!("wi-fi: deleting the failed profile for {ssid}: {error}");
                }
                if recovery == Recovery::NewPassword {
                    Joined::NewPassword
                } else {
                    Joined::Failed(failure_text(&ssid, failed.reason))
                }
            }
            Err(error) => Joined::Failed(format!("Joining {ssid} failed: {error}")),
        }
    }

    async fn bluetooth(
        sender: mpsc::Sender<Update>,
        mut requests: UnboundedReceiver<BluetoothRequest>,
        waker: Option<ShellWaker>,
    ) {
        let bluez = match BlueZ::system().await {
            Ok(bluez) => bluez,
            Err(error) => {
                tracing::warn!("bluetooth: BlueZ is not reachable: {error}");
                return;
            }
        };
        let (snapshots, mut changed) = watch::channel(Snapshot::default());
        let forward = async {
            while changed.changed().await.is_ok() {
                let snapshot = changed.borrow_and_update().clone();
                if !deliver(&sender, waker.as_ref(), Update::Bluetooth(snapshot)) {
                    break;
                }
            }
        };
        let questions = std::cell::RefCell::new(Questions::default());
        let (prompts, mut asked) = tokio::sync::mpsc::unbounded_channel();
        if let Err(error) = bluez.register_agent(prompts).await {
            tracing::warn!("bluetooth: BlueZ refused lyr's pairing agent: {error}");
        }
        let ask = async {
            while let Some(asked) = asked.recv().await {
                let update = match asked {
                    Asked::Prompt(prompt) => {
                        let id = questions.borrow_mut().ask(prompt.answer);
                        Update::BluetoothAsk(id, prompt.device, prompt.question)
                    }
                    Asked::Cancel => {
                        questions.borrow_mut().cancel();
                        Update::BluetoothCancel
                    }
                };
                if !deliver(&sender, waker.as_ref(), update) {
                    break;
                }
            }
        };
        let act = async {
            while let Some(request) = requests.recv().await {
                if let BluetoothRequest::Answer(id, reply) = request {
                    if !questions.borrow_mut().answer(id, reply) {
                        tracing::debug!(
                            "bluetooth: question {id} was answered after BlueZ gave up"
                        );
                    }
                    continue;
                }
                // What it changed comes back as a change; a refusal is logged.
                let done = match &request {
                    BluetoothRequest::Switch(adapter, switch, on) => {
                        bluez.set(adapter, *switch, *on).await
                    }
                    BluetoothRequest::Scan(adapter, on) => bluez.scan(adapter, *on).await,
                    BluetoothRequest::Device(device, action) => bluez.device(device, *action).await,
                    BluetoothRequest::Forget(adapter, device) => {
                        bluez.forget(adapter, device).await
                    }
                    BluetoothRequest::Rename(device, name) => bluez.rename(device, name).await,
                    BluetoothRequest::Answer(..) => Ok(()),
                };
                if let Err(error) = done {
                    tracing::warn!("bluetooth: {request:?}: {error}");
                }
            }
        };
        let (watched, (), (), ()) = tokio::join!(bluez.watch(&snapshots), forward, ask, act);
        if let Err(error) = watched {
            tracing::warn!("bluetooth: BlueZ stopped answering: {error}");
        }
    }

    async fn clock(sender: mpsc::Sender<Update>, waker: Option<ShellWaker>) {
        let timedate = match TimeDate::system().await {
            Ok(timedate) => timedate,
            Err(error) => {
                tracing::warn!("clock: timedate1 is not reachable: {error}");
                return;
            }
        };
        let (zones, mut changed) = watch::channel(None);
        let forward = async {
            while changed.changed().await.is_ok() {
                let zone = changed.borrow_and_update().clone();
                if !deliver(&sender, waker.as_ref(), Update::Zone(zone)) {
                    break;
                }
            }
        };
        let (watched, ()) = tokio::join!(timedate.watch(&zones), forward);
        if let Err(error) = watched {
            tracing::warn!("clock: timedate1 stopped answering: {error}");
        }
    }

    /// The media players, and what the clock panel asks of them.
    async fn players(
        sender: mpsc::Sender<Update>,
        mut requests: UnboundedReceiver<(String, Control)>,
        waker: Option<ShellWaker>,
    ) {
        let players = match Players::session().await {
            Ok(players) => players,
            Err(error) => {
                tracing::warn!("media: the session bus is not reachable: {error}");
                return;
            }
        };
        let (lists, mut changed) = watch::channel(Vec::new());
        let forward = async {
            while changed.changed().await.is_ok() {
                let list = changed.borrow_and_update().clone();
                if !deliver(&sender, waker.as_ref(), Update::Players(list)) {
                    break;
                }
            }
        };
        let press = async {
            while let Some((bus_name, control)) = requests.recv().await {
                // The player's new state comes back as a change.
                if let Err(error) = players.control(&bus_name, control).await {
                    tracing::warn!("media: {bus_name}: {error}");
                }
            }
        };
        let (watched, (), ()) = tokio::join!(players.watch(&lists), forward, press);
        if let Err(error) = watched {
            tracing::warn!("media: the session bus stopped answering: {error}");
        }
    }

    async fn power_profile(
        sender: mpsc::Sender<Update>,
        mut requests: UnboundedReceiver<String>,
        waker: Option<ShellWaker>,
    ) {
        let daemon = match PowerProfiles::system().await {
            Ok(daemon) => daemon,
            Err(error) => {
                tracing::warn!("power profile: power-profiles-daemon is not reachable: {error}");
                return;
            }
        };
        let (states, mut changed) = watch::channel(None);
        let forward = async {
            while changed.changed().await.is_ok() {
                let state = changed.borrow_and_update().clone();
                if !deliver(&sender, waker.as_ref(), Update::PowerProfile(state)) {
                    break;
                }
            }
        };
        let switch = async {
            while let Some(name) = requests.recv().await {
                // The new profile comes back as a change; a refusal changes
                // nothing on screen and is logged.
                if let Err(error) = daemon.set(&name).await {
                    tracing::warn!("power profile: switching to {name}: {error}");
                }
            }
        };
        let (watched, (), ()) = tokio::join!(daemon.watch(&states), forward, switch);
        if let Err(error) = watched {
            tracing::warn!("power profile: the daemon stopped answering: {error}");
        }
    }

    async fn workspaces(
        sender: mpsc::Sender<Update>,
        mut focus: UnboundedReceiver<Workspace>,
        waker: Option<ShellWaker>,
    ) {
        let run = async {
            let hyprland = Hyprland::from_env().await?;
            let (mut state, mut events) = hyprland.subscribe().await?;
            if !deliver(&sender, waker.as_ref(), Update::Workspaces(state.clone())) {
                return Ok(());
            }
            loop {
                tokio::select! {
                    event = events.next() => {
                        let Some(event) = event? else { break };
                        if state.apply(event)
                            && !deliver(&sender, waker.as_ref(), Update::Workspaces(state.clone()))
                        {
                            break;
                        }
                    }
                    Some(workspace) = focus.recv() => {
                        // A refused switch changes nothing on screen, which
                        // is the truth; it is only logged.
                        if let Err(error) = hyprland.focus(&workspace).await {
                            tracing::warn!("workspaces: {error}");
                        }
                    }
                }
            }
            Ok::<_, lyr_wm::hyprland::Error>(())
        };
        if let Err(error) = run.await {
            tracing::warn!("workspaces: {error}");
        }
    }

    /// Carries out the power menu's entries as they come. A refusal is only
    /// logged: nothing on screen claims it happened.
    async fn power_menu(mut requests: UnboundedReceiver<Entry>) {
        while let Some(entry) = requests.recv().await {
            if let Err(error) = power(entry).await {
                tracing::warn!("power menu: {}: {error:#}", entry.label());
            }
        }
    }

    async fn power(entry: Entry) -> anyhow::Result<()> {
        match entry {
            Entry::Lock => Logind::system().await?.lock().await?,
            Entry::Sleep => Logind::system().await?.sleep().await?,
            Entry::Hibernate => Logind::system().await?.hibernate().await?,
            Entry::Shutdown => Logind::system().await?.power_off().await?,
            Entry::Reboot => Logind::system().await?.reboot().await?,
            Entry::Logout => Hyprland::from_env().await?.exit().await?,
        }
        Ok(())
    }

    /// Serves the notification daemon on the session bus, unless another
    /// daemon has the name, and carries out what the bar asks of it.
    async fn notifications(
        sender: mpsc::Sender<Update>,
        mut requests: UnboundedReceiver<NotificationRequest>,
        replace: bool,
        waker: Option<ShellWaker>,
    ) {
        let store = Arc::new(Mutex::new(Store::default()));
        let (changes, mut changed) = tokio::sync::mpsc::unbounded_channel();
        let daemon = Daemon {
            store: store.clone(),
            changes,
        };
        let served = match serve_session(daemon, replace).await {
            Ok(served) => served,
            Err(error) => {
                tracing::info!("notifications: {error}");
                return;
            }
        };
        tracing::info!("notifications: lyr is the notification daemon");
        if !deliver(&sender, waker.as_ref(), Update::Notifications(store)) {
            return;
        }
        let forward = async {
            while let Some(change) = changed.recv().await {
                if !deliver(&sender, waker.as_ref(), Update::Notification(change)) {
                    break;
                }
            }
        };
        let act = async {
            while let Some(request) = requests.recv().await {
                let done = match request {
                    NotificationRequest::Close(id, reason) => {
                        served.close(id, reason).await.map(|_held| ())
                    }
                    NotificationRequest::Invoke(id, key) => served.invoke(id, &key).await,
                    NotificationRequest::Clear => served.clear().await.map(|_cleared| ()),
                };
                if let Err(error) = done {
                    tracing::warn!("notifications: {error}");
                }
            }
        };
        tokio::join!(forward, act);
    }
}
