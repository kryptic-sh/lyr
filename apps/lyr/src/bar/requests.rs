//! What the running bar does with a control request (`lyr reload`, …).

#[cfg(target_os = "linux")]
use std::sync::PoisonError;

use anyhow::Context as _;
use crcbl_shell::Shell;
use lyr_config::{Module, Platform};
use lyr_core::clock::Tick;
use lyr_core::control::{Event, Panel, Request};

use super::{App, load_text, next_tick};
use crate::control::{self, Incoming};
use crate::menu_panel::Place;

impl App {
    /// Carries out `incoming` and replies to it. Returns whether it asked the
    /// bar to quit.
    pub(super) fn request(
        &mut self,
        shell: &mut dyn Shell,
        incoming: Incoming,
    ) -> anyhow::Result<bool> {
        match incoming.request {
            Request::Quit => {
                incoming.reply(control::OK);
                return Ok(true);
            }
            Request::Reload => self.reload(shell, incoming)?,
            Request::Toggle => {
                if self.hidden {
                    let bar = self.config.bar.clone();
                    self.keep_bars(shell, &bar, None)
                        .context("creating the bars")?;
                } else {
                    self.close_all(shell)?;
                }
                self.hidden = !self.hidden;
                incoming.reply(control::OK);
            }
            Request::Panel(Panel::Clock) => {
                let place = self.place_under(Module::Clock);
                self.toggle_clock_panel(shell, place)?;
                incoming.reply(control::OK);
            }
            Request::Panel(Panel::Bluetooth) => {
                let place = self.place_under(Module::Bluetooth);
                match self.toggle_bluetooth_menu(shell, place)? {
                    Ok(()) => incoming.reply(control::OK),
                    Err(why) => incoming.reply(format!("error: {why}")),
                }
            }
            Request::Panel(Panel::Wifi) => {
                let place = self.place_under(Module::Network);
                match self.toggle_wifi_menu(shell, place, std::time::Instant::now())? {
                    Ok(()) => incoming.reply(control::OK),
                    Err(why) => incoming.reply(format!("error: {why}")),
                }
            }
            Request::Panel(Panel::Audio) => {
                let place = self.place_under(Module::Volume);
                match self.toggle_audio_menu(shell, place, lyr_core::audio_menu::Tab::Output)? {
                    Ok(()) => incoming.reply(control::OK),
                    Err(why) => incoming.reply(format!("error: {why}")),
                }
            }
            request @ (Request::Dnd | Request::Clear) => {
                let reply = self.notifications(request);
                incoming.reply(reply);
            }
            Request::Event(Event::Workspace) => {
                // AeroSpace pushes nothing, so its hook asks for a read;
                // Linux compositors push their own changes.
                #[cfg(target_os = "macos")]
                {
                    self.commands.refresh_workspaces();
                    incoming.reply(control::OK);
                }
                #[cfg(not(target_os = "macos"))]
                incoming.reply("error: workspace events are read from AeroSpace, on macOS only");
            }
        }
        Ok(false)
    }

    /// Where `lyr panel` opens a menu: on the focused output, under `module`
    /// where its bar shows it, else at that output's edge
    /// (`docs/plan/10-popups-and-menus.md`). With no window manager to say
    /// which output is focused, under the first bar that shows it.
    fn place_under(&self, module: Module) -> Place {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        let focused = self
            .data
            .workspaces
            .as_ref()
            .and_then(lyr_wm::State::focused_output);
        #[cfg(windows)]
        let focused: Option<&str> = None;
        self.windows
            .values()
            .filter(|window| focused.is_none() || window.monitor.as_deref() == focused)
            .find_map(|window| {
                window.place_under(
                    &lyr_core::draw::Target::Module(module),
                    self.config.bar.position,
                )
            })
            .unwrap_or(Place::EDGE)
    }

    /// `lyr reload`. An invalid config is logged and the running one kept
    /// (plan 05); so is a font that will not load.
    fn reload(&mut self, shell: &mut dyn Shell, incoming: Incoming) -> anyhow::Result<()> {
        let reloaded = lyr_config::load(self.config_path.as_deref(), Platform::current())
            .map_err(anyhow::Error::from)
            .and_then(|(new, _)| {
                let new_text = if new.theme.font == self.config.theme.font
                    && new.theme.font_size == self.config.theme.font_size
                    && new.theme.calendar_font == self.config.theme.calendar_font
                {
                    None
                } else {
                    Some(load_text(&new)?)
                };
                Ok((new, new_text))
            });
        match reloaded {
            Err(error) => {
                tracing::warn!("reload: keeping the running config: {error:#}");
                incoming.reply(format!("error: {error:#}"));
            }
            Ok((new, new_text)) => {
                if let Some(new_text) = new_text {
                    self.text = new_text;
                }
                if new.bar != self.config.bar && !self.hidden {
                    // The power menu hangs from a bar that may move.
                    if let Some(menu) = self.power_menu.take() {
                        menu.close(shell, false)?;
                    }
                    self.bars
                        .reshape(shell, &new.bar)
                        .context("reshaping the bars")?;
                    self.keep_bars(shell, &new.bar, None)
                        .context("creating the bars")?;
                }
                self.tick = Tick::of(&new.clock.format);
                self.next_tick = next_tick(self.tick);
                self.config = new;
                self.redraw_all();
                // A new font can draw the same list differently.
                for window in self.windows.values_mut() {
                    window.shown = None;
                }
                tracing::info!("reload: config reloaded");
                incoming.reply(control::OK);
            }
        }
        Ok(())
    }

    /// `lyr dnd` and `lyr clear`, which need lyr to be the notification
    /// daemon; the `toggle_dnd` built-in asks for the first too. Returns the
    /// reply.
    #[cfg(target_os = "linux")]
    pub(super) fn notifications(&mut self, request: Request) -> String {
        let Some(store) = &self.data.notifications else {
            return "error: lyr is not the notification daemon".to_owned();
        };
        if request == Request::Dnd {
            let mut store = store.lock().unwrap_or_else(PoisonError::into_inner);
            store.dnd = !store.dnd;
            tracing::info!("do-not-disturb {}", if store.dnd { "on" } else { "off" });
        } else {
            self.commands.clear_notifications();
            self.toasts.take_down();
        }
        self.redraw_all();
        self.refresh_panel();
        control::OK.to_owned()
    }

    /// Windows and macOS keep their own notification centres.
    #[cfg(not(target_os = "linux"))]
    pub(super) fn notifications(&mut self, _request: Request) -> String {
        "error: lyr is not the notification daemon on this platform".to_owned()
    }
}
