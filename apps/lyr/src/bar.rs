//! Running the bar: the main loop that keeps a bar on every output, draws it
//! and answers the pointer (`docs/plan/02-architecture.md`).
//!
//! It draws the modules that have data: the power button and the clock
//! everywhere, and on Linux the battery, power profile, Bluetooth and the
//! workspaces once their sources report ([`crate::sources`]). A module with no
//! data yet is left out rather than drawn empty.
//!
//! The loop's state is [`App`]; what it does with a control request is in
//! `requests`, with a window-system event in `events`, and one bar window's
//! own state is `window`.

mod audio;
mod events;
mod requests;
mod tray;
mod wifi;
mod window;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::PoisonError;
use std::time::{Duration, Instant};

use anyhow::Context as _;
use chrono::Local;
use crcbl_shell::{Shell, ShellEvent, WindowId};
use lyr_config::{Config, Platform};
use lyr_core::clock::Tick;
use lyr_render::text::Text;
use lyr_shell::bars::Bars;

use crate::audio_menu::AudioMenu;
use crate::bluetooth_menu::BluetoothMenu;
use crate::clock_panel::{ClockPanel, Contents};
use crate::control;
use crate::menu_panel::Place;
use crate::modules::{self, Data};
use crate::power_menu::PowerMenu;
use crate::sources::{self, Commands, Update};
use crate::stats::Stats;
use crate::toasts::Toasts;
use crate::tooltips::{Anchor, Look};
use crate::tray_menu::TrayMenu;
use crate::wifi_menu::WifiMenu;
use window::Window;

/// Everything the main loop keeps between wake-ups.
struct App {
    config_path: Option<PathBuf>,
    config: Config,
    text: Text,
    tick: Tick,
    /// When the clock's text next changes, so every bar is redrawn then.
    next_tick: Instant,
    bars: Bars,
    windows: HashMap<WindowId, Window>,
    data: Data,
    commands: Commands,
    toasts: Toasts,
    /// The clock panel, while it is open.
    clock_panel: Option<ClockPanel>,
    /// The Bluetooth menu, while it is open.
    bluetooth_menu: Option<BluetoothMenu>,
    /// The Wi-Fi menu, while it is open.
    wifi_menu: Option<WifiMenu>,
    /// The audio menu, while it is open.
    audio_menu: Option<AudioMenu>,
    /// The power menu, while it is open.
    power_menu: Option<PowerMenu>,
    /// A tray item's menu, while it is open.
    tray_menu: Option<TrayMenu>,
    /// The event batch that last dismissed the power menu: a click outside it
    /// that the bar also receives must not open it again at once.
    menu_dismissed_at: Option<Instant>,
    /// `lyr toggle` closes the bars rather than unmapping them, so their
    /// space is given back on every platform; while hidden, outputs coming
    /// and going make no bars.
    hidden: bool,
    stats: Stats,
}

/// Loads the theme's font and the calendar's, saying what was missing.
fn load_text(config: &Config) -> anyhow::Result<Text> {
    let size = config.theme.font_size as f32;
    let bar = load_family(&config.theme.font).context("loading the bar font")?;
    let calendar = match &config.theme.calendar_font {
        Some(family) if *family != config.theme.font => {
            load_family(family).context("loading the calendar font")?
        }
        _ => bar.clone(),
    };
    let calendar =
        Text::new(calendar.font, size * lyr_render::text::SMALL).with_faces(calendar.faces);
    Ok(Text::new(bar.font, size)
        .with_faces(bar.faces)
        .with_calendar(calendar))
}

/// Loads the font `family`, logging what was missing.
fn load_family(family: &str) -> anyhow::Result<lyr_render::fonts::Loaded> {
    let font = lyr_render::fonts::load(family)?;
    if let Some(family) = &font.missing {
        tracing::warn!("font `{family}` is not installed; using a monospace font");
    }
    for (path, reason) in &font.unreadable {
        tracing::warn!("font {} skipped: {reason}", path.display());
    }
    Ok(font)
}

/// How long after a time zone change the clock is drawn again: chrono
/// 0.4.45's `Local` reads `/etc/localtime` again when its mtime changed, but
/// looks at most once a second (`src/offset/local/unix.rs`), so a draw at
/// the change itself can still show the old zone.
#[cfg(target_os = "linux")]
const ZONE_SETTLED: std::time::Duration = std::time::Duration::from_millis(1100);

/// When the clock's text next changes under `tick`.
fn next_tick(tick: Tick) -> Instant {
    Instant::now() + tick.until_next(&Local::now().fixed_offset())
}

