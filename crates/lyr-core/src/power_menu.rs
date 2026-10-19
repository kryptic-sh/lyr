//! The power menu (`docs/plan/10-popups-and-menus.md#power-menu`): its
//! entries, as rows of a list menu ([`crate::list_menu`]).
//!
//! The entries and their labels are `.menu-power`'s, in its order. The look is
//! lyr's own, since `.menu-power` drew in another program's: the bar's module
//! box (background, border and radius) holding one row per entry, the
//! selected row's text in the active workspace's colour.

use lyr_config::Platform;

use crate::list_menu::Row;

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

/// The menu's rows: each entry by its label.
#[must_use]
pub fn rows(entries: &[Entry]) -> Vec<Row<Entry>> {
    entries
        .iter()
        .map(|&entry| Row::new(entry.label(), entry))
        .collect()
}

#[cfg(test)]
mod tests;
