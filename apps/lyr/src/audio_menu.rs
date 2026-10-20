//! The audio menu (`docs/plan/10-popups-and-menus.md#audio-menu`): a panel
//! at the top left, under the volume or mic module where a bar shows one,
//! taking the keyboard. A click on a tab shows it; a click on a row's
//! button or slider, or Enter on a button the arrow keys moved to, is the
//! caller's to carry out, by the row's id. A stream's device and a
//! configuration row's choice open a dropdown of their options over the
//! button, as wiremix's do ([`ListPopup`]). `Escape`, losing focus and the
//! same click or binding again close it.
//!
//! Its panel is [`MenuPanel`]'s, recreated when its size changes (a stream
//! starting).

use crcbl_shell::{Shell, ShellEvent, WindowId};
use lyr_config::Position;
use lyr_core::audio_menu::{self, Nodes, Step, Tab, Target, level_at};
use lyr_core::list_menu::Row;
use lyr_render::text::Text;

use crate::keys::{self, Key};
use crate::list_popup::{self, ListPopup};
use crate::menu_panel::{Event, MenuPanel, Place};
use crate::tooltips::Look;

/// The open menu.
pub struct AudioMenu {
    panel: MenuPanel<Target>,
    tab: Tab,
    /// The shown tab's rows' ids, in the order drawn.
    ids: Vec<String>,
    /// What it shows, for a dropdown's options.
    nodes: Nodes,
    /// A dropdown open over a row's button.
    choice: Option<Choice>,
}

/// A dropdown of the options a row's button chooses among.
struct Choice {
    tab: Tab,
    /// The row's id.
    id: String,
    /// Whether it routes a stream, or sets a configuration row.
    routes: bool,
    popup: ListPopup<String>,
}

