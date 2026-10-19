//! The power menu (`docs/plan/10-popups-and-menus.md#power-menu`): its
//! entries, which one is selected, and drawing it.
//!
//! The entries and their labels are `.menu-power`'s, in its order. The look is
//! lyr's own, since `.menu-power` drew in another program's: the bar's module
//! box (background, border and radius) holding one row per entry, the
//! selected row's text in the active workspace's colour.

use lyr_config::{Platform, Theme};

use crate::draw::{DrawList, Item, Size};
use crate::hit::{Rect, Regions};

/// One power action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Entry {
    Lock,
    Sleep,
    Hibernate,
    Shutdown,
    Logout,
    Reboot,
}

impl Entry {
    /// `.menu-power`'s label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Lock => "Lock",
            Self::Sleep => "Sleep",
            Self::Hibernate => "Hibernate",
            Self::Shutdown => "Shutdown",
            Self::Logout => "Logout",
            Self::Reboot => "Reboot",
        }
    }
}

/// The entries `platform` offers, in `.menu-power`'s order. macOS has no
/// hibernate to offer.
#[must_use]
pub fn entries(platform: Platform) -> Vec<Entry> {
    [
        Entry::Lock,
        Entry::Sleep,
        Entry::Hibernate,
        Entry::Shutdown,
        Entry::Logout,
        Entry::Reboot,
    ]
    .into_iter()
    .filter(|&entry| !(platform == Platform::Macos && entry == Entry::Hibernate))
    .collect()
}

/// The space between the menu's edge and its rows.
pub const PADDING: f32 = 8.0;
/// The space around a row's label, across and down.
pub const ROW_PADDING: (f32, f32) = (12.0, 6.0);

/// An open menu: its entries and the one selected.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Menu {
    entries: Vec<Entry>,
    selected: usize,
}

impl Menu {
    /// A menu of `entries`, the first selected. `None` with no entries.
    #[must_use]
    pub fn new(entries: Vec<Entry>) -> Option<Self> {
        (!entries.is_empty()).then_some(Self {
            entries,
            selected: 0,
        })
    }

    /// The selected entry.
    #[must_use]
    pub fn selected(&self) -> Entry {
        self.entries[self.selected]
    }

    /// Moves the selection up one row, stopping at the top.
    pub fn up(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    /// Moves the selection down one row, stopping at the bottom.
    pub fn down(&mut self) {
        self.selected = (self.selected + 1).min(self.entries.len() - 1);
    }

    /// Selects `entry`, as the pointer moving onto its row does. Returns
    /// whether the selection changed.
    pub fn select(&mut self, entry: Entry) -> bool {
        match self.entries.iter().position(|&held| held == entry) {
            Some(at) if at != self.selected => {
                self.selected = at;
                true
            }
            _ => false,
        }
    }

    /// The menu's draw list, its size and its rows' input regions, its text
    /// measured by `measure` in lines `line_height` tall.
    #[must_use]
    pub fn draw(
        &self,
        measure: impl Fn(&str) -> f32,
        line_height: f32,
        theme: &Theme,
    ) -> (DrawList, Size, Regions<Entry>) {
        let (across, down) = ROW_PADDING;
        let text_width = self
            .entries
            .iter()
            .map(|entry| measure(entry.label()))
            .fold(0.0, f32::max);
        let row = Size {
            width: text_width + 2.0 * across,
            height: line_height + 2.0 * down,
        };
        let size = Size {
            width: row.width + 2.0 * PADDING,
            height: row.height * self.entries.len() as f32 + 2.0 * PADDING,
        };
        let mut items = vec![Item::Box {
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: size.width,
                height: size.height,
            },
            radius: theme.radius as f32,
            fill: theme.module_background,
            border: Some((theme.border_width as f32, theme.border)),
        }];
        let mut regions = Regions::default();
        for (index, &entry) in self.entries.iter().enumerate() {
            let rect = Rect {
                x: PADDING,
                y: PADDING + row.height * index as f32,
                width: row.width,
                height: row.height,
            };
            items.push(Item::Text {
                rect: Rect {
                    x: rect.x + across,
                    y: rect.y + down,
                    width: text_width,
                    height: line_height,
                },
                text: entry.label().to_owned(),
                colour: if index == self.selected {
                    theme.workspace_active
                } else {
                    theme.foreground
                },
            });
            regions.push(rect, entry);
        }
        (DrawList { items }, size, regions)
    }
}

#[cfg(test)]
mod tests;
