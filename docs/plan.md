# Plan — lyr

A Wayland status panel (a bar, like waybar) written in Rust.

**Status:** living draft `v0.1` (2026-08-07). Nothing concrete is decided yet.
Open questions are listed in [§7](#7-open-questions); this document is updated
as decisions land.

## 1. Goal

A fast, low-power, configurable Wayland status panel with the module set and
layout of the author's current waybar config, but with the Wayland-specific
parts implemented in-house and rendering done through Vulkan.

## 2. Layout

Three zones, configurable via config file (order, enabled modules, gaps,
spacers, separators):

```
[power_button, perf_mode(balanced|performance|battery_saver), network(lan|wifi), bluetooth]
  (gap/spacer)
[workspaces]
  (gap/spacer)
[tray, volume, mic, calendar, notifications]
```

- **Left:** power button, performance profile, network, bluetooth
- **Center:** workspaces (per the author's waybar: workspaces centered, not
  left-anchored next to the other modules)
- **Right:** tray, volume, mic, calendar, notifications

The author's current waybar also shows a **battery** module — see
[question Q1](#q1).

## 3. Modules

### 3.1 Workspaces (center)

- One workspace indicator per workspace; active/urgent states; click switches
  workspace, scroll cycles.
- `all-outputs` behaviour like waybar's `hyprland/workspaces` config.
- Must speak the **Hyprland Lua-protocol dispatch form**
  (`hl.dsp.focus({ workspace = N })`) — the old `dispatch workspace N` is dead
  under Hyprland 0.55+ Lua configs (this is the bug that made the author
  replace waybar).
- Live updates from the compositor event socket (active workspace, workspace
  add/remove, urgent).

### 3.2 Window title (center)

- Active window class/title, updated on focus change.

### 3.3 Power button (left)

- Click opens a power menu (power off, reboot, suspend, hibernate, logout).
- Open question: built-in menu vs shell out to `krypt menu power` (Q7).

### 3.4 Performance profile (left)

- Read/write the power profile (balanced / performance / power-saver) via
  power-profiles-daemon (D-Bus).
- Icon reflects the active profile.

### 3.5 Network (left)

- LAN vs WiFi indicator, disconnect state; tooltip with ifname/IP/essid/signal.
- Open question: built-in menu vs `krypt menu wifi` (Q7).

### 3.6 Bluetooth (left)

- On/off/disabled state icon; connected-device battery in tooltip.
- Open question: built-in menu vs `krypt menu bluetooth` (Q7).

### 3.7 Tray (right)

- System tray via the StatusNotifierItem (SNI) D-Bus protocol — Steam, Teams,
  etc.

### 3.8 Volume & mic (right)

- Click toggles mute; scroll adjusts level.
- Spec'd semantics (confirmed in Q9):
  - Muting **output** also mutes the **mic**.
  - Muting **mic** only mutes the mic.
- Audio backend: PipeWire/PulseAudio (Q8).

### 3.9 Calendar (right)

- Date render (per author's waybar format: `YYYY/MM/DD HH:MM`).
- Click expands a calendar showing the current month, with a **month** selector
  (`<` / `>`) and a **year** selector.
- Open question: also a clock time render with 1 Hz updates, or date only? (Q12)

### 3.10 Notifications (right)

- Icon reflecting state (notifications pending / none / DND).
- Click opens a **notification drawer** (history center).
- Open question: own `org.freedesktop.Notifications` daemon (like ashell —
  replaces swaync) vs a client of an existing daemon (Q6).

## 4. Technology

### 4.1 Language & foundations

- Rust, single binary.
- Build on **wayr** (the author's Wayland-first windowing toolkit): it already
  implements the Wayland protocol client in-house (wayland-client +
  wayland-protocols), has layer-shell support, an event loop, and
  `raw-window-handle` 0.6 for handing surfaces to GPU crates. "Implement
  Wayland-specific things ourselves" is satisfied by wayr being in-house
  (Q2 confirms the boundary).

### 4.2 Rendering — Vulkan

- Vulkan rendering via the surface wayr provides (Q3: raw `ash` vs `wgpu` vs
  `vulkano`; Vulkan-only with no fallback).
- Immediate-mode-ish UI: redraw **on change only** — no continuous frame loop.

### 4.3 Compositor IPC

- Hyprland first (the author's daily driver); socket events + the Lua-protocol
  dispatch form. Other compositors later (Q4).

## 5. Performance & power

- **Event-driven:** no per-frame rendering; surfaces repaint only when state
  changes (clock/date 1 Hz timer, module events, pointer interaction).
- **Present mode:** FIFO/vsync; stop presenting when nothing changes (the
  surface just stays idle).
- **No polling:** all inputs are event sources (compositor socket, D-Bus,
  PipeWire, pointer/seat).
- **Timers coalesced**; minimum wakeup count.
- Open question: measurable idle target (Q13), e.g. 0% CPU, 0% GPU while
  static.

## 6. Config

- File format open (Q5), TOML preferred (matches wayr's ecosystem).
- Must express: module order + enabled set per zone, gaps/spacers, bar
  position/anchors/height, per-module options, theming (Q10).

## 7. Open questions

### Q1. Battery module

The spec'd layout omits battery, but the author's current waybar shows one
(icon + charging/plugged states + tooltip). Include it (default: yes), where?

### Q2. Wayland scope boundary

wayr already implements the Wayland client (protocol bindings + layer-shell +
event loop). "Implement everything Wayland-specific ourselves" = **use wayr**,
correct? Or does it mean greenfield (no wayr dependency)?

### Q3. Vulkan stack

Raw `ash`, `wgpu` (Vulkan backend), or `vulkano`? Vulkan-only, or a software
fallback for machines without Vulkan?

### Q4. Compositor targets

Hyprland only for now, or design the compositor abstraction for sway/niri from
the start?

### Q5. Config format

TOML (matches wayr) — confirm.

### Q6. Notification daemon

Own `org.freedesktop.Notifications` daemon (replaces swaync, like ashell) vs
client of the existing swaync? Own daemon gives the drawer + DND natively but
retires the swaync config; the author's swaync center has a MPRIS widget —
is that in scope for the drawer?

### Q7. Menus

Built-in menus for power / wifi / bluetooth, or shell out to `krypt menu …`?

### Q8. Audio backend

PipeWire (pipewire-rs / wireplumber) vs PulseAudio (libpulse) API?

### Q9. Mute semantics

Confirm: muting output volume **also mutes the mic**; muting the mic mutes
only the mic. (Deliberate — most UIs treat them independently.)

### Q10. Theming

waybar-style CSS, or a TOML theme? Icon set: Nerd Font glyphs (matches the
current waybar config) or bundled vector icons?

### Q11. Multi-monitor

One bar per output, or a single bar? Workspaces per-output vs all-outputs?

### Q12. Clock

Date only (with calendar on click), or date **and** time (1 Hz updates, costs a
tiny bit of idle wakeup)? The current waybar shows `YYYY/MM/DD HH:MM`.

### Q13. Power budget

Any measurable idle target (e.g. 0% CPU / 0% GPU while static) to hold the
design to?
