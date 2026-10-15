# 13 — Testing

Every test is shown to fail against a broken implementation before it is trusted
(break it, see it go red, restore it). The split in
[02](02-architecture.md#workspace-layout) exists so that most of the bar can be
tested with no display and no GPU, on every CI runner.

## 1. Unit tests (pure, most of the coverage)

| Area                 | Crate         | What                                                                                                                                                                                                      |
| -------------------- | ------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Format strings       | `lyr-core`    | every placeholder of every module against fixed state; icon ramps at their boundaries (battery 0/30/100 %)                                                                                                |
| Clock                | `lyr-core`    | strftime formats; the tick interval derived from the format (minute vs second); next-boundary maths                                                                                                       |
| Calendar             | `lyr-core`    | month grids, ISO week numbers across year boundaries, first day of the week per locale                                                                                                                    |
| Layout               | `lyr-core`    | zone placement, centred centre zone, overlap clipping, spacers, scale factors                                                                                                                             |
| `DrawList` snapshots | `lyr-core`    | the default bar for representative states, compared with stored snapshots (structure, not pixels)                                                                                                         |
| Hit testing          | `lyr-core`    | positions → module and action, including borders and margins                                                                                                                                              |
| Config               | `lyr-config`  | **defaults parity**: the embedded defaults produce exactly the modules, formats, icons and actions of the waybar config (table-driven); validation errors name the key path; layering rules; reload diffs |
| Hyprland events      | `lyr-wm`      | the socket2 line parser against recorded event lines, including malformed and unknown ones                                                                                                                |
| Hyprland dispatch    | `lyr-wm`      | the exact bytes written for each action, in Lua and legacy dialects                                                                                                                                       |
| Sources              | `lyr-sources` | each source's mapping from platform data to state, fed from a fake (a test D-Bus service through zbus, recorded PulseAudio events, canned Win32 / CoreAudio values)                                       |

## 2. Integration tests

- **Hyprland fake:** a test serves both Hyprland sockets in a temp
  `XDG_RUNTIME_DIR`, scripts events, and asserts on the workspace state and on
  the dispatch requests lyr sends.
- **D-Bus fakes:** a private session bus (`dbus-daemon --session` on the Linux
  runner) with fake UPower, BlueZ, power-profiles-daemon, NetworkManager, MPRIS
  and StatusNotifierItem services, so the Linux sources and menus run against
  real D-Bus traffic.
- **Notification daemon:** test clients on the same private bus send what
  `.swaync-test` sends (urgencies, actions, images, long bodies, `replaces_id`)
  and assert on the history, the signals lyr emits, and the toasts in the
  `DrawList`.
- **Headless render:** crcbl's headless shell backend (which crcbl's own CI runs
  everything through) renders a bar to an image. Image comparisons are limited
  to the Linux runner, because GPU and software rasterisation differ per
  platform.

## 3. Manual checklists

Kept in `docs/manual-checks.md` from M1, one list per platform. They cover what
automation cannot see: space reservation with real windows, multi-monitor
hotplug, fractional scaling, tray apps (Steam, Teams), DPMS off and on, and the
idle measurement in [12](12-performance-and-power.md).

## Not covered by automated tests (stated as a gap)

- Real compositor, Explorer and WindowServer behaviour: layer-shell configure,
  appbar work-area changes, panel levels.
- Real GPU drivers. CI has no GPU, so only the headless backend runs there.
