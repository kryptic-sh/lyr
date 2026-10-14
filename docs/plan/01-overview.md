# 01 — Overview

## What lyr is

`lyr` is a desktop status bar written in Rust: a waybar replacement. It draws
one bar per monitor with three zones of modules (workspaces, audio, network,
clock and so on), is driven entirely by events, and renders on the GPU.

It targets **Linux (Wayland), Windows and macOS** as first-class platforms.
Linux is where it replaces waybar. On Windows and macOS it is the same bar, with
the modules each platform can actually support
([11-platforms.md](11-platforms.md)).

## Why

The owner's waybar broke under Hyprland's Lua config: the workspace module sends
`dispatch workspace N`, which a Lua config rejects. lyr owns its compositor IPC
instead ([06](06-workspaces.md)), and shares a language, config style and
release pipeline with the rest of the kryptic-sh tools.

## Baseline

The feature baseline is **mxaddict's current waybar setup**, which lyr ships as
its built-in defaults:

- [`dotfiles/.config/waybar/config.jsonc`](https://github.com/mxaddict/dotfiles/blob/main/.config/waybar/config.jsonc):
  modules, formats, icons, click and scroll actions.
- [`dotfiles/.config/waybar/style.css`](https://github.com/mxaddict/dotfiles/blob/main/.config/waybar/style.css):
  font, colours, spacing.
- [`dotfiles/.config/swaync/config.json`](https://github.com/mxaddict/dotfiles/blob/main/.config/swaync/config.json):
  the notification panel the bar's notification module drives.

A fresh install with no config file looks and behaves like that bar on Hyprland.
The layout:

```
[power, battery, power-profile, network, bluetooth]   [workspaces]   [tray, volume, mic, clock, notifications]
```

## MVP scope

The MVP is a daily driver on the owner's Hyprland desktop, and it runs on
Windows and macOS with the modules those platforms support:

1. **Bar surfaces** — one bar per output, top, 50 px, reserving its space
   (Wayland layer-shell exclusive zone, Windows appbar). On macOS it sits in the
   menu-bar strip ([03](03-surfaces-and-input.md)).
2. **Rendering** — GPU rendering of the waybar look: rounded, bordered module
   boxes, Nerd Font glyphs, opacity. Repaints only on change
   ([04](04-rendering-and-theme.md)).
3. **Every module in the current bar** — workspaces, power button, battery,
   power profile, network, bluetooth, tray, volume, mic, clock and notifications
   ([06](06-workspaces.md) – [09](09-notifications.md)).
4. **Popups** — tooltips, calendar, power menu and tray menus
   ([10](10-popups-and-menus.md)).
5. **Config** — TOML, with the waybar setup embedded as defaults and reloaded
   when the file changes ([05](05-layout-and-config.md)).
6. **Idle at zero** — no polling and no wakeups while nothing changes, apart
   from the clock's once-a-minute tick ([12](12-performance-and-power.md)).

## Non-goals (for now)

- **Reading waybar's JSON or CSS.** Config is TOML (house style). The defaults
  are a hand-made port, and an importer is a backlog item.
- **A general GUI toolkit.** lyr draws a fixed set of widgets: boxes, text,
  icons, a calendar grid and menus. pikr's
  [backend research](https://github.com/kryptic-sh/pikr/blob/main/docs/gui-backend-research.md)
  rejected a general toolkit for the same reason.
- **Being the notification daemon.** lyr is a swaync client
  ([09](09-notifications.md)).
- **X11.** Wayland only on Linux.
- **Compositors other than Hyprland** on Linux for MVP. The workspace backend is
  a trait ([06](06-workspaces.md)), so sway and niri are additions, not a
  redesign.
- **Hosting other apps' tray icons on Windows and macOS.** Neither platform
  allows it ([08](08-tray.md)).

## Decisions taken in this plan

| Decision         | Choice                                                                                                   | Why                                                                                                                                                                          |
| ---------------- | -------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Platforms        | Linux (Wayland), Windows, macOS; CI on all three                                                         | Requested                                                                                                                                                                    |
| Platform layer   | **crcbl** (`crcbl-shell` + `crcbl-hal` + its UI draw pass), with the bar's needs added to crcbl upstream | One in-house shell and GPU layer on all three platforms, and the same foundation pikr moves to. crcbl's first crates.io release is imminent ([03](03-surfaces-and-input.md)) |
| Coordination     | lyr's and pikr's requirements tracked as one list against crcbl                                          | The owner's direction: crcbl gains what both apps need ([03](03-surfaces-and-input.md#what-crcbl-must-provide))                                                              |
| Fallback         | wayr on Linux + wgpu + native Win32 / AppKit, if crcbl declines a requirement                            | Recorded so a "no" from crcbl has a ready answer ([16](16-open-questions.md))                                                                                                |
| Rendering        | `crcbl-hal`: Vulkan on Linux and Windows (D3D12 fallback), Metal on macOS                                | Vulkan as the draft asked; in-house backends ([04](04-rendering-and-theme.md))                                                                                               |
| Text             | `crcbl-ui` fonts (skrifa + its glyph atlas), system font lookup with `fontdb`                            | Bar text is left-to-right labels and Nerd Font glyphs, so crcbl-ui's unshaped text is enough; `fontdb` is buffr precedent                                                    |
| Font             | Hack Nerd Font Propo, 16 px                                                                              | The current waybar `style.css`                                                                                                                                               |
| Async runtime    | `tokio` for sources; the platform loop stays on the main thread                                          | Org precedent; zbus, sockets and timers all fit it; AppKit and Win32 need the main thread                                                                                    |
| D-Bus            | `zbus` 5                                                                                                 | Org precedent (buffr); pure Rust                                                                                                                                             |
| Linux audio      | PulseAudio protocol via `pipewire-pulse`                                                                 | Matches today's `pactl` behaviour and default-device tracking ([07](07-modules.md#audio-sources))                                                                            |
| Hyprland actions | Lua dispatch form, written to `.socket.sock` directly                                                    | The bug that started lyr ([06](06-workspaces.md))                                                                                                                            |
| Notifications    | swaync client                                                                                            | Keeps the existing daemon and config ([09](09-notifications.md))                                                                                                             |
| Config           | TOML, defaults embedded, hot reload                                                                      | House style ([05](05-layout-and-config.md))                                                                                                                                  |
| Theme            | the current `style.css` values                                                                           | The bar should look the same the day it replaces waybar                                                                                                                      |

Each provisional call is listed with its alternatives in
[16-open-questions.md](16-open-questions.md).

## Naming

- Binary and repo: `lyr`. Crates: `lyr-*` under `crates/`, the binary under
  `apps/lyr`.
- Config at `~/.config/lyr/config.toml` on every platform (`%APPDATA%` is not
  used, so the dotfiles carry one path). Logs at `~/.local/state/lyr/logs/`.
- Wayland layer namespace `lyr`, so compositor rules can match the bar (Hyprland
  `layerrule`).
