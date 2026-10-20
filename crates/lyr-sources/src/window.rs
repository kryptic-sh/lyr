//! The window module's source on platforms whose window manager is not a
//! workspace backend (`docs/plan/07-modules.md`): [`windows`] reads the
//! foreground window. On Linux the module follows Hyprland, through
//! `lyr_wm`.

#[cfg(windows)]
pub mod windows;

/// The focused window: its title, and its class, which on Windows is its
/// program's name, as Hyprland's class is usually the program's.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Active {
    pub class: String,
    pub title: String,
}
