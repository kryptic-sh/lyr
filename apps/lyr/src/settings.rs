//! Windows' Settings pages, which a menu lyr has not built there opens in
//! its place (`docs/plan/10-popups-and-menus.md#windows-and-macos`).

/// A Settings page a menu stands in for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    /// The Wi-Fi menu's.
    Network,
    /// The Bluetooth menu's.
    Bluetooth,
    /// The audio menu's.
    Sound,
}

impl Page {
    /// The page's `ms-settings:` URI.
    #[must_use]
    pub const fn uri(self) -> &'static str {
        match self {
            Self::Network => "ms-settings:network",
            Self::Bluetooth => "ms-settings:bluetooth",
            Self::Sound => "ms-settings:sound",
        }
    }

    /// The `cmd` command that opens it: `start` hands a URI to its handler,
    /// and its first quoted argument is the window title, left empty.
    #[must_use]
    pub fn command(self) -> String {
        format!("start \"\" {}", self.uri())
    }
}

/// Opens `page`, logging when it cannot.
#[cfg(windows)]
pub fn open(page: Page) {
    if let Err(error) = crate::exec::spawn(&page.command()) {
        tracing::warn!("{} did not open: {error}", page.uri());
    }
}

#[cfg(test)]
mod tests {
    use super::Page;

    #[test]
    fn each_menu_has_its_settings_page() {
        assert_eq!(Page::Network.uri(), "ms-settings:network");
        assert_eq!(Page::Bluetooth.uri(), "ms-settings:bluetooth");
        assert_eq!(Page::Sound.uri(), "ms-settings:sound");
        assert_eq!(Page::Sound.command(), r#"start "" ms-settings:sound"#);
    }
}
