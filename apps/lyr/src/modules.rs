//! What each module shows, from the sources' latest data, and drawing a bar
//! from them.

#[cfg(target_os = "linux")]
use std::sync::PoisonError;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, FixedOffset};
use lyr_config::{Config, Item, Module};
use lyr_core::battery::{self, Level, Reading};
use lyr_core::clock;
use lyr_core::draw::{self, Bar, Content, DrawList, Target};
use lyr_core::hit::Regions;
use lyr_render::text::Text;

use crate::sources::Update;

/// The sources' latest states.
#[derive(Default)]
pub struct Data {
    pub battery: Option<Reading>,
    #[cfg(any(target_os = "linux", windows))]
    pub power_profile: Option<lyr_sources::power_profile::Profiles>,
    #[cfg(any(target_os = "linux", windows))]
    pub network: Option<lyr_core::network::Reading>,
    /// The Wi-Fi device, its networks and profiles, for the Wi-Fi menu.
    /// `None` where there is no source for them.
    pub wifi: Option<lyr_core::wifi_menu::Wifi>,
    #[cfg(target_os = "linux")]
    pub bluetooth: Option<lyr_core::bluetooth::Reading>,
    /// The adapter and devices, for the Bluetooth menu. Empty where there
    /// is no source for them.
    pub bluetooth_adapter: Option<lyr_core::bluetooth_menu::Adapter>,
    pub bluetooth_devices: Vec<lyr_core::bluetooth_menu::Device>,
    /// The pairing agent's question waiting for an answer: its id, the
    /// device's path, and the question.
    pub bluetooth_ask: Option<(u64, String, lyr_core::bluetooth_menu::Question)>,
    #[cfg(target_os = "linux")]
    pub workspaces: Option<lyr_wm::hyprland::State>,
    /// The system time zone's name, from timedate1.
    #[cfg(target_os = "linux")]
    pub zone: Option<String>,
    /// The notification store, while lyr is the notification daemon.
    #[cfg(target_os = "linux")]
    pub notifications: Option<Arc<Mutex<lyr_core::notifications::Store>>>,
    /// The media players, for the clock panel. Empty where there is no
    /// source for them.
    pub players: Vec<lyr_core::media::Player>,
    /// The tray's items, in the watcher's order. Empty where there is no
    /// tray to host.
    pub tray: Vec<lyr_core::tray::Item>,
    /// The default output and input, where there is a source for them.
    pub sink: Option<lyr_core::audio::Sink>,
    pub source: Option<lyr_core::audio::Source>,
    /// What the audio menu lists, where there is a source for it.
    pub audio_nodes: Option<lyr_core::audio_menu::Nodes>,
    /// The foreground window, on Windows; Linux reads it from Hyprland.
    #[cfg(windows)]
    pub active_window: Option<lyr_sources::window::Active>,
}

impl Data {
    /// The tray's items the bar shows, in order: a tray button's index is
    /// its item's place here.
    pub fn tray_shown(&self) -> impl Iterator<Item = &lyr_core::tray::Item> {
        self.tray.iter().filter(|item| item.shown())
    }

