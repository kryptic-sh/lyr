//! A tray item's menu, as its `com.canonical.dbusmenu` object lays it out:
//! each entry's properties as the interface documents them (libdbusmenu's
//! `dbus-menu.xml`, as GNUstep's `libs-dbuskit` carries it, read
//! 2026-10-20), with their defaults where an entry leaves one out, and
//! the rows a list menu shows for one.

use crate::list_menu::Row;

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
    pub children: Vec<Self>,
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

/// What a row of an item's menu stands for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pick {
    /// The entry with this id, clicked.
    Entry(i32),
    /// The submenu of the entry with this id, opened.
    Open(i32),
    /// Back to the menu this submenu is in.
    Back,
}

/// A toggle's mark before its label: a check mark or a dot when on.
const CHECKED: &str = "\u{2713} ";
const CHOSEN: &str = "\u{2022} ";
/// After a label whose entry opens a submenu.
const SUBMENU: &str = " \u{203A}";
/// The row that leaves a submenu.
const BACK: &str = "\u{2039} Back";

/// The rows of `menu`'s shown entries, for a list menu: a separator as a
/// separator, an entry that cannot be clicked as a row that cannot be
/// chosen, and one with children as a way into them. Inside a submenu
/// (`nested`), a Back row comes first.
#[must_use]
pub fn rows(menu: &Entry, nested: bool) -> Vec<Row<Pick>> {
    let mut rows = Vec::new();
    if nested {
        rows.push(Row::new(BACK, Pick::Back));
    }
    for entry in menu.shown() {
        if entry.separator {
            rows.push(Row {
                label: String::new(),
                value: None,
                separator: true,
            });
            continue;
        }
        let mark = match entry.toggle {
            Some(Toggle {
                radio: false,
                on: Some(true),
            }) => CHECKED,
            Some(Toggle {
                radio: true,
                on: Some(true),
            }) => CHOSEN,
            _ => "",
        };
        let submenu = !entry.children.is_empty();
        let tail = if submenu { SUBMENU } else { "" };
        rows.push(Row {
            label: format!("{mark}{}{tail}", entry.label),
            value: entry.clickable().then_some(if submenu {
                Pick::Open(entry.id)
            } else {
                Pick::Entry(entry.id)
            }),
            separator: false,
        });
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::{Entry, Pick, Toggle, label, rows, toggle};

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

    #[test]
    fn rows_mark_toggles_submenus_and_what_cannot_be_clicked() {
        let mut root = Entry::new(0);
        let mut more = Entry {
            label: "More".to_owned(),
            ..Entry::new(4)
        };
        more.children.push(Entry::new(5));
        root.children = vec![
            Entry {
                label: "Sync".to_owned(),
                toggle: Some(Toggle {
                    radio: false,
                    on: Some(true),
                }),
                ..Entry::new(1)
            },
            Entry {
                separator: true,
                ..Entry::new(2)
            },
            Entry {
                label: "Quit".to_owned(),
                enabled: false,
                ..Entry::new(3)
            },
            more,
            Entry {
                label: "Fast".to_owned(),
                toggle: Some(Toggle {
                    radio: true,
                    on: Some(true),
                }),
                ..Entry::new(6)
            },
            Entry {
                label: "Slow".to_owned(),
                toggle: Some(Toggle {
                    radio: true,
                    on: Some(false),
                }),
                ..Entry::new(7)
            },
        ];
        let shown: Vec<(String, Option<Pick>, bool)> = rows(&root, false)
            .into_iter()
            .map(|row| (row.label, row.value, row.separator))
            .collect();
        assert_eq!(
            shown,
            [
                ("\u{2713} Sync".to_owned(), Some(Pick::Entry(1)), false),
                (String::new(), None, true),
                ("Quit".to_owned(), None, false),
                ("More \u{203A}".to_owned(), Some(Pick::Open(4)), false),
                ("\u{2022} Fast".to_owned(), Some(Pick::Entry(6)), false),
                ("Slow".to_owned(), Some(Pick::Entry(7)), false),
            ]
        );
        let nested = rows(root.find(4).expect("More"), true);
        assert_eq!(nested[0].value, Some(Pick::Back));
        assert_eq!(nested[1].value, Some(Pick::Entry(5)));
        assert_eq!(rows(&root, false)[0].value, Some(Pick::Entry(1)), "no Back");
    }
}
