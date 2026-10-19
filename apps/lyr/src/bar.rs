//! Running the bar: the main loop that keeps a bar on every output, draws it
//! and answers the pointer (`docs/plan/02-architecture.md`).
//!
//! It draws the modules that have data: the clock everywhere, and on Linux
//! the battery and the workspaces once their sources report
//! ([`crate::sources`]). A module with no data yet is left out rather than
//! drawn empty.

use std::collections::HashMap;
use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::Context as _;
use chrono::Local;
use crcbl_shell::{ShellEvent, WindowId};
use lyr_config::{Config, Module, Platform};
use lyr_core::clock::Tick;
use lyr_core::control::Request;
use lyr_core::draw::Target;
use lyr_core::hit::Regions;
use lyr_render::surface::Surface;
use lyr_render::text::Text;
use lyr_render::translate::translate;
use lyr_shell::bars::Bars;

use crate::control;
use crate::input::{self, Pointer};
use crate::modules::{self, Data};
use crate::sources;

/// One bar window: its surface, its size, and what it drew last.
struct Window {
    surface: Surface,
    /// Logical pixels.
    width: f32,
    /// Device pixels per logical pixel.
    scale: f32,
    /// The output it is on, by name, for a per-output workspace list.
    monitor: Option<String>,
    pointer: Pointer,
    /// The input regions of the last frame.
    regions: Regions<Target<Module>>,
    /// The workspaces the last frame's buttons stand for.
    workspaces: Vec<i64>,
    /// The workspace button the pointer was over when it was last drawn.
    hovered: Option<usize>,
    /// Whether it must be drawn again: what it shows changed since.
    dirty: bool,
}

