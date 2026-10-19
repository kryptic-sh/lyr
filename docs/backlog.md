# Backlog

Work raised and not finished, decisions still open, and gaps in what has been
verified. Delete an entry when it ships; `git log` keeps the history.

## Decisions awaiting the owner

The provisional decisions in
[plan/16-open-questions.md](plan/16-open-questions.md) are the defaults the
implementation follows until they are answered. The ones that block a milestone:

- **New dependencies** `libpulse-binding` and an rtnetlink crate (`rtnetlink` or
  `neli`) block M4's audio and network sources.
- **The calendar's first weekday and names from the system locale** (M5) need
  `libc` on Linux (`nl_langinfo` with `_NL_TIME_FIRST_WEEKDAY`, as waybar reads
  it) and `windows-sys` on Windows (`GetLocaleInfoEx` with
  `LOCALE_IFIRSTDAYOFWEEK`); lyr depends on neither directly. The alternative
  without them is a `clock.first_weekday` config key defaulting to Monday, with
  English names, which a Sunday-first locale would have to set by hand.
- **Bluetooth favourites need somewhere to live** (M7). bluetui (master
  `ff92b24`, `src/favorite.rs`, read 2026-10-19) keeps them as device addresses
  in `favorites.txt` under its data directory, sorts favourites first among
  paired devices, and marks them. lyr writes no state today. The options: a file
  lyr writes (`~/.local/share/lyr/`, bluetui's way; the menu toggles it), or a
  `bluetooth.favourites` list of addresses in `config.toml` (declarative, but
  lyr never writes the config, so the menu could not toggle it). Or drop
  favourites: BlueZ's `Trusted` already marks the devices that matter.

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
- **crcbl: Wayland present path.** It can block on an output that is off
  (hypridle DPMS), by crcbl's reading of its own source on 2026-10-19, not
  reproduced: its Wayland backend requests no `wl_surface.frame`, `Pacing::Auto`
  is FIFO, and crcbl-vk acquires with an infinite timeout. crcbl decided on
  `Shell::request_frame(window)` and `ShellEvent::FrameReady { window }`, with
  `ShellCaps::FRAME_CALLBACK` set where the compositor really paces (Wayland,
  and AppKit if it has a real signal); elsewhere `FrameReady` comes at the next
  pump, so lyr has one code path. A window destroyed with a frame pending gets
  no `FrameReady`. It is queued after opacity groups, the sRGB helper and AppKit
  IME, which make 0.6.0; it ships in 0.6.0 if it is done before that is cut,
  otherwise in 0.7.0, and crcbl will say which. lyr's loop then becomes: on a
  change mark the bar dirty; with no frame pending, present and request one; on
  `FrameReady`, present again if still dirty
  ([plan/04](plan/04-rendering-and-theme.md#repaint-on-change-only)). crcbl
  declined a finite acquire timeout, which would turn the hang into a busy
  retry.
- **crcbl: changing a panel in place.** crcbl-shell 0.5.0 sets a panel's size
  and margins only when it is created, so lyr's toast stack, whose height
  changes with every toast, recreates its panel. Asked crcbl on 2026-10-19;
  crcbl took `Shell::set_panel(window, size, &PanelDesc)` (all but namespace and
  monitor change in place, every backend), after pikr's work and frame pacing,
  likely in 0.7.0. lyr's bar would use it too, to change height on reload
  without recreating.
- **crcbl: render on the GPU driving the window's display** (the owner's rule,
  relayed by pikr on 2026-10-19, shared with pikr): the adapter that drives the
  output a bar is on, else the low-power one, `CRCBL_ADAPTER` still overriding.
  lyr takes crcbl's own adapter order today (`Surface::open`, with
  `crcbl_app::adapter::pin` for the override), so it follows the rule without
  code of its own: crcbl decided (2026-10-19, relayed by pikr) that a device
  request takes a power preference; an explicit pick or `CRCBL_ADAPTER` wins, a
  display driven by a discrete adapter uses it, and otherwise (a hybrid laptop's
  iGPU panel, or an unknown display) the preference decides. crcbl-app's
  `Presenter` defaults to low power, so lyr gets the rule by keeping that
  default. It ships with crcbl's D3D12 start-up work, release not yet named. On
  this Windows machine lyr drew on the discrete RX 7900 XTX. Check which adapter
  each bar gets once crcbl ships it.
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
  watched rotating in a running bar). `colour.rs` copies crcbl-ui's private
  `srgb_to_linear`. crcbl's main (41859726, 2026-10-19, unreleased, for 0.6.0)
  exports it as `crcbl_ui::style::from_srgb8` and has opacity groups
  (`DrawList::push_opacity`/`pop_opacity`); lyr drops the copy and stops
  approximating group opacity once 0.6.0 is published (lyr depends on released
  crcbl only). crcbl's CI had not verified the opacity layer on Metal then.
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
  cannot drop the literal text around an empty placeholder, so a battery with no
  power reading shows `80% W Discharging`. Measuring text (the widths `layout`
  takes) needs crcbl-ui's font atlas, so it is done by the caller in
  `lyr-render`.
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
  wrapped at spaces) (crcbl-ui is adding UAX #14 line breaking, approved
  2026-10-19; lyr's wrap can move to it once it is released), both tested and
  mutation-checked. The bar serves it on Linux (`apps/lyr/src/sources.rs`) and
  leaves notifications to another daemon that has the name, logging which,
  unless `notifications.replace = true` (read at start; a reload does not hand
  the name over). Toasts are on screen (`apps/lyr/src/toasts.rs`): one Overlay
  panel at the top right of the output the user is on, 16 px from the edges and
  clear of the bar, recreated whenever the stack's size changes (crcbl 0.5.0
  cannot resize a panel); a click invokes the `default` action when there is one
  and takes the toast down, a middle click dismisses the notification. Tested on
  crcbl's headless shell and mutation-checked; **never seen on screen**: Windows
  has no daemon and WSL has no compositor, so the owner's Hyprland is the first
  real run. The bell is drawn from the store while lyr is the daemon. A toast
  shows when it arrived as swaync words it ("Now", "2 mins ago", from
  `get_relative_time` at v0.12.6), redrawn as the words change; a summary long
  enough to reach the time is cut short of it (`toast::TIME_GAP`), where swaync
  ellipsizes. The toast under the pointer shows swaync's close button
  (`toast::close_button`: red, 20 px, 6 px in from the top right, from swaync
  0.12.6's `NotificationCloseButton` and the dotfiles' style; the `toast_close`
  theme keys), which dismisses; tested on the headless shell and
  mutation-checked. The notification centre's entries show it too, in the same
  place (`centre::Shown::hovered`): swaync's `.control-center .close-button`
  margins (10 px top, 6 px right) would put it partly outside the entry, since
  lyr's entries do not keep the 18 px notification margin they count from, so
  lyr keeps the toast's placement there. A body's markup is drawn
  (`lyr_core::markup`: the spec's `<b>`, `<i>`, `<u>` and `<a href>`, XML's
  entities, `<img>` left out, anything else shown as text; `toast::wrap` wraps
  the styled spans), and `body-markup` is advertised. Not built: a toast's
  images (the panel's Clear All, switch and media players work; `lyr dnd` and
  `lyr clear` work; on Windows they answer that lyr is not the daemon, run
  there). Each action but `default` gets a button under the body (swaync's
  style, at the bar's text size until the owner rules on swaync's `0.2rem`), in
  toasts and the panel's history; a click invokes it. A transient notification,
  kept in no history, closes as expired when its toast times out.
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
  the network, volume and mic sources (the notification daemon is its own
  entry). Network and audio wait on the new-dependency decision above.
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
  answer that lyr is not the daemon elsewhere. `lyr panel clock` opens the clock
  panel; the other panels answer "not implemented yet" (M7). Not built:
  `lyr event workspace` for AeroSpace (M10).
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
  milestone starts: NetworkManager's D-Bus names checked against the installed
  NetworkManager (plan 10 has them from the online API reference) (M7);
  bluetui's and wiremix's source, beyond the READMEs plan 10 now follows (M7);
  the Windows and macOS menu APIs (M9, M10).
