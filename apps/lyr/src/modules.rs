//! What each module shows, from the sources' latest data, and drawing a bar
//! from them.

use std::collections::HashMap;
#[cfg(target_os = "linux")]
use std::sync::PoisonError;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, FixedOffset};
use lyr_config::{Config, Item, Module};
use lyr_core::battery::{self, Level, Reading};
use lyr_core::clock;
use lyr_core::draw::{self, Bar, Content, DrawList, Entry, Orientation, Size, Target};
use lyr_core::hit::Regions;
use lyr_render::text::Text;

use crate::sources::Update;

/// The sources' latest states.
#[derive(Default)]
pub struct Data {
    pub battery: Option<Reading>,
    pub power_profile: Option<lyr_sources::power_profile::Profiles>,
    pub network: Option<lyr_core::network::Reading>,
    /// The Wi-Fi device, its networks and profiles, for the Wi-Fi menu.
    /// `None` where there is no source for them.
    pub wifi: Option<lyr_core::wifi_menu::Wifi>,
    pub bluetooth: Option<lyr_core::bluetooth::Reading>,
    /// The adapter and devices, for the Bluetooth menu. Empty where there
    /// is no source for them.
    pub bluetooth_adapter: Option<lyr_core::bluetooth_menu::Adapter>,
    pub bluetooth_devices: Vec<lyr_core::bluetooth_menu::Device>,
    /// The pairing agent's question waiting for an answer: its id, the
    /// device's path, and the question.
    pub bluetooth_ask: Option<(u64, String, lyr_core::bluetooth_menu::Question)>,
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    pub workspaces: Option<lyr_wm::State>,
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
    /// The foreground window, on Windows; Linux reads it from the compositor.
    #[cfg(windows)]
    pub active_window: Option<lyr_sources::window::Active>,
    /// Each custom module's last output, by its index in the config's
    /// custom tables and its bar's output: `None` leaves it out.
    pub custom: HashMap<(usize, Option<String>), Option<lyr_core::custom::Output>>,
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

