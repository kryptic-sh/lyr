# Backlog

Work raised and not finished, decisions still open, and gaps in what has been
verified. Delete an entry when it ships; `git log` keeps the history.

## Decisions awaiting the owner

The provisional decisions in
[plan/16-open-questions.md](plan/16-open-questions.md) are the defaults the
implementation follows until they are answered. The ones that block a milestone:

- **New dependencies** `libpulse-binding` and an rtnetlink crate (`rtnetlink` or
  `neli`) block M4's audio and network sources.

## Cross-repo work

- **crcbl: what lyr uses, and at which release.** lyr pins `crcbl-shell` at
  `=0.5.0` (crcbl's `rust-version` is 1.98, and lyr's follows). crcbl 0.4.0
  (2026-10-18) brought `crcbl-app`, the window-plus-device crate lyr asked for
  so that it links no engine, audio (cpal, ALSA) or physics; 0.5.0's only
  breaking change is in crcbl-phys. M1 adds `crcbl-app`, `crcbl-hal`, `crcbl-ui`
  (with `system-fonts`), `crcbl-frame`, `crcbl-font` and the backend per
  platform (Vulkan on Linux, D3D12 on Windows, Metal on macOS), each pinned
  exactly, and `crcbl-image` with M6. The GPU path, as crcbl reported it (not
  read yet at 0.4.0 or later; read before M1 relies on it):
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
- **crcbl: Wayland present path.** Whether crcbl presents only after a frame
  callback, or can block in FIFO `vkQueuePresentKHR` on an output that is off
  (hypridle DPMS), is unverified
  ([plan/04](plan/04-rendering-and-theme.md#repaint-on-change-only)).
  crcbl-shell 0.5.0 has no frame-callback event for lyr to pace on. Asked crcbl
  on 2026-10-19 whether the path can block and, if it can, for a
  `FrameReady`-style event on every backend; waiting on the answer.
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
  source). The dialect comes from Hyprland itself (`j/status`'s
  `configProvider`, as ironbar reads it), tested against the fake on Linux under
  WSL. Named workspaces get no request in Lua until there is a quoting rule.
  Click is `focus_request` and scroll is `State::step`, both pure and tested.
  The bar shows the workspaces (`apps/lyr/src/sources.rs` runs the backend and
  hands its `State` to the main loop), and a click on a button or, with
  `workspaces.scroll`, a wheel step sends the switch through the same task
  (`apps/lyr/src/input.rs`, `lyr_core::pointer`). Not run against a real
  Hyprland. `lyr --stats` logs the idle counts (`apps/lyr/src/stats.rs`); on
  Windows an idle bar showed 3 wake-ups, 2 events and 2 presents over 86 s (the
  start and one per clock minute). The idle test itself, on Hyprland with every
  source running, has not been run. The `WorkspaceBackend` trait waits for the
  Windows backend (M9). Hyprland main's changes for the next release are in
  [plan/06](plan/06-workspaces.md#linux--hyprland).
- **Render, rest of M1.** `lyr-render` has the GPU-free half: the bar font found
  with crcbl-font and given the system's fallback faces (`fonts::load`), text
  measured and drawn as one glyph run per face (`text::Text`), sRGB colours in
  linear light (`colour::linear`), and lyr's draw list as crcbl-ui commands
  (`translate`), tested on Windows and Linux (WSL) and mutation-checked.
  `fonts::load` itself is untested: it depends on what is installed. Presenting
  is `surface::Surface` (crcbl-app's `Presenter` per bar, D3D12 asked for first
  on Windows, crcbl-frame's `UiRenderer::with_ring`, as
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
  watched rotating in a running bar). `colour.rs` copies crcbl-ui's private
  `srgb_to_linear`; crcbl's next release exports it as
  `crcbl_ui::style::from_srgb8` (and `crcbl_core::srgb`), and lyr drops the copy
  then. Group opacity is approximated until crcbl's opacity groups ship.
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
  against a running waybar. Not built: the modules' actions (click, scroll;
  M3/M4), the clock's source (the system zone's name, from `timedate1` on Linux;
  the view, `clock::view`, and its tick are built), images (tray, M8), and
  clipping text that overflows a box. A format cannot drop the literal text
  around an empty placeholder, so a battery with no power reading shows
  `80% W Discharging`. Measuring text (the widths `layout` takes) needs
  crcbl-ui's font atlas, so it is done by the caller in `lyr-render`.
