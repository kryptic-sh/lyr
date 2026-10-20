# 05 — Layout and config

## Layout

Three zones per bar: **left**, **centre**, **right**. The centre zone is centred
on the bar regardless of how wide the side zones are (waybar's
`modules-center`). If the zones would overlap, the centre zone wins and the side
zones are clipped at their inner edge, with the clipped module dropping its text
before its icon. Once one module does not fit, the ones after it (nearer the
centre) are dropped too, so a zone never has a gap. With an empty centre zone,
the inner edge is the bar's middle. This is `lyr_core::layout::layout`, which
takes each module's full and icon-only widths.

Within a zone, modules sit in config order. Each one is a box with the theme's
padding, border and margins ([04](04-rendering-and-theme.md#theme)). A `spacer`
pseudo-module takes fixed width (`spacer = 20`) and `separator` draws a thin
rule, so that gaps and separators are config rather than code.

## Config file

TOML, loaded with `hjkl-config` from `~/.config/lyr/config.toml` (paths via
`hjkl-xdg`, the same on every platform, [01](01-overview.md#naming)).
`lyr -c <file>` overrides the path.

The **embedded defaults** are the waybar port
(`crates/lyr-config/src/defaults.toml`). A user file is layered over them: a
table in the user file replaces the matching default table's keys, and a list
replaces the whole list. A missing user file means pure defaults. The order is:
the defaults, the defaults' `[platform.<os>]` table, the user file, then the
user file's own `[platform.<os>]` table, so one user file can serve every
machine.

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
format_muted = "\uEEE8 {volume}%"
scroll_step = 5
on_click = "toggle_mute"
on_click_right = "open_audio_menu"

[network]
on_click = "open_wifi_menu"

[theme]
font = "Hack Nerd Font Propo"
font_size = 16
foreground = "#cdd6f4"
module_background = "#1a1b26"
module_opacity = 0.8  # the whole box, text included, like CSS opacity
border = "#2bc5e3"
border_width = 2
radius = 8
```

This is a sketch of the shape. The `[bar]`, `[layout]`, `[workspaces]`,
`[clock]` and `[theme]` tables are built (`lyr-config`'s `schema.rs`), with the
theme's box metrics and colours as separate keys (`bar_background`,
`module_padding_x`, `workspace_active`, …) holding plan 04's values. A zone
entry is a module name, `"separator"` or `{ spacer = 20 }`. The other modules'
tables come with M4, and the schema is documented in the README when M3 ends.

**Actions** are bound in a module's `[<module>.on]` table, by pointer action
(`click`, `click_right`, `click_middle`, `scroll_up`, `scroll_down`). Each is
either a built-in name (`toggle_mute`, `volume_up`, `volume_down`,
`cycle_profile`, `cycle_profile_back`, `open_clock_panel`, `open_power_menu`,
`open_wifi_menu`, `open_bluetooth_menu`, `open_audio_menu`, `toggle_dnd`) or
`{ exec = "…" }`. A built-in acts on the module it is bound in, so `toggle_mute`
on the mic mutes the microphone, and one that means nothing there (`volume_up`
on the clock) is a config error. `volume_up` and `volume_down` step by the
module's `scroll_step`. A workspace button always focuses its workspace, and
`workspaces.scroll` turns scrolling on. An `exec` runs through the platform
shell (`sh -c` on Linux and macOS, `cmd /C` on Windows) with no console window
on Windows. Like tmxr, lyr treats a config file as code: anything with `exec`
runs as the user (SECURITY.md).

### Per-platform defaults

The embedded defaults differ per platform only in the module lists and the
`exec` actions. On Windows and macOS, modules that cannot work there are left
out ([11](11-platforms.md)), and a menu with no native version there opens the
system settings page ([10](10-popups-and-menus.md#windows-and-macos)). One
defaults file holds all three, as `[platform.linux]`, `[platform.windows]` and
`[platform.macos]` override tables applied before the user file.

## Validation

The config is parsed fully before anything is applied. An unknown key, an
unknown module name, a module that is unsupported on this platform, or a bad
colour or format string is an error that names the file, the key path and the
reason, such as ``theme.border: `teal` is not a colour`` or
``layout.right[0]: the `tray` module is not supported on windows``. Errors serde
raises get their key from `serde_path_to_error`. At start-up an invalid config
is fatal. On reload it is logged and the running config is kept.

## Reload

The config file is watched (inotify on Linux, `ReadDirectoryChangesW` on
Windows, FSEvents on macOS, through one watcher task). It is also reloaded on
`lyr reload` ([02](02-architecture.md#control-socket)). A reload diffs the old
and new configs:

- a theme or format change re-lays out and repaints;
- a layout or module change starts and stops only the affected sources;
- a bar change (position, height, layer) changes the bar windows in place
  (`Shell::set_panel`), and a change of outputs adds or removes bars.

## Migration from waybar

The dotfiles change in M8 ([15](15-milestones.md)):

- `hyprland.lua`: `hl.exec_cmd("waybar")` becomes `hl.exec_cmd("lyr")`, and
  `hl.exec_cmd("swaync")` goes.
- `hyprland.lua` bindings: `mod+N`, `mod+shift+N` and `mod+alt+N` run
  `lyr panel clock`, `lyr dnd` and `lyr clear` instead of `swaync-client`
  (`apps.notify` in `apps.template.lua`). `mod+W`, `mod+A` and `mod+U` run
  `lyr panel wifi`, `lyr panel audio` and `lyr panel bluetooth` instead of
  `krypt menu …`.
- `.krypt/commands.toml`: the `menu` group's `wifi`, `audio` and `bluetooth`
  commands go, with `.local/bin/.menu-wifi`, or become thin `lyr panel …`
  wrappers if the owner wants the `krypt menu` names kept. `swaync-test` keeps
  its `notify-send` calls and drops its swaync restart.
- `.krypt/deps.toml`: `swaync` goes. `wiremix` and `bluetui` go unless the owner
  keeps them as standalone tools.
- `.krypt/links.toml`: the `.config/swaync/**/*` link goes with
  `.config/swaync/`, and the `bluetui` and `wiremix` links go if those tools do.
  `.local/bin/.swaync-test`'s link is renamed if the script is.
- `.local/bin/.menu-time` restarts the bar after changing the time zone
  (`pkill waybar` / `waybar &`). With lyr re-reading `timedate1`
  ([07](07-modules.md#clock)) those two lines are deleted, not ported, and the
  description in `.krypt/commands.toml` ("...and restarts waybar") changes with
  them.
- `.krypt/deps.toml`: `"waybar"` becomes `"lyr"` once lyr is packaged (M11).
  Until then lyr is installed from source.
- `.krypt/links.toml`: the `.config/waybar/**/*` link goes when
  `.config/waybar/` does, and a `.config/lyr/` link comes in if a config file is
  needed.
- Comments that say "opened from waybar" (`.config/bluetui/config.toml`,
  `.config/wiremix/wiremix.toml`), and `README.md`'s "Bar: Waybar" and
  "Notifications: swaync" lines and its `mod+W`, `mod+A`, `mod+U` and `mod+N`
  rows.
- `.config/waybar/` stays until lyr has run as the daily driver for a while,
  then it is removed. `.config/swaync/` goes the same way.
- A `.config/lyr/config.toml` is only needed for differences from the defaults.
  With the defaults being the waybar port, that file starts empty.

`lyr import-waybar FILE` converts a waybar `config.jsonc` to a lyr config,
listing what it cannot carry; reading `style.css` into `[theme]` is a backlog
item.
