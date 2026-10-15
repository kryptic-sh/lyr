# Backlog

Work raised and not finished, decisions still open, and gaps in what has been
verified. Delete an entry when it ships; `git log` keeps the history.

## Decisions awaiting the owner

The provisional decisions in
[plan/16-open-questions.md](plan/16-open-questions.md) are the defaults the
implementation follows until they are answered. The ones that block a milestone:

- **New dependencies** `libpulse-binding` and an rtnetlink crate (`rtnetlink` or
  `neli`) block M4's audio and network sources.
- **crcbl's release** blocks M9. The owner decided (2026-10-15) that crcbl's
  first crates.io release waits until S1 – S27 are built and the shell and UI
  are usable without the rest of the engine.

## Cross-repo work

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
  commit. M1 needs S1, S2, S6, S7, S10 and S11.
- **crcbl: window-kind API, backend work.** crcbl main `6483b744` (CI green) has
  the role API plus the Wayland step: layer-shell v4 panels on a named monitor,
  exclusive zone, margins, keyboard modes, namespace, fractional scale on
  panels; `ShellEvent::MonitorRemoved` on every backend (through
  `monitor::queue_removed`); `wl_output.name` in `MonitorInfo`;
  `Font::parse(impl Into<FontData>)` for runtime font bytes. That covers M1's
  S1, S2, S6, S7, S10 and S11, so M1 can pin `6483b744`. Win32 panels and AppKit
  NSPanels are on crcbl branches with CI green but not merged; S3 (`xdg_popup`)
  comes after them.

  Recorded in crcbl's backlog under its backend steps, not built yet:
  - Resizing and repositioning a live popup, with a caps flag where xdg_wm_base
    v3 is missing.
  - `ABN_FULLSCREENAPP` as an event, and `ABM_REMOVE` on drop and on a
    crash-safe path.
  - AppKit `Top` at `NSStatusWindowLevel`.
  - `MA_NOACTIVATE`, a tracking area and `acceptsMouseMovedEvents`.
  - DirectComposition for S16 on Win32.
  - S21 on layer surfaces and popups.

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
- **dotfiles: switch from waybar to lyr** at M6 (`hyprland.lua`
  `hl.exec_cmd("waybar")`;
  [plan/05](plan/05-layout-and-config.md#migration-from-waybar)).

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

- **Hyprland dispatch over the socket:** that `.socket.sock` accepts
  `dispatch hl.dsp.focus({ workspace = N })` exactly as `hyprctl dispatch` sends
  it is unverified (M2 checks it). How ironbar's `detect_lua_config()` detects a
  Lua config was not read; lyr's check for `hyprland.lua` is a guess.
- **Nerd Font glyphs through crcbl-ui:** that Hack Nerd Font Propo's private-use
  glyphs resolve through crcbl-ui's cmap lookup is untested (M1's first test).
- **swaync D-Bus method names** (`ToggleVisibility`, `ToggleDnd`, the subscribe
  signals) are from memory. Confirm them against the installed swaync's
  introspection before M4.
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
- **Own notification daemon** (replacing swaync): toasts, history, DND, the
  MPRIS widget.
- **Built-in network, bluetooth and audio menus**, replacing `krypt menu …` and
  the Settings pages.
- **`lyr import-waybar`**: convert `config.jsonc` + `style.css` to TOML.
- **sway and niri workspace backends**, and komorebi or GlazeWM on Windows.

## Not yet verified

- Nothing has run beyond the scaffold: the binary parses its command line and
  reports that no bar exists yet.
- The release pipeline (phase 2 of [plan/14](plan/14-ci-and-release.md)) does
  not exist yet.
