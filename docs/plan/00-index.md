# lyr plan — index

lyr is a Rust desktop status bar, a waybar replacement, for Linux (Wayland),
Windows and macOS. It is built on crcbl's shell and GPU layers and ships the
owner's waybar setup as its defaults.

Section numbers are document identity. Build order is
[15-milestones.md](15-milestones.md).

| #   | Section                                                    | Covers                                                                                              |
| --- | ---------------------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| 00  | [00-index.md](00-index.md)                                 | This list                                                                                           |
| 01  | [01-overview.md](01-overview.md)                           | Goals, baseline, MVP scope, non-goals, decisions, naming                                            |
| 02  | [02-architecture.md](02-architecture.md)                   | Process and threads, workspace crates, control socket, a click's data flow, prior art, dependencies |
| 03  | [03-surfaces-and-input.md](03-surfaces-and-input.md)       | Bar and popup windows per platform, outputs, input, **what crcbl must provide**                     |
| 04  | [04-rendering-and-theme.md](04-rendering-and-theme.md)     | Draw pipeline, repaint-on-change, text, the theme from `style.css`, icons                           |
| 05  | [05-layout-and-config.md](05-layout-and-config.md)         | Zones, TOML config, defaults, validation, reload, migrating from waybar                             |
| 06  | [06-workspaces.md](06-workspaces.md)                       | Hyprland IPC and Lua dispatch, Windows virtual desktops, macOS                                      |
| 07  | [07-modules.md](07-modules.md)                             | Every module: behaviour, defaults, sources per platform                                             |
| 08  | [08-tray.md](08-tray.md)                                   | StatusNotifierItem, DBusMenu, icons                                                                 |
| 09  | [09-notifications.md](09-notifications.md)                 | The notification daemon, Windows and macOS                                                          |
| 10  | [10-popups-and-menus.md](10-popups-and-menus.md)           | Popups and panels: tooltips, clock panel, power, Wi-Fi, Bluetooth and audio menus                   |
| 11  | [11-platforms.md](11-platforms.md)                         | Per-platform windowing and module support tables                                                    |
| 12  | [12-performance-and-power.md](12-performance-and-power.md) | Idle targets and how the design meets them                                                          |
| 13  | [13-testing.md](13-testing.md)                             | Unit, integration with fakes, manual checklists                                                     |
| 14  | [14-ci-and-release.md](14-ci-and-release.md)               | CI checks now, release pipeline later                                                               |
| 15  | [15-milestones.md](15-milestones.md)                       | Build order, crcbl dependencies, exit checks                                                        |
| 16  | [16-open-questions.md](16-open-questions.md)               | Provisional decisions awaiting the owner's call                                                     |
