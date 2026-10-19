//! What the running bar does with a window-system event: keeping a bar on
//! every output, sizing bars and tooltips, and the pointer.

use std::time::Instant;

use anyhow::Context as _;
use crcbl_shell::{Shell, ShellEvent};
use lyr_core::hit::Regions;
use lyr_render::surface::Surface;

use super::App;
use super::window::Window;
use crate::input::{self, Pointer};
use crate::tooltips::Tooltips;

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
                    // Its tooltip's window went with it.
                    gone.tooltips.destroyed()?;
                    gone.surface.destroy()?;
                }
            }
            _ => {
                let Some(window) = event.window().and_then(|id| self.windows.get_mut(&id)) else {
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
                for press in presses {
                    if let Some(target) = window.pointer.target(&window.regions) {
                        input::act(
                            target,
                            press,
                            &self.config,
                            &self.data,
                            &window.workspaces,
                            window.monitor.as_deref(),
                            &self.commands,
                        );
                    }
                }
            }
        }
        Ok(())
    }
}
