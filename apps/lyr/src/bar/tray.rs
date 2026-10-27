//! What the running bar does for the tray: a press on an item's button, and
//! the item's menu.

use crcbl_shell::{Shell, ShellEvent, WindowId};
use lyr_core::hit::Rect;
use lyr_core::pointer::Press;

use super::App;
use crate::tooltips::Look;
use crate::tray_menu::Outcome;

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
                        // Where every menu of the bar's opens: under the
                        // module's box, not the item's button inside it.
                        anchor: self.windows.get(&bar).map_or(rect, |window| {
                            let tray = lyr_core::draw::Target::Module(lyr_config::Module::Tray);
                            crate::menu_panel::popup_anchor(
                                window.regions.rect(&tray).unwrap_or(rect),
                                window.height,
                                self.config.bar.position,
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
        if let Some(menu) = self.tray_menu.take() {
            menu.close(shell, false)?;
        }
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