    /// What the custom module `index` shows on the bar on `monitor`: a
    /// static one shows its format alone; one with a script, its script's
    /// last output, and nothing before the first.
    pub fn custom_view(
        &self,
        index: usize,
        config: &Config,
        monitor: Option<&str>,
    ) -> Option<lyr_core::custom::View> {
        let custom = config.custom.get(index)?;
        if custom.exec.is_none() {
            return lyr_core::custom::view(&lyr_core::custom::Output::default(), custom);
        }
        let output = self
            .custom
            .get(&(index, monitor.map(str::to_owned)))?
            .as_ref()?;
        lyr_core::custom::view(output, custom)
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
            Update::Battery(reading) => self.battery = reading,
            Update::PowerProfile(profiles) => self.power_profile = profiles,
            Update::Network(reading) => self.network = reading,
            #[cfg(any(target_os = "linux", windows))]
            Update::Wifi(wifi) => self.wifi = Some(wifi),
            // The Wi-Fi menu takes it.
            #[cfg(any(target_os = "linux", windows))]
            Update::WifiJoined(_) => {}
            #[cfg(any(target_os = "linux", windows))]
            Update::BluetoothAsk(id, device, question) => {
                self.bluetooth_ask = Some((id, device, question));
            }
            #[cfg(any(target_os = "linux", windows))]
            Update::BluetoothCancel => self.bluetooth_ask = None,
            Update::Bluetooth(snapshot) => {
                self.bluetooth = snapshot.reading;
                // macOS shows the module only.
                #[cfg(any(target_os = "linux", windows))]
                {
                    self.bluetooth_adapter = snapshot.adapter;
                    self.bluetooth_devices = snapshot.devices;
                }
            }
            #[cfg(any(target_os = "linux", target_os = "macos"))]
            Update::Workspaces(state) => self.workspaces = Some(state),
            #[cfg(target_os = "linux")]
            Update::Zone(zone) => self.zone = zone,
            #[cfg(target_os = "linux")]
            Update::Notifications(store) => self.notifications = Some(store),
            #[cfg(target_os = "linux")]
            Update::Players(players) => self.players = players,
            #[cfg(target_os = "linux")]
            Update::Tray(items) => self.tray = items,
            Update::Audio(sink, source) => {
                self.sink = sink;
                self.source = source;
            }
            Update::AudioNodes(nodes) => self.audio_nodes = Some(nodes),
            #[cfg(windows)]
            Update::ActiveWindow(active) => self.active_window = Some(active),
            // The bar opens it, and changes it in place.
            #[cfg(target_os = "linux")]
            Update::TrayMenu(_) | Update::TrayMenuLaidOut(..) => {}
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

/// A zone's `items` as the bar draws them, leaving out each module that has
/// nothing to show.
fn entries(
    items: &[Item],
    config: &Config,
    data: &Data,
    now: &DateTime<FixedOffset>,
    monitor: Option<&str>,
    hovered: Option<usize>,
) -> Vec<Entry<Module>> {
    items
        .iter()
        .filter_map(|item| match item {
            Item::Named(name) if name == "separator" => Some(Entry::Separator),
            Item::Named(name) => {
                let module = config.module(name)?;
                content(module, config, data, now, monitor, hovered).map(|content| {
                    Entry::Module(draw::Module {
                        content,
                        target: module,
                    })
                })
            }
            &Item::Spacer { spacer } => Some(Entry::Spacer(spacer as f32)),
        })
        .collect()
}

/// Lays out one bar in a window `window` big, on the output named
/// `monitor`, with the pointer over workspace button `hovered`. The window
/// gives the bar's length; the config, its thickness.
pub fn draw(
    window: Size,
    monitor: Option<&str>,
    hovered: Option<usize>,
    config: &Config,
    text: &Text,
    data: &Data,
    now: &DateTime<FixedOffset>,
) -> Drawn {
    let modules = |items: &[Item]| entries(items, config, data, now, monitor, hovered);
    let (left, center, right) = (
        modules(&config.layout.left),
        modules(&config.layout.center),
        modules(&config.layout.right),
    );
    let thickness = config.bar.thickness() as f32;
    let (width, height, orientation) = if config.bar.position.is_vertical() {
        let line_height = text.line_height();
        (
            thickness,
            window.height,
            Orientation::Vertical { line_height },
        )
    } else {
        (window.width, thickness, Orientation::Horizontal)
    };
    let bar = Bar {
        width,
        height,
        orientation,
        left: &left,
        center: &center,
        right: &right,
    };
    let (list, regions) = draw::build(&bar, &config.theme, |s| text.measure(s));
    // The same order the workspace buttons were made in.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
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
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
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
    monitor: Option<&str>,
    #[cfg_attr(not(target_os = "linux"), allow(unused_variables))] hovered: Option<usize>,
) -> Option<Content> {
    let theme = &config.theme;
    let label = |text: String, colour| Content::Label {
        text,
        compact: None,
        colour,
    };
    // A label that falls back to its icon alone where its text does not fit.
    let with_icon = |text: String, compact: Option<String>, colour| Content::Label {
        text,
        compact,
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
            Some(with_icon(shown.text, shown.compact, colour))
        }
        Module::PowerProfile => {
            let profiles = data.power_profile.as_ref()?;
            let shown = lyr_core::power_profile::view(&profiles.active, &config.power_profile);
            Some(with_icon(shown.text, shown.compact, theme.foreground))
        }
        Module::Network => {
            let shown = data.network.as_ref()?.view(&config.network);
            Some(with_icon(shown.text, shown.compact, theme.foreground))
        }
        Module::Bluetooth => {
            let shown = lyr_core::bluetooth::view(data.bluetooth.as_ref()?, &config.bluetooth);
            Some(with_icon(shown.text, shown.compact, theme.foreground))
        }
        #[cfg(any(target_os = "linux", target_os = "macos"))]
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
            Some(with_icon(shown.text, shown.compact, theme.volume))
        }
        Module::Mic => {
            let shown = lyr_core::audio::mic(data.source.as_ref()?, &config.mic);
            Some(label(shown.text, theme.mic))
        }
        Module::Custom(index) => {
            let shown = data.custom_view(index, config, monitor)?;
            Some(label(shown.text, shown.colour.unwrap_or(theme.foreground)))
        }
        Module::Tray => {
            // Each item's icon, or its letter when none was found.
            let buttons: Vec<draw::Button> = lyr_core::tray::faces(&data.tray)
                .into_iter()
                .map(|face| draw::Button {
                    text: face.letter,
                    colour: if face.status == lyr_core::tray::Status::NeedsAttention {
                        theme.workspace_urgent.unwrap_or(theme.foreground)
                    } else {
                        theme.foreground
                    },
                    picture: face
                        .picture
                        .map(|picture| picture.sized(config.tray.icon_size as f32)),
                })
                .collect();
            // No padding: waybar pads only its workspace buttons, so the
            // icons are `spacing` apart.
            (!buttons.is_empty()).then_some(Content::Buttons {
                buttons,
                padding: 0.0,
                gap: config.tray.spacing as f32,
            })
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
        #[cfg(any(target_os = "linux", target_os = "macos"))]
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
                    picture: None,
                })
                .collect();
            Some(Content::Buttons {
                buttons,
                padding: theme.workspace_padding_x as f32,
                gap: 0.0,
            })
        }
        // Every module has an arm on Linux; elsewhere some are not built yet.
        #[cfg(not(target_os = "linux"))]
        _ => None,
    }
}

