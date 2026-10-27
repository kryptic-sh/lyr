use crcbl_shell::{
    ClipboardOffer, ClipboardRequestId, CloseReply, CursorIcon, DisplayMode, HeadlessShell,
    KeyCode, MimeType, MonitorInfo, PhysicalPoint, PointerMode, PopupDesc, PopupInput, Shell,
    ShellBackend, ShellCaps, ShellError, ShellEvent, SizeConstraints, SurfaceTarget, WindowDesc,
    WindowId, WindowRole, WindowState,
};
use lyr_config::{Platform, Position, Theme, from_sources};
use lyr_core::hit::Rect;
use lyr_core::tray::menu::Entry;

use super::{Outcome, Read, TrayMenu};
use crate::tooltips::Look;

/// Every character is 10 px wide.
fn measure(text: &str) -> f32 {
    text.chars().count() as f32 * 10.0
}

fn look(theme: &Theme) -> Look<'_> {
    Look {
        theme,
        measure: &measure,
        measure_in: &|text, _| measure(text),
        lay_out: &|_, _| Vec::new(),
        line_height: 20.0,
        calendar_line_height: 20.0,
    }
}

/// "Open" (1), then "More" (4) holding "Sync" (5).
fn root() -> Entry {
    let mut more = Entry {
        label: "More".to_owned(),
        ..Entry::new(4)
    };
    more.children.push(Entry {
        label: "Sync".to_owned(),
        ..Entry::new(5)
    });
    let mut root = Entry::new(0);
    root.children = vec![
        Entry {
            label: "Open".to_owned(),
            ..Entry::new(1)
        },
        more,
    ];
    root
}

/// The headless shell with a compositor's rule for grabbing popups
/// (xdg-shell's `xdg_popup.grab`): one opened while another grabs, from
/// a parent other than that one, is dismissed at once, as Hyprland 0.56.2
/// dismisses it.
#[derive(Debug)]
struct Grabbing {
    inner: HeadlessShell,
    /// The grabbing popups open, oldest first.
    grabs: Vec<WindowId>,
}

impl std::ops::Deref for Grabbing {
    type Target = HeadlessShell;

    fn deref(&self) -> &HeadlessShell {
        &self.inner
    }
}

impl std::ops::DerefMut for Grabbing {
    fn deref_mut(&mut self) -> &mut HeadlessShell {
        &mut self.inner
    }
}

