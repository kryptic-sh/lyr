//! A list menu: rows of text in the bar's module box, one selected, moved
//! with the arrow keys or the pointer. The power menu and a tray item's menu
//! are list menus. A row may be a separator or disabled, and neither can be
//! selected.

use lyr_config::Theme;

use crate::draw::{DrawList, Item, Size, TextStyle};
use crate::hit::{Rect, Regions};

/// The space between the menu's edge and its rows.
pub const PADDING: f32 = 8.0;
/// The space around a row's label, across and down.
pub const ROW_PADDING: (f32, f32) = (12.0, 6.0);
/// How faint a disabled row's label is: lyr's own choice.
pub const DISABLED_OPACITY: f32 = 0.5;

/// One row: its label, and what choosing it stands for, `None` when it
/// cannot be chosen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row<T> {
    pub label: String,
    pub value: Option<T>,
    /// A line between groups of rows, with no label.
    pub separator: bool,
}

impl<T> Row<T> {
    /// A row that can be chosen.
    pub fn new(label: impl Into<String>, value: T) -> Self {
        Self {
            label: label.into(),
            value: Some(value),
            separator: false,
        }
    }
}

/// An open menu: its rows and the one selected.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Menu<T> {
    rows: Vec<Row<T>>,
    /// The selected row's index; `None` when no row can be chosen.
    selected: Option<usize>,
}

impl<T: Clone + PartialEq> Menu<T> {
    /// A menu of `rows`, the first that can be chosen selected. `None` with
    /// no rows.
    #[must_use]
    pub fn new(rows: Vec<Row<T>>) -> Option<Self> {
        let selected = rows.iter().position(|row| row.value.is_some());
        (!rows.is_empty()).then_some(Self { rows, selected })
    }

    /// What the selected row stands for.
    #[must_use]
    pub fn selected(&self) -> Option<&T> {
        self.rows[self.selected?].value.as_ref()
    }

    /// Moves the selection to the row above that can be chosen, stopping at
    /// the top.
    pub fn up(&mut self) {
        if let Some(at) = self.selected {
            self.selected = Some(
                self.rows[..at]
                    .iter()
                    .rposition(|row| row.value.is_some())
                    .unwrap_or(at),
            );
        }
    }

    /// Moves the selection to the row below that can be chosen, stopping at
    /// the bottom.
    pub fn down(&mut self) {
        if let Some(at) = self.selected {
            self.selected = Some(
                self.rows[at + 1..]
                    .iter()
                    .position(|row| row.value.is_some())
                    .map_or(at, |below| at + 1 + below),
            );
        }
    }

    /// Selects the row standing for `value`, as the pointer moving onto it
    /// does. Returns whether the selection changed.
    pub fn select(&mut self, value: &T) -> bool {
        match self
            .rows
            .iter()
            .position(|row| row.value.as_ref() == Some(value))
        {
            Some(at) if Some(at) != self.selected => {
                self.selected = Some(at);
                true
            }
            _ => false,
        }
    }

    /// The menu's draw list, its size and its rows' input regions, its text
    /// measured by `measure` in lines `line_height` tall.
    #[must_use]
    pub fn draw(
        &self,
        measure: impl Fn(&str) -> f32,
        line_height: f32,
        theme: &Theme,
    ) -> (DrawList, Size, Regions<T>) {
        let (across, down) = ROW_PADDING;
        let text_width = self
            .rows
            .iter()
            .map(|row| measure(&row.label))
            .fold(0.0, f32::max);
        let row_width = text_width + 2.0 * across;
        let row_height = line_height + 2.0 * down;
        // A separator is as tall as a row's padding.
        let height = |row: &Row<T>| {
            if row.separator {
                2.0 * down
            } else {
                row_height
            }
        };
        let size = Size {
            width: row_width + 2.0 * PADDING,
            height: self.rows.iter().map(height).sum::<f32>() + 2.0 * PADDING,
        };
        let mut items = vec![Item::Box {
            rect: Rect {
                x: 0.0,
                y: 0.0,
                width: size.width,
                height: size.height,
            },
            radius: theme.radius as f32,
            fill: theme.module_background,
            border: Some((theme.border_width as f32, theme.border)),
        }];
        let mut regions = Regions::default();
        let mut y = PADDING;
        for (index, row) in self.rows.iter().enumerate() {
            let rect = Rect {
                x: PADDING,
                y,
                width: row_width,
                height: height(row),
            };
            y += rect.height;
            if row.separator {
                let line = theme.border_width as f32;
                items.push(Item::Box {
                    rect: Rect {
                        x: rect.x + across,
                        y: rect.y + (rect.height - line) / 2.0,
                        width: text_width,
                        height: line,
                    },
                    radius: 0.0,
                    fill: theme.border,
                    border: None,
                });
                continue;
            }
            let label = Item::Text {
                rect: Rect {
                    x: rect.x + across,
                    y: rect.y + down,
                    width: text_width,
                    height: line_height,
                },
                text: row.label.clone(),
                colour: if Some(index) == self.selected {
                    theme.workspace_active
                } else {
                    theme.foreground
                },
                style: TextStyle::PLAIN,
            };
            match &row.value {
                Some(value) => {
                    items.push(label);
                    regions.push(rect, value.clone());
                }
                None => items.push(Item::Group {
                    opacity: DISABLED_OPACITY,
                    items: vec![label],
                }),
            }
        }
        (DrawList { items }, size, regions)
    }
}

#[cfg(test)]
mod tests;
