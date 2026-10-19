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

mod events;
mod requests;
mod window;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::Context as _;
use chrono::Local;
use crcbl_shell::{Shell, WindowId};
use lyr_config::{Config, Platform};
use lyr_core::clock::Tick;
use lyr_render::text::Text;
use lyr_render::translate::translate;
use lyr_shell::bars::Bars;

use crate::control;
use crate::modules::{self, Data};
use crate::power_menu::PowerMenu;
use crate::sources::{self, Commands, Update};
use crate::stats::Stats;
use crate::toasts::Toasts;
use crate::tooltips::{Anchor, Look};
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
    /// The power menu, while it is open.
    power_menu: Option<PowerMenu>,
    /// The event batch that last dismissed the power menu: a click outside it
    /// that the bar also receives must not open it again at once.
    menu_dismissed_at: Option<Instant>,
    /// `lyr toggle` closes the bars rather than unmapping them, so their
    /// space is given back on every platform; while hidden, outputs coming
    /// and going make no bars.
    hidden: bool,
    stats: Stats,
}

/// Loads the theme's font, saying what was missing.
fn load_text(config: &Config) -> anyhow::Result<Text> {
    let font = lyr_render::fonts::load(&config.theme.font).context("loading the bar font")?;
    if let Some(family) = &font.missing {
        tracing::warn!("font `{family}` is not installed; using a monospace font");
    }
    for (path, reason) in &font.unreadable {
        tracing::warn!("fallback font {} skipped: {reason}", path.display());
    }
    Ok(Text::new(font.font, config.theme.font_size as f32))
}

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

    let mut bars = Bars::default();
    bars.sync(shell.as_mut(), &config.bar)
        .context("creating the bars")?;
    // Read once: a reload does not hand the daemon's name over.
    let (updates, commands) = sources::start(shell.waker(), config.notifications.replace)
        .context("starting the sources")?;
    let mut app = App {
        config_path: config_path.map(Path::to_path_buf),
        config,
        text,
        tick,
        next_tick: next_tick(tick),
        bars,
        windows: HashMap::new(),
        data: Data::default(),
        commands,
        toasts: Toasts::default(),
        power_menu: None,
        menu_dismissed_at: None,
        hidden: false,
        stats: Stats::new(Instant::now()),
    };

    loop {
        shell.wait_events(Some(app.wait()));
        app.stats.wakeups += 1;
        if log_stats {
            app.stats.log_if_due(Instant::now());
        }
        if Instant::now() >= app.next_tick {
            app.next_tick = next_tick(app.tick);
            app.redraw_all();
        }
        let mut quit = false;
        while let Ok(incoming) = requests.try_recv() {
            quit |= app.request(shell.as_mut(), incoming)?;
        }
        if quit {
            app.toasts.close(shell.as_mut())?;
            return app.close_all(shell.as_mut());
        }
        while let Ok(update) = updates.try_recv() {
            app.update(update, Instant::now());
            app.stats.events += 1;
            app.redraw_all();
        }
        let mut events = Vec::new();
        shell.pump(&mut |event| events.push(event));
        app.stats.events += events.len() as u64;
        let instant = Instant::now();
        for event in &events {
            app.event(shell.as_mut(), event, instant)?;
        }
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
            .any(|window| window.dirty || window.tooltips.dirty())
            || self.power_menu.as_ref().is_some_and(PowerMenu::dirty)
            || self.toasts.dirty()
        {
            return Duration::ZERO;
        }
        let wake = self
            .windows
            .values()
            .filter_map(|window| window.tooltips.next_wake())
            .chain(self.toasts.next_wake())
            .fold(self.next_tick, Instant::min);
        wake.saturating_duration_since(Instant::now())
    }

    /// Applies a source's update.
    fn update(&mut self, update: Update, now: Instant) {
        #[cfg(target_os = "linux")]
        {
            use lyr_sources::notifications::linux::Change;
            match &update {
                Update::Notifications(store) => self.toasts.serve(store.clone()),
                Update::Notification(Change::Notified(id)) => self.toasts.notified(*id, now),
                Update::Notification(Change::Closed(id, _)) => self.toasts.closed(*id),
                _ => {}
            }
        }
        #[cfg(not(target_os = "linux"))]
        let _ = now;
        self.data.apply(update);
    }

    /// Marks every bar to be drawn again.
    fn redraw_all(&mut self) {
        self.windows
            .values_mut()
            .for_each(|window| window.dirty = true);
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
            ..
        } = self;
        let now = Local::now().fixed_offset();
        let look = Look {
            theme: &config.theme,
            measure: &|s: &str| text.measure(s),
            line_height: text.line_height(),
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
                |module| modules::tooltip(module, config, data, &now),
                Instant::now(),
            )?;
            window.tooltips.draw(text)?;
        }
        if let Some(menu) = power_menu {
            menu.draw(text)?;
        }
        toasts.sync(shell, &look, Instant::now())?;
        toasts.draw(text)?;
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
            let presented = window.surface.draw(&translate(&drawn.list, text))?;
            window.regions = drawn.regions;
            window.workspaces = drawn.workspaces;
            window.hovered = hovered;
            window.dirty = !presented;
            if presented {
                stats.presented(window.monitor.as_deref());
            }
        }
        Ok(())
    }
}
