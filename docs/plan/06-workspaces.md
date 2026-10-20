# 06 — Workspaces

The centre zone. The bar shows one button per workspace, highlights the active
one in `#2bc5e3` (and the hovered one, as `style.css` does), and switches on
click. Like waybar's `hyprland/workspaces` with `all-outputs: true` and
`format: "{}"`: every bar shows every workspace, labelled by name.

Two things waybar's setup does not do are off by default. Urgent workspaces are
tracked (waybar adds an `urgent` class), but `style.css` gives that class no
style, so they look like any other until `theme.workspace_urgent` sets a colour.
Scrolling to cycle workspaces is not in the config, and waybar has no built-in
scroll for this module, so it waits for `workspaces.scroll = true`. When on, a
scroll moves to the next or previous button in id order, wrapping at the ends
(`State::step`). A bar showing only its own output's workspaces steps through
those, and does nothing while the active workspace is on another output.

What a "workspace" is differs per platform, so every backend in `lyr-wm`
produces the same `lyr_wm::State`: the buttons in display order (each with an
id, a name, its output and whether it is urgent), the active one, and the
focused window's class and title. On Linux `lyr_wm::Compositor` picks the
backend at runtime: the first of Hyprland, sway and niri whose environment
variable is set wins. It subscribes (a first `State`, then each changed one,
pushed, never polled), switches workspaces, and ends the session for the power
menu's Log out. When no backend is found, the module logs why and is left out of
the bar. It never shows an empty zone that looks like it works.

There is no trait: the backends are an enum, since the set is closed and each
lives in `lyr-wm`. Hyprland folds its events into the state one by one; sway and
niri build a new state from their own replies and hand it on only when it
differs from the last.

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
- **Detecting the config dialect:** lyr asks Hyprland. `j/status` answers
  `{"configProvider": "lua" | "hyprlang", "backend": …}` at v0.56.2
  (`Helpers::SystemInfo::getStatus`, `Config::typeToString`), and ironbar's
  `detect_lua_config()` reads the same field (read 2026-10-18 at ironbar
  `149c0d0d`). A Hyprland that answers `unknown request` has no `status`
  command, which means it predates Lua configs, so it is hyprlang. This replaces
  an earlier guess from `~/.config/hypr/hyprland.lua` existing, which missed
  `hyprland --config`. The legacy form is kept for hyprlang configs so lyr also
  works for people who have not migrated.
- **Window title** (waybar's `hyprland/window`) is not in the current bar. The
  `window` module, built after the MVP and left out of the default layout,
  follows the `activewindow` event, seeded from `j/activeworkspace` and
  `j/clients`.

## Linux — sway

Detected by `SWAYSOCK`, the path of sway's IPC socket. The protocol is i3's
(`sway-ipc(7)`, read at sway 1.12): each message is `i3-ipc`, a 32-bit payload
length and a 32-bit type in native byte order, then the JSON payload; replies
and events are framed the same, events with the type's high bit set.

- **State:** `GET_WORKSPACES` (type 1) gives each workspace's `id`, `num` (`-1`
  for a name that does not start with a number), `name`, `output`, `focused` and
  `urgent`. The buttons keep the reply's order, which is sway's own: by output,
  and on each output numbered workspaces by number, then named ones in the order
  they were made (`output_sort_workspaces`). The active workspace is the
  `focused` one. The focused window is the node with `focused: true` in
  `GET_TREE` (type 4) that is a window (`type` `con` or `floating_con`): its
  `app_id`, or `window_properties.class` under Xwayland, and its `name` as the
  title. An empty workspace has the focus itself, and then there is no window.
- **Events:** a second connection sends `SUBSCRIBE` (type 2) with
  `["workspace","window","output","shutdown"]`. Any workspace or output event
  re-reads both replies, and a window event does only for the changes that can
  move focus or retitle it (`focus`, `title`, `close`, `new`). The state is
  handed on only when it differs from the last. A `shutdown` event, the socket
  closing, or a re-read finding sway's socket gone (sway sends workspace and
  output events while it quits, seen in CI) ends the stream.
