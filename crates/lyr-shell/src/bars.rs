//! One bar per output, kept in step with the outputs and the config
//! (`docs/plan/03-surfaces-and-input.md#outputs`).

use crcbl_shell::{
    CloseReply, LogicalSize, Margins, MonitorId, MonitorInfo, PanelAnchor, PanelDesc,
    PanelKeyboard, PanelLayer, Reserve, Shell, ShellError, ShellEvent, WindowDesc, WindowId,
    WindowRole,
};
use lyr_config::{Bar, Layer, Position};

/// The layer-shell namespace and app id, which compositor rules match on.
pub const NAMESPACE: &str = "lyr";

/// The window for a bar on `monitor`: a panel along the configured edge,
/// stretched across the output and reserving its own height.
#[must_use]
pub fn window_desc(bar: &Bar, monitor: MonitorId) -> WindowDesc<'static> {
    WindowDesc {
        title: NAMESPACE,
        app_id: NAMESPACE,
        size: size(bar),
        transparent: true,
        role: WindowRole::Panel(panel_desc(bar, monitor)),
        ..WindowDesc::default()
    }
}

/// A bar's size: the width is the output's, since a panel anchored to both
/// sides takes its size along that axis from the monitor, not from here.
fn size(bar: &Bar) -> LogicalSize {
    LogicalSize {
        width: 0.0,
        height: f64::from(bar.height),
    }
}

/// The panel a bar on `monitor` is.
fn panel_desc(bar: &Bar, monitor: MonitorId) -> PanelDesc<'static> {
    let edge = match bar.position {
        Position::Top => PanelAnchor::TOP,
        Position::Bottom => PanelAnchor::BOTTOM,
    };
    PanelDesc {
        namespace: NAMESPACE,
        monitor: Some(monitor),
        layer: match bar.layer {
            Layer::Background => PanelLayer::Background,
            Layer::Bottom => PanelLayer::Bottom,
            Layer::Top => PanelLayer::Top,
            Layer::Overlay => PanelLayer::Overlay,
        },
        anchor: edge | PanelAnchor::LEFT | PanelAnchor::RIGHT,
        reserve: Reserve::OwnExtent,
        margins: Margins::default(),
        keyboard: PanelKeyboard::None,
    }
}

/// Whether `monitor` gets a bar: every output when `bar.output` is empty,
/// otherwise the outputs it names.
fn wanted(bar: &Bar, monitor: &MonitorInfo) -> bool {
    bar.output.is_empty() || bar.output.contains(&monitor.name)
}

/// The bar windows, by the monitor each is on.
#[derive(Debug, Default)]
pub struct Bars {
    windows: Vec<(MonitorId, WindowId)>,
}

impl Bars {
    /// The bar windows, by monitor.
    pub fn windows(&self) -> impl Iterator<Item = (MonitorId, WindowId)> + '_ {
        self.windows.iter().copied()
    }

    /// Whether `window` is one of the bars.
    #[must_use]
    pub fn contains(&self, window: WindowId) -> bool {
        self.windows.iter().any(|&(_, bar)| bar == window)
    }

    /// Makes the bars match `shell`'s monitors and `bar`: a bar on each
    /// monitor that should have one and none on any other. Returns whether a
    /// window was created or destroyed.
    ///
    /// # Errors
    ///
    /// When the shell refuses to create or destroy a window. The bars made
    /// before the error are kept and tracked.
    pub fn sync(&mut self, shell: &mut dyn Shell, bar: &Bar) -> Result<bool, ShellError> {
        let mut changed = false;
        let mut keep = Vec::with_capacity(self.windows.len());
        for (monitor, window) in std::mem::take(&mut self.windows) {
            let still_wanted = shell.monitor(monitor).is_some_and(|info| wanted(bar, info));
            if still_wanted {
                keep.push((monitor, window));
            } else {
                // The window may already be on its way out (its monitor was
                // unplugged), in which case the handle is stale and there is
                // nothing left to destroy.
                match shell.destroy_window(window) {
                    Ok(()) | Err(ShellError::InvalidWindow { .. }) => changed = true,
                    Err(error) => {
                        self.windows = keep;
                        return Err(error);
                    }
                }
            }
        }
        self.windows = keep;

        let missing: Vec<MonitorId> = shell
            .monitors()
            .iter()
            .filter(|info| wanted(bar, info))
            .map(|info| info.id)
            .filter(|id| !self.windows.iter().any(|&(monitor, _)| monitor == *id))
            .collect();
        for monitor in missing {
            let window = shell.create_window(&window_desc(bar, monitor))?;
            self.windows.push((monitor, window));
            changed = true;
        }
        Ok(changed)
    }

    /// Gives every bar `bar`'s height, layer and edge in place, the window
    /// and its surface kept: the new size comes back as each window's next
    /// `Resized`. [`Bars::sync`] then adds or removes bars if `bar` changed
    /// which outputs have one.
    ///
    /// # Errors
    ///
    /// When the shell refuses a change; the bars before it have changed.
    pub fn reshape(&mut self, shell: &mut dyn Shell, bar: &Bar) -> Result<(), ShellError> {
        for &(monitor, window) in &self.windows {
            shell.set_panel(window, size(bar), &panel_desc(bar, monitor))?;
        }
        Ok(())
    }

    /// Applies a shell event that concerns the set of bars. Returns whether
    /// the bars changed, so the caller can lay out and draw the new ones.
    ///
    /// A bar asked to close (its monitor was unplugged, or the compositor took
    /// it away) is closed and forgotten, and the next monitor change puts back
    /// any bar that is still wanted.
    ///
    /// # Errors
    ///
    /// As [`Bars::sync`], and when the shell refuses the close reply.
    pub fn handle(
        &mut self,
        shell: &mut dyn Shell,
        bar: &Bar,
        event: &ShellEvent,
    ) -> Result<bool, ShellError> {
        match *event {
            ShellEvent::MonitorsChanged => self.sync(shell, bar),
            ShellEvent::CloseRequested { window } if self.contains(window) => {
                self.forget(window);
                shell.reply_close_request(window, CloseReply::Close)?;
                Ok(true)
            }
            ShellEvent::WindowDestroyed { window } => Ok(self.forget(window)),
            _ => Ok(false),
        }
    }

    /// Drops `window` from the bars. Returns whether it was one.
    fn forget(&mut self, window: WindowId) -> bool {
        let before = self.windows.len();
        self.windows.retain(|&(_, bar)| bar != window);
        self.windows.len() != before
    }
}

#[cfg(test)]
mod tests;
