//! A tray item's menu (`docs/plan/08-tray.md`): its DBusMenu entries in a
//! list popup ([`crate::list_popup`]) under the item's button. An entry
//! with a submenu opens it in place, with a Back row; a grab popup cannot
//! change size, so each step opens the popup again.

use crcbl_shell::{Shell, ShellEvent, WindowId};
use lyr_config::Position;
use lyr_core::hit::Rect;
use lyr_core::tray::menu::{Entry, Pick, rows};
use lyr_render::text::Text;

use crate::list_popup::{ListPopup, Outcome as PopupOutcome};
use crate::tooltips::Look;

/// An item's menu as read, and where to show it. Only Linux has a tray
/// to host, so elsewhere no menu is ever opened.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
#[derive(Clone, Debug)]
pub struct Read {
    /// The item's bus name and its menu's object path.
    pub bus_name: String,
    pub menu: String,
    pub root: Entry,
    /// The bar the item's button is on, and the button.
    pub bar: WindowId,
    pub anchor: Rect,
}

/// The open menu.
pub struct TrayMenu {
    popup: ListPopup<Pick>,
    read: Read,
    /// The submenus opened, outermost first.
    path: Vec<i32>,
    position: Position,
}

/// What an event did to the menu.
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    Kept,
    /// The entry with this id was clicked; the menu is to close.
    Clicked(i32),
    /// It is to close; `true` when the window system closed its window.
    Closed(bool),
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
impl TrayMenu {
    /// Opens `read`'s menu under its button on a bar at `position`. `None`
    /// when it shows nothing.
    pub fn open(
        shell: &mut dyn Shell,
        read: Read,
        position: Position,
        look: &Look<'_>,
    ) -> anyhow::Result<Option<Self>> {
        let Some(popup) = ListPopup::open(
            shell,
            read.bar,
            read.anchor,
            position,
            rows(&read.root, false),
            look,
        )?
        else {
            return Ok(None);
        };
        Ok(Some(Self {
            popup,
            read,
            path: Vec::new(),
            position,
        }))
    }

    /// The item's bus name and its menu's path.
    pub fn item(&self) -> (&str, &str) {
        (&self.read.bus_name, &self.read.menu)
    }

    /// Whether `window` is the menu's.
    pub fn owns(&self, window: WindowId) -> bool {
        self.popup.owns(window)
    }

    /// Whether the menu opened from the bar `bar`.
    pub fn opened_from(&self, bar: WindowId) -> bool {
        self.popup.opened_from(bar)
    }

    /// Whether it has a frame to present.
    pub fn dirty(&self) -> bool {
        self.popup.dirty()
    }

    /// The entry whose children are shown.
    fn shown(&self) -> Option<&Entry> {
        match self.path.last() {
            Some(&id) => self.read.root.find(id),
            None => Some(&self.read.root),
        }
    }

    /// Opens the popup again on the submenu `path` leads to.
    fn reopen(&mut self, shell: &mut dyn Shell, look: &Look<'_>) -> anyhow::Result<Outcome> {
        let Some(shown) = self.shown() else {
            return Ok(Outcome::Closed(false));
        };
        let rows = rows(shown, !self.path.is_empty());
        let Some(popup) = ListPopup::open(
            shell,
            self.read.bar,
            self.read.anchor,
            self.position,
            rows,
            look,
        )?
        else {
            return Ok(Outcome::Closed(false));
        };
        std::mem::replace(&mut self.popup, popup).close(shell, false)?;
        Ok(Outcome::Kept)
    }

    /// Applies an event for the menu's window.
    pub fn event(
        &mut self,
        shell: &mut dyn Shell,
        event: &ShellEvent,
        look: &Look<'_>,
    ) -> anyhow::Result<Outcome> {
        match self.popup.event(shell, event, look)? {
            PopupOutcome::Kept => Ok(Outcome::Kept),
            PopupOutcome::Dismissed => Ok(Outcome::Closed(true)),
            PopupOutcome::Chosen(Pick::Entry(id)) => Ok(Outcome::Clicked(id)),
            PopupOutcome::Chosen(Pick::Open(id)) => {
                self.path.push(id);
                self.reopen(shell, look)
            }
            PopupOutcome::Chosen(Pick::Back) => {
                self.path.pop();
                self.reopen(shell, look)
            }
        }
    }

    /// Draws the menu if it has a frame to present.
    pub fn draw(&mut self, text: &Text) -> anyhow::Result<()> {
        self.popup.draw(text)
    }

    /// Closes the menu: its surface, then its window unless the window
    /// system has closed it already (`window_gone`).
    pub fn close(self, shell: &mut dyn Shell, window_gone: bool) -> anyhow::Result<()> {
        self.popup.close(shell, window_gone)
    }
}

#[cfg(test)]
mod tests;
