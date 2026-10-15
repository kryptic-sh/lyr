# 10 — Popups and menus

Everything that opens out of the bar is a **popup**: tooltips, the calendar, the
power menu and tray menus. The draft plan treated these as details. They are a
platform feature that the windowing layer has to provide on every platform
([03](03-surfaces-and-input.md#window-kinds)), and so they are a milestone of
their own ([15](15-milestones.md)).

## Popup surfaces

A popup is anchored to a rectangle on a bar (the module that opened it). It
opens below the bar (above it for a bottom bar), is clamped to its output, and
closes when the pointer leaves it (tooltips), when a click lands outside it, or
on `Escape`.

All popups share one renderer and theme, and draw the same `DrawList`
([04](04-rendering-and-theme.md)) as the bar.

## Tooltips

Every module has one except notifications, which waybar's config turns off
(`"tooltip": false`). The text comes from the module's tooltip format, and it
shows after the platform's hover delay. The theme matches waybar's `tooltip`
block: background `#1e1e2e`, text `#cdd6f4`.

## Calendar

Shown while the pointer is over the clock ([07](07-modules.md#clock)), as a
popup that takes no input (`PopupInput::Passthrough`), so clicks and scrolls
still reach the clock, which switches the mode and shifts the range.

- Opens in **year** mode (`mode: "year"`): twelve months in 3 columns
  (`mode-mon-col: 3`). Month mode is a single month.
- ISO week numbers in a right-hand column (`weeks-pos: right`), prefixed `W`
  (`<b>W{}</b>`).
- Monospace and smaller than the bar's text, as waybar's `<tt><small>` makes it.
  The monospace family is the theme's `calendar_font`, defaulting to the bar
  font, which is proportional (Hack Nerd Font **Propo**), so the default needs
  checking against waybar's look in M5.
- Colours come from the waybar calendar format: months `#ffead3`, days
  `#ecc6d9`, weeks `#99ffdd`, weekdays `#ffcc66`, today `#ff6699` underlined.
  All bold.
- The first day of the week follows the system locale.

## Power menu

Opened from the power button, closed by a second click on it. Entries, in the
order and with the labels `.menu-power` uses today: Lock, Sleep, Hibernate,
Shutdown, Logout, Reboot. Each entry is a configured command or a built-in
action.

| Action    | Linux (logind over D-Bus)          | Windows                       | macOS                                  |
| --------- | ---------------------------------- | ----------------------------- | -------------------------------------- |
| Lock      | `loginctl lock-session` (hyprlock) | `LockWorkStation`             | `pmset displaysleepnow` (lock on wake) |
| Log out   | compositor exit (`hl.dsp.exit()`)  | `ExitWindowsEx(EWX_LOGOFF)`   | System Events `log out` (AppleScript)  |
| Sleep     | what `systemctl sleep` does        | `SetSuspendState(FALSE, ...)` | `pmset sleepnow`                       |
| Hibernate | `Manager.Hibernate`                | `SetSuspendState(TRUE, ...)`  | not offered                            |
| Reboot    | `Manager.Reboot`                   | `ExitWindowsEx(EWX_REBOOT)`   | System Events `restart`                |
| Power off | `Manager.PowerOff`                 | `ExitWindowsEx(EWX_POWEROFF)` | System Events `shut down`              |

Windows reboot and power-off need the `SE_SHUTDOWN_NAME` privilege enabled on
the process token first.

`.menu-power` sleeps with `systemctl sleep`, not `systemctl suspend`. That verb
(systemd 256 and later) picks suspend, hibernate or suspend-then-hibernate from
`sleep.conf`, so lyr's Sleep must do the same rather than call
`Manager.Suspend`. The logind method behind it is confirmed against the
installed systemd's introspection when M5 starts, not taken from memory.

## Menus

The current waybar delegates network, bluetooth and audio menus to
`krypt menu wifi|bluetooth|audio`, and the power button to `krypt menu power`.
Those are bash scripts (`dotfiles/.krypt/commands.toml`, `group = "menu"`) and
only exist on Linux.

Provisional default ([16](16-open-questions.md)):

- The **power menu** and **calendar** are built in on every platform, because
  they are small and fixed.
- **Network, bluetooth and audio** actions are a configurable command. On Linux
  the defaults are the current `krypt menu …` calls. On Windows the defaults
  open the matching Settings page (`ms-settings:network`,
  `ms-settings:bluetooth`, `ms-settings:sound`). On macOS they open the matching
  System Settings pane through its `x-apple.systempreferences:` URL. Built-in
  network, bluetooth and audio menus are a post-MVP option.
