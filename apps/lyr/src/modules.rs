//! What each module shows, from the sources' latest data, and drawing a bar
//! from them.

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
    #[cfg(target_os = "linux")]
    pub power_profile: Option<lyr_sources::power_profile::Profiles>,
    #[cfg(target_os = "linux")]
    pub bluetooth: Option<lyr_core::bluetooth::Reading>,
    #[cfg(target_os = "linux")]
    pub workspaces: Option<lyr_wm::hyprland::State>,
}

impl Data {
    pub fn apply(&mut self, update: Update) {
        match update {
            #[cfg(target_os = "linux")]
            Update::Battery(reading) => self.battery = reading,
            #[cfg(target_os = "linux")]
            Update::PowerProfile(profiles) => self.power_profile = profiles,
            #[cfg(target_os = "linux")]
            Update::Bluetooth(reading) => self.bluetooth = reading,
            #[cfg(target_os = "linux")]
            Update::Workspaces(state) => self.workspaces = Some(state),
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
        #[cfg(target_os = "linux")]
        Module::PowerProfile => {
            let profiles = data.power_profile.as_ref()?;
            let shown = lyr_core::power_profile::view(&profiles.active, &config.power_profile);
            Some(label(shown.text, theme.foreground))
        }
        #[cfg(target_os = "linux")]
        Module::Bluetooth => {
            let shown = lyr_core::bluetooth::view(data.bluetooth.as_ref()?, &config.bluetooth);
            Some(label(shown.text, theme.foreground))
        }
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
        _ => None,
    }
}

/// The tooltip `module` shows now, or `None` when it has none or no data.
/// The workspaces have none, as in waybar.
pub fn tooltip(
    module: Module,
    config: &Config,
    data: &Data,
    now: &DateTime<FixedOffset>,
) -> Option<String> {
    match module {
        Module::Clock => Some(clock::view(now, zone().as_deref(), &config.clock.format).tooltip),
        Module::Battery => Some(battery::view(data.battery.as_ref()?, &config.battery).tooltip),
        #[cfg(target_os = "linux")]
        Module::PowerProfile => Some(
            lyr_core::power_profile::view(
                &data.power_profile.as_ref()?.active,
                &config.power_profile,
            )
            .tooltip,
        ),
        #[cfg(target_os = "linux")]
        Module::Bluetooth => {
            Some(lyr_core::bluetooth::view(data.bluetooth.as_ref()?, &config.bluetooth).tooltip)
        }
        _ => None,
    }
}

/// The system time zone's name, such as `Asia/Manila`: `TZ` when it names
/// one, else where `/etc/localtime` links into the zoneinfo database, which
/// is what systemd's `timedatectl` reports.
#[cfg(target_os = "linux")]
fn zone() -> Option<String> {
    match std::env::var("TZ") {
        Ok(tz) => zone_name(&tz),
        Err(_) => zone_name(std::fs::read_link("/etc/localtime").ok()?.to_str()?),
    }
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
fn zone() -> Option<String> {
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
    use super::zone_name;

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
