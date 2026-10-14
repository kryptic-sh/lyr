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

| #   | Milestone                  | Needs from crcbl         | Delivers                                                                                                     | Exit check                                                                                                                                      |
| --- | -------------------------- | ------------------------ | ------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| M0  | Scaffold                   | —                        | Workspace with every crate stubbed honestly, CI checks matrix, repo meta, this plan                          | CI green on ubuntu, macos and windows                                                                                                           |
| M1  | Bar + clock on Hyprland    | S1, S2, S6, S7, S10, S11 | One layer-shell bar per output, theme boxes, Nerd Font text, the clock module; outputs coming and going      | a bar on every output on the owner's machine; `DrawList` snapshot tests; the Nerd Font glyph test from [04](04-rendering-and-theme.md#text)     |
| M2  | Workspaces + idle          | —                        | `lyr-wm` Hyprland backend (events, Lua dispatch); click and scroll; `lyr --stats`                            | event-parser tests on recorded lines; dispatch payload verified against `hyprctl`; idle test from [12](12-performance-and-power.md#checking-it) |
| M3  | Config                     | —                        | TOML schema, embedded defaults (the waybar port), per-platform overrides, validation, reload, control socket | defaults-parity test against the waybar config; reload diff tests                                                                               |
| M4  | Linux modules              | —                        | battery, power profile, network, bluetooth, volume, mic, power button actions, notifications (swaync)        | each source tested against a fake bus or socket; manual checklist on the owner's machine                                                        |
| M5  | Popups                     | S3                       | tooltips, calendar, power menu                                                                               | calendar maths tests; popup placement tests; manual checklist                                                                                   |
| M6  | Tray + cutover (Linux MVP) | —                        | StatusNotifierItem watcher and host, DBusMenu menus, icons; the dotfiles switch from waybar to lyr           | Steam and Teams icons and menus work; the dotfiles commit lands                                                                                 |
| M7  | Windows                    | S3, S4, S7               | appbar bars, popups, Windows sources ([11](11-platforms.md#modules))                                         | manual checklist on Windows 11; per-source tests on the Windows runner                                                                          |
| M8  | macOS                      | S3, S5, S7               | status panels, popups, macOS sources                                                                         | manual checklist on macOS; per-source tests on the macOS runner                                                                                 |
| M9  | Release                    | S15 (crcbl on crates.io) | completions and man page, release jobs + `pkg/`, README install docs, site entry                             | first tag's CI run fully green, with artifacts in every channel                                                                                 |

M1 – M6 is the Linux MVP, which is what replaces waybar. M7 and M8 make it
cross-platform. M9 is cut when the owner decides the MVP is good enough, and
cannot happen before crcbl is on crates.io.
