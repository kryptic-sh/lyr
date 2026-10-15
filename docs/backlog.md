# Backlog

Work raised and not finished, decisions still open, and gaps in what has been
verified. Delete an entry when it ships; `git log` keeps the history.

## Decisions awaiting the owner

The provisional decisions in
[plan/16-open-questions.md](plan/16-open-questions.md) are the defaults the
implementation follows until they are answered. The ones that block a milestone:

- **New dependencies** `libpulse-binding` and an rtnetlink crate (`rtnetlink` or
  `neli`) block M4's audio and network sources.
- **crcbl's release** blocks M11. The owner decided (2026-10-15) that crcbl's
  first crates.io release waits until S1 – S27 are built and the shell and UI
  are usable without the rest of the engine.

## Cross-repo work

- **crcbl: an image and asset crate (S28)**, asked for on 2026-10-16 at the
  owner's request, for lyr and pikr both, instead of each app adding `image`,
  `resvg` and `freedesktop-icons`. It covers PNG, JPEG and SVG decoding to
  RGBA8, the raw buffers the apps receive (SNI ARGB32, notification
  `image-data`, Win32 BGRA DIBs), resizing, XDG icon-theme lookup with pikr's
  caching, limits for untrusted input, and CPU-only `Send` types. lyr needs it
  for M6 (notification images and icons) and M8 (tray). pikr's session was told
  the same day. Not acknowledged by crcbl yet.
- **crcbl: window kinds and platform features for lyr and pikr.** Requirements
  S1 – S27 in
  [plan/03-surfaces-and-input.md](plan/03-surfaces-and-input.md#what-crcbl-must-provide)
  are crcbl's top-priority program ("lyr and pikr before the first release" in
  crcbl's `docs/backlog.md`), started 2026-10-15. The order is: the seam (S10,
  S15); the loop (S6, S19, S25); window kinds (S2, S3, S8, S16); Wayland (S1,
  S3, S17, S21); Win32 (S4, S27); AppKit (S5); outputs (S7); text (S11, S12,
  S22); input (S9, S20); UI fit (S23, S24); start-up and failure (S13, S26);
  FreeBSD CI (S18); AccessKit (S14) last. crcbl's session reports each row as it
  lands on main. Until the release, lyr pins crcbl by git revision on a green
  commit. M1 needs S1, S2, S6, S7, S10 and S11. Since 2026-10-16 lyr also needs
  S9, S12, S17, S19 and S22, which were pikr's alone before: the native
  notification centre and menus need text entry, markup and images, panel focus,
  a caret timer and emoji. crcbl's session was told the same day and replied
  with where each stands. **Reported, not read yet:**
  - S9: compose and dead keys on Wayland and X11, and text-input-v3 at crcbl
    `da20fe58`. A window gets an input method only while
    `Shell::set_text_input_enabled(window, true)` is on (`TextPump::engage` does
    it); `set_text_input_area` places the candidate window. Missing: pre-edit
    during a compose sequence, surrounding text, and content types (always
    "normal"). The Wi-Fi password field wants a password hint, so the input
    method does not learn it; lyr asked for it before M7.
  - S17: on-demand layer panels report `WindowState::focused = false` when
    another window takes focus (a sway e2e test). Not tried on Hyprland.
  - S19: `Shell::waker` / `ShellWaker`, and waits that take a deadline.
  - S12, partial: one-line spans with their own colour, size and family, and
    images (`Content::Image`, `Ui::set_image`). Missing: spans wrapping as one
    paragraph, font weight (bold is a separately registered family), italic,
    underline, and link hit areas, all of which notification markup needs (M6).
  - S22: not started.

  crcbl 0.1.0 is being published to crates.io, slowed by the new-crate rate
  limit; crcbl's session reports when every crate is up.

