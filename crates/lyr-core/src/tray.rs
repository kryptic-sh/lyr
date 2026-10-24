//! The tray (`docs/plan/08-tray.md`): StatusNotifierItems as the module
//! sees them, who registered where, and what a click or a wheel step on one
//! asks of it, following waybar's `src/modules/sni` (master `c15ec02`, read
//! 2026-10-20). The platforms' sources fill it in (`lyr_sources::tray`).

pub mod menu;

use crate::draw::Picture;
use crate::pointer::Press;

/// Where an item lives when it registers with only its bus name.
pub const ITEM_PATH: &str = "/StatusNotifierItem";

/// An item's `Status`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Status {
    /// Shown only when it needs no attention: waybar hides it unless
    /// `show-passive-items`, which the dotfiles do not set.
    Passive,
    #[default]
    Active,
    NeedsAttention,
}

impl Status {
    /// From the property's text; an unknown status is active.
    #[must_use]
    pub fn parse(text: &str) -> Self {
        match text {
            "Passive" => Self::Passive,
            "NeedsAttention" => Self::NeedsAttention,
            _ => Self::Active,
        }
    }
}

/// One of an item's `IconPixmap` sizes: `width` × `height` pixels of ARGB32
/// in network byte order, as the specification lays them out.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Pixmap {
    pub width: i32,
    pub height: i32,
    pub data: Vec<u8>,
}

/// Where an item's icon may come from, in the order to try them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IconSource<'a> {
    /// A name in the icon theme, or a file's path.
    Name(&'a str),
    /// Pixels the item sent, in each size it sent.
    Pixmaps(&'a [Pixmap]),
}

/// The icon a tray item draws when none of its own can be found, as waybar
/// draws it.
pub const MISSING_ICON: &str = "image-missing";

/// One StatusNotifierItem's properties the tray uses.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Item {
    /// Its bus name and object path, which identify it.
    pub bus_name: String,
    pub path: String,
    pub id: String,
    pub title: String,
    pub status: Status,
    pub icon_name: String,
    pub icon_pixmap: Vec<Pixmap>,
    pub attention_icon_name: String,
    pub attention_icon_pixmap: Vec<Pixmap>,
    pub icon_theme_path: String,
    /// The `ToolTip`'s title.
    pub tooltip: String,
    /// Its `com.canonical.dbusmenu` object, when it has one.
    pub menu: Option<String>,
    /// A left click opens its menu rather than activating it.
    pub item_is_menu: bool,
    /// Its icon as found from [`Item::icon_sources`], once the host has
    /// looked; `None` draws [`Item::letter`].
    pub picture: Option<Picture>,
}

impl Item {
    /// Whether the tray shows it.
    #[must_use]
    pub fn shown(&self) -> bool {
        self.status != Status::Passive
    }

    /// Its tooltip: the `ToolTip`'s title, else its `Title`, as waybar's
    /// `Item::setProperty` falls back; `None` when both are empty.
    #[must_use]
    pub fn tooltip_text(&self) -> Option<&str> {
        [&self.tooltip, &self.title]
            .into_iter()
            .find(|text| !text.is_empty())
            .map(String::as_str)
    }

    /// Where its icon may come from, in waybar's order (`Item::getIconPixbuf`,
    /// master, read 2026-10-25): while it needs attention, its attention
    /// icon's name and then pixels; then its icon's name and then pixels.
    /// Empty names and pixmap lists are left out. waybar's attention movie
    /// and overlay icon are not read.
    #[must_use]
    pub fn icon_sources(&self) -> Vec<IconSource<'_>> {
        let attention = (self.status == Status::NeedsAttention).then_some([
            IconSource::Name(&self.attention_icon_name),
            IconSource::Pixmaps(&self.attention_icon_pixmap),
        ]);
        attention
            .into_iter()
            .flatten()
            .chain([
                IconSource::Name(&self.icon_name),
                IconSource::Pixmaps(&self.icon_pixmap),
            ])
            .filter(|source| match source {
                IconSource::Name(name) => !name.is_empty(),
                IconSource::Pixmaps(pixmaps) => !pixmaps.is_empty(),
            })
            .collect()
    }

    /// What stands in for its icon when it has no picture: the first letter
    /// of its title, else of its id, upper-cased.
    #[must_use]
    pub fn letter(&self) -> String {
        [&self.title, &self.id]
            .into_iter()
            .find_map(|name| name.chars().find(|c| c.is_alphanumeric()))
            .map_or_else(|| "?".to_owned(), |c| c.to_uppercase().collect())
    }
}

