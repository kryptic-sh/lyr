//! The notifications module, the bell (`docs/plan/09-notifications.md#the-bell-module`):
//! whether the history holds anything, and whether do-not-disturb is on.
//! waybar drew swaync's eight states; with no inhibitors in lyr they are
//! four, and the dotfiles' icons make them three glyphs.

use lyr_config::Notifications as Config;

use crate::icon::pick;
use crate::notifications::Store;

/// The bell's text for `store` under `config`.
#[must_use]
pub fn view(store: &Store, config: &Config) -> String {
    // swaync counts what its control centre lists, which leaves transient
    // notifications out, as the history does.
    let state = match (store.dnd, store.history().is_empty()) {
        (false, false) => "notification",
        (false, true) => "none",
        (true, false) => "dnd-notification",
        (true, true) => "dnd-none",
    };
    let icon = pick(&config.icons, &[state], 0);
    config
        .format
        .render(|name| (name == "icon").then_some(icon))
}

#[cfg(test)]
mod tests {
    use lyr_config::{Platform, from_sources};

    use super::view;
    use crate::notifications::{Incoming, Store};

    fn incoming(transient: bool) -> Incoming {
        Incoming {
            app_name: "app".to_owned(),
            summary: "hello".to_owned(),
            transient,
            ..Incoming::default()
        }
    }

    #[test]
    fn the_bell_shows_the_history_and_do_not_disturb() {
        let config = from_sources(None, Platform::Linux).expect("the defaults");
        let config = &config.notifications;
        let mut store = Store::default();
        assert_eq!(view(&store, config), "\u{F0A2}");
        store.dnd = true;
        assert_eq!(view(&store, config), "\u{F1F7}");
        store.notify(incoming(false));
        assert_eq!(view(&store, config), "\u{F071}");
        store.dnd = false;
        assert_eq!(view(&store, config), "\u{F071}");
    }

    #[test]
    fn a_transient_notification_leaves_the_bell_empty() {
        let config = from_sources(None, Platform::Linux).expect("the defaults");
        let mut store = Store::default();
        store.notify(incoming(true));
        assert_eq!(view(&store, &config.notifications), "\u{F0A2}");
    }
}
