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
  it), Bluetooth (from BlueZ, with a connected device's battery) and the
  Hyprland workspaces. The other modules are not drawn yet.
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
  the Bluetooth menu; the other panels are not implemented yet. A second `lyr`
  exits when one is already running.
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
  asks Hyprland to exit; on Windows and macOS the entries are not built yet.
  `[power.menu]` sets an entry to a command instead
  (`lock = { exec = "hyprlock" }`).
- On Linux, the Bluetooth menu, opened by a click on the Bluetooth module or
  `lyr panel bluetooth`: the adapter's power, discoverable and pairable
  switches, a scan toggle, and each device with connect or disconnect, pair,
  trust, rename (a paired device) and forget. lyr is the pairing agent: a
  passkey to confirm shows at the top of the menu with Yes and No, and a PIN
  asked for by a device that pairs the legacy way is typed into it. The arrow
  keys move through its buttons and Enter presses the outlined one.
- The clock panel, opened by a click on the clock or `lyr panel clock`: waybar's
  year calendar, three months across with week numbers and today underlined; a
  right click switches to the month, scrolling moves through years or months,
  Escape closes it. Weeks start on Monday for now. On Linux, while lyr is the
  notification daemon, the notification centre sits below the calendar: Clear
  All, the do-not-disturb switch, the media players playing (title, artist,
  previous, play/pause, next) and the history, grouped by app, each entry with a
  close button while the pointer is over it.
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
