# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `lyr` with no command runs the bar: one transparent bar along the top of every
  output (a layer-shell panel on Wayland, an appbar on Windows), drawn through
  crcbl, with the power button and the clock module, and on Linux the battery
  (from UPower), the power profile (from power-profiles-daemon; a click cycles
  it), the network (from NetworkManager: the interface with the default route,
  its address and, on Wi-Fi, the network's name and signal), Bluetooth (from
  BlueZ, with a connected device's battery) and the Hyprland workspaces. The
  other modules are not drawn yet.
- On Windows the volume and mic modules show the default output and input, their
  level and mute, updated as they change, and their click and scroll bindings
  mute and step them.
- On Windows the power-profile module shows Windows' power mode (Best power
  efficiency, Balanced, Best performance and the rest), updated when it changes;
  switching it from the bar is not built there yet.
- On Windows the network module shows the interface with the default route, its
  address and, on Wi-Fi, the network's name and signal, updated as they change.
- On Windows the battery module shows the battery, from Windows' power status,
  updated when the power source or the charge changes.
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
  asks Hyprland to exit; on Windows each entry is its Win32 call (locking,
  sleep, hibernate, log out, reboot, shut down); on macOS the entries are not
  built yet. `[power.menu]` sets an entry to a command instead
  (`lock = { exec = "hyprlock" }`).
- `lyr import-waybar FILE` prints a waybar `config.jsonc` as a lyr config: the
  bar's settings, the layout and the modules lyr has, with what it could not
  carry listed at the top. waybar's `style.css` is not read.
- A `window` module, not in the default layout: the focused window's title on
  Hyprland, as waybar's `hyprland/window` shows it, with `{title}` and `{class}`
  placeholders and `max_length`.
- On Linux, the tray: lyr hosts StatusNotifierItems (Steam, Teams and the like),
  and serves `org.kde.StatusNotifierWatcher` when nothing else does. Each item
  shows as a button with its title's first letter until lyr draws icons, passive
  items hidden; a click activates it, a middle click secondary-activates it, the
  wheel scrolls it, and a right click (or a left one on an item that is only a
  menu) opens its menu under the button, with submenus, check marks and disabled
  entries, or its own menu when it serves none.
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
- The clock panel, opened by a click on the clock or `lyr panel clock`: waybar's
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
  filters it). The file is capped at 10 MiB: past it, it becomes `lyr.log.1`,
  replacing the one before.
- The config file `~/.config/lyr/config.toml` (`-c` to choose another), layered
  over built-in defaults that reproduce the waybar setup lyr replaces, with
  `[platform.linux]`, `[platform.windows]` and `[platform.macos]` override
  tables. An invalid config is an error naming the key, such as
  ``theme.border: `teal` is not a colour``.