/// The tooltip `target` shows now on the bar on `monitor`, or `None` when it
/// has none or no data: a module's, or a tray item's. The workspaces have
/// none, as in waybar.
pub fn tooltip(
    target: &Target<Module>,
    config: &Config,
    data: &Data,
    monitor: Option<&str>,
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
        Module::PowerProfile => Some(
            lyr_core::power_profile::view(
                &data.power_profile.as_ref()?.active,
                &config.power_profile,
            )
            .tooltip,
        ),
        Module::Network => Some(data.network.as_ref()?.view(&config.network).tooltip),
        #[cfg(any(target_os = "linux", target_os = "macos"))]
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
        Module::Bluetooth => {
            Some(lyr_core::bluetooth::view(data.bluetooth.as_ref()?, &config.bluetooth).tooltip)
        }
        Module::Custom(index) => data.custom_view(index, config, monitor)?.tooltip,
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
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub fn shown_workspaces<'a>(
    state: &'a lyr_wm::State,
    config: &Config,
    monitor: Option<&str>,
) -> Vec<&'a lyr_wm::Workspace> {
    state
        .workspaces()
        .filter(|workspace| {
            config.workspaces.all_outputs
                || monitor.is_some_and(|monitor| workspace.monitor.as_deref() == Some(monitor))
        })
        .collect()
}

#[cfg(all(test, target_os = "linux"))]
mod snapshots;

#[cfg(test)]
mod tests {
    use chrono::DateTime;
    use lyr_config::{Item as LayoutItem, Module, Platform, from_sources};
    use lyr_core::draw::{Content, Entry, Target};
    use lyr_core::tray::{Item, Status};