impl Shell for Grabbing {
    fn backend(&self) -> ShellBackend {
        self.inner.backend()
    }
    fn caps(&self) -> ShellCaps {
        self.inner.caps()
    }
    fn create_window(&mut self, desc: &WindowDesc<'_>) -> Result<WindowId, ShellError> {
        let window = self.inner.create_window(desc)?;
        if let WindowRole::Popup(PopupDesc {
            parent,
            input: PopupInput::Grab,
            ..
        }) = desc.role
        {
            if self.grabs.last().is_some_and(|&top| top != parent) {
                self.inner.request_close(window)?;
            }
            self.grabs.push(window);
        }
        Ok(window)
    }
    fn destroy_window(&mut self, window: WindowId) -> Result<(), ShellError> {
        self.grabs.retain(|&grab| grab != window);
        self.inner.destroy_window(window)
    }
    fn window_state(&self, window: WindowId) -> Result<WindowState, ShellError> {
        self.inner.window_state(window)
    }
    fn set_title(&mut self, window: WindowId, title: &str) -> Result<(), ShellError> {
        self.inner.set_title(window, title)
    }
    fn set_visible(&mut self, window: WindowId, visible: bool) -> Result<(), ShellError> {
        self.inner.set_visible(window, visible)
    }
    fn set_mode(&mut self, window: WindowId, mode: DisplayMode) -> Result<(), ShellError> {
        self.inner.set_mode(window, mode)
    }
    fn set_constraints(
        &mut self,
        window: WindowId,
        constraints: SizeConstraints,
    ) -> Result<(), ShellError> {
        self.inner.set_constraints(window, constraints)
    }
    fn monitors(&self) -> &[MonitorInfo] {
        self.inner.monitors()
    }
    fn pump(&mut self, sink: &mut dyn FnMut(ShellEvent)) {
        self.inner.pump(sink);
    }
    fn keep_alive(&mut self) {
        self.inner.keep_alive();
    }
    fn surface_target(&self, window: WindowId) -> Result<SurfaceTarget, ShellError> {
        self.inner.surface_target(window)
    }
    fn set_pointer_mode(&mut self, window: WindowId, mode: PointerMode) -> Result<(), ShellError> {
        self.inner.set_pointer_mode(window, mode)
    }
    fn set_cursor(
        &mut self,
        window: WindowId,
        cursor: Option<CursorIcon>,
    ) -> Result<(), ShellError> {
        self.inner.set_cursor(window, cursor)
    }
    fn warp_pointer(
        &mut self,
        window: WindowId,
        position: PhysicalPoint,
    ) -> Result<(), ShellError> {
        self.inner.warp_pointer(window, position)
    }
    fn reply_close_request(
        &mut self,
        window: WindowId,
        reply: CloseReply,
    ) -> Result<(), ShellError> {
        self.inner.reply_close_request(window, reply)
    }
    fn clipboard_offer(
        &mut self,
        window: WindowId,
        offers: &[ClipboardOffer<'_>],
    ) -> Result<(), ShellError> {
        self.inner.clipboard_offer(window, offers)
    }
    fn clipboard_request(
        &mut self,
        window: WindowId,
        mime: MimeType,
    ) -> Result<ClipboardRequestId, ShellError> {
        self.inner.clipboard_request(window, mime)
    }
    fn request_frame(&mut self, window: WindowId) -> Result<(), ShellError> {
        self.inner.request_frame(window)
    }
}

struct Setup {
    shell: Grabbing,
    theme: Theme,
    menu: TrayMenu,
    bar: WindowId,
}

fn setup() -> Setup {
    let mut shell = Grabbing {
        inner: HeadlessShell::new(),
        grabs: Vec::new(),
    };
    let bar = shell
        .create_window(&WindowDesc {
            title: "lyr",
            app_id: "lyr",
            ..WindowDesc::default()
        })
        .expect("a bar");
    let theme = from_sources(None, Platform::Linux)
        .expect("the defaults")
        .theme;
    let read = Read {
        bus_name: ":1.7".to_owned(),
        menu: "/MenuBar".to_owned(),
        root: root(),
        bar,
        anchor: Rect {
            x: 400.0,
            y: 6.0,
            width: 24.0,
            height: 38.0,
        },
    };
    let menu = TrayMenu::open(&mut shell, read, Position::Top, &look(&theme))
        .expect("opened")
        .expect("a menu");
    Setup {
        shell,
        theme,
        menu,
        bar,
    }
}

impl Setup {
    fn key(&mut self, key: KeyCode) -> Outcome {
        let window = self.menu.popup.window();
        self.shell.key_press(window, key).expect("a key");
        self.settle()
    }

    /// Applies the events the shell has queued for the menu.
    fn settle(&mut self) -> Outcome {
        let mut events = Vec::new();
        self.shell.pump(&mut |event| events.push(event));
        let theme = self.theme.clone();
        let mut outcome = Outcome::Kept;
        for event in events.iter().filter(|event| {
            !matches!(
                event,
                ShellEvent::Resized { .. } | ShellEvent::ScaleFactorChanged { .. }
            )
        }) {
            if event.window() == Some(self.menu.popup.window()) {
                outcome = self
                    .menu
                    .event(&mut self.shell, event, &look(&theme))
                    .expect("applied");
            }
        }
        outcome
    }
}

#[test]
fn an_entry_is_clicked_with_enter() {
    let mut setup = setup();
    assert_eq!(setup.menu.item(), (":1.7", "/MenuBar"));
    assert!(setup.menu.opened_from(setup.bar));
    assert_eq!(setup.key(KeyCode::Enter), Outcome::Clicked(1));
}

#[test]
fn a_submenu_opens_in_place_and_back_leaves_it() {
    let mut setup = setup();
    let first = setup.menu.popup.window();
    setup.key(KeyCode::ArrowDown);
    assert_eq!(setup.key(KeyCode::Enter), Outcome::Kept, "into More");
    let second = setup.menu.popup.window();
    assert_ne!(second, first, "opened again");
    assert!(setup.shell.title(first).is_err(), "the first popup is gone");
    assert_eq!(setup.settle(), Outcome::Kept, "and the compositor keeps it");
    // Back, then Sync.
    setup.key(KeyCode::ArrowDown);
    assert_eq!(setup.key(KeyCode::Enter), Outcome::Clicked(5));
    setup.key(KeyCode::ArrowUp);
    assert_eq!(setup.key(KeyCode::Enter), Outcome::Kept, "Back");
    assert_eq!(
        setup.key(KeyCode::Enter),
        Outcome::Clicked(1),
        "the top again"
    );
}

#[test]
fn escape_closes_it() {
    let mut setup = setup();
    assert_eq!(setup.key(KeyCode::Escape), Outcome::Closed(true));
}
