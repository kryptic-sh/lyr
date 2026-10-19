use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crcbl_shell::{
    ButtonState, HeadlessShell, PhysicalPoint, PointerButton, Shell, ShellEvent, WindowId,
};
use lyr_config::{Platform, Theme, from_sources};
use lyr_core::notifications::{Incoming, NORMAL_TIMEOUT, Store};

use super::{Clicked, Toasts};
use crate::tooltips::Look;

/// Every character is 10 px wide.
fn measure(text: &str) -> f32 {
    text.chars().count() as f32 * 10.0
}

struct Setup {
    shell: HeadlessShell,
    theme: Theme,
    store: Arc<Mutex<Store>>,
    toasts: Toasts,
}

fn setup() -> Setup {
    let store = Arc::new(Mutex::new(Store::default()));
    let mut toasts = Toasts::default();
    toasts.serve(store.clone());
    Setup {
        shell: HeadlessShell::new(),
        theme: from_sources(None, Platform::Linux)
            .expect("the defaults")
            .theme,
        store,
        toasts,
    }
}

impl Setup {
    fn notify(&mut self, summary: &str, now: Instant) -> u32 {
        let id = self.store.lock().expect("store").notify(Incoming {
            summary: summary.to_owned(),
            ..Incoming::default()
        });
        self.toasts.notified(id, false, now);
        id
    }

    fn sync(&mut self, now: Instant) -> Option<WindowId> {
        let look = Look {
            theme: &self.theme,
            measure: &measure,
            line_height: 20.0,
        };
        self.toasts.expire(now);
        self.toasts
            .sync(&mut self.shell, &look, now)
            .expect("synced");
        self.toasts.panel.as_ref().map(|panel| panel.window)
    }

    /// Feeds the panel its queued input events, returning the last click.
    /// Sizing events are left out: they open a GPU surface.
    fn pump(&mut self, now: Instant) -> Option<Clicked> {
        let mut events = Vec::new();
        self.shell.pump(&mut |event| events.push(event));
        let mut clicked = None;
        for event in events.iter().filter(|event| {
            !matches!(
                event,
                ShellEvent::Resized { .. } | ShellEvent::ScaleFactorChanged { .. }
            )
        }) {
            if let Some(click) = self.toasts.event(&self.shell, event, now).expect("applied") {
                clicked = Some(click);
            }
        }
        clicked
    }

    fn click(&mut self, window: WindowId, button: PointerButton, now: Instant) -> Option<Clicked> {
        let at = PhysicalPoint { x: 20.0, y: 20.0 };
        self.shell
            .move_pointer(window, at, (0.0, 0.0))
            .expect("moved");
        self.shell
            .button(window, button, ButtonState::Pressed, Some(at))
            .expect("clicked");
        self.pump(now)
    }
}

#[test]
fn a_toast_opens_a_panel_that_closes_when_its_time_is_up() {
    let mut setup = setup();
    let start = Instant::now();
    assert_eq!(setup.sync(start), None, "nothing to show");
    let id = setup.notify("hello", start);
    let window = setup.sync(start).expect("a panel");
    assert!(setup.toasts.owns(window) && setup.shell.title(window).is_ok());
    assert_eq!(setup.toasts.next_wake(), Some(start + NORMAL_TIMEOUT));
    assert_eq!(setup.sync(start + NORMAL_TIMEOUT), None);
    assert!(setup.shell.title(window).is_err(), "its window is gone");
    assert!(
        setup.store.lock().expect("store").get(id).is_some(),
        "it stays in the history"
    );
}

#[test]
fn a_second_toast_grows_the_panel_and_the_same_size_keeps_it() {
    let mut setup = setup();
    let start = Instant::now();
    setup.notify("first", start);
    let first = setup.sync(start).expect("a panel");
    setup.notify("second", start);
    let grown = setup.sync(start).expect("a panel");
    assert_ne!(first, grown, "recreated at its new size");
    assert!(setup.shell.title(first).is_err());
    let id = setup.store.lock().expect("store").notify(Incoming {
        summary: "replaced".to_owned(),
        replaces_id: 1,
        ..Incoming::default()
    });
    setup.toasts.notified(id, false, start);
    assert_eq!(setup.sync(start), Some(grown), "same size, same panel");
}

#[test]
fn a_click_invokes_and_a_middle_click_dismisses() {
    let mut setup = setup();
    let start = Instant::now();
    let first = setup.notify("first", start);
    let window = setup.sync(start).expect("a panel");
    assert_eq!(
        setup.click(window, PointerButton::Left, start),
        Some(Clicked::Invoke(first))
    );
    assert_eq!(setup.sync(start), None, "the toast went with the click");
    let second = setup.notify("second", start);
    let window = setup.sync(start).expect("a panel");
    assert_eq!(
        setup.click(window, PointerButton::Middle, start),
        Some(Clicked::Dismiss(second))
    );
}

#[test]
fn the_pointer_over_a_toast_holds_it_up() {
    let mut setup = setup();
    let start = Instant::now();
    setup.notify("hello", start);
    let window = setup.sync(start).expect("a panel");
    setup
        .shell
        .move_pointer(window, PhysicalPoint { x: 20.0, y: 20.0 }, (0.0, 0.0))
        .expect("moved");
    assert_eq!(setup.pump(start + Duration::from_secs(1)), None);
    assert_eq!(
        setup.toasts.next_wake(),
        Some(start + Duration::from_secs(60)),
        "paused: only its time's next change"
    );
    assert_eq!(setup.sync(start + NORMAL_TIMEOUT * 3), Some(window));
}

#[test]
fn only_a_transient_toast_that_times_out_is_to_close() {
    let mut setup = setup();
    let start = Instant::now();
    let kept = setup.notify("kept", start);
    let transient = setup.store.lock().expect("store").notify(Incoming {
        summary: "transient".to_owned(),
        transient: true,
        ..Incoming::default()
    });
    setup.toasts.notified(transient, false, start);
    assert!(setup.toasts.expire(start).is_empty());
    assert_eq!(setup.toasts.expire(start + NORMAL_TIMEOUT), [transient]);
    assert!(setup.store.lock().expect("store").get(kept).is_some());
}

#[test]
fn a_toast_up_for_a_minute_is_redrawn_for_its_new_time() {
    let mut setup = setup();
    let start = Instant::now();
    let id = setup.store.lock().expect("store").notify(Incoming {
        summary: "stays".to_owned(),
        expiry: Some(lyr_core::notifications::Expiry::Never),
        ..Incoming::default()
    });
    setup.toasts.notified(id, false, start);
    setup.sync(start);
    let minute = start + Duration::from_secs(60);
    assert_eq!(setup.toasts.next_wake(), Some(minute));
    setup.sync(minute);
    let panel = setup.toasts.panel.as_ref().expect("a panel");
    assert!(
        panel.list.items.iter().any(|item| matches!(
            item,
            lyr_core::draw::Item::Text { text, .. } if text == "1 min ago"
        )),
        "redrawn with its new time"
    );
    assert_eq!(
        setup.toasts.next_wake(),
        Some(start + Duration::from_secs(120))
    );
}
