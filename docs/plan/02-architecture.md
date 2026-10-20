# 02 — Architecture

## Process

One process, no daemon split:

```
            main thread                                    tokio runtime (worker threads)
 ┌───────────────────────────────────┐            ┌─────────────────────────────────────────────┐
 │ platform loop (lyr-shell over     │            │ sources (lyr-sources, lyr-wm)               │
 │  crcbl-shell: Wayland / Win32 /   │  wake +    │  hyprland socket2 · zbus (UPower, BlueZ,    │
 │  AppKit)                          │◄───────────┤  PPD, SNI, NM, MPRIS, logind; Notifications │
 │                                   │  Event     │  server) · libpulse · rtnetlink ·           │
 │                                   │            │  WASAPI/CoreAudio callbacks ·               │
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

| Path                 | Crate         | Owns                                                                                                                                                                                                                                             |
| -------------------- | ------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `apps/lyr`           | `lyr`         | The binary: CLI, logging, wiring config + shell + renderer + sources together, the main loop                                                                                                                                                     |
| `crates/lyr-config`  | `lyr-config`  | TOML schema, the embedded defaults (the waybar port, per platform), loading, validation, reload diffing                                                                                                                                          |
| `crates/lyr-core`    | `lyr-core`    | Module state types, `Event`/`Action`, format strings and icon ramps, zone layout, `DrawList` building, hit testing, calendar maths                                                                                                               |
| `crates/lyr-render`  | `lyr-render`  | The crcbl-hal device, per-surface swapchains, font lookup and loading, translating `lyr-core`'s `DrawList` into crcbl's UI pass                                                                                                                  |
| `crates/lyr-shell`   | `lyr-shell`   | Bar policy over crcbl-shell: one bar per output, space reservation, popup placement, outputs coming and going, pointer input to hit tests                                                                                                        |
| `crates/lyr-wm`      | `lyr-wm`      | The `WorkspaceBackend` trait and its backends: Hyprland IPC (event parser, Lua dispatch), Windows virtual desktops, AeroSpace                                                                                                                    |
| `crates/lyr-sources` | `lyr-sources` | Every other module's source, one module per source (`audio/`, `battery/`, `bluetooth/`, `network/`, `power_profile/`, `tray/`, `notify/`, `mpris/`, `clock`), each with its per-platform backends. On Linux `notify/` is the notification daemon |

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
- `lyr panel clock|wifi|bluetooth|audio` (toggle a panel on the focused output),
  `lyr dnd` (toggle do-not-disturb) and `lyr clear` (clear the notifications),
  which the Hyprland bindings run ([09](09-notifications.md),
  [10](10-popups-and-menus.md));
- external hooks pushing events in, such as AeroSpace's
  `exec-on-workspace-change = ['lyr', 'event', 'workspace']`
  ([06](06-workspaces.md#macos--spaces));
- single-instance detection: a second `lyr` exits with a message rather than
  drawing a second set of bars.

A request is one line of text (`panel wifi`), so the socket can be driven by
hand, and the reply is one line: `ok`, or `error: …`. The requests and their
wire form are `lyr_core::control::Request`, and the command line maps each
command to one (`apps/lyr/src/control.rs`).

- **Where:** on Unix, `lyr.sock` in `$XDG_RUNTIME_DIR` (private to the user by
  the XDG spec), else `$TMPDIR` (per user on macOS); with neither set, lyr
  refuses to start rather than use a shared `/tmp`. The socket is `0600`. On
  Windows, the named pipe `\\.\pipe\lyr-<user>`, with Windows' default pipe
  security, which gives other users read access only, and reading cannot send a
  request. An owner-only DACL, as tmxr sets, would need `widestring`, which lyr
  does not take on for this.
- **Single instance:** a starting lyr first connects to the socket; if a bar
  answers, it exits saying so. That check comes first because a Windows pipe
  name accepts a second listener silently. On Unix a socket file nobody answers,
  left by a lyr that died, is replaced.

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

crcbl 0.5.0 is on crates.io (released 2026-10-19) and has everything lyr asked
of it, the asset crate (S28), `crcbl-font` and `crcbl-app` included, so lyr
depends on released crcbl crates only. A git dependency would block lyr's own
release, because `cargo publish` rejects one ([15](15-milestones.md)).

## Dependencies (initial)

House-precedent versions. Each is added with `cargo add` in the milestone that
first uses it, not at scaffold time. Anything not on this list needs a call
before it is added.

| Crate                                                                              | Used by               | Notes                                                                                                                                                                                                                                                                                                                   |
| ---------------------------------------------------------------------------------- | --------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| crcbl's image and asset crate (S28)                                                | sources               | image decoding, raw-buffer conversion, resizing and icon-theme lookup, shared with pikr instead of lyr adding `image`, `resvg` and `freedesktop-icons` ([03](03-surfaces-and-input.md#what-crcbl-must-provide))                                                                                                         |
| `crcbl-font` (`system-fonts` feature)                                              | render                | system font lookup and the glyph fallback order, shared with pikr instead of lyr adding `fontdb` (the owner's choice, 2026-10-17); the `crcbl` umbrella's `system-fonts` feature turns it on                                                                                                                            |
| `crcbl-shell`, `crcbl-hal`, `crcbl-vk`, `crcbl-mtl`, `crcbl-dx12`, crcbl's UI pass | shell, render         | Exact versions (`=0.x.y`), bumped deliberately in their own commit, because crcbl breaks every release                                                                                                                                                                                                                  |
| `crcbl-app` (since crcbl 0.4.0)                                                    | render                | the window-plus-device layer (`GpuContext`, backend fall-through, `open_window`, `wait_for_configure`, `TextPump`), split out of the `crcbl` umbrella at lyr's request so lyr links no engine, audio (cpal, ALSA) or physics                                                                                            |
| `clap` 4 (derive)                                                                  | app                   | CLI                                                                                                                                                                                                                                                                                                                     |
| `anyhow`                                                                           | app                   | binary-level errors                                                                                                                                                                                                                                                                                                     |
| `thiserror` 2                                                                      | library crates        | typed errors                                                                                                                                                                                                                                                                                                            |
| `tracing`, `tracing-subscriber`                                                    | all                   | logs to `~/.local/state/lyr/logs/`                                                                                                                                                                                                                                                                                      |
| `serde`, `toml`                                                                    | config                | config schema                                                                                                                                                                                                                                                                                                           |
| `serde_path_to_error`                                                              | config                | names the full key of an error serde raises inside a table (approved by the owner 2026-10-18)                                                                                                                                                                                                                           |
| `chrono` (`clock`, no default features)                                            | core, sources         | the clock's formatting and time zone, the calendar's dates (approved by the owner 2026-10-18; house precedent)                                                                                                                                                                                                          |
| `serde_json`                                                                       | wm (Unix), config     | Hyprland's `j/` replies (approved by the owner 2026-10-16)                                                                                                                                                                                                                                                              |
| `hjkl-config`, `hjkl-xdg`                                                          | config                | TOML loading and paths, as tmxr does                                                                                                                                                                                                                                                                                    |
| `tokio`                                                                            | app, sources, wm      | source runtime                                                                                                                                                                                                                                                                                                          |
| `interprocess` 2                                                                   | app                   | control socket (tmxr precedent)                                                                                                                                                                                                                                                                                         |
| `zbus` 6 (no default features; `tokio`, `proxy`)                                   | sources (Linux)       | UPower, BlueZ (and its pairing agent), PPD, SNI, DBusMenu, NetworkManager, MPRIS, logind, timedate1; serves `org.freedesktop.Notifications`. 6 is what resolves today; the house precedent was 5                                                                                                                        |
| `futures-util`                                                                     | sources (Linux)       | `StreamExt` for zbus's property and signal streams (approved by the owner 2026-10-18)                                                                                                                                                                                                                                   |
| `libpulse-binding`                                                                 | sources (Linux)       | **new to the org**, needs the owner's approval                                                                                                                                                                                                                                                                          |
| `rtnetlink` or `neli`                                                              | sources (Linux)       | **new to the org**, needs the owner's approval                                                                                                                                                                                                                                                                          |
| `windows` 0.62 / `windows-sys` 0.61                                                | sources, wm (Windows) | COM (WASAPI, virtual desktops, WinRT radios) needs `windows`; plain Win32 uses `windows-sys`                                                                                                                                                                                                                            |
| `objc2` + `objc2-core-audio`, …                                                    | sources (macOS)       | crcbl precedent for `objc2`; taken so far: `objc2-core-foundation` (crcbl's own), `objc2-io-kit` (the battery's power sources), `objc2-core-audio` (volume and mic), `objc2-foundation` with `objc2` and `block2` (Low Power Mode's notification), and `objc2-system-configuration` and `objc2-core-wlan` (the network) |
| dev: `tempfile`                                                                    | tests                 | temp config dirs                                                                                                                                                                                                                                                                                                        |
