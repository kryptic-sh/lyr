# 02 — Architecture

## Process

One process, no daemon split:

```
            main thread                                    tokio runtime (worker threads)
 ┌───────────────────────────────────┐            ┌─────────────────────────────────────────────┐
 │ platform loop (lyr-shell over     │            │ sources (lyr-sources, lyr-wm)               │
 │  crcbl-shell: Wayland / Win32 /   │  wake +    │  hyprland socket2 · zbus (UPower, BlueZ,    │
 │  AppKit)                          │◄───────────┤  PPD, SNI, swaync, logind) · libpulse ·      │
 │                                   │  Event     │  rtnetlink · WASAPI/CoreAudio callbacks ·   │
 │ state (lyr-core)  ──► DrawList ──►│            │  clock tick · config file watcher           │
 │ renderer (lyr-render, crcbl-hal)  │  Action    │                                             │
 │ pointer input ──► hit test ──────►├───────────►│  focus workspace · set volume · toggle DND… │
 └───────────────────────────────────┘            └─────────────────────────────────────────────┘
```

- The **main thread** owns the platform loop, the bar state, the renderer and
  every surface. AppKit requires the main thread, and Win32 windows belong to
  the thread that created them. So this is a requirement, not a choice.
- **Sources** run as tasks on a tokio runtime. Each one turns its platform
  notifications into `Event`s (`Event::Audio(AudioState)`,
  `Event::Workspaces(..)`, …) and sends them to the main thread **with a
  wakeup**. The wakeup is a cross-thread proxy that crcbl-shell does not have
  yet (requirement S6 in
  [03](03-surfaces-and-input.md#what-crcbl-must-provide)): an eventfd on
  Wayland, `PostMessageW` on Win32, a `CFRunLoop` source on AppKit.
- **Actions** go the other way: a click becomes an `Action` (focus workspace 3,
  volume −5 %, toggle mute) that is sent to the owning source task, which
  performs it. The main thread never blocks on I/O.
- The state is **plain data**. `lyr-core` turns `(state, config, theme)` into a
  `DrawList` and hit regions, with no platform types involved. That keeps nearly
  all the bar's logic testable on every platform without a display.

## Workspace layout

Follows the org convention of `apps/<bin>` + `crates/<bin>-*`.

| Path                 | Crate         | Owns                                                                                                                                                                                    |
| -------------------- | ------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `apps/lyr`           | `lyr`         | The binary: CLI, logging, wiring config + shell + renderer + sources together, the main loop                                                                                            |
| `crates/lyr-config`  | `lyr-config`  | TOML schema, the embedded defaults (the waybar port, per platform), loading, validation, reload diffing                                                                                 |
| `crates/lyr-core`    | `lyr-core`    | Module state types, `Event`/`Action`, format strings and icon ramps, zone layout, `DrawList` building, hit testing, calendar maths                                                      |
| `crates/lyr-render`  | `lyr-render`  | The crcbl-hal device, per-surface swapchains, font lookup and loading, translating `lyr-core`'s `DrawList` into crcbl's UI pass                                                         |
| `crates/lyr-shell`   | `lyr-shell`   | Bar policy over crcbl-shell: one bar per output, space reservation, popup placement, outputs coming and going, pointer input to hit tests                                               |
| `crates/lyr-wm`      | `lyr-wm`      | The `WorkspaceBackend` trait and its backends: Hyprland IPC (event parser, Lua dispatch), Windows virtual desktops, AeroSpace                                                           |
| `crates/lyr-sources` | `lyr-sources` | Every other module's source, one module per source (`audio/`, `battery/`, `bluetooth/`, `network/`, `power_profile/`, `tray/`, `notify/`, `clock`), each with its per-platform backends |

`lyr-core` and `lyr-config` have **no platform code at all**. That rule is what
makes the layout, formatting and calendar logic testable on every CI runner.
`lyr-shell` holds no platform code either: the per-platform window work lives in
crcbl-shell. `lyr-sources` and `lyr-wm` keep each platform in its own module
(`linux.rs` / `windows.rs` / `macos.rs` or a directory), behind `#[cfg]` with
every arm implemented. A source that a platform cannot support returns a typed
`Unsupported { reason }` error, so "not available here" is never confused with
"working" ([07](07-modules.md)).

`lyr-sources` is the crate most likely to sprawl. Each source is a directory
from day one, so no file becomes the monolith.

## Control socket

A small local socket (a Unix socket on Linux and macOS, a named pipe on
Windows), modelled on tmxr's `interprocess` use. It is used by:

- `lyr reload`, `lyr quit`, and `lyr toggle` (hide or show the bars);
- external hooks pushing events in, such as AeroSpace's
  `exec-on-workspace-change = ['lyr', 'event', 'workspace']`
  ([06](06-workspaces.md#macos--spaces));
- single-instance detection: a second `lyr` exits with a message rather than
  drawing a second set of bars.

## Data flow of a click

1. The pointer is pressed on bar `DP-1` at (912, 24). The platform shell reports
   `PointerButton { surface, pos, button: Left }`.
2. `lyr-core` hit-tests against the regions of the last layout. The hit is
   workspace `3`.
3. The bound action is `Action::FocusWorkspace(3)`. It is sent to the `lyr-wm`
   task.
4. The Hyprland backend writes `dispatch hl.dsp.focus({ workspace = 3 })` to
   `.socket.sock` ([06](06-workspaces.md)).
5. Hyprland emits `workspacev2>>3,3` on `.socket2.sock`. The task parses it and
   sends `Event::Workspaces(..)` with a wakeup.
6. The main thread applies the event. The state changed, so it rebuilds the
   centre zone's `DrawList`, marks every bar dirty (all outputs show all
   workspaces), and each bar repaints once on its next frame callback.

The bar redraws because of the **event**, not the click. If Hyprland rejected
the dispatch, nothing would change on screen, which is the truth.

## Prior art in the org

Checked when choosing the stack (October 2026):

| Repo    | Windowing                                                                    | Rendering                             | Role for lyr                                                                                                                                                                                                                                                                                                |
| ------- | ---------------------------------------------------------------------------- | ------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `crcbl` | own shell: Wayland (own protocol scanner), X11, Win32, AppKit, web, headless | own HAL: Vulkan, Metal, D3D12, WebGPU | **The platform layer.** Idles properly (`wait_events` blocks on every native backend), has per-monitor-v2 DPI on Win32, `NSScreen` enumeration, fractional scale, and a UI draw pass (`UiRenderer`, `DrawList`). Missing what a bar needs; listed in [03](03-surfaces-and-input.md#what-crcbl-must-provide) |
| `wayr`  | Wayland only; toplevel + **layer-shell**, fractional scale, multi-output     | none (raw-window-handle for wgpu)     | **Fallback** for Linux if crcbl declines layer-shell. It has its own gaps: a 50 ms idle wakeup cap, no output choice for layer surfaces, no `xdg_popup`, no external wake source                                                                                                                            |
| `buffr` | winit 0.30 on all three platforms                                            | wgpu 30, `fontdb` + `fontdue`         | Precedent for `fontdb` (system font lookup). winit has no layer-shell, appbar or panel API                                                                                                                                                                                                                  |
| `pikr`  | floem on **forks** of floem and winit that add layer-shell                   | vello (wgpu)                          | Not a base. pikr is the **other consumer** of the crcbl requirements: its own research wants to leave the forks ([03](03-surfaces-and-input.md#what-crcbl-must-provide))                                                                                                                                    |

crcbl is not on crates.io yet. The owner expects its first release soon, but it
is not yet scheduled in crcbl itself, and lyr cannot publish while it depends on
crcbl from git. Until then lyr depends on crcbl by **git revision** for
development only, and milestone M9 (release) is blocked on crcbl's release
([15](15-milestones.md)).

## Dependencies (initial)

House-precedent versions. Each is added with `cargo add` in the milestone that
first uses it, not at scaffold time. Anything not on this list needs a call
before it is added.

| Crate                                                                              | Used by               | Notes                                                                                                  |
| ---------------------------------------------------------------------------------- | --------------------- | ------------------------------------------------------------------------------------------------------ |
| `crcbl-shell`, `crcbl-hal`, `crcbl-vk`, `crcbl-mtl`, `crcbl-dx12`, crcbl's UI pass | shell, render         | Exact versions (`=0.x.y`), bumped deliberately in their own commit, because crcbl breaks every release |
| `fontdb`                                                                           | render                | system font lookup by family name (buffr precedent)                                                    |
| `clap` 4 (derive)                                                                  | app                   | CLI                                                                                                    |
| `anyhow`                                                                           | app                   | binary-level errors                                                                                    |
| `thiserror` 2                                                                      | library crates        | typed errors                                                                                           |
| `tracing`, `tracing-subscriber`                                                    | all                   | logs to `~/.local/state/lyr/logs/`                                                                     |
| `serde`, `toml`                                                                    | config                | config schema                                                                                          |
| `hjkl-config`, `hjkl-xdg`                                                          | config                | TOML loading and paths, as tmxr does                                                                   |
| `tokio`                                                                            | app, sources, wm      | source runtime                                                                                         |
| `interprocess` 2                                                                   | app                   | control socket (tmxr precedent)                                                                        |
| `resvg`                                                                            | sources (tray icons)  | pikr precedent; SVG icons rasterised once per size                                                     |
| `zbus` 5                                                                           | sources (Linux)       | UPower, BlueZ, PPD, SNI, DBusMenu, swaync, logind, timedate1                                           |
| `libpulse-binding`                                                                 | sources (Linux)       | **new to the org**, needs the owner's approval                                                         |
| `rtnetlink` or `neli`                                                              | sources (Linux)       | **new to the org**, needs the owner's approval                                                         |
| `windows` 0.62 / `windows-sys` 0.61                                                | sources, wm (Windows) | COM (WASAPI, virtual desktops, WinRT radios) needs `windows`; plain Win32 uses `windows-sys`           |
| `objc2` + `objc2-core-audio`, …                                                    | sources (macOS)       | crcbl precedent for `objc2`                                                                            |
| dev: `tempfile`                                                                    | tests                 | temp config dirs                                                                                       |
