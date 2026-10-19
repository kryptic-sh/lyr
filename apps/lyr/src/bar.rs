//! Running the bar: the main loop that keeps a bar on every output and draws
//! it (`docs/plan/02-architecture.md`).
//!
//! Milestone M1 draws the clock alone. The other modules have views but no
//! sources wired in yet, so they are left out of the bar rather than drawn
//! empty.

use std::collections::HashMap;
use std::path::Path;

use anyhow::Context as _;
use chrono::{DateTime, FixedOffset, Local};
use crcbl_shell::{ShellEvent, WindowId};
use lyr_config::{Config, Item, Module, Platform};
use lyr_core::clock::{self, Tick};
use lyr_core::control::Request;
use lyr_core::draw::{self, Bar, Content};
use lyr_render::surface::Surface;
use lyr_render::text::Text;
use lyr_render::translate::translate;
use lyr_shell::bars::Bars;

use crate::control;

/// One bar window's surface and the size it was last drawn at.
struct Window {
    surface: Surface,
    /// Logical pixels.
    width: f32,
}

/// Runs the bar until `lyr quit` stops it.
pub fn run(config: Option<&Path>) -> anyhow::Result<()> {
    let (config, _) =
        lyr_config::load(config, Platform::current()).context("loading the config")?;
    let mut shell = crcbl_shell::open().context("opening the window system")?;
    // Bound before anything else is loaded or drawn, so a second lyr stops
    // here.
    let requests = control::listen(control::path()?, shell.waker())?;

    let font = lyr_render::fonts::load(&config.theme.font).context("loading the bar font")?;
    if let Some(family) = &font.missing {
        eprintln!("lyr: font `{family}` is not installed; using a monospace font");
    }
    for (path, reason) in &font.unreadable {
        eprintln!("lyr: fallback font {} skipped: {reason}", path.display());
    }
    let text = Text::new(font.font, config.theme.font_size as f32);
    let tick = Tick::of(&config.clock.format);

    let mut bars = Bars::default();
    bars.sync(shell.as_mut(), &config.bar)
        .context("creating the bars")?;
    let mut windows: HashMap<WindowId, Window> = HashMap::new();

    loop {
        let now = Local::now().fixed_offset();
        shell.wait_events(Some(tick.until_next(&now)));
        let mut quit = false;
        while let Ok(incoming) = requests.try_recv() {
            if incoming.request == Request::Quit {
                quit = true;
                incoming.reply(control::OK);
            } else {
                let what = incoming.request.to_string();
                incoming.reply(format!("error: `{what}` is not implemented yet"));
            }
        }
        if quit {
            for (_, window) in windows.drain() {
                window.surface.destroy()?;
            }
            for (_, bar) in bars.windows().collect::<Vec<_>>() {
                shell.destroy_window(bar)?;
            }
            return Ok(());
        }
        let mut events = Vec::new();
        shell.pump(&mut |event| events.push(event));
        for event in &events {
            bars.handle(shell.as_mut(), &config.bar, event)
                .context("keeping a bar on every output")?;
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
                    match windows.get_mut(&window) {
                        Some(open) => {
                            open.surface.resize(device, scale)?;
                            open.width = width;
                        }
                        None => {
                            let surface = Surface::open(shell.as_ref(), window, device, scale)
                                .context("opening a bar's surface")?;
                            eprintln!("lyr: drawing on {}", surface.describe());
                            windows.insert(window, Window { surface, width });
                        }
                    }
                }
                ShellEvent::WindowDestroyed { window } => {
                    if let Some(gone) = windows.remove(&window) {
                        gone.surface.destroy()?;
                    }
                }
                _ => {}
            }
        }
        let now = Local::now().fixed_offset();
        for window in windows.values_mut() {
            draw_bar(window, &config, &text, &now)?;
        }
    }
}

/// Lays out and draws one bar at `now`.
fn draw_bar(
    window: &mut Window,
    config: &Config,
    text: &Text,
    now: &DateTime<FixedOffset>,
) -> anyhow::Result<()> {
    let modules = |items: &[Item]| -> Vec<draw::Module<Module>> {
        items
            .iter()
            .filter_map(|item| match item {
                Item::Named(name) if Module::from_name(name) == Some(Module::Clock) => {
                    Some(draw::Module {
                        content: Content::Label {
                            text: clock::view(now, "", &config.clock.format).text,
                            compact: None,
                            colour: config.theme.foreground,
                        },
                        target: Module::Clock,
                    })
                }
                _ => None,
            })
            .collect()
    };
    let (left, center, right) = (
        modules(&config.layout.left),
        modules(&config.layout.center),
        modules(&config.layout.right),
    );
    let bar = Bar {
        width: window.width,
        height: config.bar.height as f32,
        left: &left,
        center: &center,
        right: &right,
    };
    let (list, _regions) = draw::build(&bar, &config.theme, |s| text.measure(s));
    window.surface.draw(&translate(&list, text))?;
    Ok(())
}
