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

Every module has one. The text comes from the module's tooltip format, and it
shows after the platform's hover delay. The theme matches waybar's `tooltip`
block: background `#1e1e2e`, text `#cdd6f4`.

## Calendar

Opened from the clock ([07](07-modules.md#clock)).

- A month grid with ISO week numbers in a right-hand column. The current waybar
  uses `weeks-pos: right`, and its year mode has 3 columns (`mode-mon-col: 3`).
- A header with `<` / `>` buttons for the month and the year. Scroll moves by a
  month, and a year view is one click on the header.
- Colours come from the waybar calendar format: months `#ffead3`, days
  `#ecc6d9`, weeks `#99ffdd`, weekdays `#ffcc66`, today `#ff6699` underlined.
  All bold.
- The first day of the week follows the system locale.

## Power menu

Opened from the power button. Entries are lock, log out, suspend, hibernate,
reboot and power off. Each entry is a configured command or a built-in action.

| Action    | Linux (logind over D-Bus)          | Windows                       | macOS                                  |
| --------- | ---------------------------------- | ----------------------------- | -------------------------------------- |
| Lock      | `loginctl lock-session` (hyprlock) | `LockWorkStation`             | `pmset displaysleepnow` (lock on wake) |
| Log out   | compositor exit (`hl.dsp.exit()`)  | `ExitWindowsEx(EWX_LOGOFF)`   | System Events `log out` (AppleScript)  |
| Suspend   | `Manager.Suspend`                  | `SetSuspendState(FALSE, ...)` | `pmset sleepnow`                       |
| Hibernate | `Manager.Hibernate`                | `SetSuspendState(TRUE, ...)`  | not offered                            |
| Reboot    | `Manager.Reboot`                   | `ExitWindowsEx(EWX_REBOOT)`   | System Events `restart`                |
| Power off | `Manager.PowerOff`                 | `ExitWindowsEx(EWX_POWEROFF)` | System Events `shut down`              |

Windows reboot and power-off need the `SE_SHUTDOWN_NAME` privilege enabled on
the process token first.

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