- **crcbl: window-kind API, backend work.** Read against the source at crcbl
  main `6483b744` (CI green): layer-shell v4 panels on a named monitor,
  exclusive zone, margins, keyboard modes, namespace, fractional scale on
  panels; `ShellEvent::MonitorRemoved` on every backend (through
  `monitor::queue_removed`); `wl_output.name` in `MonitorInfo`;
  `Font::parse(impl Into<FontData>)`. That covers M1's S1, S2, S6, S7, S10 and
  S11.

  Reported by crcbl's session since then, at `72999dd7`, `531903c7` and
  `5bbdd180`. **Not read yet**; read it before M1 relies on it:
  - Win32 panels as tool windows, with `Reserve::OwnExtent` as a real appbar and
    `WS_EX_NOACTIVATE` for `PanelKeyboard::None`.
  - AppKit `NSPanel` at the status level; `PANEL_RESERVE` clear on macOS.
  - Popups (S3) on all three backends; `PopupInput::Passthrough` as an empty
    input region, `HTTRANSPARENT` or `ignoresMouseEvents`.
  - `TRANSPARENT` on Wayland.
  - `ShellEvent::FullScreenApp` for Win32 appbars.
  - `Shell::reposition_popup` behind `ShellCaps::POPUP_REPOSITION`. The Wayland
    path needs xdg_wm_base 3 and has not run anywhere.
  - A CSS `cursor` property with `Ui::cursor()`; `ImageAtlas::unregister`.
  - A D3D12/WARP fallback on Windows, and `GpuError::NoBackend` elsewhere.
  - `ui.scrollbar(..)`: vertical only.

  Still open in crcbl: Win32 per-pixel alpha (DirectComposition), no flash on
  map, colour emoji (S22), cold-start timing (S13), FreeBSD running (S18),
  AccessKit (S14).

- **crcbl: release pipeline.** Built and pushed by crcbl's session on 2026-10-15
  (tmxr-style: crates.io, AUR, Homebrew, Scoop, GitHub releases; no Alpine,
  because crcbl links ALSA), with the org secrets granted. It is not used until
  the release above.