    use super::{Data, content, entries, tooltip, zone_name};

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
            tooltip(&Target::Module(Module::Power), &config, &data, None, &now).as_deref(),
            Some("Power Button")
        );
    }

    #[test]
    fn the_volume_falls_back_to_its_icon_alone() {
        let config = from_sources(None, Platform::current()).expect("the defaults");
        let now = DateTime::parse_from_rfc3339("2026-10-19T09:05:00+08:00").expect("a time");
        let data = Data {
            sink: Some(lyr_core::audio::Sink {
                volume: 40,
                ..lyr_core::audio::Sink::default()
            }),
            ..Data::default()
        };
        // The default ramp's middle step, as `{icon} {volume}%` shows it.
        let icon = char::from_u32(0xF027).expect("a char").to_string();
        assert_eq!(
            content(Module::Volume, &config, &data, &now, None, None),
            Some(Content::Label {
                text: format!("{icon} 40%"),
                compact: Some(icon),
                colour: config.theme.volume,
            })
        );
    }

    #[test]
    fn a_crowded_top_bar_shows_these_modules_icon_alone_rather_than_none() {
        use lyr_core::draw::{Bar, Orientation, build};

        // Each format shows its icon beside more.
        let config = from_sources(
            Some((
                std::path::Path::new("config.toml"),
                "[battery]\nformat = \"{icon} {capacity}%\"\n\
                 [power_profile]\nformat = \"{icon} {profile}\"\n\
                 [network]\nformat = \"{icon} {ifname}\"\n",
            )),
            Platform::current(),
        )
        .expect("valid");
        let now = DateTime::parse_from_rfc3339("2026-10-19T09:05:00+08:00").expect("a time");
        let data = Data {
            battery: Some(lyr_core::battery::Reading {
                capacity: 64,
                status: lyr_core::battery::Status::Discharging,
                power: None,
                time_to: lyr_core::battery::TimeTo::Unknown,
            }),
            power_profile: Some(lyr_sources::power_profile::Profiles {
                active: lyr_core::power_profile::Profile {
                    name: "balanced".to_owned(),
                    driver: "intel_pstate".to_owned(),
                },
                available: Vec::new(),
            }),
            network: Some(lyr_core::network::Reading {
                link: Some(lyr_core::network::Link {
                    ifname: "eth0".to_owned(),
                    ipaddr: "192.168.1.20".to_owned(),
                    cidr: "24".to_owned(),
                    gwaddr: "192.168.1.1".to_owned(),
                    essid: String::new(),
                    signal_strength: 0,
                    wireless: false,
                }),
                radio_blocked: false,
            }),
            bluetooth: Some(lyr_core::bluetooth::Reading {
                state: lyr_core::bluetooth::State::Connected,
                connections: 1,
                device: Some(lyr_core::bluetooth::Device {
                    alias: "Buds".to_owned(),
                    battery: Some(80),
                }),
            }),
            sink: Some(lyr_core::audio::Sink {
                volume: 40,
                ..lyr_core::audio::Sink::default()
            }),
            ..Data::default()
        };
        for name in ["battery", "power_profile", "network", "bluetooth", "volume"] {
            let left = entries(
                &[LayoutItem::Named(name.to_owned())],
                &config,
                &data,
                &now,
                None,
                None,
            );
            // With every character 10 px wide, the left zone's 60 px holds
            // the icon's box (20 px of content, a 2 px border and 12 px of
            // padding each side, and a 10 px margin) but not the text's.
            let bar = Bar {
                width: 120.0,
                height: 50.0,
                orientation: Orientation::Horizontal,
                left: &left,
                center: &[],
                right: &[],
            };
            let (list, regions) = build(&bar, &config.theme, |text| {
                text.chars().count() as f32 * 10.0
            });
            let shown: Vec<String> = list
                .items
                .iter()
                .filter_map(|item| match item {
                    lyr_core::draw::Item::Group { items, .. } => Some(items),
                    _ => None,
                })
                .flatten()
                .filter_map(|item| match item {
                    lyr_core::draw::Item::Text { text, .. } => Some(text.clone()),
                    _ => None,
                })
                .collect();
            let Some(Entry::Module(lyr_core::draw::Module {
                content:
                    Content::Label {
                        text,
                        compact: Some(icon),
                        ..
                    },
                ..
            })) = left.first()
            else {
                panic!("{name} has a compact form: {left:?}");
            };
            assert_eq!(icon.chars().count(), 1, "{name}'s icon alone, not {text:?}");
            assert_eq!(
                shown,
                std::slice::from_ref(icon),
                "{name} is drawn as its icon"
            );
            assert!(
                regions.hit(20.0, 20.0).is_some(),
                "{name} still takes clicks"
            );
        }
    }

    #[test]
    fn a_zone_s_separators_and_spacers_are_kept_in_place() {
        let config = from_sources(None, Platform::current()).expect("the defaults");
        let now = DateTime::parse_from_rfc3339("2026-10-19T09:05:00+08:00").expect("a time");
        let items = [
            LayoutItem::Named("power".to_owned()),
            LayoutItem::Named("separator".to_owned()),
            LayoutItem::Spacer { spacer: 20 },
            LayoutItem::Named("power".to_owned()),
        ];
        let shown: Vec<String> = entries(&items, &config, &Data::default(), &now, None, None)
            .into_iter()
            .map(|entry| match entry {
                Entry::Module(module) => module.target.name().to_owned(),
                Entry::Separator => "|".to_owned(),
                Entry::Spacer(width) => format!("{width} px"),
            })
            .collect();
        assert_eq!(shown, ["power", "|", "20 px", "power"]);
    }

    #[test]
    fn a_custom_module_shows_its_own_bar_s_output() {
        let config = from_sources(
            Some((
                std::path::Path::new("config.toml"),
                "[layout]\nleft = [\"custom/a\", \"custom/fixed\"]\n\
                 [custom.a]\nexec = \"x\"\ncolours = { warn = \"#ff0000\" }\n\
                 [custom.fixed]\nformat = \"F\"\ntooltip = false\n",
            )),
            Platform::current(),
        )
        .expect("valid");
        let now = DateTime::parse_from_rfc3339("2026-10-19T09:05:00+08:00").expect("a time");
        let a = config.module("custom/a").expect("a module");
        let Module::Custom(index) = a else {
            panic!("{a:?} is not custom");
        };
        let mut data = Data::default();
        let output = lyr_core::custom::Output {
            text: "on DP-1".to_owned(),
            tooltip: "tip".to_owned(),
            class: vec!["warn".to_owned()],
            ..lyr_core::custom::Output::default()
        };
        data.custom
            .insert((index, Some("DP-1".to_owned())), Some(output));
        data.custom.insert((index, Some("DP-2".to_owned())), None);
        let shown = |monitor| content(a, &config, &data, &now, monitor, None);
        assert_eq!(
            shown(Some("DP-1")),
            Some(Content::Label {
                text: "on DP-1".to_owned(),
                compact: None,
                colour: lyr_config::Colour::parse("#ff0000").expect("a colour"),
            })
        );
        assert_eq!(shown(Some("DP-2")), None, "its script left it out");
        assert_eq!(shown(Some("DP-3")), None, "no output yet");
        let tip = |monitor| tooltip(&Target::Module(a), &config, &data, monitor, &now);
        assert_eq!(tip(Some("DP-1")).as_deref(), Some("tip"));
        assert_eq!(tip(Some("DP-3")), None);
        let fixed = config.module("custom/fixed").expect("a module");
        assert_eq!(
            content(fixed, &config, &data, &now, None, None),
            Some(Content::Label {
                text: "F".to_owned(),
                compact: None,
                colour: config.theme.foreground,
            })
        );
        assert_eq!(
            tooltip(&Target::Module(fixed), &config, &data, None, &now),
            None
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
        let tip = |index| {
            tooltip(
                &Target::Button(Module::Tray, index),
                &config,
                &data,
                None,
                &now,
            )
        };
        assert_eq!(
            tip(0).as_deref(),
            Some("Steam"),
            "the passive one is not shown"
        );
        assert_eq!(tip(1), None);
        assert_eq!(
            tooltip(
                &Target::Button(Module::Workspaces, 0),
                &config,
                &data,
                None,
                &now
            ),
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
