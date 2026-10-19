//! A tray item's menu, as its `com.canonical.dbusmenu` object lays it out:
//! each entry's properties as the interface documents them (libdbusmenu's
//! `dbus-menu.xml`, as GNUstep's `libs-dbuskit` carries it, read
//! 2026-10-20), with their defaults where an entry leaves one out.

/// A toggle's kind and state (`toggle-type` and `toggle-state`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Toggle {
    /// One of a group, rather than a check mark on its own.
    pub radio: bool,
    /// On, off, or `None` for neither.
    pub on: Option<bool>,
}

/// One entry of a menu.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Entry {
    /// What the menu's events name it by.
    pub id: i32,
    /// Its text, access-key underscores taken out ([`label`]).
    pub label: String,
    pub enabled: bool,
    pub visible: bool,
    /// A line between groups of entries rather than an entry.
    pub separator: bool,
    pub toggle: Option<Toggle>,
    /// Its submenu's entries.
    pub children: Vec<Entry>,
}

impl Entry {
    /// An entry with the interface's defaults: standard, enabled, visible,
    /// with no label and no toggle.
    #[must_use]
    pub fn new(id: i32) -> Self {
        Self {
            id,
            enabled: true,
            visible: true,
            ..Self::default()
        }
    }

    /// Whether a click on it does anything: an enabled entry that is not a
    /// separator.
    #[must_use]
    pub fn clickable(&self) -> bool {
        self.enabled && !self.separator
    }

    /// The entries the menu shows: the visible ones.
    pub fn shown(&self) -> impl Iterator<Item = &Self> {
        self.children.iter().filter(|entry| entry.visible)
    }

    /// The entry with `id`, here or in a submenu.
    #[must_use]
    pub fn find(&self, id: i32) -> Option<&Self> {
        if self.id == id {
            return Some(self);
        }
        self.children.iter().find_map(|child| child.find(id))
    }
}

/// A `label` as shown: `__` is one underscore, and any other underscore,
/// which marks the access key, is not shown.
#[must_use]
pub fn label(raw: &str) -> String {
    let mut shown = String::with_capacity(raw.len());
    let mut chars = raw.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '_' {
            shown.push(c);
        } else if chars.peek() == Some(&'_') {
            chars.next();
            shown.push('_');
        }
    }
    shown
}

/// A toggle from `toggle-type` and `toggle-state`: `checkmark` or `radio`,
/// 0 off, 1 on and anything else neither. `None` for an entry that does not
/// toggle.
#[must_use]
pub fn toggle(kind: &str, state: i32) -> Option<Toggle> {
    let radio = match kind {
        "checkmark" => false,
        "radio" => true,
        _ => return None,
    };
    let on = match state {
        0 => Some(false),
        1 => Some(true),
        _ => None,
    };
    Some(Toggle { radio, on })
}

#[cfg(test)]
mod tests {
    use super::{Entry, Toggle, label, toggle};

    #[test]
    fn a_label_loses_its_access_key_marks() {
        assert_eq!(label("_Quit"), "Quit");
        assert_eq!(label("Save __as"), "Save _as");
        assert_eq!(label("a_b__c_"), "ab_c");
        assert_eq!(label("___x"), "_x");
        assert_eq!(label(""), "");
    }

    #[test]
    fn a_toggle_reads_its_kind_and_state() {
        assert_eq!(
            toggle("checkmark", 1),
            Some(Toggle {
                radio: false,
                on: Some(true)
            })
        );
        assert_eq!(
            toggle("radio", 0),
            Some(Toggle {
                radio: true,
                on: Some(false)
            })
        );
        assert_eq!(toggle("radio", -1).and_then(|t| t.on), None);
        assert_eq!(toggle("", 1), None);
    }

    #[test]
    fn entries_default_shown_and_are_found_in_submenus() {
        let mut root = Entry::new(0);
        let mut sub = Entry::new(2);
        sub.children.push(Entry::new(3));
        root.children = vec![
            Entry::new(1),
            sub,
            Entry {
                visible: false,
                ..Entry::new(4)
            },
        ];
        assert!(Entry::new(1).clickable());
        assert!(
            !Entry {
                separator: true,
                ..Entry::new(1)
            }
            .clickable()
        );
        assert!(
            !Entry {
                enabled: false,
                ..Entry::new(1)
            }
            .clickable()
        );
        let shown: Vec<i32> = root.shown().map(|entry| entry.id).collect();
        assert_eq!(shown, [1, 2]);
        assert_eq!(root.find(3).map(|entry| entry.id), Some(3));
        assert_eq!(root.find(0).map(|entry| entry.id), Some(0));
        assert!(root.find(9).is_none());
    }
}
