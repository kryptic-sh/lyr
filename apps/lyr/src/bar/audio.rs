//! What the running bar does for the audio menu: opening and closing it,
//! drawing it as the streams and devices change, and carrying out what it
//! asks.

use anyhow::Context as _;
use crcbl_shell::{Shell, ShellEvent};
use lyr_core::audio_menu::Tab;

use super::App;
use crate::audio_menu::{Action, AudioMenu, Outcome};
use crate::menu_panel::Place;
use crate::tooltips::Look;

impl App {
    /// Opens the audio menu on `tab` at `place`, or
    /// closes it if it is open. Returns why it did not open: there is no
    /// audio source on this platform yet.
    pub(super) fn toggle_audio_menu(
        &mut self,
        shell: &mut dyn Shell,
        place: Place,
        tab: Tab,
    ) -> anyhow::Result<Result<(), &'static str>> {
        if let Some(menu) = self.audio_menu.take() {
            menu.close(shell, false)?;
            return Ok(Ok(()));
        }
        let Some(nodes) = &self.data.audio_nodes else {
            return Ok(Err("there is no audio source"));
        };
        let look = Look {
            theme: &self.config.theme,
            measure: &|s: &str| self.text.measure(s),
            measure_in: &|s: &str, style| self.text.measure_in(s, style),
            line_height: self.text.line_height(),
            calendar_line_height: self.text.calendar_line_height(),
        };
        self.audio_menu = Some(
            AudioMenu::open(shell, place, &look, nodes, tab).context("opening the audio menu")?,
        );
        Ok(Ok(()))
    }

    /// Draws the audio menu again, if it is open, for the streams and
    /// devices as they are now.
    pub(super) fn refresh_audio_menu(&mut self, shell: &mut dyn Shell) -> anyhow::Result<()> {
        let (Some(menu), Some(nodes)) = (&mut self.audio_menu, &self.data.audio_nodes) else {
            return Ok(());
        };
        let look = Look {
            theme: &self.config.theme,
            measure: &|s: &str| self.text.measure(s),
            measure_in: &|s: &str, style| self.text.measure_in(s, style),
            line_height: self.text.line_height(),
            calendar_line_height: self.text.calendar_line_height(),
        };
        menu.refresh(shell, &look, nodes)
    }

    /// Applies `event`, which is for the audio menu's window.
    pub(super) fn audio_event(
        &mut self,
        shell: &mut dyn Shell,
        event: &ShellEvent,
    ) -> anyhow::Result<()> {
        let Some(menu) = &mut self.audio_menu else {
            return Ok(());
        };
        if let ShellEvent::WindowDestroyed { .. } = event {
            if let Some(menu) = self.audio_menu.take() {
                menu.close(shell, true)?;
            }
            return Ok(());
        }
        let look = Look {
            theme: &self.config.theme,
            measure: &|s: &str| self.text.measure(s),
            measure_in: &|s: &str, style| self.text.measure_in(s, style),
            line_height: self.text.line_height(),
            calendar_line_height: self.text.calendar_line_height(),
        };
        match menu.event(shell, event, &look)? {
            Outcome::Kept => Ok(()),
            outcome @ (Outcome::Close | Outcome::Closed) => {
                if let Some(menu) = self.audio_menu.take() {
                    menu.close(shell, outcome == Outcome::Closed)?;
                }
                Ok(())
            }
            Outcome::Act(tab, id, action) => {
                self.audio_act(tab, id, action);
                Ok(())
            }
            Outcome::Redraw => self.refresh_audio_menu(shell),
        }
    }

    /// macOS: carries out a click in the audio menu on a device: a device is
    /// muted, set, or stepped by the volume or mic module's `scroll_step`, as
    /// its flow is, and "make default" sets the default through CoreAudio.
    /// macOS lists no streams, so their tabs never ask.
    #[cfg(target_os = "macos")]
    fn audio_act(&self, tab: Tab, id: String, action: Action) {
        use lyr_core::audio_menu::Step;
        use lyr_sources::audio::macos::{Flow, Which};
        let flow = match tab {
            Tab::Output | Tab::Playback => Flow::Output,
            Tab::Input | Tab::Recording => Flow::Input,
            // macOS lists nothing to configure, so the menu never shows that
            // tab.
            Tab::Configuration => return,
        };
        let Ok(device) = id.parse::<u32>() else {
            tracing::warn!("audio menu: `{id}` is not a CoreAudio device");
            return;
        };
        let which = Which::Device(flow, device);
        let step = if flow == Flow::Input {
            self.config.mic.scroll_step
        } else {
            self.config.volume.scroll_step
        };
        let step = i32::try_from(step).unwrap_or(i32::MAX);
        match action {
            Action::Mute => self.commands.toggle_mute(which),
            Action::Step(Step::Up) => self.commands.step_volume(which, step),
            Action::Step(Step::Down) => self.commands.step_volume(which, -step),
            Action::Set(percent) => self.commands.set_volume(which, percent),
            Action::Default => self.commands.set_default(flow, device),
            // Its streams carry no device and it lists nothing to configure,
            // so the menu draws neither button.
            Action::Route(_) | Action::Setting(_) => {}
        }
    }

    /// Linux and Windows: carries out a click in the audio menu on a stream or
    /// device, by its id or name, through crcbl's mixer; steps go by the
    /// volume or mic module's `scroll_step`, as its flow is. What changed
    /// comes back from the source. Windows lists nothing to configure and no
    /// stream's device, so it draws neither a setting nor a route.
    #[cfg(any(target_os = "linux", windows))]
    fn audio_act(&self, tab: Tab, id: String, action: Action) {
        use lyr_core::audio_menu::Step;
        use lyr_sources::audio::mixer::{Flow, Target};
        let flow = match tab {
            Tab::Playback | Tab::Output => Flow::Output,
            Tab::Recording | Tab::Input => Flow::Input,
            Tab::Configuration => {
                // A row's only control is its choice: a card's profile or a
                // device's port, its kind as the source listed it.
                let kind = self
                    .data
                    .audio_nodes
                    .as_ref()
                    .and_then(|nodes| nodes.settings.as_ref())
                    .and_then(|settings| settings.iter().find(|setting| setting.id == id))
                    .map(|setting| setting.kind);
                if let (Action::Setting(option), Some(kind)) = (action, kind) {
                    self.commands.choose(kind, id, option);
                }
                return;
            }
        };
        let target = if tab.devices() {
            Target::Device(flow, id.clone())
        } else {
            let Ok(index) = id.parse() else {
                tracing::warn!("audio menu: `{id}` is not a stream's index");
                return;
            };
            Target::Stream(flow, index)
        };
        let step = if flow == Flow::Input {
            self.config.mic.scroll_step
        } else {
            self.config.volume.scroll_step
        };
        let step = i32::try_from(step).unwrap_or(i32::MAX);
        match action {
            Action::Mute => self.commands.toggle_mute(target),
            Action::Step(Step::Up) => self.commands.step_volume(target, step),
            Action::Step(Step::Down) => self.commands.step_volume(target, -step),
            Action::Set(percent) => self.commands.set_volume(target, percent),
            Action::Default => self.commands.set_default(flow, id),
            Action::Route(device) => {
                if let Target::Stream(_, index) = target {
                    self.commands.move_stream(flow, index, device);
                }
            }
            // On the configuration tab, handled above.
            Action::Setting(_) => {}
        }
    }
}
