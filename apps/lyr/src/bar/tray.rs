//! What the running bar does for the tray: a press on an item's button, and
//! the item's menu.

use crcbl_shell::{Shell, ShellEvent, WindowId};
use lyr_core::hit::Rect;
use lyr_core::pointer::Press;

use super::App;
use crate::tooltips::Look;
use crate::tray_menu::Outcome;

/// The box an item's menu opens beside: where every menu of the bar's
/// opens, past the tray module's box rather than the item's button inside
/// it, but level with the button on a side bar, where the tray is a column
/// and its top can be far from the item clicked.
#[cfg(target_os = "linux")]
fn menu_box(module: Rect, button: Rect, edge: lyr_config::Position) -> Rect {
    if edge.is_vertical() {
        Rect {
            y: button.y,
            height: button.height,
            ..module
        }
    } else {
        module
    }
}

/// Where the tray item whose menu is `menu` on the bus name `bus_name` is
/// among the tray's `shown` items: its button's index, which moves as items
/// come and go.
pub(super) fn button_of<'a>(
    mut shown: impl Iterator<Item = &'a lyr_core::tray::Item>,
    (bus_name, menu): (&str, &str),
) -> Option<usize> {
    shown.position(|item| item.bus_name == bus_name && item.menu.as_deref() == Some(menu))
}

impl App {
    /// Carries out `press` on the tray button `index` of the bar `bar`,
    /// drawn at `rect`.
    #[cfg(target_os = "linux")]
    pub(super) fn tray_press(&self, bar: WindowId, rect: Rect, index: usize, press: Press) {
        use lyr_core::tray::{Click, click};

        use crate::sources::TrayRequest;
        let Some(item) = self.data.tray_shown().nth(index) else {
            return;
        };
        let (bus_name, path) = (item.bus_name.clone(), item.path.clone());
        // Wayland gives a window no place on the screen, so the item is
        // told the point under its button on the bar.
        let at = (
            (rect.x + rect.width / 2.0) as i32,
            (rect.y + rect.height) as i32,
        );
        let request = match press {
            Press::ScrollUp | Press::ScrollDown => TrayRequest::Scroll {
                bus_name,
                path,
                up: press == Press::ScrollUp,
            },
            _ => match (click(press, item.item_is_menu), &item.menu) {
                (Some(Click::Menu), Some(menu)) => TrayRequest::Menu {
                    path,
                    at,
                    read: crate::tray_menu::Read {
                        bus_name,
                        menu: menu.clone(),
                        root: lyr_core::tray::menu::Entry::default(),
                        bar,
                        anchor: self.windows.get(&bar).map_or(rect, |window| {
                            let tray = lyr_core::draw::Target::Module(lyr_config::Module::Tray);
                            let position = self.config.bar.position;
                            crate::menu_panel::popup_anchor(
                                menu_box(
                                    window.regions.rect(&tray).unwrap_or(rect),
                                    rect,
                                    position,
                                ),
                                window.thickness(position),
                                position,
                            )
                        }),
                    },
                },
                (Some(click), _) => TrayRequest::Click {
                    bus_name,
                    path,
                    click,
                    at,
                },
                (None, _) => return,
            },
        };
        self.commands.tray(request);
    }

    /// Windows and macOS have no tray to host.
    #[cfg(not(target_os = "linux"))]
    pub(super) fn tray_press(&self, _bar: WindowId, _rect: Rect, _index: usize, _press: Press) {}

    /// Opens the menu an item's source read, closing any other.
    #[cfg(target_os = "linux")]
    pub(super) fn open_tray_menu(
        &mut self,
        shell: &mut dyn Shell,
        read: crate::tray_menu::Read,
    ) -> anyhow::Result<()> {
        self.close_menus(shell)?;
        let look = Look {
            theme: &self.config.theme,
            measure: &|s: &str| self.text.measure(s),
            measure_in: &|s: &str, style| self.text.measure_in(s, style),
            lay_out: &|spans, width| self.text.lay_out(spans, width),
            line_height: self.text.line_height(),
            calendar_line_height: self.text.calendar_line_height(),
        };
        self.tray_menu =
            crate::tray_menu::TrayMenu::open(shell, read, self.config.bar.position, &look)?;
        Ok(())
    }