/// Runs the bar until `lyr quit` stops it, logging [`Stats`] once a minute
/// when `log_stats` is set.
pub fn run(config_path: Option<&Path>, log_stats: bool) -> anyhow::Result<()> {
    let (config, _) =
        lyr_config::load(config_path, Platform::current()).context("loading the config")?;
    let mut shell = crcbl_shell::open().context("opening the window system")?;
    // Bound before anything else is loaded or drawn, so a second lyr stops
    // here.
    let requests = control::listen(control::path()?, shell.waker())?;

    let text = load_text(&config)?;
    let tick = Tick::of(&config.clock.format);
    crate::gpu::remember();

    // Read once: a reload does not hand the daemon's name over.
    let (updates, commands) = sources::start(shell.waker(), config.notifications.replace)
        .context("starting the sources")?;
    let mut app = App {
        config_path: config_path.map(Path::to_path_buf),
        config,
        text,
        tick,
        next_tick: next_tick(tick),
        bars: Bars::default(),
        windows: HashMap::new(),
        data: Data::default(),
        commands,
        toasts: Toasts::default(),
        clock_panel: None,
        bluetooth_menu: None,
        wifi_menu: None,
        audio_menu: None,
        power_menu: None,
        tray_menu: None,
        menu_dismissed_at: None,
        hidden: false,
        stats: Stats::new(Instant::now()),
    };
    let bar = app.config.bar.clone();
    app.keep_bars(shell.as_mut(), &bar, None)
        .context("creating the bars")?;

    loop {
        shell.wait_events(Some(app.wait()));
        let woke = Instant::now();
        app.stats.wakeups += 1;
        if log_stats {
            app.stats.log_if_due(Instant::now());
        }
        if app.wifi_wake().is_some_and(|wake| Instant::now() >= wake) {
            // The wait for a scan is up.
            app.refresh_wifi_menu(shell.as_mut())?;
        }
        if Instant::now() >= app.next_tick {
            app.stats.changed(woke);
            app.next_tick = next_tick(app.tick);
            app.redraw_all();
            // Its calendar's today and its history's "2 mins ago" move on.
            app.refresh_panel();
        }
        let mut quit = false;
        while let Ok(incoming) = requests.try_recv() {
            quit |= app.request(shell.as_mut(), incoming)?;
        }
        if quit {
            app.toasts.close(shell.as_mut())?;
            app.close_menus(shell.as_mut())?;
            return app.close_all(shell.as_mut());
        }
        while let Ok(update) = updates.try_recv() {
            #[cfg(target_os = "linux")]
            if let Update::TrayMenu(read) = update {
                app.open_tray_menu(shell.as_mut(), read)?;
                continue;
            }
            #[cfg(target_os = "linux")]
            if let Update::TrayMenuLaidOut(bus_name, menu, root) = update {
                app.tray_menu_laid_out(shell.as_mut(), (&bus_name, &menu), root)?;
                continue;
            }
            app.update(update, Instant::now());
            app.refresh_bluetooth_menu(shell.as_mut())?;
            app.refresh_wifi_menu(shell.as_mut())?;
            app.refresh_audio_menu(shell.as_mut())?;
            app.stats.events += 1;
            app.stats.changed(woke);
            app.redraw_all();
        }
        let mut events = Vec::new();
        shell.pump(&mut |event| events.push(event));
        app.stats.events += events.len() as u64;
        if !events.is_empty() {
            app.stats.changed(woke);
        }
        let events = crate::keys::settled_focus(events);
        let instant = Instant::now();
        for event in &events {
            app.observe(event);
            app.event(shell.as_mut(), event, instant)?;
        }
        // The source follows the streams while the menu is open.
        app.commands.audio_menu(app.audio_menu.is_some());
        #[cfg(target_os = "linux")]
        app.commands
            .tray_menu(app.tray_menu.as_ref().map(TrayMenu::item));
        #[cfg(any(target_os = "linux", windows))]
        app.commands.wifi_menu(app.wifi_menu.is_some());
        app.draw(shell.as_mut())?;
    }
}