    /// The pairing agent's question, about its device by name where BlueZ
    /// gives one.
    pub fn bluetooth_question(&self) -> Option<lyr_core::bluetooth_menu::Ask<'_>> {
        let (_, path, question) = self.bluetooth_ask.as_ref()?;
        let device = self
            .bluetooth_devices
            .iter()
            .find(|device| device.path == *path)
            .map_or(path.as_str(), |device| device.alias.as_str());
        Some(lyr_core::bluetooth_menu::Ask {
            device,
            question: *question,
            typed: "",
        })
    }

    /// The notification store, while lyr is the notification daemon.
    pub const fn notification_store(&self) -> Option<&Arc<Mutex<lyr_core::notifications::Store>>> {
        #[cfg(target_os = "linux")]
        return self.notifications.as_ref();
        #[cfg(not(target_os = "linux"))]
        None
    }

    pub fn apply(&mut self, update: Update) {
        match update {
            #[cfg(any(target_os = "linux", windows))]
            Update::Battery(reading) => self.battery = reading,
            #[cfg(any(target_os = "linux", windows))]
            Update::PowerProfile(profiles) => self.power_profile = profiles,
            #[cfg(any(target_os = "linux", windows))]
            Update::Network(reading) => self.network = reading,
            #[cfg(target_os = "linux")]
            Update::Wifi(wifi) => self.wifi = Some(wifi),
            // The Wi-Fi menu takes it.
            #[cfg(target_os = "linux")]
            Update::WifiJoined(_) => {}
            #[cfg(target_os = "linux")]
            Update::BluetoothAsk(id, device, question) => {
                self.bluetooth_ask = Some((id, device, question));
            }
            #[cfg(target_os = "linux")]
            Update::BluetoothCancel => self.bluetooth_ask = None,
            #[cfg(target_os = "linux")]
            Update::Bluetooth(snapshot) => {
                self.bluetooth = snapshot.reading;
                self.bluetooth_adapter = snapshot.adapter;
                self.bluetooth_devices = snapshot.devices;
            }
            #[cfg(target_os = "linux")]
            Update::Workspaces(state) => self.workspaces = Some(state),
            #[cfg(target_os = "linux")]
            Update::Zone(zone) => self.zone = zone,
            #[cfg(target_os = "linux")]
            Update::Notifications(store) => self.notifications = Some(store),
            #[cfg(target_os = "linux")]
            Update::Players(players) => self.players = players,
            #[cfg(target_os = "linux")]
            Update::Tray(items) => self.tray = items,
            #[cfg(windows)]
            Update::Audio(sink, source) => {
                self.sink = sink;
                self.source = source;
            }
            #[cfg(windows)]
            Update::AudioNodes(nodes) => self.audio_nodes = Some(nodes),
            #[cfg(windows)]
            Update::ActiveWindow(active) => self.active_window = Some(active),
            // The bar opens it.
            #[cfg(target_os = "linux")]
            Update::TrayMenu(_) => {}
            // The toasts follow each change; the bell reads the store.
            #[cfg(target_os = "linux")]
            Update::Notification(_) => {}
        }
    }
}

/// A bar as drawn: its draw list, its input regions, and the workspaces its
/// buttons stand for, in order, so a click finds the one it was drawn for.
pub struct Drawn {
    pub list: DrawList,
    pub regions: Regions<Target<Module>>,
    pub workspaces: Vec<i64>,
}

/// Lays out one bar `width` logical pixels wide, on the output named
/// `monitor`, with the pointer over workspace button `hovered`.
pub fn draw(
    width: f32,
    monitor: Option<&str>,
    hovered: Option<usize>,
    config: &Config,
    text: &Text,
    data: &Data,
    now: &DateTime<FixedOffset>,
) -> Drawn {
    let modules = |items: &[Item]| -> Vec<draw::Module<Module>> {
        items
            .iter()
            .filter_map(|item| match item {
                Item::Named(name) => Module::from_name(name),
                Item::Spacer { .. } => None,
            })
            .filter_map(|module| {
                content(module, config, data, now, monitor, hovered).map(|content| draw::Module {
                    content,
                    target: module,
                })
            })
            .collect()
    };
    let (left, center, right) = (
        modules(&config.layout.left),
        modules(&config.layout.center),
        modules(&config.layout.right),
    );
    let bar = Bar {
        width,
        height: config.bar.height as f32,
        left: &left,
        center: &center,
        right: &right,
    };
    let (list, regions) = draw::build(&bar, &config.theme, |s| text.measure(s));
    // The same order the workspace buttons were made in.
    #[cfg(target_os = "linux")]
    let workspaces = data
        .workspaces
        .as_ref()
        .map(|state| {
            shown_workspaces(state, config, monitor)
                .iter()
                .map(|workspace| workspace.id)
                .collect()
        })
        .unwrap_or_default();
    #[cfg(not(target_os = "linux"))]
    let workspaces = Vec::new();
    Drawn {
        list,
        regions,
        workspaces,
    }
}

