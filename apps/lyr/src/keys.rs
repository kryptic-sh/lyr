//! The keys lyr's menus and panels answer to, read from the symbol the
//! keyboard layout produces (`keysym`) rather than from the physical key:
//! an xkb remap (`caps:escape`) or a virtual keyboard's own keymap moves the
//! symbol, not the scancode crcbl names `key_code` after. `wtype`'s Down
//! arrived as `key_code: Escape` and closed the Wi-Fi menu.

use crcbl_shell::{ButtonState, Keysym, ShellEvent, WindowId};

/// A key lyr's menus answer to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Escape,
    /// Return or the keypad's Enter.
    Enter,
    Up,
    Down,
    Backspace,
    Delete,
}

/// The key `event` presses, of those lyr answers to.
pub fn pressed(event: &ShellEvent) -> Option<Key> {
    let ShellEvent::Key {
        keysym,
        state: ButtonState::Pressed,
        ..
    } = *event
    else {
        return None;
    };
    Some(match keysym {
        Keysym::ESCAPE => Key::Escape,
        Keysym::RETURN | Keysym::KP_ENTER => Key::Enter,
        Keysym::UP => Key::Up,
        Keysym::DOWN => Key::Down,
        Keysym::BACKSPACE => Key::Backspace,
        Keysym::DELETE => Key::Delete,
        _ => return None,
    })
}

/// The keyboard's place among `count` targets after `key`, from `focus`:
/// Down to the next (the first when there is none), Up to the one before,
/// each held to the ends. `None` for any other key, or with no targets.
pub fn moved(focus: Option<usize>, count: usize, key: Option<Key>) -> Option<usize> {
    let last = count.checked_sub(1)?;
    match key? {
        Key::Down => Some(focus.map_or(0, |at| (at + 1).min(last))),
        Key::Up => Some(focus.map_or(0, |at| at.saturating_sub(1).min(last))),
        _ => None,
    }
}

/// `events` without a keyboard focus that leaves a window and comes back to
/// it later in the same batch, which never left as far as a panel cares.
/// sway, following the mouse, sends a panel holding the exclusive keyboard
/// a `leave` and at once an `enter` again when the pointer crosses a window
/// (sway 1.12, 2026-10-21); heard alone, the `leave` closed the panel with
/// no click.
pub fn settled_focus(events: Vec<ShellEvent>) -> Vec<ShellEvent> {
    let comes_back = |after: usize, window: WindowId| {
        events[after + 1..].iter().any(|later| {
            matches!(*later, ShellEvent::Focus { window: back, focused: true } if back == window)
        })
    };
    let passing: Vec<bool> = events
        .iter()
        .enumerate()
        .map(|(at, event)| {
            !matches!(*event, ShellEvent::Focus { window, focused: false } if comes_back(at, window))
        })
        .collect();
    events
        .into_iter()
        .zip(passing)
        .filter_map(|(event, passes)| passes.then_some(event))
        .collect()
}

#[cfg(test)]
mod tests {
    use crcbl_shell::{ButtonState, KeyCode, Keysym, ShellEvent};

    use super::{Key, moved, pressed, settled_focus};

    /// A key event as a keyboard whose keymap puts `keysym` on the scancode
    /// evdev calls Escape, as `wtype`'s does.
    fn key(keysym: u32, state: ButtonState) -> ShellEvent {
        let mut shell = crcbl_shell::HeadlessShell::new();
        let window =
            crcbl_shell::Shell::create_window(&mut shell, &Default::default()).expect("a window");
        shell.key_press(window, KeyCode::Escape).expect("a key");
        let mut events = Vec::new();
        crcbl_shell::Shell::pump(&mut shell, &mut |event| events.push(event));
        let mut event = events
            .into_iter()
            .find(|event| matches!(event, ShellEvent::Key { .. }))
            .expect("the key's event");
        if let ShellEvent::Key {
            keysym: held,
            state: was,
            ..
        } = &mut event
        {
            *held = Keysym(keysym);
            *was = state;
        }
        event
    }

    #[test]
    fn the_symbol_decides_not_the_physical_key() {
        assert_eq!(pressed(&key(0xFF54, ButtonState::Pressed)), Some(Key::Down));
        assert_eq!(
            pressed(&key(0xFF1B, ButtonState::Pressed)),
            Some(Key::Escape)
        );
        assert_eq!(
            pressed(&key(0xFF8D, ButtonState::Pressed)),
            Some(Key::Enter)
        );
        assert_eq!(pressed(&key(0x61, ButtonState::Pressed)), None, "a letter");
        assert_eq!(pressed(&key(0xFF54, ButtonState::Released)), None);
    }

    #[test]
    fn the_arrows_move_through_the_targets_and_stop_at_the_ends() {
        let (down, up) = (Some(Key::Down), Some(Key::Up));
        assert_eq!(moved(None, 3, down), Some(0), "onto the first");
        assert_eq!(moved(None, 3, up), Some(0));
        assert_eq!(moved(Some(0), 3, down), Some(1));
        assert_eq!(moved(Some(2), 3, down), Some(2), "held at the last");
        assert_eq!(moved(Some(0), 3, up), Some(0), "held at the first");
        assert_eq!(moved(Some(2), 3, up), Some(1));
        assert_eq!(moved(Some(5), 3, up), Some(2), "a place past the end");
        assert_eq!(moved(None, 0, down), None, "nothing to move through");
        assert_eq!(moved(Some(1), 3, Some(Key::Enter)), None);
        assert_eq!(moved(Some(1), 3, None), None);
    }

    #[test]
    fn a_focus_that_comes_back_in_the_batch_never_left() {
        let mut shell = crcbl_shell::HeadlessShell::new();
        let panel =
            crcbl_shell::Shell::create_window(&mut shell, &Default::default()).expect("a window");
        let other =
            crcbl_shell::Shell::create_window(&mut shell, &Default::default()).expect("a window");
        let focus = |window, focused| ShellEvent::Focus { window, focused };
        let settled = settled_focus(vec![
            focus(panel, false),
            focus(other, false),
            focus(panel, true),
        ]);
        assert_eq!(
            settled,
            [focus(other, false), focus(panel, true)],
            "the panel's leave goes, the other window's stays"
        );
        // A leave with no enter after it is a real one.
        let lost = vec![focus(panel, true), focus(panel, false)];
        assert_eq!(settled_focus(lost.clone()), lost);
    }
}
