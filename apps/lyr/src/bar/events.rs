//! What the running bar does with a window-system event: keeping a bar on
//! every output, sizing bars and tooltips, and the pointer.

use std::time::Instant;

use anyhow::Context as _;
use crcbl_shell::{Shell, ShellEvent};
use lyr_core::hit::Regions;
use lyr_render::surface::Surface;

use lyr_config::{Builtin, Platform};
use lyr_core::centre::Target;
use lyr_core::control::Request;
use lyr_core::pointer::Press;
use lyr_core::power_menu;

use super::App;
use super::window::Window;
use crate::clock_panel::{Contents, Outcome as ClockOutcome};
use crate::control;
use crate::input::{self, Pointer};
use crate::power_menu::{Outcome, PowerMenu};
use crate::toasts::Clicked;
use crate::tooltips::{Look, Tooltips};

impl App {
    /// Applies `event`, which arrived at `instant`.
    pub(super) fn event(
        &mut self,
        shell: &mut dyn Shell,
        event: &ShellEvent,
        instant: Instant,
    ) -> anyhow::Result<()> {
        if !self.hidden {
            self.bars
                .handle(shell, &self.config.bar, event)
                .context("keeping a bar on every output")?;
        }
        if self
            .clock_panel
            .as_ref()
            .is_some_and(|panel| event.window().is_some_and(|window| panel.owns(window)))
        {
            return self.panel_event(shell, event);
        }
        if event
            .window()
            .is_some_and(|window| self.toasts.owns(window))
        {
            let clicked = self.toasts.event(shell, event, instant)?;
            self.toast_clicked(clicked);
            return Ok(());
        }
        if self
            .power_menu
            .as_ref()
            .is_some_and(|menu| event.window().is_some_and(|window| menu.owns(window)))
        {
            return self.menu_event(shell, event, instant);
        }
        // A tooltip's own window: sized, or gone with its bar.
        if let Some(owner) = event.window().and_then(|popup| {
            self.windows
                .values_mut()
                .find(|window| window.tooltips.owns(popup))
        }) {
            match *event {
                ShellEvent::Resized {
                    size, scale_factor, ..
                }
                | ShellEvent::ScaleFactorChanged {
                    size, scale_factor, ..
                } => owner
                    .tooltips
                    .resized(shell, (size.width, size.height), scale_factor as f32)
                    .context("opening a tooltip's surface")?,
                ShellEvent::WindowDestroyed { .. } => owner.tooltips.destroyed()?,
                _ => {}
            }
            return Ok(());
        }
        match *event {
            ShellEvent::Resized {
                window,
                size,
                scale_factor,
            }
            | ShellEvent::ScaleFactorChanged {
                window,
                size,
                scale_factor,
            } if self.bars.contains(window) => {
                let device = (size.width, size.height);
                let scale = scale_factor as f32;
                let width = size.width as f32 / scale;
                if let Some(open) = self.windows.get_mut(&window) {
                    open.surface.resize(device, scale)?;
                    open.width = width;
                    open.scale = scale;
                    open.dirty = true;
                } else {
                    let surface = Surface::open(shell, window, device, scale)
                        .context("opening a bar's surface")?;
                    tracing::info!("drawing on {}", surface.describe());
                    let monitor = self
                        .bars
                        .windows()
                        .find(|&(_, bar)| bar == window)
                        .and_then(|(monitor, _)| shell.monitor(monitor))
                        .map(|info| info.name.clone());
                    self.windows.insert(
                        window,
                        Window {
                            surface,
                            width,
                            scale,
                            monitor,
                            pointer: Pointer::default(),
                            regions: Regions::default(),
                            workspaces: Vec::new(),
                            hovered: None,
                            dirty: true,
                            tooltips: Tooltips::default(),
                        },
                    );
                }
            }
            ShellEvent::WindowDestroyed { window } => {
                if let Some(mut gone) = self.windows.remove(&window) {
                    // Its tooltip's and menu's windows went with it.
                    gone.tooltips.destroyed()?;
                    gone.surface.destroy()?;
                    if let Some(menu) = self.power_menu.take_if(|menu| menu.opened_from(window)) {
                        menu.close(shell, true)?;
                    }
                }
            }
            _ => {
                let Some((id, window)) = event
                    .window()
                    .and_then(|id| Some((id, self.windows.get_mut(&id)?)))
                else {
                    return Ok(());
                };
                let presses = window.pointer.handle(event, window.scale);
                if window.hovered() != window.hovered {
                    window.dirty = true;
                }
                window
                    .tooltips
                    .hover(window.pointer.target(&window.regions), instant);
                if !presses.is_empty() {
                    window.tooltips.press(instant);
                }
                // Applied once the bar is no longer borrowed.
                let mut toggle_dnd = false;
                let mut toggle_panel = false;
                for press in presses {
                    let Some(target) = window.pointer.target(&window.regions) else {
                        continue;
                    };
                    let opens = input::act(
                        target,
                        press,
                        &self.config,
                        &self.data,
                        &window.workspaces,
                        window.monitor.as_deref(),
                        &self.commands,
                    );
                    if opens == Some(Builtin::ToggleDnd) {
                        toggle_dnd = !toggle_dnd;
                        continue;
                    }
                    if opens == Some(Builtin::OpenClockPanel) {
                        toggle_panel = !toggle_panel;
                        continue;
                    }
                    if opens != Some(Builtin::OpenPowerMenu) {
                        continue;
                    }
                    // A second click closes it.
                    if let Some(menu) = self.power_menu.take() {
                        menu.close(shell, false)?;
                    } else if self.menu_dismissed_at != Some(instant)
                        && let Some(anchor) = window.regions.rect(target)
                    {
                        let look = Look {
                            theme: &self.config.theme,
                            measure: &|s: &str| self.text.measure(s),
                            line_height: self.text.line_height(),
                        };
                        self.power_menu = PowerMenu::open(
                            shell,
                            id,
                            anchor,
                            self.config.bar.position,
                            power_menu::entries(Platform::current()),
                            &look,
                        )
                        .context("opening the power menu")?;
                    }
                }
                if toggle_panel {
                    self.toggle_clock_panel(shell)?;
                }
                if toggle_dnd {
                    let reply = self.notifications(Request::Dnd);
                    if reply != control::OK {
                        tracing::warn!("toggle_dnd: {reply}");
                    }
                }
            }
        }
        Ok(())
    }

