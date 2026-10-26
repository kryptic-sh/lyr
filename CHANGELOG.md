# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-10-26

### Breaking

- For the library crates: `lyr_core::toast::draw` returns its action buttons and
  links as `toast::Area`s, `lyr_core::markup::Style::link` is the link's index
  (`Option<u32>`), `lyr_core::draw::Content::Buttons` takes a `padding`,
  `lyr_core::notifications::Incoming` gains `icon` and `image`, and
  `lyr_sources::notifications::linux::Daemon` is built with `Daemon::new`.

### Added

- Notification toasts and the notification centre draw the notification's image
  (`image-data`, `image-path` or `icon_data`) and the app's icon, laid out as
  swaync draws them with the dotfiles' style. The app's icon is `app_icon`, else
  the icon of the app's desktop entry (found by the `desktop-entry` hint or the
  app's name). Pictures may be PNG, JPEG, SVG or XPM. lyr now advertises
  `icon-static`.
- A click on a link in a notification opens it in the browser through
  `xdg-open`, for `http` and `https` links only; other schemes are not opened.
  The toast stays up. lyr now advertises `body-hyperlinks`.

### Changed

- Built on crcbl 0.11.0 (from 0.9.0). Icon theme lookups are faster and take the
  icon nearest the size drawn.
- Dependencies at their latest releases: `hjkl-config` and `hjkl-xdg` 0.43,
  `tokio` 1.54, `zbus` 6.1, `interprocess` 2.4.7. `windows-core` and
  `windows-collections` stay at the versions `windows` 0.62 (its latest) is
  built on, since 0.100 of either alone would split the Windows types in two.

### Fixed

- The tray's icons are `[tray] spacing` apart, as waybar's are. They also took
  the workspace buttons' `workspace_padding_x` on each side, which put the
  default 10 px spacing at 30 px.

## [0.1.0] - 2026-10-26

### Added

