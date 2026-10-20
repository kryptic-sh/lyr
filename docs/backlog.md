# Backlog

Work raised and not finished, decisions still open, and gaps in what has been
verified. Delete an entry when it ships; `git log` keeps the history.

## Deferred: questions for the owner

Each item below waits on a decision that is the owner's to make, and is deferred
until it is answered. The question comes first, then the options and what each
costs. The provisional decisions in
[plan/16-open-questions.md](plan/16-open-questions.md) are the defaults the
implementation follows meanwhile.

- **Audio: may lyr take `libpulse-binding`?** (M4 volume and mic modules, M7
  audio menu.) **Q:** Add `libpulse-binding` (the PulseAudio protocol through
  `pipewire-pulse`, plan 07's default), or go native PipeWire with WirePlumber
  metadata instead (no libpulse, more code)? Nothing for audio is built until
  this is answered.
- **Network without NetworkManager: build the rtnetlink fallback?** (M4.) **Q:**
  The network module reads NetworkManager over zbus, which the dotfiles use.
  Should lyr also support machines without it, through a new rtnetlink crate
  (`rtnetlink` or `neli`) and a 30 s nl80211 read for the signal (plan 07), or
  leave the module out there?
- **Calendar locale: may lyr take `libc`?** (M5.) **Q:** Read the first weekday
  and day and month names from the system locale, as waybar does (`nl_langinfo`
  with `_NL_TIME_FIRST_WEEKDAY` on Linux, which needs `libc`, not on plan 02's
  list; on Windows `GetLocaleInfoEx` with `LOCALE_IFIRSTDAYOFWEEK` through
  `windows-sys`, which is), or add a `clock.first_weekday` config key defaulting
  to Monday, with English names, which a Sunday-first locale sets by hand?
- **Week numbers: waybar's or ISO?** (M5.) **Q:** The calendar numbers weeks as
  waybar does for a Monday start (`%W`), since the dotfiles do not set
  `iso8601`. ISO weeks (`%V`) are a one-line change. Keep `%W`?
- **Calendar font: proportional or monospace by default?** (M5.) **Q:**
  `theme.calendar_font` defaults to the bar's font, where waybar's `<tt>` makes
  the calendar monospace. Should the default be `"monospace"`? Judge it against
  waybar's look on the owner's machine.
- **Images: may lyr take `crcbl-image`?** (M6, M8.) **Q:** Add `crcbl-image`
  (crcbl's decoder for PNG, JPEG, SVG and icon themes) so toasts show `app_icon`
  and `image-data`, players their art, and the tray its icons? crcbl-ui already
  draws images. Until then the tray shows each item's first letter, and
  `icon-static` is not advertised.
- **Notification links: how should a click open one?** (M6.) **Q:** lyr draws a
  body's `<a href>` (in `toast_link`, underlined) but opens nothing and does not
  advertise `body-hyperlinks`. A link comes from any program on the session bus,
  so: open it with the system's URL handler (`xdg-open`)? Which schemes (only
  `http`/`https`)? With a confirmation first?
- **Two swaync style values that look like slips** (M6). **Q:** The dotfiles'
  swaync style has `font-family: 'Hadk Nerd Font'`, which names no font (swaync
  falls back; lyr uses the bar's font), and action buttons at `0.2rem`, about 3
  px (lyr uses the bar's text size). Copy them as they are, or treat them as
  typos ([plan/09](plan/09-notifications.md#what-swaync-does-today))?
- **Bluetooth favourites: where do they live?** (M7.) **Q:** bluetui (master
  `ff92b24`, `src/favorite.rs`) keeps favourites as addresses in `favorites.txt`
  under its data directory, sorts them first and marks them. lyr writes no state
  today. Options: a file lyr writes (`~/.local/share/lyr/`; the menu can toggle
  it); a `bluetooth.favourites` list in `config.toml` (declarative, but the menu
  cannot toggle it, since lyr never writes the config); or no favourites, since
  BlueZ's `Trusted` already marks the devices that matter.
- **Config reload on save: may lyr take `notify`?** (M3.) **Q:** Reloading the
  config when the file is saved needs a file watcher; `notify` is the usual
  crate and new to lyr. Add it, or keep reloading on `lyr reload` only?
- **The defaults-parity test: where does it read `config.jsonc`?** (M3.) **Q:**
  The test that checks lyr's defaults against the dotfiles' waybar config needs
  that file, which is at `~/Projects/mxaddict/dotfiles` on the owner's machine
  and not on CI. Check a copy into lyr as a test fixture, or read a path from
  the environment and skip the test on CI?
- **The `lyr` name on crates.io is taken** (M11). **Q:** `lyr` on crates.io is
  someone else's (Serial-ATA's lyrics tool, 0.3.0, last published 2024-07-23;
  checked 2026-10-20), so the app crate cannot be published under it. The
  library names (`lyr-core`, `lyr-config`, `lyr-render`, `lyr-shell`,
  `lyr-sources`, `lyr-wm`) are free, and `lyr` and `lyr-bin` are free on the
  AUR. Options: publish the app as another package (say `lyr-bar`) with its
  binary still named `lyr`; ask the owner of `lyr` for the name; or do not
  publish the app to crates.io (the libraries only, with binaries from the
  release page, the AUR, Homebrew and Scoop).
- **Release pipeline: build it now, and may it take `clap_complete` and
  `clap_mangen`?** (M11.) **Q:** crcbl is on crates.io, which was phase 2's hard
  block (plan 14). Copy tmxr's release jobs and `pkg/` templates now, untried
  until the first tag, or when the owner decides to cut the first release? The
  binary's `--completions` and `--man` need `clap_complete` and `clap_mangen`,
  new to lyr. The org secrets' allowlists and the site entry are the owner's to
  set.
- **The dotfiles' standalone tools** (M8). **Q:** When the dotfiles switch to
  lyr, do `wiremix` and `bluetui` stay installed, and do the `krypt menu` names
  stay as wrappers?
- **Windows Bluetooth and Wi-Fi menu: build them on a machine that cannot show
  them?** (M9.) **Q:** The development machine's Bluetooth adapter is in a PnP
  error state and WinRT's `Radio::GetRadiosAsync` lists no radios at all
  (checked 2026-10-20), Wi-Fi included: it is on Ethernet. So a Windows
  Bluetooth source (`Windows.Devices.Radios` for on/off/disabled, a
  `DeviceWatcher` on `BluetoothDevice` and `BluetoothLEDevice`'s connected
  selectors for the devices) could only be seen hiding the module, and the
  native Wi-Fi menu (Native Wifi's `WlanScan`, `WlanGetAvailableNetworkList` and
  `WlanConnect`, plan 10) only seen finding no interface; the network module's
  own Wi-Fi reading (`WlanQueryInterface`) is built and equally unseen. Build
  them checked only that far, wait for a machine or dongle with working radios,
  or leave them out on Windows? Until then a click opens `ms-settings:bluetooth`
  or `ms-settings:network`.
- **Windows workspaces: native virtual desktops on an Insider build?** (M9.)
  **Q:** Plan 06's default is the undocumented `IVirtualDesktopManagerInternal`
  with a per-build IID table, refusing an unknown build. The development machine
  runs build 26300, which public IID tables (VirtualDesktopAccessor's and
  others') may not cover yet, and a wrong IID calls through the wrong vtable
  rather than failing cleanly, so it cannot be guessed. Options: build the table
  for the released builds (24H2, 26100) and leave the module out on this
  machine; target komorebi or GlazeWM's IPC instead (stable, but only if one of
  them gets used); or leave workspaces out on Windows.
- **Windows power profile: switch it through an undocumented export?** (M9.)
  **Q:** Plan 16's question 13 defaults to `PowerSetActiveOverlayScheme`, which
  neither `windows-sys` nor `windows` binds: it would be an `extern` on
  `powrprof.dll` with the overlay GUIDs written out from outside sources, and
  plan 11 has each undocumented API refuse a build missing from its table. The
  development machine runs Insider build 26300, which such a table would not
  hold, so the switch would refuse here and could not be seen working (reading
  the mode back through `PowerGetEffectiveOverlayScheme` would be the check).
  Build it with a table for the released builds, or keep the module read-only on
  Windows, as on macOS? Until then a click logs that `cycle_profile` is not
  implemented.
- **`lyr import-waybar`: carry `style.css` into `[theme]`?** (Post-MVP.) **Q:**
  The importer carries `config.jsonc` only. Mapping `style.css` means parsing
  CSS and choosing which of waybar's selectors (`#battery`, `window#waybar`,
  `.module` and so on) stand for which `[theme]` key. Parsing it well needs a
  CSS parser lyr does not have (`cssparser` or `lightningcss`, new
  dependencies), or a hand-written reader for the declarations the dotfiles'
  file uses, which would misread anything else. Add a parser, read only the
  dotfiles' shapes, or leave the theme to the defaults (which are already the
  dotfiles' `style.css` values, plan 04)?
- **The window module: carry waybar's `rewrite` and `separate-outputs`?**
  (Post-MVP.) **Q:** waybar's `hyprland/window` can rewrite titles by regular
  expression and show each output's own focused window. Neither is in plan 07;
  `rewrite` would need the `regex` crate, new to lyr, and `separate-outputs`
  needs Hyprland's per-monitor active window. Carry them, or keep the module to
  `{title}`, `{class}` and `max_length`? (Its app icon waits on images, the
  `crcbl-image` question above.)

## Cross-repo work

- **crcbl: what lyr uses, and at which release.** lyr pins every crcbl crate at
  `=0.6.0` (published 2026-10-20; crcbl's `rust-version` is 1.98, and lyr's
  follows). crcbl 0.4.0 (2026-10-18) brought `crcbl-app`, the window-plus-device
  crate lyr asked for so that it links no engine, audio (cpal, ALSA) or physics;
  0.5.0's only breaking change is in crcbl-phys. M1 adds `crcbl-app`,
  `crcbl-hal`, `crcbl-ui` (with `system-fonts`), `crcbl-frame`, `crcbl-font` and
  the backend per platform (Vulkan on Linux, D3D12 on Windows, Metal on macOS),
  each pinned exactly, and `crcbl-image` with M6. The GPU path, as crcbl
  reported it (not read yet at 0.4.0 or later; read before M1 relies on it):
  `crcbl_app::window::open_window(shell, elapsed, desc)`, `wait_for_configure`
  for the extent, then
  `crcbl_app::present::Presenter::open(&shell, window, extent, &PresenterDesc)`
  (with `adapter_pin: crcbl_app::adapter::pin().as_deref()` to honour
  `CRCBL_ADAPTER`); each frame `acquire`, record, `submit_and_present`, plus
  `resize`, `drain` and `destroy`. The UI pass is crcbl-frame's
  `UiRenderer::with_ring` at `presenter.frames_in_flight()`, and
  `crates/crcbl-app/examples/ui_window.rs` is the worked example. Two caveats
  from crcbl: the Presenter has run only offscreen and headless, so lyr's bar is
  its first real window and mismatches go back to crcbl; and `frames_in_flight`
  stays at its default until crcbl ties it to the UI pass's ring.

  API facts read at crcbl v0.3.0 that M1 – M7 build on (2026-10-18; a
  sub-agent's read, spot-checked):
  - The only wait is `Shell::wait_events(timeout: Option<Duration>)`, advisory
    and behind `ShellCaps::EVENT_WAIT`, then `pump`; lyr computes the time to
    its next tick (the clock's `Tick::until_next`). Other threads wake it
    through `Shell::waker()`.
  - A panel targets `PanelDesc::monitor: Option<MonitorId>`; lyr maps output
    names through `MonitorInfo::name`. Reservation is
    `Reserve::{None, OwnExtent, IgnoreOthers}`.
  - Popups are
    `WindowRole::Popup(PopupDesc { parent, anchor, placement, input })`;
    `reposition_popup` needs `ShellCaps::POPUP_REPOSITION`.
  - Focus is `ShellEvent::Focus { window, focused }`, with
    `ShellEvent::FocusRefused`.
  - Text input is `Shell::set_text_input(window, Option<TextPurpose>)` with
    `Normal`, `Password` and `Pin`, and `set_text_input_area`; `TextPump` now
    lives in `crcbl-app`.
  - Rich text is `Ui::paragraph` with `run` and `run_keyed`; a link's click is
    the field `Response.clicked`. Fonts come from `crcbl-font`
    (`SystemFonts::find`, `fallbacks`) through `Ui::register_system_font`, with
    `Ui::set_fallback_fonts` for emoji.
  - `crcbl-image` decodes PNG, JPEG, BMP, ICO, WebP, XPM and an SVG subset that
    refuses what it cannot draw (lyr shows a generic icon then), converts SNI
    pixmaps and notification `image-data`, resizes, and looks up theme icons
    (`IconThemes::lookup`).

- **crcbl: every lyr and pikr feature on every backend.** At the owner's request
  (2026-10-17), crcbl makes each S-row work on every backend or fail loudly. Its
  matrix is in [plan/03](plan/03-surfaces-and-input.md#backend-matrix). Since
  then, by crcbl's reports: in 0.4.0, `TEXT_IME` means a real input method
  (`TEXT_COMPOSE` is the old commit-only meaning), `PANEL_EXCLUSIVE_KEYBOARD`
  and `CURSOR_SHAPE` caps, cursor-shape-v1 on Wayland, the fall-through to D3D12
  for a transparent Win32 window, and Vulkan refusing a composite alpha the
  surface lacks; on main after 0.5.0 (`f35f4228`), X11 cursor shapes through
  libxcb-cursor. Still open: AT-SPI (through `accesskit_unix`, which pulls in
  zbus; crcbl is confirming that with the owner) and AppKit accessibility;
  AppKit's input-method switch, secure input and pre-edit; X11 panels, popups
  and transparency; tests for the untested cells, including a Hyprland job, a
  FreeBSD VM job and S21 (fractional scale on a layer panel, which lyr's bar
  is); no flash on map on AppKit; cold-start timing (S13).
- **crcbl: gaps that bear on lyr's milestones.** S9: no pre-edit during a
  compose sequence, no surrounding text, and losing the keyboard mid-sequence is
  untested (crcbl judges the Wi-Fi password field needs none of it). S17: a
  panel's focus loss is tested on sway only, and a click on the empty desktop
  does not count as one (M5 measures Hyprland). S3: the Wayland
  `reposition_popup` path needs xdg_wm_base 3 and has not run anywhere.
  `ui.scrollbar` is vertical only.
- **crcbl: Wayland present path.** It can block on an output that is off
  (hypridle DPMS), by crcbl's reading of its own source on 2026-10-19, not
  reproduced: its Wayland backend requests no `wl_surface.frame`, `Pacing::Auto`
  is FIFO, and crcbl-vk acquires with an infinite timeout. crcbl decided on
  `Shell::request_frame(window)` and `ShellEvent::FrameReady { window }`, with
  `ShellCaps::FRAME_CALLBACK` set where the compositor really paces (Wayland,
  and AppKit if it has a real signal); elsewhere `FrameReady` comes at the next
  pump, so lyr has one code path. A window destroyed with a frame pending gets
  no `FrameReady`. It is not in 0.6.0; crcbl reports it on its main (2026-10-20)
  for the next release, with crcbl-app's own loop waiting for `FrameReady` as a
  follow-up. lyr's loop then becomes: on a change mark the bar dirty; with no
  frame pending, present and request one; on `FrameReady`, present again if
  still dirty
  ([plan/04](plan/04-rendering-and-theme.md#repaint-on-change-only)). crcbl
  declined a finite acquire timeout, which would turn the hang into a busy
  retry. crcbl reported on 2026-10-20 that the fix is built, going to its main
  for the next release: crcbl-app's `Presenter` gains
  `request_frame(&mut shell)`, `observe(&ShellEvent)` (which opens the gate on
  the window's `FrameReady`, or on its `WindowDestroyed` so it cannot stick) and
  `frame_due()`. While a requested frame is unanswered, `acquire` returns
  `Ok(None)` before touching the device, so the blocking acquire is never
  reached; a caller that never requests a frame behaves as before. lyr's part on
  that release, through `lyr_render::surface::Surface`: hand every pumped event
  to each surface's `observe`; after `acquire` hands over a frame and before
  `submit_and_present`, call `request_frame`; when `acquire` returns `None`,
  keep the window dirty and skip it, and let the loop sleep in `wait_events`
  while no surface's `frame_due()` is true. crcbl has not yet seen it on a
  powered-off output (an end-to-end test in its backlog).
- **crcbl: changing a panel in place.** crcbl-shell 0.5.0 sets a panel's size
  and margins only when it is created, so lyr's toast stack, whose height
  changes with every toast, recreates its panel. Asked crcbl on 2026-10-19;
  crcbl built `Shell::set_panel(window, size, &PanelDesc)`, on its main at
  `0c562bb5` (reported 2026-10-20), unreleased, shipping in the next release
  with frame pacing and `font::breaks`. It changes size, anchor, margins,
  reserve, layer and keyboard in place: layer-surface setters and a commit on
  Wayland (the new size arrives as the usual `Resized`), `SetWindowPos` with the
  appbar re-registered or removed on Win32, frame and level on AppKit. Namespace
  or monitor changes, or a non-panel window, are `InvalidDescriptor`; X11 and
  the web are `Unsupported`, as for panels. crcbl's CI ran it on sway, a real
  Win32 desktop (an appbar growing from 30 to 50 px and giving the edge back)
  and macOS. Once lyr is on that release: the toast stack resizes its panel
  instead of recreating it, so do the menus (`MenuPanel::show` recreates on a
  size change, `Place` then a set margin), and the bar changes height on reload
  without recreating.
- **pikr moves to crcbl** (the owner's decision, 2026-10-15), dropping floem,
  the floem and winit forks, iced and the SCTK fallback. pikr needs S1, S6 and
  S8 – S27. pikr's session tracks the move in pikr's own backlog and
  `gui-backend-research.md`.
- **wayr's README still names pikr as a consumer.** That is now wrong, and no
  wayr session exists to fix it. It is a one-line change in the wayr repo, for
  the owner.
- **dotfiles: switch from waybar, swaync and the `krypt menu` Wi-Fi, Bluetooth
  and audio commands to lyr** at M8
  ([plan/05](plan/05-layout-and-config.md#migration-from-waybar)). What happens
  to `wiremix`, `bluetui` and the `krypt menu` names is a question above.

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
  v0.56.2's formats: the event parser, the `Tracker` that folds events into the
  shared `lyr_wm::State`, the JSON seed, and `Hyprland` (socket I/O, `from_env`,
  `subscribe`, `focus`). Its tests run against a fake Hyprland on Unix sockets,
  not a real one. Not verified on a running Hyprland: the Lua focus request (the
  framing was read from hyprctl's source). The dialect comes from Hyprland
  itself (`j/status`'s `configProvider`, as ironbar reads it), tested against
  the fake on Linux under WSL. A named workspace is `name:NAME` in Lua too,
  quoted as a Lua long string (`request::lua_long_string`), after Hyprland's
  `hlFocus` (main `12f2425`); not run against a real Hyprland. Click is
  `focus_request` and scroll is `State::step`, both pure and tested. The bar
  shows the workspaces (`apps/lyr/src/sources.rs` runs the backend and hands its
  `State` to the main loop), and a click on a button or, with
  `workspaces.scroll`, a wheel step sends the switch through the same task
  (`apps/lyr/src/input.rs`, `lyr_core::pointer`). Not run against a real
  Hyprland. `lyr --stats` logs the idle counts (`apps/lyr/src/stats.rs`); on
  Windows an idle bar showed 3 wake-ups, 2 events and 2 presents over 86 s (the
  start and one per clock minute). The idle test itself, on Hyprland with every
  source running, has not been run. `lyr_wm::Compositor` picks it, or sway or
  niri, at start. Hyprland main's changes for the next release are in
  [plan/06](plan/06-workspaces.md#linux--hyprland).
- **Render, rest of M1.** `lyr-render` has the GPU-free half: the bar font found
  with crcbl-font and given the system's fallback faces (`fonts::load`), text
  measured and drawn as one glyph run per face (`text::Text`), sRGB colours in
  linear light (`colour::linear`), and lyr's draw list as crcbl-ui commands
  (`translate`), tested on Windows and Linux (WSL) and mutation-checked.
  `fonts::load` itself is untested: it depends on what is installed, including
  the family's bold and italic faces it now loads for markup (`text::Text` picks
  among them with crcbl-ui's `select_face`, which synthesises a face the family
  lacks; underlines sit where the font's `post` table says, transcribed from
  crcbl-ui's private `tree::emit::underline`). Presenting is `surface::Surface`
  (crcbl-app's `Presenter` per bar, D3D12 asked for first on Windows,
  crcbl-frame's `UiRenderer::with_ring`, as
  `crates/crcbl-app/examples/ui_window.rs` does), and `lyr` with no command now
  runs the main loop (`apps/lyr/src/bar.rs`): the config and font, a bar per
  output, a surface opened on each bar's first resize, and a redraw on the
  clock's tick, or on a source's update. It draws the clock, and on Linux the
  battery and workspaces; the other modules wait on their sources. **Run once,
  on Windows** (2026-10-19, with the owner's go-ahead): Windows 11 at 2560 ×
  1440, D3D12 on an AMD Radeon RX 7900 XTX as asked, the bar an appbar reserving
  50 px. It drew the transparent bar and the clock box with its border, radius
  and text, as a screenshot showed, and the work area came back when the process
  was killed. Hack Nerd Font Propo is not installed on that machine, so it drew
  in the monospace fallback, and lyr said so. Not run on Hyprland, which is M1's
  real check. `lyr quit` stops it. It repaints only when something it shows
  changed (plan 04), but not yet paced by Wayland's frame callback, which plan
  04 requires for an output that is off (hypridle's DPMS); see "crcbl: Wayland
  present path". It logs through `tracing` to `~/.local/state/lyr/logs/lyr.log`
  and to stderr (`apps/lyr/src/log.rs`, `RUST_LOG` to filter), capped at
  `log::MAX_BYTES` by moving it to `lyr.log.1` (tested on Windows and Linux, not
  watched rotating in a running bar). Colours go through crcbl-ui's
  `style::from_srgb8`, and a module's opacity is a crcbl-ui opacity group
  (`DrawList::push_opacity`), faded as one layer as CSS `opacity` is (crcbl
  0.6.0); neither has been seen on screen since the change. crcbl's CI had not
  verified the opacity layer on Metal when it reported it.
- **Shell, rest of M1.** `lyr-shell` has the bars (`bars::Bars`): one
  transparent layer panel per output (or per named output), stretched along the
  configured edge and reserving its height, created and destroyed as monitors
  come and go and as the config changes, tested against crcbl-shell's
  `HeadlessShell` and mutation-checked. The main loop draws into them through
  crcbl-app (`lyr_render::surface::Surface`), follows resizes and scale changes,
  and hands pointer events to the hit tests (`apps/lyr/src/bar.rs`). Not done: a
  run on a real compositor (the owner's Hyprland is the M1 check; this machine
  is Windows, where the same panels are Win32 appbars, and has run there), and a
  scale change seen on a real output.
- **Core, rest of M1.** `lyr-core` has the zone layout (`layout::layout`, tested
  and mutation-checked). Icon ramps are `icon::ramp`, transcribing waybar
  0.15.0's `ALabel::getIcon` and tested against its integer steps. Hit testing
  is `hit::Regions` (topmost region wins; boxes are half-open, so a shared edge
  belongs to one box). The battery, power-profile, network and bluetooth views
  (`battery`, `power_profile`, `network`, `bluetooth`) and the volume and mic
  views (`audio::volume`, `audio::mic`) follow waybar 0.15.0's `battery.cpp`,
  `power_profiles_daemon.cpp`, `network.cpp`, `bluetooth.cpp` and
  `pulseaudio.cpp`, with their config tables; `power_profile::cycle` is the
  click. The defaults are guarded by a test that fails on any raw private-use
  glyph. The bar's draw list is `draw::build`: lyr's own `DrawList` (boxes,
  text, and a group per module faded as one, like CSS `opacity`) and the input
  regions, sized by the CSS box model `style.css` is written for (`min-width`
  bounds the content; padding and the 2 px border sit outside it). That GTK3
  sizes waybar's boxes exactly so, and centres a label in a box held open by
  `min-width`, is from the CSS model and GTK's label default, not checked
  against a running waybar. The click and scroll actions are resolved by
  `pointer::action` (see "Pointer input"). The clock's tooltip names the zone on
  Linux from `TZ` (which the clock follows), else timedate1's `Timezone`, else
  the `/etc/localtime` link (`apps/lyr/src/modules.rs`); Windows and macOS show
  the offset alone. timedate1 is followed on `PropertiesChanged`
  (`lyr_sources::clock::linux::TimeDate`, tested against a fake timedated and
  mutation-checked, not run against a real one), so a zone change redraws the
  bar at once. The clock's text takes the new zone because chrono 0.4.45's
  `Local` re-reads `/etc/localtime` when its mtime changes, checking at most
  once a second (`src/offset/local/unix.rs`); a change within a second of the
  last draw shows on the next tick instead. Text is cut at its rect's edges
  (`lyr_render::translate` clips each text item), so a label wider than its box
  no longer runs over its neighbours. Not built: images (tray, M8). A format
  cannot drop the literal text around an empty placeholder; the one case that
  came up, a battery with no power reading, now shows 0 W as waybar 0.15.0 does
  (`total_power / 1e6`), and UPower always reports a rate. Measuring text (the
  widths `layout` takes) needs crcbl-ui's font atlas, so it is done by the
  caller in `lyr-render`.
- **Notification daemon, M6.** Built: the store (`lyr_core::notifications`): ids
  from 1 that skip 0 and held ids, `replaces_id` kept as the id and replacing in
  place (even for an id lyr does not hold, as the spec says), swaync's timeouts
  by urgency unless the app set its own, the do-not-disturb rule (critical still
  shows; nothing while the panel is open), and the history's order (transient
  left out, grouped by `desktop-entry` or app, critical groups first, then
  newest first), tested and mutation-checked. The D-Bus server too
  (`lyr_sources::notifications::linux::Daemon`): `Notify` with its hints,
  `CloseNotification` with its signal and the spec's empty error,
  `GetServerInformation` (spec 1.3), and each capability once it works
  (`notifications::linux::CAPABILITIES`: `actions`, `body` and `persistence` so
  far; not markup, hyperlinks or icons), tested against an application on a
  peer-to-peer connection. Claiming the name is `notifications::linux::serve`:
  `DO_NOT_QUEUE`, so a name another daemon holds comes back as `Exists` and lyr
  takes its object down and names the owner and its pid, or `REPLACE_EXISTING`
  when told to replace; it allows being replaced in turn. Not run against a real
  bus (the tests use peer-to-peer connections, which have no names).
  `Served::close` and `Served::invoke` are lyr's side, with their signals (an
  invoked action closes a notification that is not `resident`), tested and
  mutation-checked. The toast stack is `lyr_core::toast::stack::Stack` (newest
  on top, timeouts paused under the pointer) and a toast's draw list
  `lyr_core::toast` (swaync's look with the `toast_*` theme keys, its body
  wrapped at spaces) (crcbl-ui 0.6.0 has UAX #14 line breaking, but only inside
  `TextLayout`, which lays out one font, and its break finder `font::breaks` is
  crate-private; lyr's bodies mix bold, italic and plain spans, so lyr asked
  crcbl on 2026-10-20 to export the break opportunities; crcbl made
  `crcbl_ui::font::breaks` public on its main for the next release
  (`paragraphs(text)` and `soft_breaks(paragraph) -> Vec<usize>`, UAX #14
  opportunities at cluster starts), and lyr keeps breaking at spaces until that
  release is published), both tested and mutation-checked. The bar serves it on
  Linux (`apps/lyr/src/sources.rs`) and leaves notifications to another daemon
  that has the name, logging which, unless `notifications.replace = true` (read
  at start; a reload does not hand the name over). Toasts are on screen
  (`apps/lyr/src/toasts.rs`): one Overlay panel at the top right of the output
  the user is on, 16 px from the edges and clear of the bar, recreated whenever
  the stack's size changes (crcbl 0.5.0 cannot resize a panel); a click invokes
  the `default` action when there is one and takes the toast down, a middle
  click dismisses the notification. Tested on crcbl's headless shell and
  mutation-checked; **never seen on screen**: Windows has no daemon and WSL has
  no compositor, so the owner's Hyprland is the first real run. The bell is
  drawn from the store while lyr is the daemon. A toast shows when it arrived as
  swaync words it ("Now", "2 mins ago", from `get_relative_time` at v0.12.6),
  redrawn as the words change; a summary long enough to reach the time is cut
  short of it (`toast::TIME_GAP`), where swaync ellipsizes. The toast under the
  pointer shows swaync's close button (`toast::close_button`: red, 20 px, 6 px
  in from the top right, from swaync 0.12.6's `NotificationCloseButton` and the
  dotfiles' style; the `toast_close` theme keys), which dismisses; tested on the
  headless shell and mutation-checked. The notification centre's entries show it
  too, in the same place (`centre::Shown::hovered`): swaync's
  `.control-center .close-button` margins (10 px top, 6 px right) would put it
  partly outside the entry, since lyr's entries do not keep the 18 px
  notification margin they count from, so lyr keeps the toast's placement there.
  A body's markup is drawn (`lyr_core::markup`: the spec's `<b>`, `<i>`, `<u>`
  and `<a href>`, XML's entities, `<img>` left out, anything else shown as text;
  `toast::wrap` wraps the styled spans), and `body-markup` is advertised. Not
  built: a toast's images (the panel's Clear All, switch and media players work;
  `lyr dnd` and `lyr clear` work; on Windows they answer that lyr is not the
  daemon, run there). Each action but `default` gets a button under the body
  (swaync's style, at the bar's text size until the owner rules on swaync's
  `0.2rem`), in toasts and the panel's history; a click invokes it. A transient
  notification, kept in no history, closes as expired when its toast times out.
- **Sources, M4.** Built: the battery source. UPower's display device is read
  over D-Bus (`lyr_sources::battery::linux::UPower`, zbus 6) once and again on
  each `PropertiesChanged`, with no polling, and turned into the view's reading
  (`battery::upower::Device`). It is tested under WSL against a fake UPower on
  zbus's peer-to-peer connection, which needs no bus daemon, and
  mutation-checked; CI's Linux job runs the same test. The bar runs it, with the
  Hyprland workspaces, on a tokio runtime of its own thread
  (`apps/lyr/src/sources.rs`), each update handed to the main loop and the loop
  woken through crcbl's `ShellWaker`. Not run against a real UPower or Hyprland,
  which needs the owner's machine. The power-profile source is built the same
  way (`lyr_sources::power_profile::linux::PowerProfiles`): the daemon found
  under `org.freedesktop.UPower.PowerProfiles` or its older `net.hadess` name,
  followed on `PropertiesChanged`, switched by the `cycle_profile` built-ins,
  and tested against a fake daemon that has only the old name. The bluetooth
  source too (`lyr_sources::bluetooth::linux::BlueZ`): BlueZ's managed objects
  read once and again on each `PropertiesChanged` under `/org/bluez` or object
  added or removed, the rfkill block from the adapter's `PowerState`, and the
  device shown the last to connect, as waybar does
  (`lyr_sources::bluetooth::Tracker`); tested against a fake BlueZ. Not built:
  the Linux volume and mic sources (the notification daemon is its own entry;
  Windows' are built, under M9), which wait on the new-dependency decision
  above. The network source is built over NetworkManager
  (`lyr_sources::network::linux::NetworkManager`): the device of
  `PrimaryConnection` (the default route, as waybar picks it), else of
  `ActivatingConnection`, its IPv4 address, prefix and gateway, and on Wi-Fi the
  access point's SSID and strength, with the radio blocked when
  `WirelessEnabled` or `WirelessHardwareEnabled` is false; the state is waybar's
  `getNetworkState` (`lyr_core::network::Reading::state`), without its carrier
  test, since NetworkManager's chosen device has one. It re-reads on
  `PropertiesChanged` from the objects the reading came from only, so a scan's
  access-point updates cost nothing. Tested against a fake NetworkManager on a
  peer-to-peer connection and mutation-checked; the names are from
  NetworkManager's introspection XML on main (2026-10-20), not checked against
  an installed one. Not built: the nl80211 fallback without NetworkManager.
- **macOS sources, M10.** Built: the battery (`lyr_sources::battery::macos`):
  IOKit's power sources (`IOPSCopyPowerSourcesInfo`), the first internal
  battery's description turned into the reading by `battery::iops` (tested and
  mutation-checked on every platform), re-read on each
  `IOPSNotificationCreateRunLoopSource` notification on a run-loop thread of its
  own. `objc2-io-kit`, `objc2-core-foundation`, `objc2-core-audio`,
  `objc2-foundation`, `objc2`, `block2`, `objc2-system-configuration` and
  `objc2-core-wlan` were taken under plan 02's `objc2` row, read as covering the
  `objc2` framework crates; say so if that row did not mean them. Checked by
  `cargo clippy` for `aarch64-apple-darwin` from WSL. On CI's macOS runner,
  which has no battery, its test reads the power sources and starts and stops
  the watch; no Mac with a battery has run it. Volume and mic are built too
  (`lyr_sources::audio::macos`): CoreAudio's default output and input, their
  volume (the main element's, else the mean of channels 1 and 2, set on each)
  and mute, and the transport type's port (`audio::coreaudio`, tested and
  mutation-checked everywhere); property listeners on the system object and on
  the default devices, moved when a default changes, run on CoreAudio's own
  notification thread, and their callback is leaked once per watch so a call in
  flight after a listener goes stays valid. Cross-checked with clippy for
  `aarch64-apple-darwin`; CI's macOS runner, whatever audio devices it has, runs
  its read and the listeners' start, follow and stop; no Mac has played through
  it. The power menu's actions are built too (the Power menu entry below), and
  the power profile (`lyr_sources::power_profile::macos`): Low Power Mode from
  `NSProcessInfo`, as power-saver or balanced (`power_profile::lowpower`, tested
  everywhere), re-read on `NSProcessInfoPowerStateDidChangeNotification` through
  `NSNotificationCenter` with no queue, so the block runs on the posting thread;
  CI's Mac reads it and starts and stops the observer, but nothing has turned
  the mode on and off under it. A click on it logs that `cycle_profile` is not
  implemented, where plan 07 asks for a tooltip saying the mode is read-only.
  The network is built too (`lyr_sources::network::macos`): the primary
  interface, router and first address from the dynamic store's global and
  interface IPv4 entities (keys made with the framework's own key functions),
  Wi-Fi's name, signal (waybar's dBm scale, `network::scdynamic::signal`) and
  power from CoreWLAN, re-read on the store's notifications for the global IPv4
  entity and every interface's IPv4, link and AirPort entities, on the shared
  run loop thread (`macos_runloop`). `lyr_core::network::Link::wireless` keeps a
  Wi-Fi link whose name macOS withholds (no Location permission) showing as
  Wi-Fi, and the radio being off shows as disabled when there is no link. The
  signal is read only on those changes, since no notification follows it; plan
  07 has waybar re-reading it on an interval. `scdynamic` is tested and
  mutation-checked everywhere; CI's Mac reads the store and starts and stops the
  watch, and no Mac on Wi-Fi has run it. Not built: Bluetooth on macOS, the
  audio menu and the other menus.
- **Windows sources, M9.** Built: the battery (`lyr_sources::battery::windows`):
  `GetSystemPowerStatus` read at start and again whenever
  `PowerSettingRegisterNotification` reports a change of power source or
  remaining charge (`GUID_ACDC_POWER_SOURCE`,
  `GUID_BATTERY_PERCENTAGE_REMAINING`) to a callback, so nothing polls and no
  window is needed; the status's mapping (`battery::win32`) follows Microsoft's
  `SYSTEM_POWER_STATUS` reference, with no draw (shown as 0 W, as waybar shows a
  missing one) and no time to full. The mapping is tested on every platform and
  mutation-checked; the registration is tested on this machine, which has no
  battery, so a real battery's changes have not been seen. Run in the bar on
  Windows (2026-10-20): it started and quit cleanly with the module hidden. The
  network too (`lyr_sources::network::windows`): the interface
  `GetBestInterface` gives for 0.0.0.0 (the default route, which Windows weighs
  with its metrics), its first IPv4 address, prefix and gateway from
  `GetAdaptersAddresses`, its `FriendlyName` as `ifname`, and on Wi-Fi the SSID
  and signal quality from `WlanQueryInterface`; changes come from
  `NotifyIpInterfaceChange`, `NotifyUnicastIpAddressChange`,
  `NotifyRouteChange2` and the WLAN service's notifications, read on a thread of
  lyr's own. The mapping (`network::win32`) is tested everywhere and
  mutation-checked, the calls on this machine; run in the bar on Windows
  (2026-10-20), its tooltip read `Ethernet 192.168.68.54/22`. Wi-Fi has not been
  seen, this machine being wired, and the radio's on-off state is not read
  (`radio_blocked` is always false). The power profile too
  (`lyr_sources::power_profile::windows`): the effective power mode, reported by
  `PowerRegisterForEffectivePowerModeNotifications` (documented, Windows 10 1809
  and later) when registered and at each change, mapped onto
  power-profiles-daemon's three names with Windows' own mode name as the
  tooltip's driver (`power_profile::win32`); run on Windows (2026-10-20), its
  tooltip read "Profile: balanced / Driver: Balanced". Switching it, which the
  clicks do on Linux, is undocumented on Windows (plan 16's question 13) and not
  built: a click there logs that `cycle_profile` is not implemented. Volume and
  mic too (`lyr_sources::audio::windows`, through `windows`, which plan 02 lists
  and which was already in the build through `crcbl-dx12`, with `windows-core`
  for its `implement` macro): the default output and input (`eConsole`), each
  one's scalar volume and mute from `IAudioEndpointVolume`, friendly name as the
  tooltip's description, and the output's form factor as waybar's port kind for
  the icon (`audio::win32`); changes from `IMMNotificationClient` and
  `IAudioEndpointVolumeCallback` wake a COM thread of lyr's own, which also
  carries out `toggle_mute`, `volume_up` and `volume_down` (by `scroll_step`,
  held to 100 %, since Windows' endpoint volume has no boost). Run on Windows
  (2026-10-20): the bar showed the output at 40 % and the input at 100 %. A
  wheel step over the volume module was run (2026-10-20): it took the output
  from 40 % to 35 %, and was set back through the audio menu. The step went down
  for an injected forward (+120) wheel event because this machine has Windows'
  reversed mouse scrolling on (`ReverseMouseWheelDirection` = 1 under
  `HKCU\Control Panel\Mouse`), which Windows applies before any program sees the
  event; lyr follows it as every other program does, so it is not a bug. The
  mute click has not been run, since it cuts what is heard, and a Bluetooth
  endpoint is not told apart (`Sink::bluetooth` is false). The notifications
  module is plan 09's Windows bell: always the `none` icon
  (`lyr_core::bell::stateless`), since Windows offers no API for its
  notifications or do-not-disturb, with both clicks bound in
  `[platform.windows.notifications.on]` to `start "" ms-actioncenter:`; run on
  Windows (2026-10-20), a click on it opened the notification centre. The Wi-Fi
  and Bluetooth menus open Windows' Settings pages in their place
  (`apps/lyr/src/settings.rs`, `ms-settings:network` and `:bluetooth` through
  `cmd /C start`), from their bindings and from `lyr panel`; run on Windows
  (2026-10-20), `lyr panel bluetooth` brought up a Settings window, though which
  page it showed was not seen (it opened behind the terminal). The audio menu is
  native on Windows (`lyr_core::audio_menu`, `apps/lyr/src/audio_menu.rs`,
  `lyr_sources::audio::windows::menu`): tabs for playback and recording streams
  (WASAPI's audio sessions on every active endpoint, named by the session, else
  its program's file name, `audio::win32::session_name`; every endpoint has a
  system-sounds session, so those off the default endpoint are left out, and
  another stream there names its endpoint, `stream_row`) and output and input
  devices, each row with mute, a step down and up by the module's `scroll_step`,
  and on a device that is not the default a Default button, which opens
  `ms-settings:sound` since Windows has no public API for setting it. The source
  re-reads the rows on each endpoint change and once a second while the menu is
  open, since session changes have no callbacks wired. Run on Windows
  (2026-10-20): `lyr panel audio` opened it listing this machine's four output
  devices, the default in the active colour; a menu panel now stays inside its
  output (`menu_panel::Place`, from the clicked bar's width; `lyr panel` with no
  bar showing the module opens at the left edge), and the audio menu opened
  ending 16 px from the screen's right edge, its Playback tab clicked for real
  redrawing it there. Each control's lookup was tested by a 0 % step on every
  device and stream; on the real machine (2026-10-20) a click on the steam
  stream's step down showed 95 % and its step up 100 % again, so a click reaches
  the source and the change comes back to the menu; Mute and a device's steps
  have not been clicked, since they change what is heard. Each row also has a
  slider (`audio_menu::Target::Level`): a click sets the volume where it lands
  (`level_at`, from `MenuPanel::picked_across`), up to `MAX_PERCENT`, 100 % for
  Windows; Enter on it does nothing, having no place. Clicked on the real
  machine (2026-10-20): a quarter of the way along steam's slider set 25 %, and
  its right end then a step up set 100 % again. A drag that starts on a slider
  sets it at every move until the button is let go (`MenuPanel::with_drags`),
  dragged on the real machine from a quarter to three quarters of steam's slider
  for 75 %. A right click on the mic module opened it on Input devices, listing
  this machine's two microphones with the default in the active colour. Not
  built: plan 10's 150 % (with a Linux source), wiremix's card configuration
  tab, and routing a stream to another device (no public API on Windows). Not
  built: the other Windows sources (Bluetooth, workspaces, both waiting on the
  owner's questions above) and the native menus plan 10 names; macOS's
  `x-apple.systempreferences:` fallbacks wait on M10.
- **Pointer input.** Built: a click or a wheel step on a bar is hit-tested
  against its last frame's regions (`apps/lyr/src/input.rs`) and resolved to an
  action by the module's bindings (`lyr_core::pointer::action`): a workspace
  button focuses its workspace, `{ exec = … }` runs through `sh -c` or `cmd /C`
  (`apps/lyr/src/exec.rs`), `cycle_profile` and `cycle_profile_back` switch the
  power profile, `open_power_menu` opens the power menu, `toggle_dnd` flips
  do-not-disturb as `lyr dnd` does (the bell's right click; not run end to end,
  since Windows draws no bell and WSL has no compositor), and the other
  built-ins say they are not implemented yet. Touchpad scrolling makes one step
  per 50 device pixels (`PIXELS_PER_STEP`, lyr's own choice; crcbl leaves it to
  the application). Scrolling away from the user steps to the previous
  workspace. Not tried with a real click: the tests cover the resolution and the
  step arithmetic, and the Windows run had no pointer input. Hover is built: the
  workspace button under the pointer takes the active colour, as waybar's style
  colours both (`apps/lyr/src/modules.rs`), and moving onto another button
  redraws the bar. Tooltips are built (`apps/lyr/src/tooltips.rs`): timed as GTK
  3 does (`lyr_core::tooltip`), a passthrough popup below the module (above it
  on a bottom bar) drawn by `lyr_core::draw::tooltip`, for the power button,
  clock, battery, power profile and Bluetooth; tested on crcbl's headless shell
  and mutation-checked. Seen on Windows: the clock's `UTC+08:00` and the power
  button's "Power Button" tooltips opened below them. **Known issue on
  Windows:** once the popup is open, the bar gets no pointer-leave from crcbl
  0.5.0's Win32 backend, so the tooltip stays up after the pointer leaves the
  bar (with no popup, the leave arrives). Seen with a scripted `SetCursorPos`,
  not a physical mouse; reported to crcbl on 2026-10-19, which found the cause
  (`TME_LEAVE` armed only on the first move after a leave, so nothing re-arms it
  once the popup cancels it). Its test with an ordinary parent window did not
  reproduce the bug; trying a panel parent, as lyr's bar is, is next in crcbl's
  backlog, so the fix has no release yet. A tooltip follows its module when the
  module moves (repositioned where the window system can, opened again
  elsewhere). A tooltip of several lines (a format with line breaks) is drawn a
  row per line.
- **Control socket, rest.** Built: the socket (`apps/lyr/src/control.rs`,
  `interprocess`), the reply line, single-instance detection and `lyr quit`,
  tested on Windows and Linux (WSL), and run end to end on Windows (a second
  `lyr` refused; `lyr quit` stopped the bar, exit 0, the work area restored).
  `lyr reload` reloads the config, keeping the running one when the new one is
  invalid or its font will not load, and recreates the bars when `[bar]`
  changed; run end to end on Windows (30 px to 40 px, then a bad `bar.height`
  refused with the bar still running). `lyr toggle` hides the bars by closing
  them, so their space is given back on every platform, and shows them by
  creating them again (run on Windows: the work area grew by 50 px and shrank
  back). `lyr dnd` and `lyr clear` act on the notification daemon on Linux and
  answer that lyr is not the daemon elsewhere. `lyr panel clock`,
  `lyr panel bluetooth` and `lyr panel wifi` open their panels;
  `lyr panel audio` opens the audio menu on Windows and answers that there is no
  audio source elsewhere (M7). Not built: `lyr event workspace` for AeroSpace
  (M10).
- **Config, rest of M3.** Built in `lyr-config`: the schema for `[bar]`,
  `[layout]`, `[workspaces]`, `[clock]`, `[theme]` and every module's table, the
  embedded defaults, `[platform.*]` layering on both the defaults and the user
  file, and validation that names the key (unknown keys and modules, modules the
  platform cannot run, colours, ranges, format placeholders, the clock's
  strftime format), with tests that were each broken and seen to fail. The
  `[<module>.on]` action bindings (`actions.rs`) are checked against the module
  they are bound in, and the main loop runs them (see "Pointer input"). Not
  built: the tray's table (M8), the file watcher that reloads on save (it needs
  a crate such as `notify`, new to lyr, so the owner's call), a finer diff (a
  reload redraws everything, and recreates the bars only when `[bar]` changed),
  the control socket (its own entry above), and the defaults-parity test against
  `config.jsonc`. The dotfiles are at `~/Projects/mxaddict/dotfiles` on the
  owner's Windows machine, not next to lyr, and not on CI, so the test needs
  either a path from the environment or a copy of `config.jsonc` checked into
  lyr as a fixture; that is open. Format strings are one `Format` type
  (`lyr-config`'s `format.rs`), parsed when the config loads and rendered with
  `Format::render`, so validation and drawing will share one parser. Every error
  names its full key (`layout.right[0]: …`, `theme.border: …`, the latter
  through `serde_path_to_error`). `lyr-config` keeps its own copy of the table
  merge, because `hjkl_config`'s `deep_merge` and `locate` are private in
  0.42.2; if hjkl exports them, lyr drops the copy.
- **Waybar parity, still to check.** The plan was reviewed against the dotfiles'
  `config.jsonc` and `style.css` (2026-10-16), with waybar's own defaults read
  in its 0.15.0 source. The calendar is drawn in `theme.calendar_font` at
  Pango's `<small>` size (`lyr_render::text::SMALL`, 1/1.2). Open: that key
  defaults to the bar's proportional font, as plan 10 has it, where waybar's
  `<tt>` is monospace; whether the default should be `monospace` is for the
  owner to judge against waybar's look (M5). The logind calls behind
  `systemctl sleep` and the other power entries are in plan 10, read from
  systemd v262.1; the owner's installed systemd is not checked. The
  defaults-parity test (M3) should read the glyphs from `config.jsonc` itself,
  not from the plan: the plan once lost every BMP private-use glyph without
  anyone noticing, and now writes them as code points.
- **Nerd Font glyphs through crcbl-ui:** that Hack Nerd Font Propo's private-use
  glyphs resolve through crcbl-ui's cmap lookup is untested (M1's first test).
- **Native notification centre and menus, not yet researched**
  ([plan/09](plan/09-notifications.md), [plan/10](plan/10-popups-and-menus.md)).
  Written from memory and the dotfiles, with each item to read when its
  milestone starts: the Windows Wi-Fi and Bluetooth menu APIs (M9; the audio
  menu's WASAPI calls were read in `windows` 0.62.2 and are built) and macOS's
  (M10).
- **Power menu, M5.** Built: the Linux actions
  (`lyr_sources::power::linux::Logind`): Lock, Sleep, Hibernate, Reboot and
  Power off as systemd v262.1's `loginctl` and `systemctl` call logind, with the
  `WithFlags` fallback, the reboot flags (`power::reboot_flags`) and the
  soft-reboot retry, tested against fake new and old loginds and
  mutation-checked. Log out is `lyr_wm::Compositor::exit`: on Hyprland
  `hl.dsp.exit()` in Lua, as the dotfiles' `.menu-power` sends, or the `exit`
  dispatcher, tested against the fake Hyprland; on sway `exit`, run against CI's
  headless sway; on niri `Quit`, tested against a fake. logind's actions are not
  run against the real thing. The menu (`apps/lyr/src/power_menu.rs`, drawn by
  `lyr_core::power_menu`) opens from `open_power_menu` as a grab popup under the
  clicked module: arrow keys and the pointer move the selection, Enter or a
  click carries the entry out on the source runtime, and Escape, a click outside
  or a second click on the module closes it. Tested on crcbl's headless shell
  and mutation-checked; run on Windows with injected input (open, Down, Escape,
  reopen, a click on the bar outside it). Escape closes it through crcbl 0.6.0's
  grab, which queues `CloseRequested` after the key on every backend (lyr closed
  on the key itself under 0.5.0, whose Win32 grab did not); on Wayland a
  grabbing popup gets the keys even when the compositor keeps focus on the
  parent, and Win32 keys injected with scancode 0 get a key code from their
  virtual key. Run again on Windows under 0.6.0 (2026-10-20): a click opened it,
  two Down keys moved the selection from Lock to Hibernate, Escape closed it,
  and a click opened it again; no entry was chosen. An arrow key injected with
  `keybd_event` needs `KEYEVENTF_EXTENDEDKEY`, or its scancode reads as the
  numpad's. Keys injected with `SendKeys` arrive with no scancode, so crcbl
  names no key for them; `keybd_event` with scancodes works. `[power.menu]` sets
  an entry to a command (`{ exec = … }`), which runs instead of lyr's action.
  The Windows actions are built (`lyr_sources::power::windows`:
  `LockWorkStation`, `SetSuspendState`, and `ExitWindowsEx` after enabling
  `SeShutdownPrivilege` for a reboot or a power-off, run on a thread of their
  own), through `windows-sys`, which plan 02 lists as house precedent and which
  was already in lyr's build through `interprocess`; enabling the privilege is
  tested on this machine by reading the token back, but no entry has been run,
  since each locks, suspends, logs off or shuts the machine down. The macOS
  actions are built (`lyr_sources::power::macos`): each entry's command from
  `power::macos_command`, plan 10's table (`pmset displaysleepnow`,
  `pmset sleepnow`, and System Events' `log out`, `restart` and `shut down`
  through `osascript`), run on the power thread and its exit status checked; the
  table is tested and mutation-checked everywhere, and only Hibernate's refusal
  is run, on CI's Mac, since every other entry would lock, sleep or end the
  session. Not checked: that a Mac asks for its password after `displaysleepnow`
  (a setting of the Mac's), and what Hyprland does with an outside click (M5's
  open question for panels applies to this grab popup too).
- **Bluetooth menu, M7.** Built: its actions on Linux
  (`lyr_sources::bluetooth::linux::BlueZ`): the adapter's power, discoverable
  and pairable switches (`set`), the scan (`scan`), and connect, disconnect,
  pair, trust and untrust (`device`) and forget (`forget`), tested against a
  recording fake BlueZ and mutation-checked; the reading now carries what the
  menu shows: the adapter's path, discoverable, pairable and discovering, and
  each device's paired and trusted. The menu's drawing is built
  (`lyr_core::bluetooth_menu::draw`): the power, discoverable and pairable
  switches and the scan toggle, then the devices, paired first and by name, each
  with connect or disconnect, pair while unpaired, trust or untrust, and Forget,
  in the power menu's box; tested and mutation-checked. The menu is up
  (`apps/lyr/src/bluetooth_menu.rs`): a panel at the top left, under the
  Bluetooth module, opened by `open_bluetooth_menu` or `lyr panel bluetooth`,
  taking the keyboard (Escape and losing focus close it), each click sent to
  BlueZ, and drawn again as BlueZ changes (recreated when its size changes).
  Tested on crcbl's headless shell and mutation-checked; never run against a
  real BlueZ, and on Windows `lyr panel bluetooth` answers that there is no
  adapter. The arrow keys move an outline through its buttons and switches in
  the order drawn, and Enter acts on the outlined one. The pairing agent
  (`bluetooth::linux::agent`) is built and registered at start as
  `DisplayYesNo`; its question shows at the top of the menu, which opens if it
  is closed. A passkey confirmation or an authorization is asked with Yes and
  No, and a PIN (BlueZ asks for one of a device that pairs the legacy way,
  whatever the capability: `pin_code_request_callback` in its `src/adapter.c`,
  read on master) is typed into a field with OK and Cancel, through crcbl's text
  input as `TextPurpose::Pin`: at most `PIN_LENGTH` characters, Backspace, Enter
  sends it, and nothing typed sends nothing. The PIN is drawn as typed, not
  masked, as GNOME's and Windows' pairing dialogs show it. A No is `Rejected`,
  an unanswered question `Canceled`, and a typed passkey always `Rejected`,
  since lyr claims no keyboard. The agent is tested against BlueZ's side on a
  peer-to-peer connection, the menu on crcbl's headless shell, both
  mutation-checked. A question BlueZ gives up on (the agent's `Cancel`) leaves
  the menu. Each question's answer is kept by id (`agent::Questions`), tested
  with the agent on the peer-to-peer connection; the loop in
  `apps/lyr/src/sources.rs` that joins it to the bar is untested glue. Not done:
  the PIN field has never had a real input method behind it; favourites (below);
  and the Windows and macOS menus. Rename is built: a paired device's Rename
  button puts a field for its name at the top of the menu (text input `Normal`,
  the end of a long name in sight), OK or Enter sets `Device1.Alias` (empty
  gives back the device's own name), and Cancel or Escape drops it; the agent's
  question takes the top over from it. Tested on the headless shell and against
  a recording device on a peer-to-peer connection, mutation-checked; not run
  against a real BlueZ.
- **Wi-Fi menu, M7.** Built: the menu's model and drawing
  (`lyr_core::wifi_menu`): an access point's security from its `Flags`,
  `WpaFlags` and `RsnFlags` (`Security::of`; WEP and 802.1X refused with a
  message), one row per SSID with the strongest signal and the most demanding
  security, hidden networks left out, the password rule NetworkManager applies
  (`password_fits`: 8 – 63 bytes or 64 hex digits for PSK, anything but empty
  for SAE), the radio switch (no click while a hardware switch blocks it), the
  scan's state in place of the list, a masked password field, Forget's
  confirmation, an error until OK, and no clicks on the rows while joining.
  Tested and mutation-checked. NetworkManager's side is
  `lyr_sources::network::linux::wifi`: the first Wi-Fi device by `GetDevices`,
  its state (10 and 20 cannot scan), `LastScan`, `GetAllAccessPoints` and the
  access point in use, the saved Wi-Fi client profiles (`802-11-wireless` with
  no mode or `infrastructure`), the radio switch, `RequestScan`, joining with
  `ActivateConnection` or `AddAndActivateConnection` and waiting for the
  device's `StateChanged` to 100 or 120, `Delete`, and a new password through
  `Update`. The time since boot, which `LastScan` counts on, is read from
  `/proc/uptime` (the kernel fills it from the boot clock). Tested against a
  fake NetworkManager and mutation-checked; not run against a real one. The
  panel is up (`apps/lyr/src/wifi_menu.rs`, its glue in
  `apps/lyr/src/bar/wifi.rs`): opened by `open_wifi_menu` or `lyr panel wifi`
  under the network module, a rescan asked for on every open and "Scanning…"
  until a newer scan lands or `wifi_menu::SCAN_WAIT` (lyr's own 15 s) is up,
  unless the last scan is under 30 s old; a password typed with text input
  `Password`, spaces kept; `recovery` applied to a failure by the source task; a
  second click on the module within GTK's default double-click time leaves it
  open. Tested on crcbl's headless shell and mutation-checked. A failure while
  the menu is closed is a notice from lyr
  (`notifications::linux::notify_session`, to whichever daemon holds the name,
  lyr's own included), its text escaped for markup (`lyr_core::markup::escape`);
  the call is tested against lyr's daemon on a peer-to-peer connection, the glue
  that sends it is not. Two equally strong access points of one SSID: the first
  listed is joined, which either would do. The header's buttons take the arrow
  keys from the right (No before Yes), as in the Bluetooth menu. Both menus drop
  the keyboard's outline when a question appears (`MenuPanel::unfocus`), so
  Enter answers it.
- **Tray, M8.** Built: the rules (`lyr_core::tray`, after waybar's
  `src/modules/sni` at master `c15ec02`): an item's status (passive items
  hidden, as waybar hides them by default), a click (left activates, or opens
  the menu when `ItemIsMenu`; right opens the menu; middle secondary-activates),
  a wheel step (`Scroll(±1, "vertical")`), how a registration names an item (a
  path alone on the caller's connection, `name/path`, or a bus name with
  `/StatusNotifierItem`), and the watcher's registry. The D-Bus side
  (`lyr_sources::tray::linux`): lyr's `org.kde.StatusNotifierWatcher`, served
  when no other process has the name (`serve`; waybar replaces an existing one,
  lyr leaves it), with its properties and signals and a vanished name's
  registrations dropped (`Served::vanished`, `follow_names`); and the host,
  which registers `org.kde.StatusNotifierHost-<pid>`, reads every listed item
  with `GetAll` (one that does not answer is named in `Listing::unreadable`),
  re-reads on the watcher's or any item's signal, and sends `Activate`,
  `SecondaryActivate`, `ContextMenu` and `Scroll`. Tested against a fake item on
  a peer-to-peer connection and mutation-checked; `follow_names` and
  `Host::register` need a bus and are untested. DBusMenu is read too
  (`Host::menu`: `AboutToShow`, then the whole `GetLayout`, into
  `lyr_core::tray::menu::Entry` with the interface's defaults, access-key
  underscores taken out, separators and toggles; `Host::menu_clicked` sends
  `clicked`), tested against a fake menu and mutation-checked; an item that
  names a menu it does not serve is an error, on which the caller falls back to
  `ContextMenu`, as waybar does for Steam. The module is on the bar
  (`apps/lyr/src/modules.rs`, `apps/lyr/src/bar/tray.rs`, the source task in
  `apps/lyr/src/sources.rs`): a button per shown item, in the watcher's order,
  coloured as urgent while it needs attention; a press goes to the item (a
  click, a wheel step, or its menu), and a menu opens under the button as a list
  popup (`apps/lyr/src/tray_menu.rs` over `ListPopup`), a submenu opening in
  place with a Back row, a click sending `clicked`; with no menu served, the
  item's `ContextMenu`. The popup is tested on crcbl's headless shell and
  mutation-checked; the glue joining source and bar is untested, and none of it
  has met a real item. Wayland gives a window no place on the screen, so
  `Activate` and `ContextMenu` get the point under the button on the bar, not on
  the screen. Each item's button has its tooltip, its `ToolTip` title or else
  its `Title` (`Item::tooltip_text`, as waybar falls back), through the bar's
  tooltips, now keyed by the hovered target rather than the module. Icons wait
  on images (the decision above); until then an item is drawn as its title's
  first letter (`Item::letter`), where plan 08 asks for a generic icon.
- **Closing a panel on an outside click.** A panel opened from a key binding
  cannot be an `xdg_popup` with a grab, because there is no input serial. Losing
  keyboard focus (S17) closes it on a click into another window, but not on a
  click on the empty desktop. M5 records what Hyprland does and picks a fix.
- **Does the year calendar fit the 500 px panel?** Not on Windows with the
  monospace fallback font (Hack Nerd Font Propo is not installed there): run on
  2026-10-20, `lyr panel clock` drew the year grid and the panel widened to fit
  it, about 607 device pixels wide by a screenshot, as `clock_panel`'s
  `WIDTH.max(...)` does. Still to measure in Hack Nerd Font Propo on the owner's
  Hyprland ([plan/10](plan/10-popups-and-menus.md#clock-panel)).
- **Clock panel, M5.** Built: the calendar maths (`lyr_core::calendar`): a
  month's rows from a first weekday, waybar's week numbers (`%W` from Monday,
  `%U` otherwise, the first row numbered by the 1st), the header order and
  scrolling by a month or a year, tested against chrono's `%W` and `%U` for
  every day of 2020 – 2030 and mutation-checked. The week numbers follow waybar,
  not ISO, because the dotfiles do not set `iso8601`; ISO would be `%V`, a
  one-line change, if the owner prefers it. The calendar is drawn too
  (`lyr_core::calendar::draw`): waybar's month blocks on a grid of cells as wide
  as the widest day or weekday name, so a proportional font still lines up, year
  mode three months across, week numbers on the right, all in bold and today in
  its colour and underlined, as the dotfiles' calendar format marks them up,
  with the `calendar_*` theme keys; tested and mutation-checked. Until the
  locale decision above is made, weeks start on Monday and names are English
  (plan 16, 19). The panel is up too (`apps/lyr/src/clock_panel.rs`): opened by
  `open_clock_panel` (a click on the clock or the bell) or `lyr panel clock`, it
  sits at the top right of the output the user is on, 16 px from the edges and
  clear of the bar, runs the output's height, and is as wide as swaync's 500 px
  or the year grid, whichever is wider (the year is wider in the fallback font
  this machine has, so the grid does not fit 500 px there). It takes the
  keyboard; Escape, losing focus and the same click or binding again close it; a
  right click switches year and month mode and scrolling moves by one of either.
  Opening it takes the toasts down, and no toast shows while it is open. Run on
  Windows: year, month and Escape seen. While lyr is the notification daemon the
  notification centre is below the calendar (`lyr_core::centre`): the title with
  Clear All, the do-not-disturb switch and the history in swaync's order, each
  entry drawn as a toast; Clear All and the switch act as `lyr clear` and
  `lyr dnd`, an entry as a toast does, and the panel redraws on every change
  (lyr's own closures now report a change as an application's do). The title and
  switch label are at the bar's text size, not swaync's 2rem. The media players'
  source is built (`lyr_sources::mpris::linux::Players`): every
  `org.mpris.MediaPlayer2.*` name on the session bus, its title, artists and
  playback status, re-read on `NameOwnerChanged` or any player's
  `PropertiesChanged`, and Previous, PlayPause and Next; reading and the
  controls tested against a fake player and mutation-checked, finding players by
  name not (it needs a bus). A centre taller than the panel scrolls, three lines
  a wheel step, cut at the calendar by a clip (`draw::Item::Clip`). While open,
  the panel is drawn again on each clock tick, so the calendar's today and the
  history's times move on (no wake-up of its own). The panel shows them too,
  between the switch and the history (swaync's order): a card per player with
  its title, artist and previous, play/pause and next, the standard media
  symbols since Nerd Font codepoints were not checked; a click presses the
  control. Not built: a player's art (lyr draws no images), reading the first
  weekday and names from the locale, and checking what Hyprland does with a
  click on the bare desktop while it is open.
- **Undocumented Windows APIs:** virtual desktops
  (`IVirtualDesktopManagerInternal`) and setting the power mode
  (`PowerSetActiveOverlayScheme`) can break with any Windows build.
- **macOS cannot reserve screen space.** The bar overlaps windows unless the
  menu bar is set to auto-hide.
- **No tray and no notification state on Windows or macOS.** Neither platform
  offers an API ([plan/08](plan/08-tray.md),
  [plan/09](plan/09-notifications.md)).

## Ideas considered and declined

- **wiremix's peak meters in the audio menu:** they need audio capture on every
  node shown, which a bar menu has no use for (plan 10). Declined by lyr's
  session on 2026-10-18, not by the owner; reopen if the owner wants them.
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

- **Window title module: built, run on Windows only.** `window`
  (`lyr_core::window`, and `lyr_wm::State::active_window`) follows Hyprland's
  `activewindow>>CLASS,TITLE` event, seeded at start from `j/activeworkspace`'s
  `lastwindowtitle` (what waybar reads) and the client's class; `{title}` and
  `{class}`, `max_length` cut with an ellipsis, the tooltip the text unless
  `tooltip_format` is set. Not in the default layout. Tested and
  mutation-checked against recorded lines, not against a running Hyprland. On
  sway it is the focused window in `GET_TREE` (its app id, or its class under
  Xwayland, and its name), re-read on focus and title events; on niri the window
  niri reports focused, by app id and title. Both are tested against fake
  sockets only; CI's real sway opens no window. waybar's `rewrite` rules,
  `separate-outputs` and app icon are not carried. On Windows
  (`lyr_sources::window::windows`) it shows the foreground window: its title,
  and its program's file name as `{class}` (`lyr_sources::program`), re-read
  through `SetWinEventHook` on a thread of its own when the foreground changes
  or its title does, lyr's own windows skipped; run on Windows (2026-10-20) with
  `window` in the centre zone, it showed the terminal's title and then
  "Notification Center" when that took the foreground. The hook callback's
  filter is checked only by that run.
- **`lyr import-waybar`: the config half.** `lyr import-waybar FILE`
  (`lyr_config::waybar`) prints a waybar `config.jsonc` as a lyr config: the
  bar's settings, the layout and every module lyr has, keys renamed from kebab
  case, `on-*` as `{ exec = … }` bindings, glyphs written as escapes; each key
  is tried against lyr's own schema, and what it refuses, like waybar's own bar
  keys and modules lyr lacks, is listed at the top. Tested and mutation-checked
  on a cut-down config; run on the dotfiles' real `config.jsonc` (2026-10-20),
  whose every carried glyph, format and icon table matched lyr's
  `defaults.toml`, the differences being waybar's `on-click` commands, which
  lyr's built-ins replace. Not built: `style.css` to `[theme]`, which would mean
  mapping waybar's selectors onto lyr's theme keys.
- **komorebi or GlazeWM on Windows**: unbuilt, as the Windows workspaces
  question above says.

## Not yet verified

- Nothing has run on Linux under a real compositor: everything Wayland, D-Bus
  and Hyprland is tested against fakes and crcbl's headless shell, so the
  owner's Hyprland is the first real run (M1's check).
- The sway backend runs against a real sway only in CI's `sway e2e` job
  (headless, two outputs; first green 2026-10-20, run 39371299842): the first
  state with a workspace on each output, a switch to a name with a space and a
  `;`, a refused `$` name, and Log out. Its first runs found that sway quits
  without answering `exit` and sends events after its socket is gone, both
  handled since. Not exercised there: the focused window's title (a headless
  sway has no window open) and urgency. The niri backend is tested only against
  a fake socket written from niri v26.04's `niri-ipc`; its Quit handling is read
  from niri's source. Neither runs on a desktop yet.
- The GPU choice (the owner's rule: `CRCBL_ADAPTER`, else the adapter driving
  the bar's display when it is discrete, else low power) is crcbl-app's
  `adapter::choose` inside the `Presenter`, with `PowerPreference::LowPower`
  named in `lyr_render::surface::Surface::open`. Seen on this Windows machine
  only, where the display's discrete RX 7900 XTX was chosen (crcbl 0.6.0,
  2026-10-20); a hybrid laptop with its panel on the integrated GPU is untried.
- The release pipeline (phase 2 of [plan/14](plan/14-ci-and-release.md)) does
  not exist yet.
