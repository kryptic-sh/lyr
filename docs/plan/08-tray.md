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

**Icons** come from `IconPixmap` (ARGB32, network byte order, choosing the size
nearest 18 px × scale) when it is set. Otherwise lyr looks `IconName` up in the
icon theme: the XDG icon-theme lookup, `IconThemePath` first, PNG and SVG. The
lookup, decoding and rasterising come from crcbl's asset crate (S28,
[03](03-surfaces-and-input.md#what-crcbl-must-provide)), and each size is
rasterised once.

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
