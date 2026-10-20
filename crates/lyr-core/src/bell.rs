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
    render(state, config)
}

/// The bell where lyr cannot read the notifications or do-not-disturb, as
/// on Windows (plan 09): always the `none` state.
#[must_use]
pub fn stateless(config: &Config) -> String {
    render("none", config)
}

/// The bell's text in `state` under `config`.
fn render(state: &str, config: &Config) -> String {
    let icon = pick(&config.icons, &[state], 0);
    config
        .format
        .render(|name| (name == "icon").then_some(icon))
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use lyr_config::{Platform, from_sources};

    use super::{stateless, view};
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
    fn the_stateless_bell_is_the_empty_one() {
        let config = from_sources(None, Platform::Windows).expect("the defaults");
        assert_eq!(stateless(&config.notifications), "\u{F0A2}");
    }

    #[test]
    fn the_bell_shows_the_history_and_do_not_disturb() {
        let config = from_sources(None, Platform::Linux).expect("the defaults");
        let config = &config.notifications;
        let mut store = Store::default();
        assert_eq!(view(&store, config), "\u{F0A2}");
        store.dnd = true;
        assert_eq!(view(&store, config), "\u{F1F7}");
        store.notify(incoming(false), Instant::now());
        assert_eq!(view(&store, config), "\u{F071}");
        store.dnd = false;
        assert_eq!(view(&store, config), "\u{F071}");
    }

    #[test]
    fn a_transient_notification_leaves_the_bell_empty() {
        let config = from_sources(None, Platform::Linux).expect("the defaults");
        let mut store = Store::default();
        store.notify(incoming(true), Instant::now());
        assert_eq!(view(&store, &config.notifications), "\u{F0A2}");
    }
}