/// What `module` shows now, or `None` when it has no data to show.
fn content(
    module: Module,
    config: &Config,
    data: &Data,
    now: &DateTime<FixedOffset>,
    #[cfg_attr(not(target_os = "linux"), allow(unused_variables))] monitor: Option<&str>,
    #[cfg_attr(not(target_os = "linux"), allow(unused_variables))] hovered: Option<usize>,
) -> Option<Content> {
    let theme = &config.theme;
    let label = |text: String, colour| Content::Label {
        text,
        compact: None,
        colour,
    };
    match module {
        Module::Power => Some(label(
            config.power.format.render(|_| None),
            theme.foreground,
        )),
        Module::Clock => Some(label(
            clock::view(now, None, &config.clock.format).text,
            theme.foreground,
        )),
        Module::Battery => {
            let shown = battery::view(data.battery.as_ref()?, &config.battery);
            let colour = match shown.level {
                Some(Level::Critical) => theme.battery_critical,
                Some(Level::Warning) => theme.battery_warning,
                None => theme.foreground,
            };
            Some(label(shown.text, colour))
        }
        #[cfg(any(target_os = "linux", windows))]
        Module::PowerProfile => {
            let profiles = data.power_profile.as_ref()?;
            let shown = lyr_core::power_profile::view(&profiles.active, &config.power_profile);
            Some(label(shown.text, theme.foreground))
        }
        #[cfg(any(target_os = "linux", windows))]
        Module::Network => {
            let shown = data.network.as_ref()?.view(&config.network);
            Some(label(shown.text, theme.foreground))
        }
        #[cfg(target_os = "linux")]
        Module::Bluetooth => {
            let shown = lyr_core::bluetooth::view(data.bluetooth.as_ref()?, &config.bluetooth);
            Some(label(shown.text, theme.foreground))
        }
        #[cfg(target_os = "linux")]
        Module::Window => {
            let (class, title) = data.workspaces.as_ref()?.active_window()?;
            let shown = lyr_core::window::view(class, title, &config.window)?;
            Some(label(shown.text, theme.foreground))
        }
        #[cfg(windows)]
        Module::Window => {
            let active = data.active_window.as_ref()?;
            let shown = lyr_core::window::view(&active.class, &active.title, &config.window)?;
            Some(label(shown.text, theme.foreground))
        }
        Module::Volume => {
            let shown = lyr_core::audio::volume(data.sink.as_ref()?, &config.volume);
            Some(label(shown.text, theme.volume))
        }
        Module::Mic => {
            let shown = lyr_core::audio::mic(data.source.as_ref()?, &config.mic);
            Some(label(shown.text, theme.mic))
        }
        Module::Tray => {
            // A letter stands in for each icon until lyr draws images.
            let buttons: Vec<draw::Button> = data
                .tray_shown()
                .map(|item| draw::Button {
                    text: item.letter(),
                    colour: if item.status == lyr_core::tray::Status::NeedsAttention {
                        theme.workspace_urgent.unwrap_or(theme.foreground)
                    } else {
                        theme.foreground
                    },
                })
                .collect();
            (!buttons.is_empty()).then_some(Content::Buttons(buttons))
        }
        #[cfg(target_os = "linux")]
        Module::Notifications => {
            let store = data.notifications.as_ref()?;
            let store = store.lock().unwrap_or_else(PoisonError::into_inner);
            Some(label(
                lyr_core::bell::view(&store, &config.notifications),
                theme.foreground,
            ))
        }
        // Windows keeps its notifications to itself: a bell that opens them.
        #[cfg(windows)]
        Module::Notifications => Some(label(
            lyr_core::bell::stateless(&config.notifications),
            theme.foreground,
        )),
        #[cfg(target_os = "linux")]
        Module::Workspaces => {
            let state = data.workspaces.as_ref()?;
            let buttons = shown_workspaces(state, config, monitor)
                .into_iter()
                .enumerate()
                .map(|(index, workspace)| draw::Button {
                    text: config
                        .workspaces
                        .format
                        .render(|name| (name == "name").then_some(workspace.name.as_str())),
                    // Active or hovered, as waybar's style colours both.
                    colour: if state.active() == Some(workspace.id) || hovered == Some(index) {
                        theme.workspace_active
                    } else if workspace.urgent {
                        theme.workspace_urgent.unwrap_or(theme.foreground)
                    } else {
                        theme.foreground
                    },
                })
                .collect();
            Some(Content::Buttons(buttons))
        }
        // Every module has an arm on Linux; elsewhere some are not built yet.
        #[cfg(not(target_os = "linux"))]
        _ => None,
    }
}

