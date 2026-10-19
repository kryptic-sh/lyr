//! Running the bar: the main loop that keeps a bar on every output and draws
//! it (`docs/plan/02-architecture.md`).
//!
//! It draws the modules that have data: the clock everywhere, and on Linux
//! the battery and the workspaces once their sources report ([`sources`]). A
//! module with no data yet is left out rather than drawn empty.
//!
//! [`sources`]: crate::sources

use std::collections::HashMap;
use std::path::Path;

use anyhow::Context as _;
use chrono::{DateTime, FixedOffset, Local};
use crcbl_shell::{ShellEvent, WindowId};
use lyr_config::{Config, Item, Module, Platform};
use lyr_core::battery::{self, Level, Reading};
use lyr_core::clock::{self, Tick};
use lyr_core::control::Request;
use lyr_core::draw::{self, Bar, Content};
use lyr_render::surface::Surface;
use lyr_render::text::Text;
use lyr_render::translate::translate;
use lyr_shell::bars::Bars;

use crate::control;
use crate::sources::{self, Update};

/// One bar window's surface and the size it was last drawn at.
struct Window {
    surface: Surface,
    /// Logical pixels.
    width: f32,
    /// The output it is on, by name, for a per-output workspace list.
    monitor: Option<String>,
}

/// The sources' latest states.
#[derive(Default)]
struct Data {
    battery: Option<Reading>,
    #[cfg(target_os = "linux")]
    workspaces: Option<lyr_wm::hyprland::State>,
}

impl Data {
    fn apply(&mut self, update: Update) {
        match update {
            #[cfg(target_os = "linux")]
            Update::Battery(reading) => self.battery = reading,
            #[cfg(target_os = "linux")]
            Update::Workspaces(state) => self.workspaces = Some(state),
        }
    }
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
    let updates = sources::start(shell.waker()).context("starting the sources")?;
    let mut data = Data::default();

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
        while let Ok(update) = updates.try_recv() {
            data.apply(update);
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
                                    monitor,
                                },
                            );
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
            draw_bar(window, &config, &text, &data, &now)?;
        }
    }
}

/// Lays out and draws one bar at `now`.
fn draw_bar(
    window: &mut Window,
    config: &Config,
    text: &Text,
    data: &Data,
    now: &DateTime<FixedOffset>,
) -> anyhow::Result<()> {
    let modules = |items: &[Item]| -> Vec<draw::Module<Module>> {
        items
            .iter()
            .filter_map(|item| match item {
                Item::Named(name) => Module::from_name(name),
                Item::Spacer { .. } => None,
            })
            .filter_map(|module| {
                content(module, config, data, now, window.monitor.as_deref()).map(|content| {
                    draw::Module {
                        content,
                        target: module,
                    }
                })
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

/// What `module` shows now, or `None` when it has no data to show.
fn content(
    module: Module,
    config: &Config,
    data: &Data,
    now: &DateTime<FixedOffset>,
    #[cfg_attr(not(target_os = "linux"), allow(unused_variables))] monitor: Option<&str>,
) -> Option<Content> {
    let theme = &config.theme;
    let label = |text: String, colour| Content::Label {
        text,
        compact: None,
        colour,
    };
    match module {
        Module::Clock => Some(label(
            clock::view(now, "", &config.clock.format).text,
            theme.foreground,
        )),
        Module::Battery => {
            let shown = battery::view(data.battery.as_ref()?, &config.battery);
            let colour = match shown.level {
                Some(Level::Critical) => theme.battery_critical,
                Some(Level::Warning) => theme.battery_warning,
                None => theme.foreground,
            };
            Some(label(shown.text, colour))
        }
        #[cfg(target_os = "linux")]
        Module::Workspaces => {
            let state = data.workspaces.as_ref()?;
            let shown = |workspace: &&lyr_wm::hyprland::Workspace| {
                config.workspaces.all_outputs
                    || monitor.is_some_and(|monitor| workspace.monitor.as_deref() == Some(monitor))
            };
            let buttons = state
                .workspaces()
                .filter(shown)
                .map(|workspace| draw::Button {
                    text: config
                        .workspaces
                        .format
                        .render(|name| (name == "name").then_some(workspace.name.as_str())),
                    colour: if state.active() == Some(workspace.id) {
                        theme.workspace_active
                    } else if workspace.urgent {
                        theme.workspace_urgent.unwrap_or(theme.foreground)
                    } else {
                        theme.foreground
                    },
                })
                .collect();
            Some(Content::Buttons(buttons))
        }
        _ => None,
    }
}
