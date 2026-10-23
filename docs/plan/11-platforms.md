# 11 — Platforms

Linux (Wayland), Windows and macOS are all tier 1. CI builds and tests on each
of them ([14](14-ci-and-release.md)). Where a platform cannot support a module,
that is stated here and reported loudly at runtime ([07](07-modules.md)). It is
never silently skipped.

## Windowing

| Concern         | Linux (Wayland)                           | Windows                     | macOS                                      |
| --------------- | ----------------------------------------- | --------------------------- | ------------------------------------------ |
| Bar             | layer-shell surface, exclusive zone       | appbar (`SHAppBarMessage`)  | `NSPanel` in the menu-bar strip            |
| Reserves space  | yes                                       | yes (shrinks the work area) | **no** (no API); menu bar set to auto-hide |
| Popups          | `xdg_popup` on the layer surface          | `WS_POPUP` tool window      | child `NSPanel`                            |
| Panels          | overlay layer surface, on-demand keyboard | `WS_POPUP` tool window      | key-capable `NSPanel`                      |
| GPU backend     | Vulkan                                    | D3D12                       | Metal                                      |
| Scale           | fractional-scale-v1                       | per-monitor-v2 DPI          | `backingScaleFactor`                       |
| Wake from tasks | eventfd                                   | `PostMessageW`              | `CFRunLoop` source                         |

All of it is provided by crcbl-shell once requirements S1 – S7 land
([03](03-surfaces-and-input.md#what-crcbl-must-provide)).

## Modules

✓ = supported, ◐ = partly (the reason is in the module's section), ✗ = not
supported (left out of that platform's defaults).

| Module         | Linux                        | Windows                                  | macOS                             |
| -------------- | ---------------------------- | ---------------------------------------- | --------------------------------- |
| Workspaces     | ✓ Hyprland, sway, niri       | ✗ left out (undocumented COM)            | ◐ AeroSpace only                  |
| Power button   | ✓ logind                     | ✓ Win32                                  | ◐ no hibernate                    |
| Battery        | ✓ UPower                     | ✓ power notifications                    | ✓ IOKit power sources             |
| Power profile  | ✓ power-profiles-daemon      | ◐ set in Settings, opened on click       | ◐ read-only (Low Power Mode)      |
| Network        | ✓ rtnetlink + NetworkManager | ✓ IP Helper + WLAN API                   | ◐ SSID needs Location permission  |
| Bluetooth      | ✓ BlueZ, with device battery | ◐ no device battery                      | ◐ no device battery               |
| Tray           | ✓ StatusNotifierItem         | ✗ owned by Explorer                      | ✗ owned by the menu bar           |
| Volume / mic   | ✓ PulseAudio protocol        | ✓ WASAPI                                 | ✓ CoreAudio                       |
| Clock          | ✓                            | ✓                                        | ✓                                 |
| Window title   | ✓ Hyprland, sway, niri       | ✓ foreground window (WinEvent hooks)     | ◐ AeroSpace only                  |
| Notifications  | ✓ lyr is the daemon          | ◐ opens Action Center, no state          | ✗ no API                          |
| Wi-Fi menu     | ✓ NetworkManager             | ◐ Native Wifi, not run on Wi-Fi yet      | ◐ Settings pane; CoreWLAN unbuilt |
| Bluetooth menu | ✓ BlueZ                      | ◐ power, scan, pair; connect in Settings | ◐ Settings pane; native unbuilt   |
| Audio menu     | ✓ PulseAudio protocol        | ◐ default device opens Settings          | ◐ no per-app volume               |

Details for each row are in [06](06-workspaces.md) –
[10](10-popups-and-menus.md). The Windows and macOS menu rows are from memory
and are checked in M9 and M10.

## Notes

- **Windows constants** that changed between `windows-sys` versions are spelled
  out locally rather than imported, per the house rule for ABI values.
- **Child processes on Windows** (`exec` actions) are started with
  `CREATE_NO_WINDOW`, so a click never flashes a console. tmxr enforces this
  with a `clippy.toml` `disallowed-methods` entry for
  `std::process::Command::new`. lyr adopts the same lint once it has a wrapper
  to point at (M4, with the first `exec` action).
- **Undocumented Windows APIs:** none. Virtual desktops and setting the power
  mode were the two, and the owner left both out ([16](16-open-questions.md)
  questions 9 and 13).
- **macOS permissions:** Location Services (WiFi SSID) is optional, and lyr
  degrades without it. lyr needs no Accessibility permission.
- **Linux without Hyprland, sway or niri:** the bar runs, and the workspace and
  window modules are left out, with a log line saying that none of
  `HYPRLAND_INSTANCE_SIGNATURE`, `SWAYSOCK` and `NIRI_SOCKET` is set.
