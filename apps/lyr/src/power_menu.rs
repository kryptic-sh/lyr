//! The power menu (`docs/plan/10-popups-and-menus.md#power-menu`): a list
//! popup ([`crate::list_popup`]) of the power entries, each carried out by
//! lyr or by the command the config sets for it.

use lyr_core::power_menu::Entry;

use crate::list_popup::ListPopup;

/// The open power menu.
pub type PowerMenu = ListPopup<Entry>;

/// The command the config sets for `entry`, or `None` for lyr's own action.
pub fn configured(menu: &lyr_config::PowerMenu, entry: Entry) -> Option<&lyr_config::Exec> {
    match entry {
        Entry::Lock => menu.lock.as_ref(),
        Entry::Sleep => menu.sleep.as_ref(),
        Entry::Hibernate => menu.hibernate.as_ref(),
        Entry::Shutdown => menu.shutdown.as_ref(),
        Entry::Logout => menu.logout.as_ref(),
        Entry::Reboot => menu.reboot.as_ref(),
    }
}

#[cfg(test)]
mod tests;
