# Backlog

Work raised and not finished, decisions still open, and gaps in what has been
verified. Delete an entry when it ships; `git log` keeps the history.

## Decisions awaiting the owner

The provisional decisions in
[plan/16-open-questions.md](plan/16-open-questions.md) are the defaults the
implementation follows until they are answered. The ones that block a milestone:

- **New dependencies** `libpulse-binding` and an rtnetlink crate (`rtnetlink` or
  `neli`) block M4's audio and network sources.
- **A date and time crate** (blocks M1's clock and M5's calendar). The clock
  formats `%Y/%m/%d %H:%M` in the system time zone, names the zone in its
  tooltip, and the calendar needs month grids and ISO week numbers. Date
  arithmetic and time zones are not something to hand-roll. `chrono` is the
  house precedent (buffr, hodl, hrdr, infr and sqeel use 0.4, mostly with
  `default-features = false, features = ["clock"]`); naming the IANA zone would
  also need `iana-time-zone`, which chrono's `clock` feature already pulls in.
  Recommended: `chrono` with `clock`, as the other repos have it.
- **`serde_path_to_error` for config errors** (M3). Plan 05 wants every config
  error to name its key path. An error serde raises while deserializing a table
  (a bad colour, a bad format string, a wrong type) carries no path in `toml`
  1.1, so lyr names only the table (`theme: …`). `serde_path_to_error` (dtolnay)
  wraps the deserializer and reports `theme.border`; it is new to lyr. Options:
  add it (recommended: one small, widely used crate, and it fixes every such
  error at once), or keep table-level errors.