- Release builds for Linux (x86_64 and aarch64, glibc 2.28+), macOS 11+ (Apple
  silicon and Intel) and Windows (x86_64) on GitHub Releases, published as
  `lyr-bin` on the AUR and crates.io (the binary is `lyr`; the crates.io name
  `lyr` is another project's), `lyr` in the `kryptic-sh/tap` Homebrew tap, and
  `lyr` in the `kryptic-sh` Scoop bucket.
- `lyr` with no command runs the bar: one transparent bar along the top of every
  output (a layer-shell panel on Wayland, an appbar on Windows), drawn through
  crcbl, with the power button and the clock module, and on Linux the battery
  (from UPower), the power profile (from power-profiles-daemon; a click cycles
  it), the network (from NetworkManager: the interface with the default route,
  its address and, on Wi-Fi, the network's name and signal), Bluetooth (from
  BlueZ, with a connected device's battery) and the workspaces, from Hyprland,
  sway or niri, whichever the session runs under. The other modules are not
  drawn yet.
- Each bar, menu, panel and toast presents only when the window system asks for
  its next frame (crcbl 0.7.0's frame gate), so an output the compositor has
  turned off cannot block the bar.
- Every bar, menu, panel and toast draws on one GPU device (crcbl 0.9.0's
  `SharedGpu`) instead of a device each, so removing a monitor no longer crashes
  NVIDIA's Wayland driver.
- The toast stack, the menus and, on a reload that changes `[bar]`, the bars
  change size and place in place (crcbl 0.7.0's `set_panel`) instead of closing
  and reopening their windows.
- A toast's body wraps where Unicode's line breaking rules allow (UAX #14,
  through crcbl-ui's `font::breaks`): after a hyphen and between ideographs as
  well as at spaces, and at every kind of line break.
- On Windows the notifications module shows a bell, and either click opens
  Windows' notification centre.
- On Windows the Bluetooth menu switches the radio, scans, pairs (asking for a
  confirmation or PIN in the menu) and forgets devices; a paired device's
  Settings button opens Settings, where Windows connects it. Without an adapter
  the menu opens Settings' Bluetooth page, and the Wi-Fi menu its network page
  without a wireless interface.
- On Windows the audio menu lists the playing and recording programs and the
  output and input devices, each with mute, a volume slider and steps; a
  device's Default button makes it the default. A right click on the volume or
  mic module, or `lyr panel audio`, opens it. Windows' volume, mic and audio
  menu read through crcbl's mixer (`crcbl-mixer`), as Linux's do, and hide while
  the audio service is away, coming back when it restarts.
- On Linux the volume and mic modules show the default output and input through
  crcbl's mixer (`crcbl-mixer`), which speaks PipeWire's own protocol, or
  PulseAudio's where PipeWire is not running, so nothing beside the sound server
  needs installing; they update as they change, hide while the sound server is
  away and come back when it restarts, a click mutes and a scroll steps the
  volume, up to 150 %.
- On Linux the audio menu lists the playing and recording streams and the output
  and input devices through crcbl's mixer, each with mute, a slider to 150 %
  (wiremix's scale) and steps, a device's Default button makes it the default, a
  stream's device button opens a dropdown of the devices to move it to, and a
  Configuration tab chooses a sound card's profile and a device's port from
  dropdowns; a right click on the volume or mic module, or `lyr panel audio`,
  opens it.
- On Windows the volume and mic modules show the default output and input, their
  level and mute, updated as they change, and their click and scroll bindings
  mute and step them; an output on Bluetooth (by its device's bus) takes the
  volume module's Bluetooth format.
- On Windows the power-profile module shows Windows' power mode (Best power
  efficiency, Balanced, Best performance and the rest), updated when it changes;
  a click opens Settings' Power page, where Windows switches it.
- On Windows the network module shows the interface with the default route, its
  address and, on Wi-Fi, the network's name and signal, updated as they change;
  with no link and every Wi-Fi radio switched off, it shows Wi-Fi as off rather
  than disconnected.
- On Windows the Wi-Fi menu is native: it lists the networks Native Wifi sees,
  scans, switches the radio, joins (writing a profile for a new network, or a
  new password for a saved one), leaves and forgets networks. A machine without
  a wireless interface or the WLAN service opens Settings instead.
- On Windows the bluetooth module shows the adapter's state and the device that
  connected last, from WinRT, updated as the radio switches and devices connect,
  disconnect, pair or unpair.
- On Windows the battery module shows the battery, from Windows' power status,
  updated when the power source or the charge changes.
- On macOS the battery module shows the internal battery, from IOKit's power
  sources, updated when macOS reports a power source change; plugged in and held
  below full (optimized charging) shows as plugged.
- On macOS the volume and mic modules show the default output and input, their
  level and mute, from CoreAudio, updated as they change; their click and scroll
  bindings mute and step the device. A device with only per-channel volumes,
  such as the built-in output, is read as their mean and set on each channel.
- On macOS the power-profile module shows Low Power Mode, as power-saver while
  it is on, updated when it turns on or off. It is read-only there.
- On macOS the network module shows the primary interface, its address, prefix
  and router, and on Wi-Fi the network's name and signal (waybar's scale from
  dBm), updated when the system's network state changes. Without Location
  permission macOS withholds the name, and the link still shows as Wi-Fi; with
  the Wi-Fi radio off and no link, it shows as disabled.
- On macOS the workspaces and window modules follow AeroSpace: its workspaces in
  its order, the focused one active, and the focused window's app and title; a
  click switches through `aerospace workspace`. AeroSpace's config runs
  `lyr event workspace` from `on-focus-changed` to have lyr read them again.
- `lyr event workspace` tells the running bar that something outside it changed
  the workspaces; only macOS acts on it.
- On macOS the Bluetooth module shows the controller's power and the last device
  to connect, from IOBluetooth, updated when the controller powers on or off or
  a device connects or disconnects.
- On macOS the Wi-Fi and Bluetooth menus open System Settings' Wi-Fi and
  Bluetooth panes, as Windows opens its Settings pages.
- On macOS the audio menu lists every output and input device with mute, a
  volume slider and steps, and a device's Default button makes it the default
  through CoreAudio. macOS has no per-program volume, so the Playback and
  Recording tabs are empty.
- Clicking a workspace switches to it, and with `workspaces.scroll = true` the
  wheel steps through them. A module's `[<module>.on]` table binds clicks and
  scrolling to a command (`{ exec = "…" }`) or a built-in action. Of the
  built-ins, `cycle_profile`, `cycle_profile_back`, `open_power_menu` and
  `toggle_dnd` work so far.
- `lyr quit` stops the running bar, and `lyr reload` reloads its config, keeping
  the running one when the new one is invalid, and `lyr toggle` hides and shows
  the bars. On Linux, while lyr is the notification daemon, `lyr dnd` turns
  do-not-disturb on or off and `lyr clear` dismisses every notification.
  `lyr panel clock` opens the clock panel and, on Linux, `lyr panel bluetooth`
  the Bluetooth menu and `lyr panel wifi` the Wi-Fi menu; the audio menu is not
  implemented yet. A second `lyr` exits when one is already running.
- On Linux lyr is the notification daemon when no other daemon (such as swaync)
  holds the name; `notifications.replace = true` takes it over. It advertises
  the `actions`, `body`, `body-markup` and `persistence` capabilities: a body's
  bold, italic, underlined and linked text is drawn so (a link is not opened),
  and `&amp;` and the other entities read as the characters they stand for.
  Notifications show as toasts at the top right, swaync's look, timed out as
  swaync times them, held up while the pointer is over them; a click runs the
  default action, a button under the body runs its action, and a middle click,
  or the close button the toast under the pointer shows, dismisses. The bell
  module shows whether any are held and whether do-not-disturb is on.
- On Linux a change of time zone (`timedatectl set-timezone`) redraws the clock
  at once.
- Tooltips on the power button, clock (the time zone and its offset), battery,
  power profile and Bluetooth modules, shown after half a second on a module and
  quickly when moving from one to the next, as waybar's GTK tooltips are.
- A click on the power button opens the power menu (Lock, Sleep, Hibernate,
  Shutdown, Logout, Reboot), driven by the arrow keys and Enter or the pointer.
  On Linux each entry asks logind as `systemctl` and `loginctl` do, and Logout
  asks the compositor (Hyprland, sway or niri) to end the session; on Windows
  each entry is its Win32 call (locking, sleep, hibernate, log out, reboot, shut
  down); on macOS Lock and Sleep are `pmset displaysleepnow` and
  `pmset sleepnow`, and Logout, Reboot and Shutdown ask System Events, with no
  Hibernate. `[power.menu]` sets an entry to a command instead
  (`lock = { exec = "hyprlock" }`).
- The clock panel's calendar starts its weeks on the locale's first weekday: on
  Linux and the BSDs from the region of the locale `LC_TIME` follows, through
  CLDR's week data and without running a program (Sunday for `en_US`, numbering
  weeks as waybar's `%U` then), on Windows and macOS from the system's setting.
- `[tray] spacing` sets the gap between the tray's items, 10 px by default as in
  the dotfiles' waybar, and `lyr import-waybar` carries waybar's `spacing`.
- `lyr import-waybar FILE` prints a waybar `config.jsonc` as a lyr config: the
  bar's settings, the layout and the modules lyr has, with what it could not
  carry listed at the top; waybar's Hyprland, sway and niri workspace and window
  modules all become lyr's. waybar's `style.css` is not read.
- A `window` module, not in the default layout: the focused window's title on
  Hyprland, sway and niri, as waybar's `hyprland/window` shows it, and on
  Windows the foreground window's, its program's name as `{class}`, with
  `{title}` and `{class}` placeholders and `max_length`.
- On Linux, the tray: lyr hosts StatusNotifierItems (Steam, Teams and the like),
  and serves `org.kde.StatusNotifierWatcher` when nothing else does. Each item
  shows as its icon, `tray.icon_size` tall (18 by default, and carried by
  `lyr import-waybar` from waybar's `icon-size`): its theme icon, looked up in
  the GTK icon theme and its own theme path, or the pixels it sends, its
  attention icon while it needs attention, and the theme's missing-image icon
  when it has none. Passive items are hidden; a click activates it, a middle
  click secondary-activates it, the wheel scrolls it, and a right click (or a
  left one on an item that is only a menu) opens its menu under the button, with
  submenus, check marks and disabled entries, or its own menu when it serves
  none.
- On Linux, the Wi-Fi menu, opened by a click on the network module or
  `lyr panel wifi`, over NetworkManager: a switch for the radio, the networks a
  scan found (one row per name, strongest first, the one in use marked), a fresh
  scan asked for on every open, Connect, which joins a saved network at once and
  asks for a new one's password in a masked field, and Forget, after a
  confirmation. WEP and enterprise networks are refused with a message. A saved
  network whose password fails is offered a new one; a new profile that fails on
  its password, or an open one that fails, is deleted. A failure after the menu
  closed comes as a notification.
- On Linux, the Bluetooth menu, opened by a click on the Bluetooth module or
  `lyr panel bluetooth`: the adapter's power, discoverable and pairable
  switches, a scan toggle, and each device with connect or disconnect, pair,
  trust, rename (a paired device) and forget. lyr is the pairing agent: a
  passkey to confirm shows at the top of the menu with Yes and No, and a PIN
  asked for by a device that pairs the legacy way is typed into it. The arrow
  keys move through its buttons and Enter presses the outlined one.
- The clock panel, opened by a click on the clock or `lyr panel clock` under the
  clock, as big as the calendar view it shows (the year or a month): waybar's
  year calendar, three months across with week numbers, in bold and today
  underlined, smaller as waybar's `<tt><small>` draws it and in
  `theme.calendar_font` when that is set; a right click switches to the month,
  scrolling moves through years or months, Escape closes it. Weeks start on
  Monday for now. On Linux, while lyr is the notification daemon, the
  notification centre sits below the calendar: Clear All, the do-not-disturb
  switch, the media players playing (title, artist, previous, play/pause, next)
  and the history, grouped by app, each entry with a close button while the
  pointer is over it.
- `lyr --stats` logs the bar's main-loop wake-ups, the events it applied and the
  frames it presented on each output, once a minute, to check that an idle bar
  stays idle.
- The bar logs to `~/.local/state/lyr/logs/lyr.log` and to stderr (`RUST_LOG`
  filters it), the window and GPU layers' messages with its own. The file is
  capped at 10 MiB: past it, it becomes `lyr.log.1`, replacing the one before.
- The config file `~/.config/lyr/config.toml` (`-c` to choose another), layered
  over built-in defaults that reproduce the waybar setup lyr replaces, with
  `[platform.linux]`, `[platform.windows]` and `[platform.macos]` override
  tables. An invalid config is an error naming the key, such as
  ``theme.border: `teal` is not a colour``.

[Unreleased]: https://github.com/kryptic-sh/lyr/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/kryptic-sh/lyr/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/kryptic-sh/lyr/releases/tag/v0.1.0
