//! The keys lyr's menus and panels answer to, read from the symbol the
//! keyboard layout produces (`keysym`) rather than from the physical key:
//! an xkb remap (`caps:escape`) or a virtual keyboard's own keymap moves the
//! symbol, not the scancode crcbl names `key_code` after. `wtype`'s Down
//! arrived as `key_code: Escape` and closed the Wi-Fi menu.

use crcbl_shell::{ButtonState, Keysym, ShellEvent};

/// A key lyr's menus answer to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Escape,
    /// Return or the keypad's Enter.
    Enter,
    Up,
    Down,
    Backspace,
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
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use crcbl_shell::{ButtonState, KeyCode, Keysym, ShellEvent};

    use super::{Key, pressed};

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
}
