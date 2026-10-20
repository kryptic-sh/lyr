//! What the running bar does for the Wi-Fi menu: opening and closing it,
//! drawing it as NetworkManager changes, and carrying out what it asks.

use std::time::{Duration, Instant};

use anyhow::Context as _;
use crcbl_shell::{Shell, ShellEvent};

use super::App;
use crate::menu_panel::Place;
use crate::tooltips::Look;
use crate::wifi_menu::{DOUBLE_CLICK, Outcome, WifiMenu};

impl App {
    /// Opens the Wi-Fi menu at `place` at `now`, or
    /// closes it if it is open, unless it opened a double click's interval
    /// ago. Returns why it did not open: there is no Wi-Fi device. Windows
    /// opens its network Settings page instead.
    pub(super) fn toggle_wifi_menu(
        &mut self,
        shell: &mut dyn Shell,
        place: Place,
        now: Instant,
    ) -> anyhow::Result<Result<(), &'static str>> {
        if let Some(menu) = &self.wifi_menu {
            // The second click of the double click that opened it.
            if now.saturating_duration_since(menu.opened()) < DOUBLE_CLICK {
                return Ok(Ok(()));
            }
            if let Some(menu) = self.wifi_menu.take() {
                menu.close(shell, false)?;
            }
            return Ok(Ok(()));
        }
        let Some(wifi) = self.data.wifi.as_ref().filter(|wifi| wifi.device.is_some()) else {
            // Windows and macOS have no Wi-Fi menu yet: their Settings page
            // stands in.
            #[cfg(any(windows, target_os = "macos"))]
            {
                crate::settings::open(crate::settings::Page::Network);
                return Ok(Ok(()));
            }
            #[cfg(target_os = "linux")]
            return Ok(Err("there is no Wi-Fi device"));
        };
        // A rescan on every open, so the list is current.
        if wifi.available && wifi.radio.enabled {
            self.wifi_scan();
        }
        let look = Look {
            theme: &self.config.theme,
            measure: &|s: &str| self.text.measure(s),
            measure_in: &|s: &str, style| self.text.measure_in(s, style),
            line_height: self.text.line_height(),
            calendar_line_height: self.text.calendar_line_height(),
        };
        self.wifi_menu = Some(
            WifiMenu::open(shell, place, &look, wifi, now, since_boot())
                .context("opening the Wi-Fi menu")?,
        );
        Ok(Ok(()))
    }

    /// Draws the Wi-Fi menu again, if it is open, for NetworkManager as it
    /// is now; closes it if the Wi-Fi device went.
    pub(super) fn refresh_wifi_menu(&mut self, shell: &mut dyn Shell) -> anyhow::Result<()> {
        let Some(menu) = &mut self.wifi_menu else {
            return Ok(());
        };
        let Some(wifi) = self.data.wifi.as_ref().filter(|wifi| wifi.device.is_some()) else {
            if let Some(menu) = self.wifi_menu.take() {
                menu.close(shell, false)?;
            }
            return Ok(());
        };
        let look = Look {
            theme: &self.config.theme,
            measure: &|s: &str| self.text.measure(s),
            measure_in: &|s: &str, style| self.text.measure_in(s, style),
            line_height: self.text.line_height(),
            calendar_line_height: self.text.calendar_line_height(),
        };
        menu.refresh(shell, &look, wifi, Instant::now())
    }

    /// When the Wi-Fi menu has to be drawn again on its own.
    pub(super) fn wifi_wake(&self) -> Option<Instant> {
        self.wifi_menu.as_ref()?.next_wake()
    }

    /// Shows how a join ended in the menu; with the menu closed, a failure
    /// is a notice.
    #[cfg(target_os = "linux")]
    pub(super) fn wifi_joined(&mut self, joined: crate::wifi_menu::Joined) {
        match &mut self.wifi_menu {
            Some(menu) => menu.joined(joined),
            None => {
                if let crate::wifi_menu::Joined::Failed(why) = joined {
                    tracing::warn!("wi-fi: {why}");
                    // It names the network, which is anyone's text.
                    self.commands
                        .notify("Wi-Fi".to_owned(), lyr_core::markup::escape(&why));
                }
            }
        }
    }

    /// Applies `event`, which is for the Wi-Fi menu's window.
    pub(super) fn wifi_event(
        &mut self,
        shell: &mut dyn Shell,
        event: &ShellEvent,
    ) -> anyhow::Result<()> {
        let Some(menu) = &mut self.wifi_menu else {
            return Ok(());
        };
        if let ShellEvent::WindowDestroyed { .. } = event {
            if let Some(menu) = self.wifi_menu.take() {
                menu.close(shell, true)?;
            }
            return Ok(());
        }
        match menu.event(shell, event)? {
            Outcome::Kept => return Ok(()),
            outcome @ (Outcome::Close | Outcome::Closed) => {
                if let Some(menu) = self.wifi_menu.take() {
                    menu.close(shell, outcome == Outcome::Closed)?;
                }
                return Ok(());
            }
            Outcome::Redraw => {}
            outcome => self.wifi_act(outcome),
        }
        self.refresh_wifi_menu(shell)
    }

    /// Asks the Wi-Fi device for a scan.
    #[cfg(target_os = "linux")]
    fn wifi_scan(&self) {
        if let Some(device) = self.data.wifi.as_ref().and_then(|wifi| wifi.device.clone()) {
            self.commands
                .wifi(crate::sources::WifiRequest::Scan(device));
        }
    }

    /// Carries out what the Wi-Fi menu asked.
    #[cfg(target_os = "linux")]
    fn wifi_act(&self, outcome: Outcome) {
        use lyr_sources::network::Join;

        use crate::sources::{JoinRequest, WifiRequest};
        let Some(wifi) = &self.data.wifi else {
            return;
        };
        let Some(device) = wifi.device.clone() else {
            return;
        };
        let profile = |ssid: &str| {
            wifi.profiles_for(ssid)
                .first()
                .map(|profile| profile.path.clone())
        };
        let request = match outcome {
            Outcome::ToggleRadio => WifiRequest::Radio(!wifi.radio.enabled),
            Outcome::Forget(ssid) => WifiRequest::Forget(
                wifi.profiles_for(&ssid)
                    .iter()
                    .map(|profile| profile.path.clone())
                    .collect(),
            ),
            Outcome::Join(network, password) => {
                let join = match profile(&network.ssid) {
                    Some(profile) => Join::Saved {
                        profile,
                        point: network.point.clone(),
                    },
                    None => Join::New {
                        ssid: network.ssid.clone(),
                        security: network.security,
                        password,
                        point: network.point.clone(),
                    },
                };
                WifiRequest::Join(JoinRequest {
                    device,
                    join,
                    ssid: network.ssid,
                    security: network.security,
                    new_password: None,
                })
            }
            Outcome::NewPassword(network, password) => {
                let Some(profile) = profile(&network.ssid) else {
                    return;
                };
                WifiRequest::Join(JoinRequest {
                    device,
                    join: Join::Saved {
                        profile,
                        point: network.point.clone(),
                    },
                    ssid: network.ssid,
                    security: network.security,
                    new_password: Some(password),
                })
            }
            Outcome::Kept | Outcome::Close | Outcome::Closed | Outcome::Redraw => return,
        };
        self.commands.wifi(request);
    }

    /// Windows and macOS: the menu has no device to open on yet.
    #[cfg(not(target_os = "linux"))]
    fn wifi_scan(&self) {}

    /// Windows and macOS: the menu has no device to open on yet.
    #[cfg(not(target_os = "linux"))]
    fn wifi_act(&self, _outcome: Outcome) {}
}

/// How long the machine has been up, on the clock NetworkManager's
/// `LastScan` uses; `None` where it cannot be read, which treats the last
/// scan as stale.
fn since_boot() -> Option<Duration> {
    #[cfg(target_os = "linux")]
    return lyr_sources::network::linux::wifi::since_boot()
        .inspect_err(|error| tracing::warn!("wi-fi: the time since boot: {error}"))
        .ok();
    #[cfg(not(target_os = "linux"))]
    None
}
