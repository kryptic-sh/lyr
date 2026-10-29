//! Telling which windows belong to an application, for a click on its
//! notification: the window's class or app id against the names the
//! notification gives, its `desktop-entry` hint and its application name.

/// Whether the window whose class or app id is `window` belongs to the
/// application `names` name. Matched without regard to case, and with a
/// desktop entry's `.desktop` dropped; a reverse-DNS id also matches by its
/// last part, so the entry `org.mozilla.Thunderbird` finds the window
/// `thunderbird`, and the window `org.gnome.Nautilus` the entry
/// `org.gnome.Nautilus.desktop`.
#[must_use]
pub fn is_app(window: &str, names: &[&str]) -> bool {
    let window = forms(window);
    names
        .iter()
        .flat_map(|name| forms(name))
        .flatten()
        .any(|form| window.iter().flatten().any(|own| *own == form))
}

/// `name` in lower case without `.desktop`, and the last part of a
/// reverse-DNS id; `None` where there is no such form.
fn forms(name: &str) -> [Option<String>; 2] {
    let name = name.trim().to_lowercase();
    let name = name.strip_suffix(".desktop").unwrap_or(&name);
    if name.is_empty() {
        return [None, None];
    }
    let last = name
        .rsplit_once('.')
        .map(|(_, last)| last)
        .filter(|last| !last.is_empty())
        .map(str::to_owned);
    [Some(name.to_owned()), last]
}

#[cfg(test)]
mod tests {
    use super::is_app;

    #[test]
    fn a_window_matches_its_entry_or_name_in_any_case() {
        assert!(is_app("thunderbird", &["Thunderbird"]), "the app's name");
        assert!(
            is_app("thunderbird", &["org.mozilla.Thunderbird"]),
            "an entry's last part"
        );
        assert!(
            is_app("org.mozilla.Thunderbird", &["thunderbird"]),
            "a window's last part"
        );
        assert!(
            is_app(
                "org.gnome.Nautilus",
                &["Files", "org.gnome.Nautilus.desktop"]
            ),
            "an entry named as its file"
        );
        assert!(!is_app("firefox", &["Thunderbird", "thunderbird"]));
        assert!(!is_app("", &["", "."]), "nothing matches no name");
        assert!(!is_app("kitty", &[]));
    }
}
