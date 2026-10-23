//! The system's Settings pages, which a menu lyr has not built on Windows or
//! macOS opens in its place, and which stand in for what Windows has no
//! public API for (`docs/plan/10-popups-and-menus.md#windows-and-macos`).

use lyr_config::Platform;

/// A Settings page a menu stands in for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    /// The Wi-Fi menu's.
    Network,
    /// The Bluetooth menu's.
    Bluetooth,
    /// The power-profile module's clicks on Windows, where switching the
    /// power mode has no public API (`docs/backlog.md`).
    #[cfg(any(windows, test))]
    Power,
}

impl Page {
    /// The page's `ms-settings:` URI on Windows.
    const fn windows_uri(self) -> &'static str {
        match self {
            Self::Network => "ms-settings:network",
            Self::Bluetooth => "ms-settings:bluetooth",
            #[cfg(any(windows, test))]
            Self::Power => "ms-settings:powersleep",
        }
    }

    /// The page's System Settings URL on macOS 13 and later: the pane's
    /// extension id under `x-apple.systempreferences:`. The ids are the ones
    /// Apple's community forums and admins' pane tables give for Ventura
    /// (2023); Apple does not document them, and no Mac here has opened them.
    const fn macos_url(self) -> &'static str {
        match self {
            Self::Network => "x-apple.systempreferences:com.apple.wifi-settings-extension",
            Self::Bluetooth => "x-apple.systempreferences:com.apple.BluetoothSettings",
            #[cfg(any(windows, test))]
            Self::Power => "x-apple.systempreferences:com.apple.Battery-Settings.extension",
        }
    }

    /// The shell command that opens it on `platform`: `cmd`'s `start` hands
    /// a URI to its handler (its first quoted argument is the window title,
    /// left empty), and macOS's `open` hands a URL to its app. `None` on
    /// Linux, where lyr draws its own menus and has no Settings to send to.
    #[must_use]
    pub fn command(self, platform: Platform) -> Option<String> {
        match platform {
            Platform::Windows => Some(format!("start \"\" {}", self.windows_uri())),
            Platform::Macos => Some(format!("open '{}'", self.macos_url())),
            Platform::Linux => None,
        }
    }
}

/// Opens `page`, logging when it cannot.
#[cfg(any(windows, target_os = "macos"))]
pub fn open(page: Page) {
    let Some(command) = page.command(Platform::current()) else {
        return;
    };
    if let Err(error) = crate::exec::spawn(&command) {
        tracing::warn!("`{command}` did not start: {error}");
    }
}

#[cfg(test)]
mod tests {
    use lyr_config::Platform;

    use super::Page;

    #[test]
    fn each_menu_has_its_windows_settings_page() {
        let command = |page: Page| page.command(Platform::Windows);
        assert_eq!(
            command(Page::Network).as_deref(),
            Some(r#"start "" ms-settings:network"#)
        );
        assert_eq!(
            command(Page::Bluetooth).as_deref(),
            Some(r#"start "" ms-settings:bluetooth"#)
        );
        assert_eq!(
            command(Page::Power).as_deref(),
            Some(r#"start "" ms-settings:powersleep"#)
        );
    }

    #[test]
    fn each_menu_has_its_macos_settings_pane() {
        let command = |page: Page| page.command(Platform::Macos);
        assert_eq!(
            command(Page::Network).as_deref(),
            Some("open 'x-apple.systempreferences:com.apple.wifi-settings-extension'")
        );
        assert_eq!(
            command(Page::Bluetooth).as_deref(),
            Some("open 'x-apple.systempreferences:com.apple.BluetoothSettings'")
        );
    }

    #[test]
    fn linux_has_no_settings_to_open() {
        assert_eq!(Page::Network.command(Platform::Linux), None);
    }
}