impl App {
    /// How long the loop may sleep. A bar or tooltip still to draw (a frame
    /// the swapchain swallowed) goes round again at once; otherwise the loop
    /// sleeps until the clock's next tick or a tooltip's delay is up.
    fn wait(&self) -> Duration {
        if self
            .windows
            .values()
            .any(|window| window.ready() || window.tooltips.ready())
            || self.power_menu.as_ref().is_some_and(PowerMenu::ready)
            || self.toasts.ready()
            || self.clock_panel.as_ref().is_some_and(ClockPanel::ready)
            || self
                .bluetooth_menu
                .as_ref()
                .is_some_and(BluetoothMenu::ready)
            || self.wifi_menu.as_ref().is_some_and(WifiMenu::ready)
            || self.audio_menu.as_ref().is_some_and(AudioMenu::ready)
            || self.tray_menu.as_ref().is_some_and(TrayMenu::ready)
        {
            return Duration::ZERO;
        }
        let wake = self
            .windows
            .values()
            .filter_map(|window| window.tooltips.next_wake())
            .chain(self.toasts.next_wake())
            .chain(self.wifi_wake())
            .fold(self.next_tick, Instant::min);
        wake.saturating_duration_since(Instant::now())
    }

    /// Hands one shell event to every surface, for the answer to its last
    /// frame's request.
    fn observe(&mut self, event: &ShellEvent) {
        for window in self.windows.values_mut() {
            window.surface.observe(event);
            window.tooltips.observe(event);
        }
        self.toasts.observe(event);
        if let Some(menu) = &mut self.power_menu {
            menu.observe(event);
        }
        if let Some(panel) = &mut self.clock_panel {
            panel.observe(event);
        }
        if let Some(menu) = &mut self.bluetooth_menu {
            menu.observe(event);
        }
        if let Some(menu) = &mut self.wifi_menu {
            menu.observe(event);
        }
        if let Some(menu) = &mut self.audio_menu {
            menu.observe(event);
        }
        if let Some(menu) = &mut self.tray_menu {
            menu.observe(event);
        }
    }

    /// Applies a source's update.
    fn update(&mut self, update: Update, now: Instant) {
        #[cfg(windows)]
        if let Update::WifiJoined(joined) = &update {
            self.wifi_joined(joined.clone());
        }
        #[cfg(target_os = "linux")]
        {
            use lyr_sources::notifications::linux::Change;
            match &update {
                Update::Notifications(store) => self.toasts.serve(store.clone()),
                Update::Notification(Change::Notified(id)) => {
                    self.toasts.notified(*id, self.clock_panel.is_some(), now);
                }
                Update::Notification(Change::Closed(id, _)) => self.toasts.closed(*id),
                Update::WifiJoined(joined) => self.wifi_joined(joined.clone()),
                // The tick comes early to draw the clock in the new zone.
                Update::Zone(_) => self.next_tick = self.next_tick.min(now + ZONE_SETTLED),
                _ => {}
            }
        }
        #[cfg(not(target_os = "linux"))]
        let _ = now;
        self.data.apply(update);
        self.refresh_panel();
    }

    /// Draws the clock panel again, if it is open, for the notifications and
    /// do-not-disturb as they are now.
    fn refresh_panel(&mut self) {
        let Some(panel) = &mut self.clock_panel else {
            return;
        };
        let look = Look {
            theme: &self.config.theme,
            measure: &|s: &str| self.text.measure(s),
            measure_in: &|s: &str, style| self.text.measure_in(s, style),
            lay_out: &|spans, width| self.text.lay_out(spans, width),
            line_height: self.text.line_height(),
            calendar_line_height: self.text.calendar_line_height(),
        };
        let store = self
            .data
            .notification_store()
            .map(|store| store.lock().unwrap_or_else(PoisonError::into_inner));
        let contents = Contents {
            today: Local::now().date_naive(),
            store: store.as_deref(),
            players: &self.data.players,
            now: Instant::now(),
        };
        panel.refresh(&look, &contents);
    }

    /// Opens the clock panel at `place`, or closes it if it is open.
    /// Opening it takes the toasts down, since it lists them.
    fn toggle_clock_panel(&mut self, shell: &mut dyn Shell, place: Place) -> anyhow::Result<()> {
        if let Some(panel) = self.clock_panel.take() {
            return panel.close(shell, false);
        }
        self.close_menus(shell)?;
        let look = Look {
            theme: &self.config.theme,
            measure: &|s: &str| self.text.measure(s),
            measure_in: &|s: &str, style| self.text.measure_in(s, style),
            lay_out: &|spans, width| self.text.lay_out(spans, width),
            line_height: self.text.line_height(),
            calendar_line_height: self.text.calendar_line_height(),
        };
        let today = Local::now().date_naive();
        self.clock_panel =
            Some(ClockPanel::open(shell, place, &look, today).context("opening the clock panel")?);
        self.toasts.take_down();
        Ok(())
    }