- **The GPU backend on Windows** (blocks M9). The plan prefers Vulkan on Windows
  with D3D12 as the fallback ([plan/04](plan/04-rendering-and-theme.md)), as the
  draft asked. Per-pixel transparency on Win32 works only on D3D12, through
  DirectComposition; Vulkan on Windows refuses a transparent window, and crcbl
  declined to build Vulkan presenting through DirectComposition, as large when
  D3D12 is always there. lyr's bar background is transparent and its module
  boxes are drawn at opacity 0.8 (plan/04's theme table). crcbl plans for a
  transparent Win32 window made through `GpuContext` to fall through to D3D12,
  so with Vulkan first the bar would be drawn on D3D12 anyway. That fall-through
  ("backend fall-through on a refused surface") is on crcbl main at `81085253`,
  after 0.3.0. Options:
  - D3D12 first on Windows (recommended): the plan says what will actually run,
    and every lyr window there takes the same path.
  - Keep Vulkan first: lyr's transparent windows still end up on D3D12 once
    crcbl's fall-through lands, and any opaque one on Vulkan.

## Cross-repo work

- **crcbl: `crcbl-app`, a window-plus-device crate** (asked 2026-10-18, accepted
  the same day). At v0.3.0, `GpuContext`, `GpuContextDesc`, `open_window`,
  `wait_for_configure` and `TextPump` live only in the `crcbl` umbrella, which
  depends on the whole engine, including crcbl-audio's `cpal` (ALSA on Linux),
  physics and networking. crcbl is moving them, with the backend fall-through
  and the adapter pin, into a new leaf crate `crcbl-app` (core, hal, shell, ui
  and the GPU backends only), re-exported from the umbrella. It lands in crcbl
  0.4.0, which is breaking anyway (`TEXT_IME`'s meaning changed), after the
  null-alpha fix and D3D12 start-up work. Drawing the bars (M1's render half)
  waits on it; crcbl reports when it is on main and when 0.4.0 is out.
- **crcbl: every lyr and pikr feature on every backend.** At the owner's request
  (2026-10-17), crcbl's session was asked to make each S-row work on Wayland,
  X11, Win32 and AppKit, and on Vulkan, D3D12 and Metal; to report which
  backends each row is verified on; and to make a backend that cannot support a
  feature fail loudly rather than succeed silently. crcbl's matrix, at main
  `4d48eaf4`, is in [plan/03](plan/03-surfaces-and-input.md#backend-matrix),
  with the cells that matter to lyr's milestones. crcbl's order of work from it:
  1. The silent gaps: cursor-shape-v1 on Wayland and libxcb-cursor on X11;
     AppKit IME on/off, secure input, pre-edit and candidate rectangle; an
     honest `TEXT_IME` cap (split, and cleared on X11); Vulkan composite-alpha
     validation; `Exclusive` panel focus kept or documented as it behaves, and
     `FocusRefused` on AppKit.
  2. A transparent Win32 window through `GpuContext` falls through to D3D12.

  On crcbl main at `81085253`, after 0.3.0, reported 2026-10-18: `TEXT_COMPOSE`
  split from `TEXT_IME`, which now means a real input method (on Wayland, set
  only when `zwp_text_input_manager_v3` is bound); a `PANEL_EXCLUSIVE_KEYBOARD`
  cap; the backend fall-through; Vulkan refusing a composite alpha the surface
  lacks. Not yet: cursor-shape-v1 on Wayland (next), and a test for S21. 3.
  AT-SPI (through `accesskit_unix`, which pulls in `zbus`; crcbl is confirming
  that dependency with the owner) and AppKit accessibility; X11 panels, popups
  and transparency. 4. Tests for the untested cells, including a Hyprland job
  and a FreeBSD VM job.

  crcbl reports each as it lands; the matrix in plan/03 is updated then.

- **crcbl: an image and asset crate (S28)**, asked for on 2026-10-16 at the
  owner's request, for lyr and pikr both, instead of each app adding `image`,
  `resvg` and `freedesktop-icons`. It covers PNG, JPEG and SVG decoding to
  RGBA8, the raw buffers the apps receive (SNI ARGB32, notification
  `image-data`, Win32 BGRA DIBs), resizing, XDG icon-theme lookup with pikr's
  caching, limits for untrusted input, and CPU-only `Send` types. lyr needs it
  for M6 (notification images and icons) and M8 (tray). pikr's session was told
  the same day. crcbl logged it as item 14 of its order of work, after S12. The
  owner chose to have crcbl write its own decoders rather than add `zune-jpeg`
  or `resvg`: PNG stays on the `png` crate crcbl already has, and JPEG, the SVG
  subset icon themes use, WebP, BMP, ICO and XPM are written in crcbl in safe
  Rust, fuzzed and capped. M6 needs the theme lookup as well as decoding, for
  notification `app_icon` names and the `desktop-entry` hint. crcbl's SVG subset
  refuses an SVG it cannot fully draw, so some theme icons will fail until its
  pass rate on real themes is good. lyr falls back to a generic icon (plan 08,
  09). It shipped as `crcbl-image` in crcbl 0.2.0 (reported 2026-10-16): PNG,
  JPEG, BMP, ICO, WebP (lossy and lossless), XPM, and SVG behind the `svg`
  feature (`svg::decode(bytes, size, scale, &limits)`, which returns an error
  for any feature it does not support, never a partial icon; filters are still
  to come, and `<image>` landed on main at `2efab975` after the release);
  `raw::convert`, with `raw::from_argb32_be` (SNI pixmaps) and
  `raw::from_image_data` (notification `image-data`); resizing (`fit_within`,
  `shrink_to_fit`, `nearest_size`); and the XDG theme lookup (`IconThemes`).
  That covers M6 and M8. The entry points were read at v0.3.0 (below). 0.2.0
  also moved `crcbl_ui::font::raster` to `crcbl_image::raster` (re-exported at
  the old path), taking `raster::Point` instead of `glam::Vec2`; lyr is affected
  only if it rasterises outlines itself.
- **crcbl: window kinds and platform features for lyr and pikr.** Requirements
  S1 – S27 in
  [plan/03-surfaces-and-input.md](plan/03-surfaces-and-input.md#what-crcbl-must-provide)
  are crcbl's top-priority program ("lyr and pikr before the first release" in
  crcbl's `docs/backlog.md`), started 2026-10-15. The order is: the seam (S10,
  S15); the loop (S6, S19, S25); window kinds (S2, S3, S8, S16); Wayland (S1,
  S3, S17, S21); Win32 (S4, S27); AppKit (S5); outputs (S7); text (S11, S12,
  S22); input (S9, S20); UI fit (S23, S24); start-up and failure (S13, S26);
  FreeBSD CI (S18); AccessKit (S14) last. crcbl's session reports each row as it
  lands on main. M1 needs S1, S2, S6, S7, S10 and S11. Since 2026-10-16 lyr also
  needs S9, S12, S17, S19 and S22, which were pikr's alone before: the native
  notification centre and menus need text entry, markup and images, panel focus,
  a caret timer and emoji. crcbl's session was told the same day and replied
  with where each stands. The APIs named below were read at v0.3.0 (below); the
  test and behaviour claims are crcbl's report:
  - S9, in: XKB compose and dead keys on Wayland and X11, and text-input-v3 at
    crcbl `da20fe58`, on main since 0.1.0. `set_text_input_area` places the
    candidate window. Not built: a pre-edit shown during a compose sequence,
    surrounding text (`delete_surrounding_text` is ignored), content types
    beyond normal, password and PIN, and AppKit secure input. Not tested: losing
    the keyboard mid-sequence, the cursor rectangle reaching the input method,
    and X11 compose through a real server. crcbl judges the Wi-Fi password field
    needs none of the missing pieces. The password hint lyr asked for is on main
    in 0.2.0: `Shell::set_text_input(window, Option<TextPurpose>)` with
    `TextPurpose::{Normal, Password, Pin}` replaces the unreleased
    `set_text_input_enabled`. On Wayland `Password` and `Pin` send
    text-input-v3's purpose with the hidden-text and sensitive-data hints; on
    Win32 they turn composition off. crcbl-ui's masked field reports it through
    `Ui::text_editing_purpose()`, and `TextPump::engage` passes it on.
  - S17, in: on-demand layer panels report `WindowState::focused = false` when
    another window takes focus (a sway e2e test). Not tested on Hyprland,
    because crcbl's CI has only sway. A click on the empty desktop is not caught
    by focus loss; that waits on M5's Hyprland measurement
    ([plan/10](plan/10-popups-and-menus.md#two-kinds-of-window)).
  - S19, in: `Shell::waker` / `ShellWaker`, and waits that take a deadline,
    wake-checked on Wayland (sway), X11 (Xvfb), Win32 and AppKit. Nothing is
    left for callers.
  - S12: 0.1.0 has one-line spans with their own colour, size and family, and
    images (`Content::Image`, `Ui::set_image`). 0.2.0 adds what notification
    markup needs (M6): `Ui::paragraph(sel, inline, |p| ..)` with
    `p.run(selector, text, ..)` and `p.run_keyed(key, selector, text, ..)`. Runs
    wrap as one paragraph and are styled by selector, so `<b>` maps to a CSS
    rule; `font-weight` and `font-style` (synthesised when no bold or italic
    face exists), `text-decoration: underline`, and link runs that report hits
    on every line fragment (`Response::clicked`, `:hover`, cursor).
  - S22, in 0.2.0: colour emoji (CBDT, sbix and COLR, drawn by the UI pass) with
    per-glyph fallback through `Ui::set_fallback_fonts`.
  - S14: a `crcbl-shell` feature `accesskit`, with
    `Shell::update_accessibility`, `ShellEvent::Accessibility`,
    `Ui::accessibility_update` and `Ui::accessibility_action`, in 0.2.0. Live on
    Windows (UIA); AT-SPI (Linux) and AppKit are still to come.

  **crcbl 0.3.0 is on crates.io** (all 42 crates; tag `v0.3.0` on `2399de10`;
  every publish job green), reported 2026-10-18. It has every row lyr needs, S28
  included, OpenType shaping (`harfrust`, so ZWJ emoji, flags and ligatures
  join), the opacity fixes below, and `crcbl-font`, which the owner chose over
  `fontdb` on 2026-10-17 ([plan/04](plan/04-rendering-and-theme.md#text)). So
  lyr depends on crates.io from the start and needs no git pin. lyr's crates are
  `crcbl-shell`, `crcbl-hal`, `crcbl-ui`, `crcbl-frame`, `crcbl-image` and
  `crcbl-font`, with `crcbl-vk`, `crcbl-dx12` or `crcbl-mtl` per platform. The
  `crcbl` umbrella's `system-fonts` feature turns on `parsed-font` and
  `crcbl-ui/system-fonts`. Adding the crates happens when M1 starts.
  crcbl-font's API: `SystemFonts::find(&FontQuery)` (file and face index for a
  family, weight and style, matched by CSS rules), `SystemFonts::fallbacks`,
  `Ui::register_system_font`, `Font::from_system`. Breaking in 0.3.0, neither of
  which lyr calls yet: `Font::kerning`, `Font::kerning_pairs`,
  `fallback::map_char`, `map_text` and `Mapped` are gone (use
  `font::shape::shape` and `fallback::cluster_face`), and every windowed
  `SurfaceTarget` variant gained `transparent: bool`.

- **crcbl: window-kind API, backend work.** Read against the source at crcbl
  main `6483b744` (CI green): layer-shell v4 panels on a named monitor,
  exclusive zone, margins, keyboard modes, namespace, fractional scale on
  panels; `ShellEvent::MonitorRemoved` on every backend (through
  `monitor::queue_removed`); `wl_output.name` in `MonitorInfo`;
  `Font::parse(impl Into<FontData>)`. That covers M1's S1, S2, S6, S7, S10 and
  S11.

  Reported by crcbl's session since then, at `72999dd7`, `531903c7` and
  `5bbdd180`, read at v0.3.0 (below):
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

  **Read at crcbl v0.3.0 on 2026-10-18** (`git show v0.3.0:…`; the panel, popup,
  waker, focus, text-input, paragraph, font, image and GPU APIs named in this
  file). Confirmed as reported, except:
  - No deadline wait. The only wait is
    `Shell::wait_events(&mut self, timeout: Option<Duration>)`, advisory and
    behind `ShellCaps::EVENT_WAIT`, followed by `pump`. lyr computes the time to
    its next tick (clock, caret) and passes that.
  - A panel targets `PanelDesc::monitor: Option<MonitorId>`, not an output name.
    lyr maps its configured output names through `MonitorInfo::name`.
  - Exclusive zones are `Reserve::{None, OwnExtent, IgnoreOthers}` (0, the
    panel's extent, -1); there is no separate zone field.
  - A link run's click is the field `Response.clicked`, not a method.
  - `reposition_popup(popup, anchor, size)` defaults to `Unsupported` and needs
    `ShellCaps::POPUP_REPOSITION`.
  - crcbl's workspace sets `rust-version = "1.98"`; lyr's moved to 1.98 with
    `crcbl-shell`.

  The GPU path M1 follows (no UI-on-shell-window example exists in crcbl):
  `crcbl::shell::open`, `crcbl::engine::open_window`, `wait_for_configure` for
  the extent, `crcbl::engine::GpuContext::open(shell, window, extent, &desc)`,
  and `crcbl_frame::ui_pass::UiRenderer::new(device, queue, format)`. Each
  frame: `Ui::begin_frame`, build, `Ui::layout`, `Ui::emit` into a `DrawList`;
  `GpuContext::acquire`, `UiRenderer::begin_frame`, a `RenderGraph` with the
  swapchain imported and `UiRenderer::add_passes`, then
  `GpuContext::submit_and_present`. Steps 1 – 4 are in crcbl's `apps/bare`, the
  frame in `PageBundle::frame` (`crates/crcbl/src/engine/page.rs`). Read by a
  sub-agent and spot-checked: `rust-version`, `wait_events`,
  `PanelDesc::monitor` and `Response.clicked`.

  Win32 per-pixel transparency is in 0.3.0: on D3D12, `ShellCaps::TRANSPARENT`
  is set and `WindowDesc::transparent` works for toplevels, panels and popups,
  through DirectComposition, with no DWM border or rounded corners on frameless
  windows. Vulkan on Windows refuses a transparent window by name.
  `SurfaceTarget::Win32` gained a `transparent` field. `GpuContext` asks for
  premultiplied alpha for a transparent window and fails with
  `GpuError::Unusable` when no adapter can do it, and crcbl-mtl no longer forces
  `CAMetalLayer.opaque`.

  Still open in crcbl, as of 0.1.0 (0.2.0's report did not restate them): no
  flash on map on AppKit, cold-start timing (S13), FreeBSD running (S18),
  AccessKit on Linux and macOS (S14).

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
  quoting rule. Click is `focus_request` and scroll is `State::step`, both pure
  and tested; still to build: wiring them into the bar (M1's window and M3's
  `workspaces.scroll`), and `lyr --stats`. The `WorkspaceBackend` trait waits
  for the Windows backend (M9). Hyprland main's changes for the next release are
  in [plan/06](plan/06-workspaces.md#linux--hyprland).
- **Shell, rest of M1.** `lyr-shell` has the bars (`bars::Bars`): one
  transparent layer panel per output (or per named output), stretched along the
  configured edge and reserving its height, created and destroyed as monitors
  come and go and as the config changes, tested against crcbl-shell's
  `HeadlessShell` and mutation-checked. Not built: drawing into them (waits on
  `crcbl-app`), scale changes, pointer input to hit tests, and a run on a real
  compositor (the owner's Hyprland is the M1 check; this machine is Windows,
  where the same panels are Win32 appbars).
- **Core, rest of M1.** `lyr-core` has the zone layout (`layout::layout`, tested
  and mutation-checked). Icon ramps are `icon::ramp`, transcribing waybar
  0.15.0's `ALabel::getIcon` and tested against its integer steps. Hit testing
  is `hit::Regions` (topmost region wins; boxes are half-open, so a shared edge
  belongs to one box). Not built: module state, the clock view (waits on the
  `chrono` decision above), and `DrawList` building with its snapshot tests.
  Measuring text (the widths `layout` takes) needs crcbl-ui's font atlas, so it
  is done by the caller in `lyr-render`.
- **Control socket.** Built: the requests (`lyr_core::control::Request`, one
  text line each) and the CLI commands that send them, including `lyr panel`,
  `lyr dnd` and `lyr clear`, which still report "not implemented". Not built:
  the socket itself (`interprocess`, a Unix socket or a named pipe), the reply
  line, single-instance detection, and `lyr event workspace` for AeroSpace
  (M10). The socket needs the bar's main loop to hand requests to (M1).
- **Config, rest of M3.** Built in `lyr-config`: the schema for `[bar]`,
  `[layout]`, `[workspaces]`, `[clock]` and `[theme]`, the embedded defaults,
  `[platform.*]` layering on both the defaults and the user file, and validation
  that names the key (unknown keys and modules, modules the platform cannot run,
  colours, ranges, workspace format placeholders), with tests that were each
  broken and seen to fail. Not built: the other modules' tables and actions
  (with M4), checking the clock's strftime format, reload (the file watcher,
  `lyr reload`, the old-and-new diff), the control socket (its own entry above),
  the README's schema docs, and the defaults-parity test against `config.jsonc`.
  The dotfiles are at `~/Projects/mxaddict/dotfiles` on the owner's Windows
  machine, not next to lyr, and not on CI, so the test needs either a path from
  the environment or a copy of `config.jsonc` checked into lyr as a fixture;
  that is open. Format strings are one `Format` type (`lyr-config`'s
  `format.rs`), parsed when the config loads and rendered with `Format::render`,
  so validation and drawing will share one parser. Errors name the key where
  lyr's own checks find them (`layout.right[0]: …`), but an error serde raises
  inside a table names only the table (`theme: ...`); see the
  `serde_path_to_error` decision above. `lyr-config` keeps its own copy of the
  table merge, because `hjkl_config`'s `deep_merge` and `locate` are private in
  0.42.2; if hjkl exports them, lyr drops the copy.
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