    /// Shows the menu `item`'s app changed, if it is the one open.
    #[cfg(target_os = "linux")]
    pub(super) fn tray_menu_laid_out(
        &mut self,
        shell: &mut dyn Shell,
        item: (&str, &str),
        root: lyr_core::tray::menu::Entry,
    ) -> anyhow::Result<()> {
        let Some(menu) = self.tray_menu.as_mut().filter(|menu| menu.item() == item) else {
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
        if let Outcome::Closed(window_gone) = menu.relaid(shell, root, &look)?
            && let Some(menu) = self.tray_menu.take()
        {
            menu.close(shell, window_gone)?;
        }
        Ok(())
    }

    /// Applies `event`, which is for the tray menu's window.
    pub(super) fn tray_menu_event(
        &mut self,
        shell: &mut dyn Shell,
        event: &ShellEvent,
    ) -> anyhow::Result<()> {
        let Some(menu) = &mut self.tray_menu else {
            return Ok(());
        };
        if let ShellEvent::WindowDestroyed { .. } = event {
            if let Some(menu) = self.tray_menu.take() {
                menu.close(shell, true)?;
            }
            return Ok(());
        }
        let look = Look {
            theme: &self.config.theme,
            measure: &|s: &str| self.text.measure(s),
            measure_in: &|s: &str, style| self.text.measure_in(s, style),
            lay_out: &|spans, width| self.text.lay_out(spans, width),
            line_height: self.text.line_height(),
            calendar_line_height: self.text.calendar_line_height(),
        };
        match menu.event(shell, event, &look)? {
            Outcome::Kept => {}
            Outcome::Clicked(id) => {
                self.tray_clicked(id);
                if let Some(menu) = self.tray_menu.take() {
                    menu.close(shell, false)?;
                }
            }
            Outcome::Closed(window_gone) => {
                if let Some(menu) = self.tray_menu.take() {
                    menu.close(shell, window_gone)?;
                }
            }
        }
        Ok(())
    }

    /// Tells the open menu's item its entry `id` was clicked.
    #[cfg(target_os = "linux")]
    fn tray_clicked(&self, id: i32) {
        if let Some(menu) = &self.tray_menu {
            let (bus_name, menu) = menu.item();
            self.commands
                .tray(crate::sources::TrayRequest::MenuClicked {
                    bus_name: bus_name.to_owned(),
                    menu: menu.to_owned(),
                    id,
                });
        }
    }

    /// Windows and macOS have no tray menus.
    #[cfg(not(target_os = "linux"))]
    fn tray_clicked(&self, _id: i32) {}
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "linux")]
    use lyr_config::Position;
    #[cfg(target_os = "linux")]
    use lyr_core::hit::Rect;
    use lyr_core::tray::Item;

    use super::button_of;
    #[cfg(target_os = "linux")]
    use super::menu_box;

    #[test]
    fn a_tray_menu_s_button_is_found_by_its_item() {
        let item = |bus_name: &str, menu: Option<&str>| Item {
            bus_name: bus_name.to_owned(),
            menu: menu.map(str::to_owned),
            ..Item::default()
        };
        let mut items = vec![item(":1.2", Some("/Menu")), item(":1.7", Some("/Menu"))];
        let open = (":1.7", "/Menu");
        assert_eq!(button_of(items.iter(), open), Some(1));
        items.remove(0);
        assert_eq!(button_of(items.iter(), open), Some(0), "it moved up");
        assert_eq!(button_of([item(":1.7", None)].iter(), open), None);
    }

    #[cfg(target_os = "linux")]
    const fn rect(x: f32, y: f32, width: f32, height: f32) -> Rect {
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_side_bar_s_tray_menu_opens_level_with_the_item() {
        let tray = rect(4.0, 300.0, 32.0, 200.0);
        let item = rect(8.0, 440.0, 24.0, 24.0);
        assert_eq!(
            menu_box(tray, item, Position::Left),
            rect(4.0, 440.0, 32.0, 24.0),
            "past the module's box, level with the item"
        );
        assert_eq!(
            menu_box(tray, item, Position::Right),
            rect(4.0, 440.0, 32.0, 24.0)
        );
        for edge in [Position::Top, Position::Bottom] {
            assert_eq!(
                menu_box(tray, item, edge),
                tray,
                "beside the module's box, as every menu of a {edge:?} bar's"
            );
        }
    }
}