    /// Applies `event`, which is for the clock panel's window.
    fn panel_event(&mut self, shell: &mut dyn Shell, event: &ShellEvent) -> anyhow::Result<()> {
        let Some(panel) = &mut self.clock_panel else {
            return Ok(());
        };
        if let ShellEvent::WindowDestroyed { .. } = event {
            if let Some(panel) = self.clock_panel.take() {
                panel.close(shell, true)?;
            }
            return Ok(());
        }
        let look = Look {
            theme: &self.config.theme,
            measure: &|s: &str| self.text.measure(s),
            line_height: self.text.line_height(),
        };
        let store = self.data.notification_store().map(|store| {
            store
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
        });
        let contents = Contents {
            today: chrono::Local::now().date_naive(),
            store: store.as_deref(),
            now: Instant::now(),
        };
        let outcome = panel.event(shell, event, &look, &contents)?;
        drop(store);
        match outcome {
            ClockOutcome::Kept => {}
            ClockOutcome::Close | ClockOutcome::Closed => {
                if let Some(panel) = self.clock_panel.take() {
                    panel.close(shell, outcome == ClockOutcome::Closed)?;
                }
            }
            ClockOutcome::Centre(target, press) => self.centre_clicked(target, press),
        }
        Ok(())
    }

    /// Carries out a click in the clock panel's notification centre: Clear
    /// All and the switch as `lyr clear` and `lyr dnd` do, and an entry as a
    /// toast does.
    fn centre_clicked(&mut self, target: Target, press: Press) {
        let reply = match (target, press) {
            (Target::ClearAll, Press::Left) => self.notifications(Request::Clear),
            (Target::Dnd, Press::Left) => self.notifications(Request::Dnd),
            (Target::Notification(id), Press::Left) => {
                self.toast_clicked(Some(Clicked::Invoke(id)));
                return;
            }
            (Target::Action(id, index), Press::Left) => {
                self.toast_clicked(Some(Clicked::Action(id, index)));
                return;
            }
            (Target::Notification(id) | Target::Action(id, _), Press::Middle) => {
                self.toast_clicked(Some(Clicked::Dismiss(id)));
                return;
            }
            _ => return,
        };
        if reply != control::OK {
            tracing::warn!("clock panel: {reply}");
        }
    }

