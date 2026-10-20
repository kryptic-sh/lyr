//! The audio menu (`docs/plan/10-popups-and-menus.md#audio-menu`): a panel
//! at the top left, under the volume or mic module where a bar shows one,
//! taking the keyboard. A click on a tab shows it; a click on a row's
//! button or slider, or Enter on a button the arrow keys moved to, is the
//! caller's to carry out, by the row's id. `Escape`, losing focus and the same click or
//! binding again close it.
//!
//! Its panel is [`MenuPanel`]'s, recreated when its size changes (a stream
//! starting).

use crcbl_shell::{Shell, ShellEvent, WindowId};
use lyr_core::audio_menu::{self, Nodes, Step, Tab, Target, level_at};
use lyr_render::text::Text;

use crate::menu_panel::{Event, MenuPanel, Place};
use crate::tooltips::Look;

/// The open menu.
pub struct AudioMenu {
    panel: MenuPanel<Target>,
    tab: Tab,
    /// The shown tab's rows' ids, in the order drawn.
    ids: Vec<String>,
}

/// What a row's button asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Mute,
    Step(Step),
    /// Set the volume to this percent: a click on the slider.
    Set(u32),
    /// Make the device the default.
    Default,
    /// Move the stream to the next device of its flow.
    Route,
}

/// What an event did to the menu.
#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    Kept,
    /// It is to close.
    Close,
    /// The window system is closing its window.
    Closed,
    /// A row's button was clicked or chosen with Enter: the row is on `tab`,
    /// named by its id.
    Act(Tab, String, Action),
    /// A tab was chosen or the keyboard moved its place: the menu is to be
    /// drawn again.
    Redraw,
}

impl AudioMenu {
    /// Opens the menu on `tab` of `nodes` at `place`.
    pub fn open(
        shell: &mut dyn Shell,
        place: Place,
        look: &Look<'_>,
        nodes: &Nodes,
        tab: Tab,
    ) -> anyhow::Result<Self> {
        let drawn = audio_menu::draw(nodes, tab, None, look.measure, look.line_height, look.theme);
        Ok(Self {
            // A slider follows a drag.
            panel: MenuPanel::open(shell, place, look.line_height, drawn)?
                .with_drags(|target| matches!(target, Target::Level(_))),
            tab,
            ids: ids(nodes, tab),
        })
    }

    /// Whether `window` is the menu.
    pub fn owns(&self, window: WindowId) -> bool {
        self.panel.owns(window)
    }

    /// Reads one shell event for its surface's frame request.
    pub fn observe(&mut self, event: &ShellEvent) {
        self.panel.observe(event);
    }

    /// Whether it has a frame to present that it can present now.
    pub fn ready(&self) -> bool {
        self.panel.ready()
    }

    /// Draws the menu again for `nodes` as they are now, recreating its
    /// window if its size changed.
    pub fn refresh(
        &mut self,
        shell: &mut dyn Shell,
        look: &Look<'_>,
        nodes: &Nodes,
    ) -> anyhow::Result<()> {
        self.ids = ids(nodes, self.tab);
        let drawn = audio_menu::draw(
            nodes,
            self.tab,
            self.panel.focused(),
            look.measure,
            look.line_height,
            look.theme,
        );
        self.panel.show(shell, drawn)?;
        Ok(())
    }

    /// Applies an event for the menu's window.
    pub fn event(&mut self, shell: &mut dyn Shell, event: &ShellEvent) -> anyhow::Result<Outcome> {
        Ok(match self.panel.event(shell, event)? {
            Event::Kept | Event::Enter => Outcome::Kept,
            Event::Close => Outcome::Close,
            Event::Closed => Outcome::Closed,
            Event::Redraw => Outcome::Redraw,
            Event::Pick(target) => self.act(target),
        })
    }

    /// What choosing `target` does: a tab is shown, with the outline gone
    /// since its place may not be on the new tab; a row's button is the
    /// caller's to carry out, and so is a click on its slider, as the volume
    /// where it landed. Enter on a slider has no place to set it to.
    fn act(&mut self, target: Target) -> Outcome {
        let (index, action) = match target {
            Target::Tab(tab) => {
                self.tab = tab;
                self.panel.unfocus();
                return Outcome::Redraw;
            }
            Target::Mute(index) => (index, Action::Mute),
            Target::Step(index, step) => (index, Action::Step(step)),
            Target::Default(index) => (index, Action::Default),
            Target::Route(index) => (index, Action::Route),
            Target::Level(index) => match self.panel.picked_across(&target) {
                Some(across) => (index, Action::Set(level_at(across))),
                None => return Outcome::Kept,
            },
        };
        match self.ids.get(index) {
            Some(id) => Outcome::Act(self.tab, id.clone(), action),
            None => Outcome::Kept,
        }
    }

    /// Draws the menu if it has a frame to present.
    pub fn draw(&mut self, shell: &mut dyn Shell, text: &Text) -> anyhow::Result<()> {
        self.panel.draw(shell, text)
    }

    /// Closes the menu: its surface, then its window unless the window
    /// system has closed it already (`window_gone`).
    pub fn close(self, shell: &mut dyn Shell, window_gone: bool) -> anyhow::Result<()> {
        self.panel.close(shell, window_gone)
    }
}

/// The ids of `tab`'s rows of `nodes`, in the order the menu draws them.
fn ids(nodes: &Nodes, tab: Tab) -> Vec<String> {
    nodes.of(tab).iter().map(|node| node.id.clone()).collect()
}

#[cfg(test)]
mod tests;