/// What the bar draws of one item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Face {
    pub letter: String,
    pub status: Status,
    pub picture: Option<Picture>,
}

/// What the bar draws of `items`: each shown item's face, in order. A
/// change to anything else, such as a tooltip or an icon's raw pixels that
/// came to the same picture, leaves it as it was.
#[must_use]
pub fn faces(items: &[Item]) -> Vec<Face> {
    items
        .iter()
        .filter(|item| item.shown())
        .map(|item| Face {
            letter: item.letter(),
            status: item.status,
            picture: item.picture.clone(),
        })
        .collect()
}

/// What a click asks of an item.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Click {
    Activate,
    SecondaryActivate,
    /// Its DBusMenu, or `ContextMenu` when it serves none.
    Menu,
}

/// What `press` on an item asks of it: a right click, or a left one on an
/// item that is a menu, opens its menu; a left click activates and a
/// middle one secondary-activates (waybar's `Item::handleClick`).
#[must_use]
pub const fn click(press: Press, item_is_menu: bool) -> Option<Click> {
    match press {
        Press::Left if item_is_menu => Some(Click::Menu),
        Press::Right => Some(Click::Menu),
        Press::Left => Some(Click::Activate),
        Press::Middle => Some(Click::SecondaryActivate),
        _ => None,
    }
}

/// `Scroll`'s delta and orientation for a wheel step: away from the user
/// is up, -1 (waybar's `GDK_SCROLL_UP`).
#[must_use]
pub const fn scroll(up: bool) -> (i32, &'static str) {
    (if up { -1 } else { 1 }, "vertical")
}

/// The bus name and object path a `RegisterStatusNotifierItem` argument
/// names, from the caller `sender`: a path alone is on the caller's
/// connection (libappindicator does this), `name/path` is both (Electron),
/// and a bus name alone has its item at [`ITEM_PATH`].
#[must_use]
pub fn item_address(service: &str, sender: &str) -> (String, String) {
    if service.starts_with('/') {
        (sender.to_owned(), service.to_owned())
    } else if let Some(slash) = service.find('/') {
        (service[..slash].to_owned(), service[slash..].to_owned())
    } else {
        (service.to_owned(), ITEM_PATH.to_owned())
    }
}

/// The watcher's registrations: items by bus name and path, in the order
/// they came, and the hosts' bus names.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Registry {
    items: Vec<(String, String)>,
    hosts: Vec<String>,
}

impl Registry {
    /// Adds the item at `bus_name` and `path`; `false` when it was there.
    pub fn register_item(&mut self, bus_name: &str, path: &str) -> bool {
        let item = (bus_name.to_owned(), path.to_owned());
        if self.items.contains(&item) {
            return false;
        }
        self.items.push(item);
        true
    }

    /// Adds a host; `true` when there was none before.
    pub fn register_host(&mut self, bus_name: &str) -> bool {
        let first = self.hosts.is_empty();
        if !self.hosts.iter().any(|host| host == bus_name) {
            self.hosts.push(bus_name.to_owned());
        }
        first
    }

    /// Drops everything `bus_name` registered, now that it is gone: the
    /// items it served, as `RegisteredStatusNotifierItems` names them, and
    /// whether the last host went.
    pub fn vanished(&mut self, bus_name: &str) -> (Vec<String>, bool) {
        let gone: Vec<String> = self
            .items
            .iter()
            .filter(|(bus, _)| bus == bus_name)
            .map(|(bus, path)| format!("{bus}{path}"))
            .collect();
        self.items.retain(|(bus, _)| bus != bus_name);
        let had_host = !self.hosts.is_empty();
        self.hosts.retain(|host| host != bus_name);
        (gone, had_host && self.hosts.is_empty())
    }

    /// `RegisteredStatusNotifierItems`: each item as its bus name and path
    /// run together, as waybar and KDE list them.
    #[must_use]
    pub fn items(&self) -> Vec<String> {
        self.items
            .iter()
            .map(|(bus, path)| format!("{bus}{path}"))
            .collect()
    }

    /// `IsStatusNotifierHostRegistered`.
    #[must_use]
    pub fn has_host(&self) -> bool {
        !self.hosts.is_empty()
    }
}

#[cfg(test)]
mod tests;