- **Switching:** `RUN_COMMAND` (type 0) with
  `workspace --no-auto-back-and-forth "NAME"`, so a click on the shown workspace
  does not bounce back where `workspace_auto_back_and_forth` is on. sway keeps
  backslashes inside quotes, replaces `$variables` in arguments, and reads
  `next`, `prev`, `next_on_output`, `prev_on_output`, `current`,
  `back_and_forth` and `number` as keywords (`sway/commands.c`,
  `sway/commands/workspace.c`). A name with `"`, `\` or `$`, or one equal to a
  keyword, could reach a different workspace, so lyr refuses to send it and logs
  why, as it does for a Hyprland name it cannot quote.
- **Log out:** `RUN_COMMAND` with `exit`. sway quits without answering it (seen
  in CI's `sway e2e` job), so the socket closing before a reply counts as done.

## Linux — niri

Detected by `NIRI_SOCKET`. Each request is one JSON line and gets one JSON line
back, `{"Ok": …}` or `{"Err": "…"}`; after `"EventStream"` the connection
carries only events, one JSON object per line, so switching uses a connection of
its own (`niri-ipc`'s `lib.rs` and `socket.rs`, read at niri v26.04).

- **State:** the event stream starts with the full state (`WorkspacesChanged`,
  `WindowsChanged`), so there is no separate query: the bar's first state is the
  one both lists make, which niri must send within the request timeout
  (`niri::ipc`'s `REQUEST_TIMEOUT`). lyr folds `WorkspacesChanged`,
  `WorkspaceActivated`, `WorkspaceUrgencyChanged`, `WindowsChanged`,
  `WindowOpenedOrChanged`, `WindowClosed` and `WindowFocusChanged`, as
  `niri_ipc::state` does, and skips every other event, including ones newer than
  v26.04.
- **Buttons:** niri's workspaces belong to an output and are numbered on it
  (`idx`), so the buttons are ordered by output, then `idx`, and labelled by
  their `name` when they have one, else by `idx`. The active workspace is the
  `is_focused` one; `is_urgent` marks urgency. The id is niri's own workspace
  id, which stays fixed while workspaces move.
- **Switching:** `{"Action":{"FocusWorkspace":{"reference":{"Id":N}}}}`, by id,
  so no name is ever quoted.
- **Log out:** `{"Action":{"Quit":{"skip_confirmation":true}}}`: the power menu
  has already asked. niri stops its event loop to quit, and that loop also
  writes the reply, so a socket closed before a reply counts as done, as on sway
  (read from niri's source, not seen).
- **Focused window:** the window with `is_focused`, its `app_id` and `title`.

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
**AeroSpace** backend instead. Without AeroSpace the module is left out on
macOS. See [16](16-open-questions.md).

`lyr_wm::aerospace` (read at AeroSpace v0.21.3-Beta) runs the `aerospace`
command: `list-workspaces --all --json --format` with the workspace, whether it
is focused and its monitor, and `list-windows --focused --json --format` with
the app's name and the window's title for the window module. The buttons keep
AeroSpace's order, each numbered by its place, the focused one active. A click
runs `aerospace workspace -- NAME`, each argument passed as is, so no name is
quoted and none is read as an option. AeroSpace pushes nothing to a bar, so its
config tells lyr when to read again over the control socket
([02](02-architecture.md#control-socket)):

```toml
on-focus-changed = ['exec-and-forget lyr event workspace']
```

`on-focus-changed` runs whenever the focused window or workspace changes, which
covers both modules; a window's title changing alone runs nothing, so the window
module can lag a title until the next focus change.

## Multi-monitor

One bar per output ([03](03-surfaces-and-input.md#outputs)). With
`all-outputs = true` (the default, matching waybar), every bar lists every
workspace. With `false`, each bar lists only the workspaces on its own output,
using the output name the backend reports (Hyprland's `monitor` field, sway's
and niri's `output`).