/// The tooltip `target` shows now, or `None` when it has none or no data:
/// a module's, or a tray item's. The workspaces have none, as in waybar.
pub fn tooltip(
    target: &Target<Module>,
    config: &Config,
    data: &Data,
    now: &DateTime<FixedOffset>,
) -> Option<String> {
    let module = match target {
        Target::Module(module) => *module,
        Target::Button(Module::Tray, index) => {
            return data
                .tray_shown()
                .nth(*index)?
                .tooltip_text()
                .map(str::to_owned);
        }
        Target::Button(..) => return None,
    };
    match module {
        Module::Power => Some(config.power.tooltip_format.render(|_| None)),
        Module::Clock => {
            Some(clock::view(now, zone(data).as_deref(), &config.clock.format).tooltip)
        }
        Module::Battery => Some(battery::view(data.battery.as_ref()?, &config.battery).tooltip),
        #[cfg(any(target_os = "linux", windows))]
        Module::PowerProfile => Some(
            lyr_core::power_profile::view(
                &data.power_profile.as_ref()?.active,
                &config.power_profile,
            )
            .tooltip,
        ),
        #[cfg(any(target_os = "linux", windows))]
        Module::Network => Some(data.network.as_ref()?.view(&config.network).tooltip),
        #[cfg(target_os = "linux")]
        Module::Window => {
            let (class, title) = data.workspaces.as_ref()?.active_window()?;
            Some(lyr_core::window::view(class, title, &config.window)?.tooltip)
        }
        #[cfg(windows)]
        Module::Window => {
            let active = data.active_window.as_ref()?;
            Some(lyr_core::window::view(&active.class, &active.title, &config.window)?.tooltip)
        }
        Module::Volume => {
            Some(lyr_core::audio::volume(data.sink.as_ref()?, &config.volume).tooltip)
        }
        Module::Mic => Some(lyr_core::audio::mic(data.source.as_ref()?, &config.mic).tooltip),
        #[cfg(target_os = "linux")]
        Module::Bluetooth => {
            Some(lyr_core::bluetooth::view(data.bluetooth.as_ref()?, &config.bluetooth).tooltip)
        }
        _ => None,
    }
}

/// The time zone lyr's clock is in, by name, such as `Asia/Manila`: `TZ`
/// when it is set, since the clock follows it; else the system's, from
/// timedate1 when it answered, or where `/etc/localtime` links into the
/// zoneinfo database, which is what timedated itself reads.
#[cfg(target_os = "linux")]
fn zone(data: &Data) -> Option<String> {
    if let Ok(tz) = std::env::var("TZ") {
        return zone_name(&tz);
    }
    if let Some(zone) = &data.zone {
        return Some(zone.clone());
    }
    zone_name(std::fs::read_link("/etc/localtime").ok()?.to_str()?)
}

/// The zone named by a `TZ` value or a zoneinfo path: a leading `:` (POSIX
/// allows one before a zone file) and the path up to `zoneinfo/` dropped.
/// `None` for an empty one.
#[cfg(any(target_os = "linux", test))]
fn zone_name(tz: &str) -> Option<String> {
    let tz = tz.strip_prefix(':').unwrap_or(tz);
    let name = tz.split_once("zoneinfo/").map_or(tz, |(_, name)| name);
    (!name.is_empty()).then(|| name.to_owned())
}

/// Windows and macOS: the zone's name is not read yet, so the tooltip shows
/// the offset alone.
#[cfg(not(target_os = "linux"))]
fn zone(_data: &Data) -> Option<String> {
    None
}