- **Notification daemon, M6.** Built: the store (`lyr_core::notifications`): ids
  from 1 that skip 0 and held ids, `replaces_id` kept as the id and replacing in
  place (even for an id lyr does not hold, as the spec says), swaync's timeouts
  by urgency unless the app set its own, the do-not-disturb rule (critical still
  shows; nothing while the panel is open), and the history's order (transient
  left out, grouped by `desktop-entry` or app, critical groups first, then
  newest first), tested and mutation-checked. The D-Bus server too
  (`lyr_sources::notifications::linux::Daemon`): `Notify` with its hints,
  `CloseNotification` with its signal and the spec's empty error,
  `GetServerInformation` (spec 1.3), and no capabilities until each works,
  tested against an application on a peer-to-peer connection. The bar does not
  serve it yet: until it draws toasts, taking the name would swallow every
  notification on a desktop with no other daemon. Not built: claiming the name
  and the check for another daemon owning it, toasts (M5's panels), markup,
  images, MPRIS and the bell module.
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
  the network, volume, mic and notification sources. Network and audio wait on
  the new-dependency decision above.
- **Pointer input.** Built: a click or a wheel step on a bar is hit-tested
  against its last frame's regions (`apps/lyr/src/input.rs`) and resolved to an
  action by the module's bindings (`lyr_core::pointer::action`): a workspace
  button focuses its workspace, `{ exec = … }` runs through `sh -c` or `cmd /C`
  (`apps/lyr/src/exec.rs`), `cycle_profile` and `cycle_profile_back` switch the
  power profile, and the other built-ins say they are not implemented yet.
  Touchpad scrolling makes one step per 50 device pixels (`PIXELS_PER_STEP`,
  lyr's own choice; crcbl leaves it to the application). Scrolling away from the
  user steps to the previous workspace. Not tried with a real click: the tests
  cover the resolution and the step arithmetic, and the Windows run had no
  pointer input. Not built: hover (the active-or-hovered workspace colour) and
  tooltips (M5).
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
  back). The other requests answer "not implemented yet": `panel` (M5), `dnd`
  and `clear` (M6). Not built: `lyr event workspace` for AeroSpace (M10).
- **Config, rest of M3.** Built in `lyr-config`: the schema for `[bar]`,
  `[layout]`, `[workspaces]`, `[clock]` and `[theme]`, the embedded defaults,
  `[platform.*]` layering on both the defaults and the user file, and validation
  that names the key (unknown keys and modules, modules the platform cannot run,
  colours, ranges, workspace format placeholders), with tests that were each
  broken and seen to fail. The `[<module>.on]` action bindings (`actions.rs`)
  are built and checked against the module they are bound in. Not built: running
  them (the main loop), the tray's table (M8), checking the clock's strftime
  format, the file watcher that reloads on save (it needs a crate such as
  `notify`, new to lyr, so the owner's call), a finer diff (a reload redraws
  everything, and recreates the bars only when `[bar]` changed), the control
  socket (its own entry above), the README's schema docs, and the
  defaults-parity test against `config.jsonc`. The dotfiles are at
  `~/Projects/mxaddict/dotfiles` on the owner's Windows machine, not next to
  lyr, and not on CI, so the test needs either a path from the environment or a
  copy of `config.jsonc` checked into lyr as a fixture; that is open. Format
  strings are one `Format` type (`lyr-config`'s `format.rs`), parsed when the
  config loads and rendered with `Format::render`, so validation and drawing
  will share one parser. Every error names its full key (`layout.right[0]: …`,
  `theme.border: …`, the latter through `serde_path_to_error`). `lyr-config`
  keeps its own copy of the table merge, because `hjkl_config`'s `deep_merge`
  and `locate` are private in 0.42.2; if hjkl exports them, lyr drops the copy.
- **Waybar parity, still to check.** The plan was reviewed against the dotfiles'
  `config.jsonc` and `style.css` (2026-10-16), with waybar's own defaults read
  in its 0.15.0 source. Open: the calendar's `<tt>` monospace look with a
  proportional default font (`calendar_font`, M5). The logind calls behind
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
  milestone starts: NetworkManager's D-Bus names checked against the installed
  NetworkManager (plan 10 has them from the online API reference) (M7);
  bluetui's and wiremix's source, beyond the READMEs plan 10 now follows (M7);
  the Windows and macOS menu APIs (M9, M10).
- **Opening a notification's link** (M6). The spec asks for `<a href>` links and
  lyr advertises `body-hyperlinks`, but a link comes from any program on the
  session bus, so how a click opens it (the system's URL handler, which schemes,
  a confirmation) is a security choice to make in M6, not a default.
- **Two swaync style values that look like slips** (M6, the owner's call):
  `font-family: 'Hadk Nerd Font'` names no font, so swaync falls back today (lyr
  defaults to the bar's font), and action buttons are `0.2rem`, about 3 px
  ([plan/09](plan/09-notifications.md#what-swaync-does-today)).
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

- **Window title module** (`hyprland/window`; the draft plan listed it, but
  today's bar does not have it). Built on Hyprland's `activewindowv2` event.
- **`lyr import-waybar`**: convert `config.jsonc` + `style.css` to TOML.
- **sway and niri workspace backends**, and komorebi or GlazeWM on Windows.

## Not yet verified

- Nothing has run beyond the scaffold: the binary parses its command line and
  reports that no bar exists yet.
- The release pipeline (phase 2 of [plan/14](plan/14-ci-and-release.md)) does
  not exist yet.
