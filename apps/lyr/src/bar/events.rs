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
use crate::bluetooth_menu::Outcome as BluetoothOutcome;
use crate::clock_panel::{Contents, Outcome as ClockOutcome};
use crate::control;
use crate::input::{self, Pointer};
use crate::list_popup::Outcome;
use crate::power_menu::PowerMenu;
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
            let bar = self.config.bar.clone();
            self.keep_bars(shell, &bar, Some(event))
                .context("keeping a bar on every output")?;
        }
        if self
            .bluetooth_menu
            .as_ref()
            .is_some_and(|menu| event.window().is_some_and(|window| menu.owns(window)))
        {
            return self.bluetooth_event(shell, event);
        }
        if self
            .wifi_menu
            .as_ref()
            .is_some_and(|menu| event.window().is_some_and(|window| menu.owns(window)))
        {
            return self.wifi_event(shell, event);
        }
        if self
            .audio_menu
            .as_ref()
            .is_some_and(|menu| event.window().is_some_and(|window| menu.owns(window)))
        {
            return self.audio_event(shell, event);
        }
        if self
            .tray_menu
            .as_ref()
            .is_some_and(|menu| event.window().is_some_and(|window| menu.owns(window)))
        {
            return self.tray_menu_event(shell, event);
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
                let height = size.height as f32 / scale;
                let info = self
                    .bars
                    .windows()
                    .find(|&(_, bar)| bar == window)
                    .and_then(|(monitor, _)| shell.monitor(monitor));
                // Unknown, a menu beside it is not held short of the far edge.
                let output_height = info.map_or(f32::INFINITY, |info| {
                    (f64::from(info.bounds.height) / info.scale_factor) as f32
                });
                let monitor = info.map(|info| info.name.clone());
                if let Some(open) = self.windows.get_mut(&window) {
                    open.surface.resize(device, scale)?;
                    open.width = width;
                    open.height = height;
                    open.output_height = output_height;
                    open.scale = scale;
                    open.dirty = true;
                    open.shown = None;
                } else {
                    let surface = Surface::open(shell, window, device, scale)
                        .context("opening a bar's surface")?;
                    tracing::info!("drawing on {}", surface.describe());
                    self.windows.insert(
                        window,
                        Window {
                            surface,
                            width,
                            height,
                            output_height,
                            scale,
                            monitor,
                            pointer: Pointer::default(),
                            regions: Regions::default(),
                            workspaces: Vec::new(),
                            hovered: None,
                            dirty: true,
                            shown: None,
                            tooltips: Tooltips::default(),
                        },
                    );
                }
            }
            // A bar the window system destroyed without asking.
            ShellEvent::WindowDestroyed { window } => self.drop_bar(shell, window, true)?,
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
                // Where the clock panel is to open; a second press in the
                // batch cancels the first.
                let mut toggle_panel = None;
                let mut bluetooth_at = None;
                let mut wifi_at = None;
                let mut audio_at = None;
                let mut tray_press = None;
                for press in presses {
                    let Some(target) = window.pointer.target(&window.regions) else {
                        continue;
                    };
                    // A tray button is its item's, not a binding's.
                    if let lyr_core::draw::Target::Button(lyr_config::Module::Tray, index) = *target
                        && let Some(rect) = window.regions.rect(target)
                    {
                        tray_press = Some((rect, index, press));
                        continue;
                    }
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
                        toggle_panel = match toggle_panel {
                            Some(_) => None,
                            None => window.place_under(target, self.config.bar.position),
                        };
                        continue;
                    }
                    if opens == Some(Builtin::OpenBluetoothMenu) {
                        bluetooth_at = window.place_under(target, self.config.bar.position);
                        continue;
                    }
                    if opens == Some(Builtin::OpenAudioMenu) {
                        // The mic's opens on the input devices, the
                        // volume's on the output devices.
                        let tab = if matches!(
                            target,
                            lyr_core::draw::Target::Module(lyr_config::Module::Mic)
                        ) {
                            lyr_core::audio_menu::Tab::Input
                        } else {
                            lyr_core::audio_menu::Tab::Output
                        };
                        audio_at = window
                            .place_under(target, self.config.bar.position)
                            .map(|place| (place, tab));
                        continue;
                    }
                    if opens == Some(Builtin::OpenWifiMenu) {
                        wifi_at = window.place_under(target, self.config.bar.position);
                        continue;
                    }
                    if opens != Some(Builtin::OpenPowerMenu) {
                        continue;
                    }
                    // A second click closes it.
                    if let Some(menu) = self.power_menu.take() {
                        menu.close(shell, false)?;
                    } else if self.menu_dismissed_at != Some(instant)
                        && let Some(anchor) = window.popup_anchor(target, self.config.bar.position)
                    {
                        let look = Look {
                            theme: &self.config.theme,
                            measure: &|s: &str| self.text.measure(s),
                            measure_in: &|s: &str, style| self.text.measure_in(s, style),
                            lay_out: &|spans, width| self.text.lay_out(spans, width),
                            line_height: self.text.line_height(),
                            calendar_line_height: self.text.calendar_line_height(),
                        };
                        self.power_menu = PowerMenu::open(
                            shell,
                            id,
                            anchor,
                            self.config.bar.position,
                            power_menu::rows(&power_menu::entries(Platform::current())),
                            &look,
                        )
                        .context("opening the power menu")?;
                    }
                }
                if let Some(place) = toggle_panel {
                    self.toggle_clock_panel(shell, place)?;
                }
                if let Some(place) = bluetooth_at
                    && let Err(why) = self.toggle_bluetooth_menu(shell, place)?
                {
                    tracing::warn!("open_bluetooth_menu: {why}");
                }
                if let Some((place, tab)) = audio_at
                    && let Err(why) = self.toggle_audio_menu(shell, place, tab)?
                {
                    tracing::warn!("open_audio_menu: {why}");
                }
                if let Some((rect, index, press)) = tray_press {
                    self.tray_press(id, rect, index, press);
                }
                if let Some(place) = wifi_at
                    && let Err(why) = self.toggle_wifi_menu(shell, place, instant)?
                {
                    tracing::warn!("open_wifi_menu: {why}");
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

    /// Applies `event`, which is for the Bluetooth menu's window.
    fn bluetooth_event(&mut self, shell: &mut dyn Shell, event: &ShellEvent) -> anyhow::Result<()> {
        let Some(menu) = &mut self.bluetooth_menu else {
            return Ok(());
        };
        if let ShellEvent::WindowDestroyed { .. } = event {
            if let Some(menu) = self.bluetooth_menu.take() {
                menu.close(shell, true)?;
            }
            return Ok(());
        }
        match menu.event(shell, event)? {
            BluetoothOutcome::Kept => {}
            outcome @ (BluetoothOutcome::Close | BluetoothOutcome::Closed) => {
                if let Some(menu) = self.bluetooth_menu.take() {
                    menu.close(shell, outcome == BluetoothOutcome::Closed)?;
                }
            }
            BluetoothOutcome::Act(target) => self.bluetooth_act(target),
            BluetoothOutcome::Answer(reply) => {
                self.bluetooth_answer(reply);
                // The question goes from the menu.
                self.refresh_bluetooth_menu(shell)?;
            }
            BluetoothOutcome::Rename(device, name) => {
                self.bluetooth_rename(device, name);
                // The name's field goes from the menu.
                self.refresh_bluetooth_menu(shell)?;
            }
            BluetoothOutcome::Redraw => self.refresh_bluetooth_menu(shell)?,
        }
        Ok(())
    }

    /// Carries out a click in the Bluetooth menu.
    #[cfg(any(target_os = "linux", windows))]
    fn bluetooth_act(&mut self, target: lyr_core::bluetooth_menu::Target) {
        use lyr_core::bluetooth_menu::{Switch, Target, order};

        use crate::sources::BluetoothRequest;
        let Some(adapter) = &self.data.bluetooth_adapter else {
            return;
        };
        let devices = order(&self.data.bluetooth_devices);
        let request = match target {
            Target::Answer(_) => return,
            Target::Switch(switch) => {
                let on = match switch {
                    Switch::Powered => adapter.powered,
                    Switch::Discoverable => adapter.discoverable,
                    Switch::Pairable => adapter.pairable,
                };
                BluetoothRequest::Switch(adapter.path.clone(), switch, !on)
            }
            Target::Scan => BluetoothRequest::Scan(adapter.path.clone(), !adapter.discovering),
            Target::Device(index, action) => {
                let Some(device) = devices.get(index) else {
                    return;
                };
                BluetoothRequest::Device(device.path.clone(), action)
            }
            Target::Rename(_) => return,
            // Where Windows connects a paired device; BlueZ's menu draws no
            // Settings button.
            Target::Settings => {
                #[cfg(windows)]
                crate::settings::open(crate::settings::Page::Bluetooth);
                return;
            }
            Target::Forget(index) => {
                let Some(device) = devices.get(index) else {
                    return;
                };
                BluetoothRequest::Forget(adapter.path.clone(), device.path.clone())
            }
        };
        self.commands.bluetooth(request);
    }

    /// Answers the pairing agent's question.
    #[cfg(any(target_os = "linux", windows))]
    fn bluetooth_answer(&mut self, reply: lyr_core::bluetooth_menu::Reply) {
        if let Some((id, _, _)) = self.data.bluetooth_ask.take() {
            self.commands
                .bluetooth(crate::sources::BluetoothRequest::Answer(id, reply));
        }
    }

    /// Gives the device at `device` the name `name` (BlueZ's own name back
    /// when it is empty).
    #[cfg(target_os = "linux")]
    fn bluetooth_rename(&self, device: String, name: String) {
        self.commands
            .bluetooth(crate::sources::BluetoothRequest::Rename(device, name));
    }

    /// macOS: the menu has no adapter to open on.
    #[cfg(target_os = "macos")]
    fn bluetooth_act(&mut self, _target: lyr_core::bluetooth_menu::Target) {}

    /// Windows and macOS: there is no device to rename (Windows draws no
    /// Rename button).
    #[cfg(not(target_os = "linux"))]
    fn bluetooth_rename(&self, _device: String, _name: String) {}

    /// macOS: there is no pairing agent to answer.
    #[cfg(target_os = "macos")]
    fn bluetooth_answer(&mut self, _reply: lyr_core::bluetooth_menu::Reply) {}

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
            measure_in: &|s: &str, style| self.text.measure_in(s, style),
            lay_out: &|spans, width| self.text.lay_out(spans, width),
            line_height: self.text.line_height(),
            calendar_line_height: self.text.calendar_line_height(),
        };
        let store = self.data.notification_store().map(|store| {
            store
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
        });
        let contents = Contents {
            today: chrono::Local::now().date_naive(),
            store: store.as_deref(),
            players: &self.data.players,
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
            (Target::Media(index, control), Press::Left) => {
                if let Some(player) = self.data.players.get(index) {
                    self.commands.media(player.bus_name.clone(), control);
                }
                return;
            }
            (Target::Action(id, index), Press::Left) => {
                self.toast_clicked(Some(Clicked::Action(id, index)));
                return;
            }
            (Target::Link(id, index), Press::Left) => {
                self.toast_clicked(Some(Clicked::Link(id, index)));
                return;
            }
            (Target::Close(id), Press::Left)
            | (
                Target::Notification(id)
                | Target::Action(id, _)
                | Target::Link(id, _)
                | Target::Close(id),
                Press::Middle,
            ) => {
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
            measure_in: &|s: &str, style| self.text.measure_in(s, style),
            lay_out: &|spans, width| self.text.lay_out(spans, width),
            line_height: self.text.line_height(),
            calendar_line_height: self.text.calendar_line_height(),
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
    /// another action, opening a link, or dismissing it.
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
            Some(Clicked::Link(id, index)) => {
                let href = self.toasts.store().and_then(|store| {
                    let store = store
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    lyr_core::markup::links(&store.get(id)?.content.body)
                        .into_iter()
                        .nth(index)
                });
                match href {
                    Some(href) if lyr_core::markup::openable(&href) => {
                        if let Err(error) = crate::exec::open_url(&href) {
                            tracing::warn!("notifications: opening {href}: {error}");
                        }
                    }
                    Some(href) => {
                        tracing::info!("notifications: not opening {href}: not http or https")
                    }
                    None => {}
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