- **Opening a notification's link** (M6). lyr draws a body's `<a href>` link (in
  `toast_link`, underlined) but does not open it, and does not advertise
  `body-hyperlinks`; a link comes from any program on the session bus, so how a
  click opens it (the system's URL handler, which schemes, a confirmation) is a
  security choice to make in M6, not a default.
- **Two swaync style values that look like slips** (M6, the owner's call):
  `font-family: 'Hadk Nerd Font'` names no font, so swaync falls back today (lyr
  defaults to the bar's font), and action buttons are `0.2rem`, about 3 px
  ([plan/09](plan/09-notifications.md#what-swaync-does-today)).
- **Power menu, M5.** Built: the Linux actions
  (`lyr_sources::power::linux::Logind`): Lock, Sleep, Hibernate, Reboot and
  Power off as systemd v262.1's `loginctl` and `systemctl` call logind, with the
  `WithFlags` fallback, the reboot flags (`power::reboot_flags`) and the
  soft-reboot retry, tested against fake new and old loginds and
  mutation-checked. Log out is `lyr_wm::hyprland::Hyprland::exit`
  (`hl.dsp.exit()` in Lua, as the dotfiles' `.menu-power` sends, or the `exit`
  dispatcher), tested against the fake Hyprland. Neither is run against the real
  thing. The menu (`apps/lyr/src/power_menu.rs`, drawn by
  `lyr_core::power_menu`) opens from `open_power_menu` as a grab popup under the
  clicked module: arrow keys and the pointer move the selection, Enter or a
  click carries the entry out on the source runtime, and Escape, a click outside
  or a second click on the module closes it. Tested on crcbl's headless shell
  and mutation-checked; run on Windows with injected input (open, Down, Escape,
  reopen, a click on the bar outside it), where the entries log that they are
  not built. crcbl 0.5.0's Win32 grab delivers Escape as a key instead of the
  close request its docs promise, so lyr closes on the key itself; reported to
  crcbl, whose fix (every backend) and the scancode-0 key codes below sit on its
  `win32-popup-leave` branch until that branch's CI passes, for 0.6.0. Keys
  injected with `SendKeys` arrive with no scancode, so crcbl names no key for
  them; `keybd_event` with scancodes works. `[power.menu]` sets an entry to a
  command (`{ exec = … }`), which runs instead of lyr's action. Not built: the
  Windows and macOS actions (plan 10 names Win32 APIs that need `windows-sys`,
  new to lyr), and checking what Hyprland does with an outside click (M5's open
  question for panels applies to this grab popup too).
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
- **Closing a panel on an outside click.** A panel opened from a key binding
  cannot be an `xdg_popup` with a grab, because there is no input serial. Losing
  keyboard focus (S17) closes it on a click into another window, but not on a
  click on the empty desktop. M5 records what Hyprland does and picks a fix.
- **Does the year calendar fit the 500 px panel?** Measured in M5; the panel
  widens if not ([plan/10](plan/10-popups-and-menus.md#clock-panel)).
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

- **Window title module** (`hyprland/window`; the draft plan listed it, but
  today's bar does not have it). Built on Hyprland's `activewindowv2` event.
- **`lyr import-waybar`**: convert `config.jsonc` + `style.css` to TOML.
- **sway and niri workspace backends**, and komorebi or GlazeWM on Windows.

## Not yet verified

- Nothing has run on Linux under a real compositor: everything Wayland, D-Bus
  and Hyprland is tested against fakes and crcbl's headless shell, so the
  owner's Hyprland is the first real run (M1's check). On Windows the bar has
  run, with tooltips, the power menu, the clock panel and the control socket
  (each entry above says what was seen).
- The release pipeline (phase 2 of [plan/14](plan/14-ci-and-release.md)) does
  not exist yet.
