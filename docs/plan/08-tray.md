# 08 — Tray

The current waybar tray uses `icon-size: 18` and `spacing: 10`. It holds the
icons of apps such as Steam and Teams, with left-click activating the app and
right-click opening that app's menu.

## Linux — StatusNotifierItem

The tray is the largest single source on Linux. Three parts are needed:

1. **A watcher.** Apps register with `org.kde.StatusNotifierWatcher`. If no
   other process owns that name, lyr owns it, the way waybar does. If one
   already does, lyr registers as a host with it instead.
2. **A host.** lyr registers `org.kde.StatusNotifierHost-<pid>`. For each item
   it reads `IconName`, `IconPixmap`, `IconThemePath`, `AttentionIconName`,
   `Status`, `ToolTip` and `Title`. It follows the `NewIcon`, `NewStatus`,
   `NewToolTip` and `NewTitle` signals and drops items whose bus name goes away.
3. **Menus.** Right-click uses the item's `com.canonical.dbusmenu` object
   (`GetLayout`, `Event`, `AboutToShow`, `LayoutUpdated`). The menu is drawn by
   lyr in a popup ([10](10-popups-and-menus.md#two-kinds-of-window)). Steam and
   Teams both depend on DBusMenu, so it is MVP for the tray, not a stretch.

**Icons** are found in waybar's order: while the item needs attention, its
`AttentionIconName` and then `AttentionIconPixmap`; then `IconName` (a theme
name or a file's path) and then `IconPixmap` (ARGB32, network byte order, the
size nearest what the host asks for). A name is looked up in the user's icon
theme with the XDG icon-theme lookup, `IconThemePath` first, PNG, SVG and XPM.
The lookup, decoding and rasterising come from crcbl's asset crate,
`crcbl-image` (S28, [03](03-surfaces-and-input.md#what-crcbl-must-provide)).
Each window resizes an icon to the pixels it covers, `tray.icon_size` tall with
its aspect kept. crcbl's SVG support is a subset that refuses an SVG it cannot
fully draw. An icon that is refused or not found gives way to the theme's
`image-missing`, and failing that the item's first letter, so an item is never
blank and stays clickable.

Clicks map to `Activate(x, y)`, `SecondaryActivate` (middle button) and
`Scroll(delta, orientation)`, with `ContextMenu` as the fallback when an item
has no DBusMenu.

## Windows and macOS — not available

- **Windows:** notification-area icons (`Shell_NotifyIcon`) belong to Explorer's
  taskbar. There is no supported way for another process to host them. Reading
  them out of Explorer's toolbar window is a fragile hack that breaks with
  taskbar updates, and it is not planned.
- **macOS:** menu extras belong to the system menu bar. There is no API to host
  or mirror them.

On both platforms the tray module is left out of the default layout and reports
"not supported on <os>" if a user config asks for it ([07](07-modules.md)). The
system tray stays where the OS puts it. That means the Windows taskbar, or the
macOS menu bar, has to stay visible while lyr runs (see
[03](03-surfaces-and-input.md#windows--appbar)).
