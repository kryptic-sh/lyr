# 15 — Milestones

Build order. Each milestone ends green on all three platforms
(`cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo test --workspace`) with its tests in place, and is committed before the
next one starts. A milestone's plan sections are trimmed to what is still open
once it lands.

Several milestones wait on crcbl. The **Needs from crcbl** column names the
requirements from [03](03-surfaces-and-input.md#what-crcbl-must-provide). Work
that does not touch a window (`lyr-core`, `lyr-config`, `lyr-wm`, `lyr-sources`)
can go ahead of crcbl, against headless tests.

| #   | Milestone                        | Needs from crcbl         | Delivers                                                                                                      | Exit check                                                                                                                                      |
| --- | -------------------------------- | ------------------------ | ------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| M0  | Scaffold                         | —                        | Workspace with every crate stubbed honestly, CI checks matrix, repo meta, this plan                           | CI green on ubuntu, macos and windows                                                                                                           |
| M1  | Bar + clock on Hyprland          | S1, S2, S6, S7, S10, S11 | One layer-shell bar per output, theme boxes, Nerd Font text, the clock module; outputs coming and going       | a bar on every output on the owner's machine; `DrawList` snapshot tests; the Nerd Font glyph test from [04](04-rendering-and-theme.md#text)     |
| M2  | Workspaces + idle                | —                        | `lyr-wm` Hyprland backend (events, Lua dispatch); click and scroll; `lyr --stats`                             | event-parser tests on recorded lines; dispatch payload verified against `hyprctl`; idle test from [12](12-performance-and-power.md#checking-it) |
| M3  | Config                           | —                        | TOML schema, embedded defaults (the waybar port), per-platform overrides, validation, reload, control socket  | defaults-parity test against the waybar config; reload diff tests                                                                               |
| M4  | Linux modules                    | —                        | battery, power profile, network, bluetooth, volume, mic, power button actions                                 | each source tested against a fake bus or socket; manual checklist on the owner's machine                                                        |
| M5  | Popups and panels                | S3, S17                  | tooltips, power menu, panels opened by click or `lyr panel`, the clock panel's calendar                       | calendar maths tests; popup and panel placement tests; how a panel closes on an outside click is recorded; manual checklist                     |
| M6  | Notification daemon              | S12, S22, S24            | `org.freedesktop.Notifications`, toasts, history in the clock panel, DND, Clear All, MPRIS, the bell module   | a D-Bus test for each `.swaync-test` case; another daemon owning the name is detected; manual checklist                                         |
| M7  | Wi-Fi, Bluetooth and audio menus | S9, S19                  | the three native menus over NetworkManager, BlueZ and the PulseAudio protocol                                 | each menu against a fake service; every `.menu-wifi` case in [10](10-popups-and-menus.md#wi-fi-menu) has a test; manual checklist               |
| M8  | Tray + cutover (Linux MVP)       | —                        | StatusNotifierItem watcher and host, DBusMenu menus, icons; the dotfiles switch from waybar and swaync to lyr | Steam and Teams icons and menus work; the dotfiles commit lands                                                                                 |
| M9  | Windows                          | S3, S4, S7               | appbar bars, popups and panels, Windows sources and menus ([11](11-platforms.md#modules))                     | manual checklist on Windows 11; per-source tests on the Windows runner                                                                          |
| M10 | macOS                            | S3, S5, S7               | status panels, popups and panels, macOS sources and menus                                                     | manual checklist on macOS; per-source tests on the macOS runner                                                                                 |
| M11 | Release                          | S15 (crcbl on crates.io) | completions and man page, release jobs + `pkg/`, README install docs, site entry                              | first tag's CI run fully green, with artifacts in every channel                                                                                 |

M1 – M8 is the Linux MVP, which is what replaces waybar, and with it swaync and
the `krypt menu` Wi-Fi, Bluetooth and audio commands (the owner chose on
2026-10-16 to build the native parts before the switch). M9 and M10 make it
cross-platform. M11 is cut when the owner decides the MVP is good enough, and
cannot happen before crcbl is on crates.io.
