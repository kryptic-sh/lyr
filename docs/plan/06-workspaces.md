# 06 — Workspaces

The centre zone. The bar shows one button per workspace, highlights the active
one and any urgent ones, switches on click and cycles on scroll. Like waybar's
`hyprland/workspaces` with `all-outputs: true` and `format: "{}"`: every bar
shows every workspace, labelled by name.

What a "workspace" is differs per platform, so the module talks to a
`WorkspaceBackend` trait in `lyr-wm`. Each platform has one implementation
selected at runtime: the first backend whose detection succeeds wins. When none
does, the module logs why and is left out of the bar. It never shows an empty
zone that looks like it works.

```rust
trait WorkspaceBackend {
    fn snapshot(&self) -> Workspaces;        // ids, names, active, urgent, output
    fn focus(&self, id: WorkspaceId) -> Result<()>;
    fn subscribe(&self, tx: EventSender);    // pushes WorkspaceEvent, never polled
}
```

## Linux — Hyprland

Detected by `HYPRLAND_INSTANCE_SIGNATURE`. Both sockets live in
`$XDG_RUNTIME_DIR/hypr/$HYPRLAND_INSTANCE_SIGNATURE/`.

- **Events:** `.socket2.sock` is a line stream (`event>>data`). lyr reads
  `workspacev2`, `createworkspacev2`, `destroyworkspacev2`, `renameworkspace`,
  `focusedmonv2`, `moveworkspacev2` and `urgent`. The parser is a pure function
  in `lyr-wm`, tested against recorded lines.
- **Initial state:** `j/workspaces` and `j/activeworkspace` requests on
  `.socket.sock` (the JSON forms `hyprctl -j` uses).
- **Actions use the Lua dispatch form.** Under a Lua config (Hyprland 0.55+,
  which the dotfiles use), the old `dispatch workspace N` is a parse error. The
  dotfiles bind workspaces as `hl.dsp.focus({ workspace = i })`
  (`dotfiles/.config/hypr/hyprland.lua`), and `hypridle.conf` already sends
  `hyprctl dispatch 'hl.dsp.dpms({ action = "on" })'`. So the `dispatch` request
  still exists and only its argument changed. lyr writes
  `/dispatch hl.dsp.focus({ workspace = N })` straight to `.socket.sock` and
  does not shell out to `hyprctl` per click. Those are the bytes `hyprctl`
  writes: it frames a request as `FLAGS/COMMAND ARGS`, so a flagless one starts
  with `/`, with no newline, and then reads until the socket closes
  (`hyprctl/src/main.cpp`, read at hyprwm/Hyprland `e2ad066c`). That Hyprland
  accepts it under a Lua config has not been run yet; the first click on the
  owner's machine is the check. A named workspace (non-positive id) is
  `workspace name:NAME` in hyprlang. lyr refuses to send it in Lua until there
  is a quoting rule, since the name would be spliced into a Lua expression.
- **The target is the released Hyprland, v0.56.2.** The event, JSON and dispatch
  formats above were read at that tag. Hyprland's main branch (286 commits past
  it on 2026-10-16) answers `dispatch` under a hyprlang config with "current
  config provider doesn't support dispatch", and drops the `id` field for named
  workspaces in `j/workspaces`. lyr has to follow both changes when the next
  release ships them.
- **Detecting the config dialect:** a Lua config is found when
  `~/.config/hypr/hyprland.lua` exists. That check is a guess at what ironbar's
  `detect_lua_config()` does (not read). The legacy form is kept for hyprlang
  configs so lyr also works for people who have not migrated.
- **Window title** (waybar's `hyprland/window`) is **not** in the current bar
  and is not planned for MVP. The `activewindowv2` event makes it cheap to add
  later; see the backlog.

## Windows — virtual desktops

Windows' documented `IVirtualDesktopManager` only answers "is this window on the
current desktop" and "move a window". It cannot list desktops, name them, switch
between them or report changes. Everything a workspace module needs is behind
the **undocumented** `IVirtualDesktopManagerInternal` /
`IVirtualDesktopNotificationService` COM interfaces. Their IIDs change between
Windows builds (every tool that switches desktops, such as
VirtualDesktopAccessor, ships a per-build IID table).

Provisional default ([16](16-open-questions.md)): implement it with a per-build
IID table, refuse to load on an unknown build with a logged error naming that
build, and leave the module out rather than guess. Tiling window managers
(komorebi, GlazeWM) have stable IPC and are a better target if the owner uses
one. The dotfiles have no config for either.

## macOS — Spaces

macOS has **no public API** for Spaces: not for listing them, switching or
watching changes. The private CGS/SkyLight calls yabai uses need SIP partly
disabled for switching. Provisional default: no native backend. Ship an
**AeroSpace** backend instead (its CLI lists workspaces and switches between
them, and its `exec-on-workspace-change` hook can push events to lyr over its
control socket, see [02](02-architecture.md#control-socket)). Without AeroSpace
the module is left out on macOS. See [16](16-open-questions.md).

## Multi-monitor

One bar per output ([03](03-surfaces-and-input.md#outputs)). With
`all-outputs = true` (the default, matching waybar), every bar lists every
workspace. With `false`, each bar lists only the workspaces on its own output,
using the output name the backend reports (Hyprland's `monitor` field).