impl Window {
    /// The workspace button the pointer is over now, by its index in the row.
    fn hovered(&self) -> Option<usize> {
        match self.pointer.target(&self.regions) {
            Some(Target::Button(Module::Workspaces, index)) => Some(*index),
            _ => None,
        }
    }
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

/// Closes every bar: their surfaces first, then their windows.
fn close_all(
    shell: &mut dyn crcbl_shell::Shell,
    bars: &mut Bars,
    windows: &mut HashMap<WindowId, Window>,
) -> anyhow::Result<()> {
    for (_, window) in windows.drain() {
        window.surface.destroy()?;
    }
    for (_, bar) in bars.windows().collect::<Vec<_>>() {
        shell.destroy_window(bar)?;
    }
    *bars = Bars::default();
    Ok(())
}

/// Runs the bar until `lyr quit` stops it.
pub fn run(config_path: Option<&Path>) -> anyhow::Result<()> {
    let (mut config, _) =
        lyr_config::load(config_path, Platform::current()).context("loading the config")?;
    let mut shell = crcbl_shell::open().context("opening the window system")?;
    // Bound before anything else is loaded or drawn, so a second lyr stops
    // here.
    let requests = control::listen(control::path()?, shell.waker())?;

    let mut text = load_text(&config)?;
    let mut tick = Tick::of(&config.clock.format);

    let mut bars = Bars::default();
    bars.sync(shell.as_mut(), &config.bar)
        .context("creating the bars")?;
    let mut windows: HashMap<WindowId, Window> = HashMap::new();
    let (updates, commands) = sources::start(shell.waker()).context("starting the sources")?;
    let mut data = Data::default();
    // `lyr toggle` closes the bars rather than unmapping them, so their
    // space is given back on every platform; while hidden, outputs coming
    // and going make no bars.
    let mut hidden = false;
    // When the clock's text next changes, so every bar is redrawn then.
    let mut next_tick = Instant::now() + tick.until_next(&Local::now().fixed_offset());

    loop {
        // A bar still to draw (a frame the swapchain swallowed) goes round
        // again at once.
        let wait = if windows.values().any(|window| window.dirty) {
            Duration::ZERO
        } else {
            next_tick.saturating_duration_since(Instant::now())
        };
        shell.wait_events(Some(wait));
        if Instant::now() >= next_tick {
            next_tick = Instant::now() + tick.until_next(&Local::now().fixed_offset());
            windows.values_mut().for_each(|window| window.dirty = true);
        }
        let mut quit = false;
        while let Ok(incoming) = requests.try_recv() {
            match incoming.request {
                Request::Quit => {
                    quit = true;
                    incoming.reply(control::OK);
                }
                Request::Reload => {
                    // An invalid config is logged and the running one kept
                    // (plan 05); so is a font that will not load.
                    let reloaded = lyr_config::load(config_path, Platform::current())
                        .map_err(anyhow::Error::from)
                        .and_then(|(new, _)| {
                            let new_text = if new.theme.font == config.theme.font
                                && new.theme.font_size == config.theme.font_size
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
                                text = new_text;
                            }
                            if new.bar != config.bar && !hidden {
                                close_all(shell.as_mut(), &mut bars, &mut windows)?;
                                bars.sync(shell.as_mut(), &new.bar)
                                    .context("creating the bars")?;
                            }
                            tick = Tick::of(&new.clock.format);
                            next_tick =
                                Instant::now() + tick.until_next(&Local::now().fixed_offset());
                            config = new;
                            windows.values_mut().for_each(|window| window.dirty = true);
                            tracing::info!("reload: config reloaded");
                            incoming.reply(control::OK);
                        }
                    }
                }
                Request::Toggle => {
                    if hidden {
                        bars.sync(shell.as_mut(), &config.bar)
                            .context("creating the bars")?;
                    } else {
                        close_all(shell.as_mut(), &mut bars, &mut windows)?;
                    }
                    hidden = !hidden;
                    incoming.reply(control::OK);
                }
                other => {
                    incoming.reply(format!("error: `{other}` is not implemented yet"));
                }
            }
        }
        if quit {
            return close_all(shell.as_mut(), &mut bars, &mut windows);
        }
        while let Ok(update) = updates.try_recv() {
            data.apply(update);
            windows.values_mut().for_each(|window| window.dirty = true);
        }
        let mut events = Vec::new();
        shell.pump(&mut |event| events.push(event));
        for event in &events {
            if !hidden {
                bars.handle(shell.as_mut(), &config.bar, event)
                    .context("keeping a bar on every output")?;
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
                } if bars.contains(window) => {
                    let device = (size.width, size.height);
                    let scale = scale_factor as f32;
                    let width = size.width as f32 / scale;
                    if let Some(open) = windows.get_mut(&window) {
                        open.surface.resize(device, scale)?;
                        open.width = width;
                        open.scale = scale;
                        open.dirty = true;
                    } else {
                        let surface = Surface::open(shell.as_ref(), window, device, scale)
                            .context("opening a bar's surface")?;
                        tracing::info!("drawing on {}", surface.describe());
                        let monitor = bars
                            .windows()
                            .find(|&(_, bar)| bar == window)
                            .and_then(|(monitor, _)| shell.monitor(monitor))
                            .map(|info| info.name.clone());
                        windows.insert(
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
                            },
                        );
                    }
                }
                ShellEvent::WindowDestroyed { window } => {
                    if let Some(gone) = windows.remove(&window) {
                        gone.surface.destroy()?;
                    }
                }
                _ => {
                    let Some(window) = event.window().and_then(|id| windows.get_mut(&id)) else {
                        continue;
                    };
                    let presses = window.pointer.handle(event, window.scale);
                    if window.hovered() != window.hovered {
                        window.dirty = true;
                    }
                    for press in presses {
                        if let Some(target) = window.pointer.target(&window.regions) {
                            input::act(
                                target,
                                press,
                                &config,
                                &data,
                                &window.workspaces,
                                window.monitor.as_deref(),
                                &commands,
                            );
                        }
                    }
                }
            }
        }
        let now = Local::now().fixed_offset();
        for window in windows.values_mut().filter(|window| window.dirty) {
            let hovered = window.hovered();
            let drawn = modules::draw(
                window.width,
                window.monitor.as_deref(),
                hovered,
                &config,
                &text,
                &data,
                &now,
            );
            let presented = window.surface.draw(&translate(&drawn.list, &text))?;
            window.regions = drawn.regions;
            window.workspaces = drawn.workspaces;
            window.hovered = hovered;
            window.dirty = !presented;
        }
    }
}