    /// Applies `event`, which is for the power menu's window.
    fn menu_event(
        &mut self,
        shell: &mut dyn Shell,
        event: &ShellEvent,
        instant: Instant,
    ) -> anyhow::Result<()> {
        let Some(menu) = &mut self.power_menu else {
            return Ok(());
        };
        if let ShellEvent::WindowDestroyed { .. } = event {
            if let Some(menu) = self.power_menu.take() {
                menu.close(shell, true)?;
            }
            return Ok(());
        }
        let look = Look {
            theme: &self.config.theme,
            measure: &|s: &str| self.text.measure(s),
            line_height: self.text.line_height(),
        };
        match menu.event(shell, event, &look)? {
            Outcome::Kept => {}
            Outcome::Chosen(entry) => {
                if let Some(menu) = self.power_menu.take() {
                    menu.close(shell, false)?;
                }
                match crate::power_menu::configured(&self.config.power.menu, entry) {
                    Some(command) => {
                        if let Err(error) = crate::exec::spawn(&command.exec) {
                            tracing::warn!("`{}` did not start: {error}", command.exec);
                        }
                    }
                    None => self.commands.power(entry),
                }
            }
            Outcome::Cancelled => {
                if let Some(menu) = self.power_menu.take() {
                    menu.close(shell, false)?;
                }
            }
            Outcome::Dismissed => {
                if let Some(menu) = self.power_menu.take() {
                    menu.close(shell, true)?;
                }
                self.menu_dismissed_at = Some(instant);
            }
        }
        Ok(())
    }

    /// Carries out a click on a toast: its `default` action when it has one,
    /// or dismissing it.
    #[cfg(target_os = "linux")]
    fn toast_clicked(&self, clicked: Option<Clicked>) {
        use lyr_core::notifications::CloseReason;
        use lyr_core::toast::DEFAULT_ACTION;
        // The action's key, as the notification's store holds it.
        let key = |id: u32, wanted: &dyn Fn(usize, &str) -> bool| {
            let store = self.toasts.store()?;
            let store = store
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            store
                .get(id)?
                .content
                .actions
                .iter()
                .enumerate()
                .find(|(index, (key, _))| wanted(*index, key))
                .map(|(_, (key, _))| key.clone())
        };
        match clicked {
            Some(Clicked::Invoke(id)) => {
                if let Some(key) = key(id, &|_, key| key == DEFAULT_ACTION) {
                    self.commands.invoke(id, key);
                }
            }
            Some(Clicked::Action(id, wanted)) => {
                if let Some(key) = key(id, &|index, _| index == wanted) {
                    self.commands.invoke(id, key);
                }
            }
            Some(Clicked::Dismiss(id)) => {
                self.commands.close_notification(id, CloseReason::Dismissed);
            }
            None => {}
        }
    }

    /// Windows and macOS have no toasts: lyr is not their notification
    /// daemon.
    #[cfg(not(target_os = "linux"))]
    fn toast_clicked(&self, _clicked: Option<Clicked>) {}
}