/// The workspaces a bar on `monitor` lists: every one, or with
/// `all_outputs = false` its own output's.
#[cfg(target_os = "linux")]
pub fn shown_workspaces<'a>(
    state: &'a lyr_wm::hyprland::State,
    config: &Config,
    monitor: Option<&str>,
) -> Vec<&'a lyr_wm::hyprland::Workspace> {
    state
        .workspaces()
        .filter(|workspace| {
            config.workspaces.all_outputs
                || monitor.is_some_and(|monitor| workspace.monitor.as_deref() == Some(monitor))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use chrono::DateTime;
    use lyr_config::{Module, Platform, from_sources};
    use lyr_core::draw::{Content, Target};
    use lyr_core::tray::{Item, Status};

    use super::{Data, content, tooltip, zone_name};

    #[test]
    fn the_power_button_is_its_glyph_with_its_tooltip() {
        let config = from_sources(None, Platform::current()).expect("the defaults");
        let now = DateTime::parse_from_rfc3339("2026-10-19T09:05:00+08:00").expect("a time");
        let data = Data::default();
        let glyph = char::from_u32(0xF011).expect("a char").to_string();
        assert_eq!(
            content(Module::Power, &config, &data, &now, None, None),
            Some(Content::Label {
                text: glyph,
                compact: None,
                colour: config.theme.foreground,
            })
        );
        assert_eq!(
            tooltip(&Target::Module(Module::Power), &config, &data, &now).as_deref(),
            Some("Power Button")
        );
    }

    #[test]
    fn a_tray_button_s_tooltip_is_its_shown_item_s() {
        let config = from_sources(None, Platform::current()).expect("the defaults");
        let now = DateTime::parse_from_rfc3339("2026-10-19T09:05:00+08:00").expect("a time");
        let item = |title: &str, status| Item {
            title: title.to_owned(),
            status,
            ..Item::default()
        };
        let data = Data {
            tray: vec![
                item("Hidden", Status::Passive),
                item("Steam", Status::Active),
            ],
            ..Data::default()
        };
        let tip = |index| tooltip(&Target::Button(Module::Tray, index), &config, &data, &now);
        assert_eq!(
            tip(0).as_deref(),
            Some("Steam"),
            "the passive one is not shown"
        );
        assert_eq!(tip(1), None);
        assert_eq!(
            tooltip(&Target::Button(Module::Workspaces, 0), &config, &data, &now),
            None
        );
    }

    #[test]
    fn a_zone_is_named_from_tz_or_a_zoneinfo_path() {
        assert_eq!(zone_name("Asia/Manila").as_deref(), Some("Asia/Manila"));
        assert_eq!(zone_name(":Asia/Manila").as_deref(), Some("Asia/Manila"));
        assert_eq!(
            zone_name("/usr/share/zoneinfo/America/St_Johns").as_deref(),
            Some("America/St_Johns")
        );
        assert_eq!(
            zone_name(":/usr/share/zoneinfo/UTC").as_deref(),
            Some("UTC")
        );
        assert_eq!(zone_name(""), None);
        assert_eq!(zone_name(":"), None);
    }
}

#[cfg(test)]
mod bluetooth_tests {
    use lyr_core::bluetooth_menu::{Device, Question};

    use super::Data;

    #[test]
    fn a_question_names_its_device_by_alias_or_else_its_path() {
        let mut data = Data::default();
        assert_eq!(data.bluetooth_question(), None);
        let path = "/org/bluez/hci0/dev_1".to_owned();
        data.bluetooth_ask = Some((7, path.clone(), Question::Confirm(1)));
        assert_eq!(
            data.bluetooth_question().map(|ask| ask.device),
            Some(path.as_str())
        );
        data.bluetooth_devices.push(Device {
            path: path.clone(),
            alias: "Buds".to_owned(),
            connected: false,
            paired: false,
            trusted: false,
            battery: None,
        });
        let ask = data.bluetooth_question().expect("a question");
        assert_eq!((ask.device, ask.question), ("Buds", Question::Confirm(1)));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_question_bluez_gave_up_on_goes() {
        use crate::sources::Update;
        let mut data = Data::default();
        let path = "/org/bluez/hci0/dev_1".to_owned();
        data.apply(Update::BluetoothAsk(7, path, Question::Pin));
        assert!(data.bluetooth_question().is_some());
        data.apply(Update::BluetoothCancel);
        assert_eq!(data.bluetooth_question(), None);
    }
}
