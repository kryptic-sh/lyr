# lyr

A desktop status bar for Linux (Wayland), Windows and macOS: a waybar
replacement. Rust binary.

[![CI](https://github.com/kryptic-sh/lyr/actions/workflows/ci.yml/badge.svg)](https://github.com/kryptic-sh/lyr/actions/workflows/ci.yml)
[![license: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

One bar per monitor, three zones of modules, driven entirely by events and drawn
on the GPU through [crcbl](https://github.com/kryptic-sh/crcbl). Part of the
[kryptic.sh](https://kryptic.sh) suite.

## What it will do

- **Workspaces** that speak Hyprland's Lua dispatch form
  (`hl.dsp.focus({ workspace = N })`), with Windows virtual desktops and
  AeroSpace on macOS.
- **Modules:** power menu, battery, power profile, network, bluetooth, tray,
  volume, mic, clock with a calendar, and notifications. Each one works on every
  platform that exposes what it needs.
- **Its own notification centre and menus:** on Linux lyr is the notification
  daemon, and draws the calendar, notification history and the Wi-Fi, Bluetooth
  and audio menus itself, with no helper programs.
- **Ships an opinionated waybar setup as its defaults:** a fresh install looks
  and behaves like it, with no config file.
- **Idle at zero:** no polling and no frame loop. The bar repaints only when
  something changes.

## Status

**Pre-alpha.** It runs, on Windows and (untried on a real compositor yet) on
Linux under Wayland:

- One transparent bar along the top of every output, with the power button, the
  clock and, on Linux, the battery (UPower), power profile
  (power-profiles-daemon), Bluetooth (BlueZ), the notification bell and the
  Hyprland workspaces. Network, volume, mic and the tray are not built yet.
- Tooltips, the power menu (its actions are built on Linux only), and the clock
  panel's calendar.
- On Linux, the notification daemon with toasts, when no other daemon (such as
  swaync) holds the name.

The design lives in [docs/plan/](docs/plan/00-index.md), the build order in
[docs/plan/15-milestones.md](docs/plan/15-milestones.md), and what is not done
yet in [docs/backlog.md](docs/backlog.md).

## Using it

`lyr` with no command runs the bar. Every other command talks to the running
bar:

| Command                 | Does                                                                      |
| ----------------------- | ------------------------------------------------------------------------- |
| `lyr reload`            | Reloads the config, keeping the running one if it is bad                  |
| `lyr toggle`            | Hides or shows the bars                                                   |
| `lyr quit`              | Stops the bar                                                             |
| `lyr dnd` / `lyr clear` | Do-not-disturb on or off / dismiss every notification                     |
| `lyr panel <name>`      | Opens a panel: `clock`; `wifi`, `bluetooth` and `audio` are not built yet |

`lyr -c FILE` uses another config file, and `lyr --stats` logs the bar's
wake-ups and redraws once a minute. Logs go to `~/.local/state/lyr/logs/lyr.log`
and to stderr; `RUST_LOG` filters them.

## Configuration

`~/.config/lyr/config.toml`, on every platform. It is layered over built-in
defaults that reproduce the waybar setup lyr replaces, so an empty file (or
none) gives that bar, and a file sets only what it changes. A
`[platform.linux]`, `[platform.windows]` or `[platform.macos]` table holds
settings for one platform. An invalid config is an error naming the key.

The defaults are the reference for every key and value:
[crates/lyr-config/src/defaults.toml](crates/lyr-config/src/defaults.toml).

| Table             | Keys                                                                                                                              |
| ----------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| `[bar]`           | `position` (`top`, `bottom`), `height`, `output` (names; empty is every output), `layer`                                          |
| `[layout]`        | `left`, `center`, `right`: module names in order                                                                                  |
| `[workspaces]`    | `all_outputs`, `format` (`{name}`), `scroll`                                                                                      |
| `[power]`         | `format`, `tooltip_format`, `[power.menu]` (`lock`, `sleep`, `hibernate`, `shutdown`, `logout`, `reboot` as `{ exec = "…" }`)     |
| `[clock]`         | `format` (strftime)                                                                                                               |
| `[battery]`       | `format`, `format_charging`, `format_plugged`, `format_full`, `icons`, `tooltip_format`, `warning`, `critical`                    |
| `[power_profile]` | `format`, `tooltip_format`, `icons`                                                                                               |
| `[network]`       | `format`, `format_<state>` and `tooltip_format_<state>` for `wifi`, `ethernet`, `linked`, `disconnected`, `disabled`, `icons`     |
| `[bluetooth]`     | `format`, `format_on`, `format_off`, `format_disabled`, `format_connected`, `format_connected_battery`, `tooltip_format`, `icons` |
| `[volume]`        | `format`, `format_muted`, `format_bluetooth`, `format_bluetooth_muted`, `tooltip_format`, `icons`, `scroll_step`                  |
| `[mic]`           | `format`, `format_muted`, `tooltip_format`, `scroll_step`                                                                         |
| `[notifications]` | `replace`, `format`, `icons`                                                                                                      |
| `[theme]`         | font, colours and box metrics; see the defaults                                                                                   |

Every module also takes an `[<module>.on]` table binding `click`, `click_right`,
`click_middle`, `scroll_up` and `scroll_down` to a command (`{ exec = "…" }`,
run through the platform shell) or a built-in: `toggle_mute`, `volume_up`,
`volume_down` (volume and mic), `cycle_profile`, `cycle_profile_back` (power
profile), `open_clock_panel`, `open_power_menu`, `open_wifi_menu`,
`open_bluetooth_menu`, `open_audio_menu`, `toggle_dnd`. Of these,
`cycle_profile`, `cycle_profile_back`, `open_power_menu` and (on Linux, while
lyr is the notification daemon) `toggle_dnd` work so far.

A config file is code: its commands run as you ([SECURITY.md](SECURITY.md)).

## Building

```sh
cargo build --release
```

The workspace MSRV is **Rust 1.98** (`rust-version` in `Cargo.toml`), which is
crcbl's.

## Layout

| Crate         | Role                                                                      |
| ------------- | ------------------------------------------------------------------------- |
| `lyr`         | Binary (`apps/lyr`): CLI and wiring                                       |
| `lyr-config`  | TOML config and the built-in defaults                                     |
| `lyr-core`    | Module state, formats, layout, draw lists, hit testing (no platform code) |
| `lyr-render`  | Drawing the bar through crcbl's GPU layer, fonts                          |
| `lyr-shell`   | Bar and popup windows over crcbl-shell, outputs, input                    |
| `lyr-wm`      | Workspace backends: Hyprland, Windows virtual desktops, AeroSpace         |
| `lyr-sources` | Module data sources per platform: audio, network, bluetooth, battery, …   |

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) or open an issue / PR.

## License

MIT. See [LICENSE](LICENSE).
