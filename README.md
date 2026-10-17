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

**Pre-alpha: nothing works yet.** This repository holds the plan and the
workspace scaffold. The binary parses its command line and reports that no bar
is implemented. The design lives in [docs/plan/](docs/plan/00-index.md) and the
build order in [docs/plan/15-milestones.md](docs/plan/15-milestones.md).

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
