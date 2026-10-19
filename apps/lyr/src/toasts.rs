//! The notification toasts on screen (`docs/plan/09-notifications.md#toasts`):
//! the stack ([`Stack`]) drawn into one panel at the top right of the output
//! the user is on, above windows and clear of the bar.
//!
//! A click on a toast invokes its `default` action when it has one and takes
//! the toast down; a middle click dismisses the notification. The panel is
//! recreated whenever the stack's size changes, since crcbl 0.5.0 cannot
//! resize a panel in place.

use std::sync::{Arc, Mutex, PoisonError};
use std::time::Instant;

use crcbl_shell::{
    Margins, PanelAnchor, PanelDesc, PanelKeyboard, PanelLayer, Reserve, Shell, ShellEvent,
    WindowDesc, WindowId, WindowRole,
};
use lyr_core::draw::{DrawList, Size};
use lyr_core::hit::Regions;
use lyr_core::notifications::Store;
use lyr_core::pointer::Press;
use lyr_core::toast::stack::{Part, Stack};
use lyr_render::surface::Surface;
use lyr_render::text::Text;
use lyr_render::translate::translate;
use lyr_shell::bars::NAMESPACE;

use crate::input::Pointer;
use crate::tooltips::Look;

/// swaync's `control-center-margin-*`: the clock panel's, and the plan gives
/// the toasts the same.
pub const EDGE_MARGIN: f64 = 16.0;

/// The panel showing the stack.
struct Panel {
    window: WindowId,
    size: Size,
    list: DrawList,
    regions: Regions<Part>,
    surface: Option<Surface>,
    scale: f32,
    dirty: bool,
}

/// What a click on a toast asks for.
#[derive(Debug, PartialEq, Eq)]
pub enum Clicked {
    /// Invoke the notification's `default` action.
    Invoke(u32),
    /// Invoke the action at this index in the notification's actions.
    Action(u32, usize),
    /// Dismiss the notification.
    Dismiss(u32),
}

/// The toasts.
#[derive(Default)]
pub struct Toasts {
    store: Option<Arc<Mutex<Store>>>,
    stack: Stack,
    panel: Option<Panel>,
    pointer: Pointer,
    /// Whether the stack changed since the panel was built.
    changed: bool,
    /// When a toast's "N mins ago" next changes, so the panel is rebuilt.
    relabel_at: Option<Instant>,
}

impl Toasts {
    /// lyr became the notification daemon, holding `store`.
    #[cfg(any(target_os = "linux", test))]
    pub fn serve(&mut self, store: Arc<Mutex<Store>>) {
        self.store = Some(store);
    }

    /// The store, while lyr is the notification daemon.
    #[cfg(target_os = "linux")]
    pub fn store(&self) -> Option<&Arc<Mutex<Store>>> {
        self.store.as_ref()
    }

    /// The notification `id` arrived or was replaced; with the clock panel
    /// open (`panel_open`) it gets no toast.
    #[cfg(any(target_os = "linux", test))]
    pub fn notified(&mut self, id: u32, panel_open: bool, now: Instant) {
        if let Some(store) = &self.store {
            let store = store.lock().unwrap_or_else(PoisonError::into_inner);
            self.stack.notified(&store, id, panel_open, now);
            self.changed = true;
        }
    }

    /// The notification `id` closed.
    pub fn closed(&mut self, id: u32) {
        self.stack.closed(id);
        self.changed = true;
    }

    /// Takes every toast down: on Clear All, and when the clock panel opens,
    /// since it lists them.
    pub fn take_down(&mut self) {
        self.stack = lyr_core::toast::stack::Stack::default();
        self.changed = true;
    }

    /// When the toasts next have something to do.
    pub fn next_wake(&self) -> Option<Instant> {
        self.stack
            .next_wake()
            .into_iter()
            .chain(self.relabel_at)
            .min()
    }

    /// Whether the panel has a frame to present.
    pub fn dirty(&self) -> bool {
        self.panel.as_ref().is_some_and(|panel| panel.dirty)
    }

    /// Whether `window` is the toasts' panel.
    pub fn owns(&self, window: WindowId) -> bool {
        self.panel
            .as_ref()
            .is_some_and(|panel| panel.window == window)
    }

    /// Takes down the toasts whose time is up. Returns the transient ones
    /// among them: kept nowhere else, they are to close as expired.
    pub fn expire(&mut self, now: Instant) -> Vec<u32> {
        let gone = self.stack.expire(now);
        if gone.is_empty() {
            return gone;
        }
        self.changed = true;
        let Some(store) = &self.store else {
            return Vec::new();
        };
        let store = store.lock().unwrap_or_else(PoisonError::into_inner);
        gone.into_iter()
            .filter(|&id| store.get(id).is_some_and(|held| held.content.transient))
            .collect()
    }

