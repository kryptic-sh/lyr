# 05 — Layout and config

## Layout

Three zones per bar: **left**, **centre**, **right**. The centre zone is centred
on the bar regardless of how wide the side zones are (waybar's
`modules-center`). If the zones would overlap, the centre zone wins and the side
zones are clipped at their inner edge, with the clipped module dropping its text
before its icon.

Within a zone, modules sit in config order. Each one is a box with the theme's
padding, border and margins ([04](04-rendering-and-theme.md#theme)). A `spacer`
pseudo-module takes fixed width (`spacer = 20`) and `separator` draws a thin
rule, so that gaps and separators are config rather than code.

## Config file

TOML, loaded with `hjkl-config` from `~/.config/lyr/config.toml` (paths via
`hjkl-xdg`, the same on every platform, [01](01-overview.md#naming)).
`lyr -c <file>` overrides the path.

The **embedded defaults** are the waybar port. A user file is layered over them:
a table in the user file replaces the matching default table's keys, and a list
replaces the whole list. A missing user file means pure defaults.

```toml
[bar]
position = "top"        # top | bottom
height = 50
output = []             # empty = every output
layer = "top"           # Wayland only: background | bottom | top | overlay

[layout]
left = ["power", "battery", "power_profile", "network", "bluetooth"]
center = ["workspaces"]
right = ["tray", "volume", "mic", "clock", "notifications"]

[workspaces]
all_outputs = true
format = "{name}"

[clock]
format = "%Y/%m/%d %H:%M"

[volume]
format = "{icon} {volume}%"
format_muted = " {volume}%"
scroll_step = 5
on_click = "toggle_mute"
on_click_right = { exec = "krypt menu audio" }

[network]
on_click = { exec = "krypt menu wifi" }

[theme]
font = "Hack Nerd Font Propo"
font_size = 16
foreground = "#cdd6f4"
module_background = "#1a1b26"
module_opacity = 0.8
border = "#2bc5e3"
border_width = 2
radius = 8
```

This is a sketch of the shape, not the final schema. The schema lands with M3
and is then documented in the README.

**Actions** are either a built-in name (`toggle_mute`, `focus`, `cycle_profile`,
`open_calendar`, `open_power_menu`) or `{ exec = "…" }`. An `exec` runs through
the platform shell (`sh -c` on Linux and macOS, `cmd /C` on Windows) with no
console window on Windows. Like tmxr, lyr treats a config file as code: anything
with `exec` runs as the user (SECURITY.md).

### Per-platform defaults

The embedded defaults differ per platform only in the module lists and the
`exec` actions. On Windows and macOS, modules that cannot work there are left
out ([11](11-platforms.md)), and menu actions open the system settings pages
([10](10-popups-and-menus.md#menus)). One defaults file holds all three, as
`[platform.linux]`, `[platform.windows]` and `[platform.macos]` override tables
applied before the user file.

## Validation

The config is parsed fully before anything is applied. An unknown key, an
unknown module name, a module that is unsupported on this platform, or a bad
colour or format string is an error that names the file, the key path and the
reason. At start-up an invalid config is fatal. On reload it is logged and the
running config is kept.

## Reload

The config file is watched (inotify on Linux, `ReadDirectoryChangesW` on
Windows, FSEvents on macOS, through one watcher task). It is also reloaded on
`lyr reload` ([02](02-architecture.md#control-socket)). A reload diffs the old
and new configs:

- a theme or format change re-lays out and repaints;
- a layout or module change starts and stops only the affected sources;
- a bar change (position, height, outputs, layer) recreates the bar windows.

## Migration from waybar

The dotfiles change in M6 ([15](15-milestones.md)):

- `hyprland.lua`: `hl.exec_cmd("waybar")` becomes `hl.exec_cmd("lyr")`.
- `.config/waybar/` stays until lyr has run as the daily driver for a while,
  then it is removed. `swaync` stays ([09](09-notifications.md)).
- A `.config/lyr/config.toml` is only needed for differences from the defaults.
  With the defaults being the waybar port, that file starts empty.

A `lyr import-waybar` converter (JSON + CSS to TOML) is a backlog item, not MVP.
