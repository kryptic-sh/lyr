//! What the running bar does with a control request (`lyr reload`, …).

#[cfg(target_os = "linux")]
use std::sync::PoisonError;

use anyhow::Context as _;
use crcbl_shell::Shell;
use lyr_config::Platform;
use lyr_core::clock::Tick;
use lyr_core::control::Request;

use super::{App, load_text, next_tick};
use crate::control::{self, Incoming};

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
                    self.bars
                        .sync(shell, &self.config.bar)
                        .context("creating the bars")?;
                } else {
                    self.close_all(shell)?;
                }
                self.hidden = !self.hidden;
                incoming.reply(control::OK);
            }
            request @ (Request::Dnd | Request::Clear) => {
                let reply = self.notifications(request);
                incoming.reply(reply);
            }
            other => {
                incoming.reply(format!("error: `{other}` is not implemented yet"));
            }
        }
        Ok(false)
    }

    /// `lyr reload`. An invalid config is logged and the running one kept
    /// (plan 05); so is a font that will not load.
    fn reload(&mut self, shell: &mut dyn Shell, incoming: Incoming) -> anyhow::Result<()> {
        let reloaded = lyr_config::load(self.config_path.as_deref(), Platform::current())
            .map_err(anyhow::Error::from)
            .and_then(|(new, _)| {
                let new_text = if new.theme.font == self.config.theme.font
                    && new.theme.font_size == self.config.theme.font_size
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
                    self.close_all(shell)?;
                    self.bars
                        .sync(shell, &new.bar)
                        .context("creating the bars")?;
                }
                self.tick = Tick::of(&new.clock.format);
                self.next_tick = next_tick(self.tick);
                self.config = new;
                self.redraw_all();
                tracing::info!("reload: config reloaded");
                incoming.reply(control::OK);
            }
        }
        Ok(())
    }

    /// `lyr dnd` and `lyr clear`, which need lyr to be the notification
    /// daemon. Returns the reply.
    #[cfg(target_os = "linux")]
    fn notifications(&mut self, request: Request) -> String {
        let Some(store) = &self.data.notifications else {
            return "error: lyr is not the notification daemon".to_owned();
        };
        if request == Request::Dnd {
            let mut store = store.lock().unwrap_or_else(PoisonError::into_inner);
            store.dnd = !store.dnd;
            tracing::info!("do-not-disturb {}", if store.dnd { "on" } else { "off" });
        } else {
            self.commands.clear_notifications();
            self.toasts.clear();
        }
        self.redraw_all();
        control::OK.to_owned()
    }

    /// Windows and macOS keep their own notification centres.
    #[cfg(not(target_os = "linux"))]
    fn notifications(&mut self, _request: Request) -> String {
        "error: lyr is not the notification daemon on this platform".to_owned()
    }
}