- **crcbl: Wayland present path.** Whether crcbl presents only after a frame
  callback, or can block in FIFO `vkQueuePresentKHR` on an output that is off
  (hypridle DPMS), is unverified
  ([plan/04](plan/04-rendering-and-theme.md#repaint-on-change-only)).
- **pikr moves to crcbl** (the owner's decision, 2026-10-15), dropping floem,
  the floem and winit forks, iced and the SCTK fallback. pikr needs S1, S6 and
  S8 – S27. pikr's session tracks the move in pikr's own backlog and
  `gui-backend-research.md`.
- **wayr's README still names pikr as a consumer.** That is now wrong, and no
  wayr session exists to fix it. It is a one-line change in the wayr repo, for
  the owner.
- **dotfiles: switch from waybar, swaync and the `krypt menu` Wi-Fi, Bluetooth
  and audio commands to lyr** at M8
  ([plan/05](plan/05-layout-and-config.md#migration-from-waybar)). Whether
  `wiremix` and `bluetui` stay installed as standalone tools, and whether the
  `krypt menu` names stay as wrappers, is the owner's call then.

## The fallback stack, if crcbl declines

wayr would be the Linux backend again. Its gaps, read in its source on
2026-10-15:

- `EventLoop::run_app` caps every wait at 50 ms (`pump_cap` in
  `wayr/src/event_loop.rs`), so it wakes 20 times a second when idle.
- No external wake source: other threads can only reach the loop through
  `EventLoopProxy`, which relies on that 50 ms cap to be noticed.
- `LayerSurfaceBuilder::build` always passes `None` for the output, so a bar
  cannot be put on a chosen monitor.
- No `xdg_popup` support, so no popups.

## Known gaps in the design

- **Hyprland backend, rest of M2.** `lyr-wm::hyprland` is built against Hyprland
  v0.56.2's formats: the event parser, the `State` reducer, the JSON seed, and
  `Hyprland` (socket I/O, `from_env`, `subscribe`, `focus`). Its tests run
  against a fake Hyprland on Unix sockets, not a real one. Not verified on a
  running Hyprland: the Lua focus request (the framing was read from hyprctl's
  source), and that `hyprland.lua` existing is how Hyprland picks a Lua config
  (ironbar's `detect_lua_config()` was not read; Hyprland also takes `--config`,
  which lyr ignores). Named workspaces get no request in Lua until there is a
  quoting rule. Still to build: click and scroll (scrolling has no waybar
  precedent in the dotfiles), wiring it into the bar, and `lyr --stats`. The
  `WorkspaceBackend` trait waits for the Windows backend (M9). Hyprland main's
  changes for the next release are in
  [plan/06](plan/06-workspaces.md#linux--hyprland).
- **Waybar parity, still to check.** The plan was reviewed against the dotfiles'
  `config.jsonc` and `style.css` (2026-10-16), with waybar's own defaults read
  in its 0.15.0 source. Open: the calendar's `<tt>` monospace look with a
  proportional default font (`calendar_font`, M5); the logind method behind
  `systemctl sleep` (M5). The defaults-parity test (M3) should read the glyphs
  from `config.jsonc` itself, not from the plan: the plan once lost every BMP
  private-use glyph without anyone noticing, and now writes them as code points.
- **Nerd Font glyphs through crcbl-ui:** that Hack Nerd Font Propo's private-use
  glyphs resolve through crcbl-ui's cmap lookup is untested (M1's first test).
- **Native notification centre and menus, not yet researched**
  ([plan/09](plan/09-notifications.md), [plan/10](plan/10-popups-and-menus.md)).
  Written from memory and the dotfiles, with each item to read when its
  milestone starts: the Desktop Notifications Specification's exact methods,
  hints and close reasons (M6); `dotfiles/.config/swaync/style.css`, 349 lines,
  not read (M6); swaync's own behaviour where the plan picked a default
  (critical notifications in DND, the history order); NetworkManager's D-Bus
  names and failure reason codes (M7); bluetui's and wiremix's actual feature
  sets (M7); the Windows and macOS menu APIs (M9, M10).
- **Closing a panel on an outside click.** A panel opened from a key binding
  cannot be an `xdg_popup` with a grab, because there is no input serial. Losing
  keyboard focus (S17) closes it on a click into another window, but not on a
  click on the empty desktop. M5 records what Hyprland does and picks a fix.
- **Does the year calendar fit the 500 px panel?** Measured in M5; the panel
  widens if not ([plan/10](plan/10-popups-and-menus.md#clock-panel)).
- **Undocumented Windows APIs:** virtual desktops
  (`IVirtualDesktopManagerInternal`) and setting the power mode
  (`PowerSetActiveOverlayScheme`) can break with any Windows build.
- **macOS cannot reserve screen space.** The bar overlaps windows unless the
  menu bar is set to auto-hide.
- **No tray and no notification state on Windows or macOS.** Neither platform
  offers an API ([plan/08](plan/08-tray.md),
  [plan/09](plan/09-notifications.md)).

## Ideas considered and declined

- **crcbl as a git-only dependency for a release:** not possible, because
  `cargo publish` rejects git dependencies. Development can use a git revision;
  a release has to wait for crcbl on crates.io.
- **floem (pikr's current stack) as lyr's base:** it rests on deep forks of
  floem and winit, and pikr's own research shelved it.
- **iced + `iced_layershell`:** Wayland-only layer-shell. Windows and macOS
  would still need appbar and panel work outside iced. It was not evaluated
  further once crcbl was chosen.
- **Hosting tray icons on Windows** by reading Explorer's notification-area
  toolbar: fragile, and broken by taskbar updates.

## Post-MVP

- **Window title module** (`hyprland/window`; the draft plan listed it, but
  today's bar does not have it). Built on Hyprland's `activewindowv2` event.
- **`lyr import-waybar`**: convert `config.jsonc` + `style.css` to TOML.
- **sway and niri workspace backends**, and komorebi or GlazeWM on Windows.

## Not yet verified

- Nothing has run beyond the scaffold: the binary parses its command line and
  reports that no bar exists yet.
- The release pipeline (phase 2 of [plan/14](plan/14-ci-and-release.md)) does
  not exist yet.