    /// Brings the panel in line with the stack at `now`: rebuilt, recreated
    /// at a new size, or closed.
    pub fn sync(
        &mut self,
        shell: &mut dyn Shell,
        look: &Look<'_>,
        now: Instant,
    ) -> anyhow::Result<()> {
        if self.relabel_at.is_some_and(|at| at <= now) {
            self.changed = true;
        }
        if !std::mem::take(&mut self.changed) {
            return Ok(());
        }
        let Some(store) = &self.store else {
            return Ok(());
        };
        let (list, size, regions) = {
            let store = store.lock().unwrap_or_else(PoisonError::into_inner);
            self.stack
                .draw(&store, now, look.measure, look.line_height, look.theme)
        };
        self.relabel_at = {
            let store = store.lock().unwrap_or_else(PoisonError::into_inner);
            self.stack.next_label_change(&store, now)
        };
        if size.height == 0.0 {
            return self.close(shell);
        }
        if let Some(panel) = &mut self.panel
            && panel.size == size
        {
            panel.list = list;
            panel.regions = regions;
            panel.dirty = true;
            return Ok(());
        }
        self.close(shell)?;
        let window = shell.create_window(&WindowDesc {
            title: NAMESPACE,
            app_id: NAMESPACE,
            size: crcbl_shell::LogicalSize {
                width: f64::from(size.width),
                height: f64::from(size.height),
            },
            transparent: true,
            role: WindowRole::Panel(PanelDesc {
                namespace: NAMESPACE,
                // Where the user is: the focused output on Hyprland.
                monitor: None,
                layer: PanelLayer::Overlay,
                anchor: PanelAnchor::TOP | PanelAnchor::RIGHT,
                reserve: Reserve::None,
                margins: Margins {
                    top: EDGE_MARGIN,
                    right: EDGE_MARGIN,
                    bottom: 0.0,
                    left: 0.0,
                },
                keyboard: PanelKeyboard::None,
            }),
            ..WindowDesc::default()
        })?;
        self.panel = Some(Panel {
            window,
            size,
            list,
            regions,
            surface: None,
            scale: 1.0,
            dirty: false,
        });
        Ok(())
    }

    /// Applies an event for the panel's window, returning what a click asks
    /// for.
    pub fn event(
        &mut self,
        shell: &dyn Shell,
        event: &ShellEvent,
        now: Instant,
    ) -> anyhow::Result<Option<Clicked>> {
        let Some(panel) = &mut self.panel else {
            return Ok(None);
        };
        // A panel recreated at a new size leaves events of the old one behind.
        if event.window() != Some(panel.window) {
            return Ok(None);
        }
        match *event {
            ShellEvent::Resized {
                size, scale_factor, ..
            }
            | ShellEvent::ScaleFactorChanged {
                size, scale_factor, ..
            } => {
                let device = (size.width, size.height);
                panel.scale = scale_factor as f32;
                match &mut panel.surface {
                    Some(surface) => surface.resize(device, panel.scale)?,
                    None => {
                        panel.surface =
                            Some(Surface::open(shell, panel.window, device, panel.scale)?);
                    }
                }
                panel.dirty = true;
                Ok(None)
            }
            ShellEvent::WindowDestroyed { .. } => {
                if let Some(surface) = self.panel.take().and_then(|panel| panel.surface) {
                    surface.destroy()?;
                }
                Ok(None)
            }
            _ => {
                let presses = self.pointer.handle(event, panel.scale);
                let target = self.pointer.target(&panel.regions).copied();
                let id = target.map(|(Part::Body(id) | Part::Action(id, _))| id);
                self.stack.hover(id, now);
                let (Some(target), Some(id)) = (target, id) else {
                    return Ok(None);
                };
                let clicked = presses.into_iter().find_map(|press| match (press, target) {
                    (Press::Left, Part::Body(_)) => Some(Clicked::Invoke(id)),
                    (Press::Left, Part::Action(_, index)) => Some(Clicked::Action(id, index)),
                    (Press::Middle, _) => Some(Clicked::Dismiss(id)),
                    _ => None,
                });
                if clicked.is_some() {
                    self.closed(id);
                }
                Ok(clicked)
            }
        }
    }

    /// Draws the panel if it has a frame to present.
    pub fn draw(&mut self, text: &Text) -> anyhow::Result<()> {
        if let Some(panel) = &mut self.panel
            && let (true, Some(surface)) = (panel.dirty, &mut panel.surface)
        {
            panel.dirty = !surface.draw(&translate(&panel.list, text))?;
        }
        Ok(())
    }

    /// Closes the panel, if it is open.
    pub fn close(&mut self, shell: &mut dyn Shell) -> anyhow::Result<()> {
        if let Some(panel) = self.panel.take() {
            if let Some(surface) = panel.surface {
                surface.destroy()?;
            }
            shell.destroy_window(panel.window)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