/// What a row's button asks for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    Mute,
    Step(Step),
    /// Set the volume to this percent: a click on the slider.
    Set(u32),
    /// Make the device the default.
    Default,
    /// Move the stream to the device with this id.
    Route(String),
    /// Choose the option with this id for a configuration row: a card's
    /// profile or a device's port.
    Setting(String),
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
            nodes: nodes.clone(),
            choice: None,
        })
    }

    /// Whether `window` is the menu or its dropdown.
    pub fn owns(&self, window: WindowId) -> bool {
        self.panel.owns(window)
            || self
                .choice
                .as_ref()
                .is_some_and(|choice| choice.popup.owns(window))
    }

    /// Reads one shell event for its surfaces' frame requests.
    pub fn observe(&mut self, event: &ShellEvent) {
        self.panel.observe(event);
        if let Some(choice) = &mut self.choice {
            choice.popup.observe(event);
        }
    }

    /// Whether it has a frame to present that it can present now.
    pub fn ready(&self) -> bool {
        self.panel.ready()
            || self
                .choice
                .as_ref()
                .is_some_and(|choice| choice.popup.ready())
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
        self.nodes = nodes.clone();
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

    /// Applies an event for the menu's window or its dropdown's.
    pub fn event(
        &mut self,
        shell: &mut dyn Shell,
        event: &ShellEvent,
        look: &Look<'_>,
    ) -> anyhow::Result<Outcome> {
        if let Some(choice) = &mut self.choice {
            // A layer panel that takes the keyboard can keep it under its
            // popup's grab (Hyprland does), so the panel's keys are the
            // dropdown's while it is open, and Escape closes only it.
            let keys = matches!(event, ShellEvent::Key { .. });
            if keys && keys::pressed(event) == Some(Key::Escape) {
                if let Some(choice) = self.choice.take() {
                    choice.popup.close(shell, false)?;
                }
                return Ok(Outcome::Kept);
            }
            if keys
                || event
                    .window()
                    .is_some_and(|window| choice.popup.owns(window))
            {
                return match choice.popup.event(shell, event, look)? {
                    list_popup::Outcome::Kept => Ok(Outcome::Kept),
                    list_popup::Outcome::Chosen(option) => {
                        let choice = self.choice.take().expect("open just above");
                        choice.popup.close(shell, false)?;
                        let action = if choice.routes {
                            Action::Route(option)
                        } else {
                            Action::Setting(option)
                        };
                        Ok(Outcome::Act(choice.tab, choice.id, action))
                    }
                    list_popup::Outcome::Dismissed => {
                        if let Some(choice) = self.choice.take() {
                            choice.popup.close(shell, true)?;
                        }
                        Ok(Outcome::Kept)
                    }
                };
            }
            // The dropdown's grab takes the keyboard from the panel; that
            // is not the menu losing focus.
            if let ShellEvent::Focus { focused: false, .. } = event {
                return Ok(Outcome::Kept);
            }
        }
        Ok(match self.panel.event(shell, event)? {
            Event::Kept | Event::Enter => Outcome::Kept,
            Event::Close => Outcome::Close,
            Event::Closed => Outcome::Closed,
            Event::Redraw => Outcome::Redraw,
            Event::Pick(target) => self.act(shell, target, look)?,
        })
    }

    /// Opens the dropdown for `target`, the button of the row `index`: the
    /// devices of a stream's flow, or a configuration row's options, the
    /// chosen one selected.
    fn open_choice(
        &mut self,
        shell: &mut dyn Shell,
        target: Target,
        index: usize,
        look: &Look<'_>,
    ) -> anyhow::Result<()> {
        if let Some(open) = self.choice.take() {
            open.popup.close(shell, false)?;
        }
        let (Some(id), Some(anchor)) = (self.ids.get(index), self.panel.rect_of(&target)) else {
            return Ok(());
        };
        let routes = matches!(target, Target::Route(_));
        let (rows, chosen): (Vec<Row<String>>, Option<String>) = if routes {
            let devices = match self.tab {
                Tab::Playback => &self.nodes.outputs,
                Tab::Recording => &self.nodes.inputs,
                Tab::Output | Tab::Input | Tab::Configuration => return Ok(()),
            };
            let stream = self.nodes.of(self.tab).iter().find(|node| node.id == *id);
            (
                devices
                    .iter()
                    .map(|device| Row::new(device.name.clone(), device.id.clone()))
                    .collect(),
                stream.and_then(|stream| stream.device.clone()),
            )
        } else {
            let Some(setting) = self
                .nodes
                .settings
                .iter()
                .flatten()
                .find(|setting| setting.id == *id)
            else {
                return Ok(());
            };
            (
                setting
                    .options
                    .iter()
                    .map(|(option, description)| Row::new(description.clone(), option.clone()))
                    .collect(),
                Some(setting.active.clone()),
            )
        };
        let Some(mut popup) = ListPopup::open(
            shell,
            self.panel.window(),
            anchor,
            Position::Top,
            rows,
            look,
        )?
        else {
            return Ok(());
        };
        if let Some(chosen) = &chosen {
            popup.select(chosen, look);
        }
        self.choice = Some(Choice {
            tab: self.tab,
            id: id.clone(),
            routes,
            popup,
        });
        Ok(())
    }

    /// What choosing `target` does: a tab is shown, with the outline gone
    /// since its place may not be on the new tab; a row's button is the
    /// caller's to carry out, and so is a click on its slider, as the volume
    /// where it landed. Enter on a slider has no place to set it to.
    fn act(
        &mut self,
        shell: &mut dyn Shell,
        target: Target,
        look: &Look<'_>,
    ) -> anyhow::Result<Outcome> {
        let (index, action) = match target {
            Target::Tab(tab) => {
                self.tab = tab;
                self.panel.unfocus();
                return Ok(Outcome::Redraw);
            }
            Target::Mute(index) => (index, Action::Mute),
            Target::Step(index, step) => (index, Action::Step(step)),
            Target::Default(index) => (index, Action::Default),
            Target::Route(index) | Target::Setting(index) => {
                self.open_choice(shell, target, index, look)?;
                return Ok(Outcome::Kept);
            }
            Target::Level(index) => match self.panel.picked_across(&target) {
                Some(across) => (index, Action::Set(level_at(across))),
                None => return Ok(Outcome::Kept),
            },
        };
        Ok(match self.ids.get(index) {
            Some(id) => Outcome::Act(self.tab, id.clone(), action),
            None => Outcome::Kept,
        })
    }

    /// Draws the menu and its dropdown if they have frames to present.
    pub fn draw(&mut self, shell: &mut dyn Shell, text: &Text) -> anyhow::Result<()> {
        self.panel.draw(shell, text)?;
        if let Some(choice) = &mut self.choice {
            choice.popup.draw(shell, text)?;
        }
        Ok(())
    }

    /// Closes the menu: its dropdown, its surface, then its window unless
    /// the window system has closed it already (`window_gone`), which takes
    /// the dropdown with it.
    pub fn close(self, shell: &mut dyn Shell, window_gone: bool) -> anyhow::Result<()> {
        if let Some(choice) = self.choice {
            choice.popup.close(shell, window_gone)?;
        }
        self.panel.close(shell, window_gone)
    }
}

/// The ids of `tab`'s rows of `nodes`, in the order the menu draws them.
fn ids(nodes: &Nodes, tab: Tab) -> Vec<String> {
    match (tab, &nodes.settings) {
        (Tab::Configuration, Some(settings)) => {
            settings.iter().map(|setting| setting.id.clone()).collect()
        }
        _ => nodes.of(tab).iter().map(|node| node.id.clone()).collect(),
    }
}

#[cfg(test)]
mod tests;