    /// Opens the Bluetooth menu at `place`, or
    /// closes it if it is open. Returns why it did not open: there is no
    /// adapter. Windows opens its Bluetooth Settings page instead.
    fn toggle_bluetooth_menu(
        &mut self,
        shell: &mut dyn Shell,
        place: Place,
    ) -> anyhow::Result<Result<(), &'static str>> {
        if let Some(menu) = self.bluetooth_menu.take() {
            menu.close(shell, false)?;
            return Ok(Ok(()));
        }
        if self.data.bluetooth_adapter.is_some() {
            self.close_menus(shell)?;
        }
        let Some(adapter) = &self.data.bluetooth_adapter else {
            // Windows and macOS have no Bluetooth menu yet (its actions are
            // BlueZ's, so macOS hands it no adapter): their Settings page
            // stands in.
            #[cfg(any(windows, target_os = "macos"))]
            {
                crate::settings::open(crate::settings::Page::Bluetooth);
                return Ok(Ok(()));
            }
            #[cfg(target_os = "linux")]
            return Ok(Err("there is no Bluetooth adapter"));
        };
        let look = Look {
            theme: &self.config.theme,
            measure: &|s: &str| self.text.measure(s),
            measure_in: &|s: &str, style| self.text.measure_in(s, style),
            lay_out: &|spans, width| self.text.lay_out(spans, width),
            line_height: self.text.line_height(),
            calendar_line_height: self.text.calendar_line_height(),
        };
        self.bluetooth_menu = Some(
            BluetoothMenu::open(
                shell,
                place,
                &look,
                adapter,
                &self.data.bluetooth_devices,
                self.data.bluetooth_question(),
            )
            .context("opening the Bluetooth menu")?,
        );
        Ok(Ok(()))
    }

    /// Draws the Bluetooth menu again, if it is open, for BlueZ as it is
    /// now; closes it if the adapter went. A question from the pairing agent
    /// opens it, since a question nobody sees times out.
    fn refresh_bluetooth_menu(&mut self, shell: &mut dyn Shell) -> anyhow::Result<()> {
        if self.bluetooth_menu.is_none() && self.data.bluetooth_ask.is_some() {
            if let Err(why) = self.toggle_bluetooth_menu(shell, Place::EDGE)? {
                tracing::warn!("bluetooth: a pairing question, but {why}");
            }
            return Ok(());
        }
        let Some(menu) = &mut self.bluetooth_menu else {
            return Ok(());
        };
        let Some(adapter) = &self.data.bluetooth_adapter else {
            if let Some(menu) = self.bluetooth_menu.take() {
                menu.close(shell, false)?;
            }
            return Ok(());
        };
        let look = Look {
            theme: &self.config.theme,
            measure: &|s: &str| self.text.measure(s),
            measure_in: &|s: &str, style| self.text.measure_in(s, style),
            lay_out: &|spans, width| self.text.lay_out(spans, width),
            line_height: self.text.line_height(),
            calendar_line_height: self.text.calendar_line_height(),
        };
        menu.refresh(
            shell,
            &look,
            adapter,
            &self.data.bluetooth_devices,
            self.data.bluetooth_question(),
        )
    }

    /// Marks every bar to be drawn again.
    fn redraw_all(&mut self) {
        self.windows
            .values_mut()
            .for_each(|window| window.dirty = true);
    }

    /// Closes every panel and popup that is open: the clock panel, the
    /// Bluetooth, Wi-Fi and audio menus, the power menu and a tray menu.
    /// Opening one closes the rest (`docs/plan/10-popups-and-menus.md`).
    fn close_menus(&mut self, shell: &mut dyn Shell) -> anyhow::Result<()> {
        if let Some(panel) = self.clock_panel.take() {
            panel.close(shell, false)?;
        }
        if let Some(menu) = self.bluetooth_menu.take() {
            menu.close(shell, false)?;
        }
        if let Some(menu) = self.wifi_menu.take() {
            menu.close(shell, false)?;
        }
        if let Some(menu) = self.audio_menu.take() {
            menu.close(shell, false)?;
        }
        if let Some(menu) = self.power_menu.take() {
            menu.close(shell, false)?;
        }
        if let Some(menu) = self.tray_menu.take() {
            menu.close(shell, false)?;
        }
        Ok(())
    }

    /// Closes the power menu and every bar: their tooltips and surfaces
    /// first, then their windows.
    fn close_all(&mut self, shell: &mut dyn Shell) -> anyhow::Result<()> {
        if let Some(menu) = self.power_menu.take() {
            menu.close(shell, false)?;
        }
        for (_, mut window) in self.windows.drain() {
            window.tooltips.close(shell)?;
            window.surface.destroy()?;
        }
        for (_, bar) in self.bars.windows().collect::<Vec<_>>() {
            shell.destroy_window(bar)?;
        }
        self.bars = Bars::default();
        Ok(())
    }

    /// Brings the bars in step with `bar` and the outputs, or with `event`
    /// when there is one ([`Bars::handle`]), each bar that goes taken down
    /// by [`App::drop_bar`] while its window is still open.
    fn keep_bars(
        &mut self,
        shell: &mut dyn Shell,
        bar: &lyr_config::Bar,
        event: Option<&ShellEvent>,
    ) -> anyhow::Result<()> {
        // Out of `self` while it runs, so the hook can borrow the rest.
        let mut bars = std::mem::take(&mut self.bars);
        let retire = |shell: &mut dyn Shell, window| self.drop_bar(shell, window, false);
        let kept = match event {
            Some(event) => bars.handle(shell, bar, event, retire),
            None => bars.sync(shell, bar, retire),
        };
        self.bars = bars;
        kept.map(drop)
    }

    /// Drops what draws into the bar `window`: its surface, its tooltip and
    /// the menus opened from it. Before its window closes, or after the
    /// window system destroyed it (`window_gone`), its popups with it.
    fn drop_bar(
        &mut self,
        shell: &mut dyn Shell,
        window: WindowId,
        window_gone: bool,
    ) -> anyhow::Result<()> {
        let Some(mut gone) = self.windows.remove(&window) else {
            return Ok(());
        };
        if window_gone {
            gone.tooltips.destroyed()?;
        } else {
            gone.tooltips.close(shell)?;
        }
        gone.surface.destroy()?;
        if let Some(menu) = self.power_menu.take_if(|menu| menu.opened_from(window)) {
            menu.close(shell, window_gone)?;
        }
        if let Some(menu) = self.tray_menu.take_if(|menu| menu.opened_from(window)) {
            menu.close(shell, window_gone)?;
        }
        Ok(())
    }

    /// Brings every tooltip up to date and draws every bar that changed.
    fn draw(&mut self, shell: &mut dyn Shell) -> anyhow::Result<()> {
        let Self {
            config,
            text,
            windows,
            data,
            stats,
            power_menu,
            toasts,
            commands,
            clock_panel,
            bluetooth_menu,
            wifi_menu,
            audio_menu,
            tray_menu,
            ..
        } = self;
        let now = Local::now().fixed_offset();
        let look = Look {
            theme: &config.theme,
            measure: &|s: &str| text.measure(s),
            measure_in: &|s: &str, style| text.measure_in(s, style),
            lay_out: &|spans, width| text.lay_out(spans, width),
            line_height: text.line_height(),
            calendar_line_height: text.calendar_line_height(),
        };
        for (&bar, window) in windows.iter_mut() {
            let anchor = Anchor {
                bar,
                regions: &window.regions,
                position: config.bar.position,
            };
            window.tooltips.sync(
                shell,
                &anchor,
                &look,
                |target| modules::tooltip(target, config, data, &now),
                Instant::now(),
            )?;
            window.tooltips.draw(shell, text)?;
        }
        if let Some(menu) = power_menu {
            menu.draw(shell, text)?;
        }
        if let Some(panel) = clock_panel {
            panel.draw(shell, text)?;
        }
        if let Some(menu) = bluetooth_menu {
            menu.draw(shell, text)?;
        }
        if let Some(menu) = wifi_menu {
            menu.draw(shell, text)?;
        }
        if let Some(menu) = audio_menu {
            menu.draw(shell, text)?;
        }
        if let Some(menu) = tray_menu {
            menu.draw(shell, text)?;
        }
        for id in toasts.expire(Instant::now()) {
            commands.close_notification(id, lyr_core::notifications::CloseReason::Expired);
        }
        toasts.sync(shell, &look, Instant::now())?;
        toasts.draw(shell, text)?;
        for window in windows.values_mut().filter(|window| window.dirty) {
            let hovered = window.hovered();
            let drawn = modules::draw(
                window.width,
                window.monitor.as_deref(),
                hovered,
                config,
                text,
                data,
                &now,
            );
            window.regions = drawn.regions;
            window.workspaces = drawn.workspaces;
            window.hovered = hovered;
            // A source that re-sent what it had, or one the bar does not
            // show, changes nothing on screen.
            if window.shown.as_ref() == Some(&drawn.list) {
                window.dirty = false;
                continue;
            }
            let presented = window.surface.draw_bar(shell, &drawn.list, text)?;
            window.dirty = !presented;
            if presented {
                window.shown = Some(drawn.list);
                stats.presented(window.monitor.as_deref(), Instant::now());
            }
        }
        stats.drawn(windows.values().any(|window| window.dirty));
        Ok(())
    }
}
