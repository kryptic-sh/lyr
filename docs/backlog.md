# Backlog

Work raised and not finished, decisions still open, and gaps in what has been
verified. Delete an entry when it ships; `git log` keeps the history.

## Deferred: questions for the owner

Each item below waits on a decision that is the owner's to make, and is deferred
until it is answered. The question comes first, then the options and what each
costs. The provisional decisions in
[plan/16-open-questions.md](plan/16-open-questions.md) are the defaults the
implementation follows meanwhile.

- **Network without NetworkManager: build the rtnetlink fallback?** (M4.) **Q:**
  The network module reads NetworkManager over zbus, which the dotfiles use.
  Should lyr also support machines without it, through a new rtnetlink crate
  (`rtnetlink` or `neli`) and a 30 s nl80211 read for the signal (plan 07), or
  leave the module out there?
- **Calendar locale: day and month names, and an override?** (M5.) **Q:** The
  first weekday follows the locale everywhere now, in `lyr_sources::locale`:
  Windows' `LOCALE_IFIRSTDAYOFWEEK`, macOS' `NSCalendar`, and elsewhere the
  region of the POSIX locale (`LC_ALL`, `LC_TIME`, `LANG`) looked up in CLDR
  47's `weekData` `firstDay`, transcribed into `from_region` and checked against
  CLDR's XML: in-process and the same on glibc, musl and the BSDs (the owner's
  rule, 2026-10-22: no external program; glibc's own `week-1stday` exists only
  on glibc). A locale glibc has been customised away from CLDR is not followed,
  and a locale with no region (`C`, `POSIX`) falls back to the panel's Monday.
  Day and month names are still English; localizing them without a program means
  CLDR's names (`icu4x`, a new dependency, or a transcription) on Linux,
  `GetLocaleInfoEx` on Windows and `NSDateFormatter` on macOS. Localize them,
  and add a `clock.first_weekday` key to override the locale? The owner's
  machine is `en_US`, where neither shows a difference.
- **Week numbers: waybar's or ISO?** (M5.) **Q:** The calendar numbers weeks as
  waybar does for a Monday start (`%W`), since the dotfiles do not set
  `iso8601`. ISO weeks (`%V`) are a one-line change. Keep `%W`?
- **Calendar font: proportional or monospace by default?** (M5.) **Q:**
  `theme.calendar_font` defaults to the bar's font, where waybar's `<tt>` makes
  the calendar monospace. Should the default be `"monospace"`? Judge it against
  waybar's look on the owner's machine.
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
- **The defaults-parity test on CI: check in a copy of `config.jsonc`?** (M3.)
  **Q:** The test is built (`lyr_config`'s
  `the_defaults_match_the_dotfiles_waybar_config`): it imports the file
  `LYR_WAYBAR_CONFIG` names and checks every value both configs set, `[*.on]`
  left out and swaync's inhibited icons named as not carried (plan 09). It is
  ignored without the variable, and passed against the dotfiles on the owner's
  Arch machine (2026-10-20, 55 values). A copy of `config.jsonc` checked in as a
  fixture would let CI run it too, at the cost of a copy that drifts from the
  dotfiles. Check one in, or keep it a local check?
- **Completions and a man page: may lyr take `clap_complete` and
  `clap_mangen`?** (M11.) **Q:** tmxr's binary has `--completions <shell>` and
  `--man`, which its AUR package, Homebrew formula and release tarball install.
  lyr's release pipeline (2026-10-26) ships without them, since both crates are
  new to lyr. Take them and add the flags and the install steps (tmxr's
  `pkg/aur/PKGBUILD-bin.in`, `pkg/homebrew/tmxr.rb.in` and its release job show
  each), or keep shipping the binary alone?
- **The dotfiles' standalone tools** (M8). **Q:** When the dotfiles switch to
  lyr, do `wiremix` and `bluetui` stay installed, and do the `krypt menu` names
  stay as wrappers?
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
  `{title}`, `{class}` and `max_length`? (An app icon for it would use
  `crcbl-image`'s theme lookup, as the tray does.)

- **The `objc2` framework crates on macOS: keep them?** (M10.) **Q:** The macOS
  sources take `objc2-io-kit`, `objc2-core-audio`, `objc2-core-foundation`,
  `objc2-foundation`, `objc2`, `block2`, `objc2-system-configuration`,
  `objc2-core-wlan` and `objc2-io-bluetooth`, under plan 02's "`objc2` +
  `objc2-core-audio`, …" row, read as covering the `objc2` family (the row
  carries no "needs the owner's approval" note, as `libpulse-binding`'s does;
  crcbl uses `objc2`, `objc2-core-foundation` and `objc2-foundation`). Keep
  them, or name the ones the row did not mean, and those sources come out until
  a decision?
- **Running lyr on the real thing: when, and on what?** (M1, M10.) **Q:** On the
  owner's Arch laptop most of it has now run, nested in Hyprland 0.56 (the
  entries below say what each showed): the bar, the Lua dispatch, the menus and
  panels clicked through `wlrctl`, the notification daemon on a private bus, the
  tray with a test item, every Linux source against the real services, the bar
  inside niri 26.04 and sway 1.12, and the owner runs it on the live desktop.
  The window title followed real windows there too, on all three compositors,
  and the frame gate held with each one's outputs turned off. Still only the
  owner can run: the frame gate on a real monitor hypridle turns off, a battery
  charging or discharging, a successful Wi-Fi join and disconnect; and on a Mac
  with a battery, Wi-Fi and AeroSpace, the macOS sources, the Settings
  fallbacks, and whether IOBluetooth's notifications arrive on crcbl's AppKit
  run loop. Which can you run, and should lyr hold any milestone for them?
- **macOS Wi-Fi and Bluetooth menus: build them natively without a Mac to try
  them?** (M10.) **Q:** Plan 10 wants native menus wherever there is a public
  API: CoreWLAN can scan and join (the scan's names need Location permission),
  IOBluetooth can list, connect and pair. Nothing here can run them, and CI's
  Mac has no Wi-Fi, so they could be checked only to compile and to find no
  networks. Build them blind, wait for a Mac, or keep the System Settings panes
  that open now (`settings::Page`, URLs from Ventura-era community sources, not
  Apple's documentation)?
- **macOS power profile: what does a click do?** (M10.) **Q:** Low Power Mode is
  read-only to lyr (setting it needs `pmset` as root). Plan 07 says a click
  shows a tooltip saying so; lyr has no click-triggered tooltip, so a click now
  logs that `cycle_profile` is not implemented. Open System Settings' Battery
  pane, where the mode is set (another undocumented pane id), build a
  click-triggered tooltip, or leave the hover tooltip as the whole module?

- **Reload: start and stop sources by the layout?** (M3.) **Q:** Plan 05 says a
  layout or module change on reload starts and stops only the affected sources.
  Today every source the platform has starts at launch and runs whatever the
  layout shows; a reload re-lays out and redraws. Doing it as planned means each
  source becomes its own abortable task (Linux runs them in one `tokio::join!`
  now) with request channels made again on each start, so `Commands` swaps
  senders while the bar runs, on all three platforms; the notification daemon
  and the power menu stay up regardless, since toasts and Log out do not depend
  on a module. A smaller step: start only the layout's sources at launch, and
  have a reload that needs another one say a restart starts it. Build it as
  planned, take the smaller step, or leave every source running (a source whose
  module is hidden still wakes the bar on its changes)?

## Cross-repo work

- **crcbl: a native notification seam, and lyr's notifications on it.** The
  owner asked (2026-10-20) that crcbl gain one API for creating, reading,
  updating and dismissing notifications on every platform, and that lyr's
  notification work use it. office-crcbl put it on crcbl's backlog the same day,
  with the Linux client and server shapes lyr's daemon already has
  (`lyr_sources::notifications::linux`), the Windows (WinRT toasts,
  `UserNotificationListener`) and macOS (`UNUserNotificationCenter`, no reading)
  shapes, per-operation capability flags, and zbus as a crcbl dependency flagged
  as the owner's call. When it lands: move lyr's daemon and `notify_session`
  onto it, and the Windows bell from `ms-actioncenter:` to real state where the
  seam can read it.
- **crcbl: an audio-control seam, and lyr's audio on it.** The owner decided
  (2026-10-22), answering whether lyr's audio should stay on `pactl` or move to
  `libpulse-binding` or native PipeWire: neither. crcbl gains a proper audio
  seam any crcbl app can use on Linux, Windows and macOS to control the system's
  mixer (volume, mute, devices, streams, defaults, card profiles and ports, with
  change notification and no polling), and lyr's volume, mic and audio menu move
  onto it, dropping `lyr_sources::audio`'s per-platform code. Its implementation
  is to be mostly in-house or pure-Rust crates (the owner, the same day), so
  that a user needs no other program installed to use it: no shelling out to
  `pactl`, and no C client library, linked or loaded with `dlopen`
  (`libpulse-binding`, `pipewire-rs`, libpulse itself): on Linux the PulseAudio
  or PipeWire wire protocol implemented natively in the crate ("needs to be
  native in the crate", the owner); on Windows and macOS, the OS APIs through
  the `windows` and `objc2` crates. The owner approved crcbl's design the same
  day: a `crcbl-mixer` crate with two Linux backends, both in-house Rust,
  PipeWire's own protocol when its socket exists (node `Props` for volume and
  mute, device `Route` and `Profile` params for ports, profiles and device
  volume, the `default` metadata for defaults, `target.object` to move streams)
  and otherwise PulseAudio's native protocol; WASAPI on Windows, with
  `IPolicyConfig` to set the default device; CoreAudio on macOS.

  **Where it stands (2026-10-23).** Linux and Windows are on it: the volume, mic
  and audio menu use `crcbl-mixer` (`lyr_sources::audio::mixer`), so `pactl` and
  lyr's own WASAPI code (`lyr_sources::audio::windows`) are gone; Windows moved
  on 2026-10-23 (Windows sources entry). macOS follows when crcbl builds
  CoreAudio. When the sound server goes away the modules hide and lyr
  reconnects, retrying on a doubling wait (`AUDIO_RETRY_FIRST` to
  `AUDIO_RETRY_LONGEST`, `audio_retry_after`, in `apps/lyr/src/sources.rs`), on
  Windows' thread as on Linux's task.

  Verified on this laptop (PipeWire 1.6.9) against `pactl`: every device's
  volume, mute, default and active port (after crcbl's fix for HDMI2/3 and the
  Intel Headset source, which first had none); MoveTo, MakeDefault on the input,
  SetProfile on the Intel card, and the error paths, through a scratch probe; a
  stream starting woke the probe 0 times with `Interest::Devices` and 3 times
  with streams (after crcbl coalesced them; it was 1 and 20); in a nested
  Hyprland, the branch's bar at the headset's 50 % and the mic's 29 %, a scroll
  down to 45 % and back, a click muting and unmuting, and the menu's Output
  devices and Configuration tabs. Not verified: MakeDefault on an output
  (Spotify was playing on the headset), SetPort between two real ports (every
  device here has one), a Bluetooth sink, the PulseAudio backend against a real
  PulseAudio daemon (only through pipewire-pulse, forced with
  `CRCBL_MIXER_BACKEND=pulse`: both live tests in `system_services.rs` pass on
  each backend), and a restart of the session's own PipeWire. The reconnect was
  run against a private `pipewire` daemon (no session manager, so no devices)
  that lyr followed through `PIPEWIRE_REMOTE`: killed and restarted twice, lyr
  logged it gone at once and back within the backoff each time, and with the
  disconnect check in `Audio::watch` removed it never noticed. Changed from the
  `pactl` source: a step stops at 150 % (`level::MAX_PERCENT`) where `pactl` had
  no ceiling; card profiles list in the server's order, not by priority, which
  the mixer does not report; Bluetooth is the mixer's flag, not the monitor's
  name. Lessons carried: PipeWire's stream-restore remembers a stream's mute and
  device per application, so a test that mutes or moves a probe stream changes
  what that application starts with next; switching a card's profile and back
  within milliseconds left this laptop's Intel nodes renamed with a `.2` suffix
  (`…HDMI1__sink.2`), which neither later switches nor the "off" profile undo
  (restarting WirePlumber did), so a live test waits for the old nodes to go.

- **crcbl: Linux and Vulkan parity for 0.6 – 0.8.** Also at the owner's request
  (2026-10-20): every crcbl feature developed on the Windows machine to be
  implemented and tested on Wayland, X11 and Vulkan. The per-feature table is on
  crcbl's main (its backlog, "Raised by lyr on 2026-10-20"). By crcbl's report
  the same day, verified on this machine's Intel Vulkan: `Presenter::resize`
  with frames in flight, `push_opacity` groups, premultiplied transparent
  windows through sway, the real popup-reposition path on sway 1.12, and a layer
  panel at 1.5 scale. lyr's own runs showed `set_panel` and the frame gate
  working, the latter with an output turned off. Still open in crcbl:
  pointer-leave between an app's own windows on Wayland and X11, X11's
  `display_adapter`, and the scripted resize and read-back as CI jobs.

- **crcbl: what lyr uses, and at which release.** lyr pins every crcbl crate at
  `=0.11.0` (2026-10-26; 0.10.0 was skipped, since its crates.io publish stopped
  halfway; none of 0.11.0's breaking changes touch lyr's code, and its icon
  theme lookups take the nearest size from a per-theme index; the bar and its
  tray icons drew in a nested Hyprland on it). The next crcbl release brings
  `crcbl_app::gpu_choice` (on crcbl's main at 8d383ef4): saving
  `Presenter::gpu_choice()` and handing it back as `PresenterDesc::gpu_choice`
  opens Vulkan on that adapter's driver alone, 8-9 ms on the owner's laptop
  against 60-77 ms for every driver, and `SharedGpu` follows the first
  presenter; lyr takes it once it is released. 0.9.0 came 2026-10-23 for
  `SharedGpu` (that bump needed no change in lyr's code; 0.8.0's `Presenter`
  drains before a reconfigure, so lyr no longer does; 0.7.0 brought `set_panel`,
  frame pacing and `font::breaks`; crcbl's `rust-version` is 1.98, and lyr's
  follows). 0.8.1 ran on Windows. crcbl 0.4.0 (2026-10-18) brought `crcbl-app`,
  the window-plus-device crate lyr asked for so that it links no engine, audio
  (cpal, ALSA) or physics; 0.5.0's only breaking change is in crcbl-phys. M1
  adds `crcbl-app`, `crcbl-hal`, `crcbl-ui` (with `system-fonts`),
  `crcbl-frame`, `crcbl-font` and the backend per platform (Vulkan on Linux,
  D3D12 on Windows, Metal on macOS), each pinned exactly, and `crcbl-image` with
  M6. The GPU path, as crcbl reported it (not read yet at 0.4.0 or later; read
  before M1 relies on it):
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
  libxcb-cursor. AT-SPI is on crcbl's main (2026-10-20, unreleased): with
  crcbl-shell's `accesskit` feature each window gets an `accesskit_unix` adapter
  Orca can read and press, raising `AccessibilityActivated`,
  `AccessibilityAction` and `AccessibilityDeactivated`. It brings zbus 5
  (`accesskit_unix` 0.24) beside lyr's zbus 6, so turning the feature on in lyr
  puts two zbus versions in its build, and crcbl ignores three RustSec
  "unmaintained" advisories on zbus 5's crates meanwhile; whether lyr takes it
  before an `accesskit_unix` on zbus 6 is the owner's call. Still open: AppKit
  accessibility; AppKit's input-method switch, secure input and pre-edit; X11
  panels, popups and transparency; tests for the untested cells, including a
  Hyprland job, a FreeBSD VM job and S21 (fractional scale on a layer panel,
  which lyr's bar is); no flash on map on AppKit; cold-start timing (S13).
- **crcbl: gaps that bear on lyr's milestones.** S9: no pre-edit during a
  compose sequence, no surrounding text, and losing the keyboard mid-sequence is
  untested (crcbl judges the Wi-Fi password field needs none of it). S17: a
  panel's focus loss is tested on sway only, and a click on the empty desktop
  does not count as one (M5 measures Hyprland). S3: the Wayland
  `reposition_popup` path needs xdg_wm_base 3 and has not run anywhere.
  `ui.scrollbar` is vertical only.
- **crcbl's frame gate: adopted, seen on outputs turned off by the compositor.**
  A FIFO present to an output the compositor has turned off (hypridle DPMS)
  could block lyr's loop, by crcbl's reading of its own source (2026-10-19).
  crcbl 0.7.0's `Presenter` gate is in: `lyr_render::surface::Surface::draw`
  asks the shell for the next frame before each present, `Surface::observe`
  reads the answers, and every pumped event reaches every surface
  (`App::observe` in `apps/lyr/src/bar.rs`); a surface whose answer has not come
  presents nothing and stays dirty, and the loop's wait counts only what can
  present now (`ready()` on each window, menu and panel), so it sleeps rather
  than spins. `Surface`'s test on crcbl's headless shell and null backend shows
  a present asking, the next draw skipped, and the pump's answer opening it
  again, mutation-checked. On Windows the bar ran 106 s with `--stats`
  (2026-10-20): 3 presents (the first and one per minute tick) over 7 wake-ups.
  On a nested Hyprland 0.56.2 (2026-10-20) with its output turned off
  (`hl.dsp.dpms({ action = "off" })`) for 80 s: the control socket answered
  every request at once, so the loop never blocked; over the 97 s around it the
  bar woke 16 times (three were those requests) and presented 3 frames, so it
  did not spin either; turned back on, the bar showed the current minute. The
  same on sway 1.12 and niri 26.04, nested in a nested Hyprland (2026-10-22),
  each with its outputs turned off for about 75 s
  (`swaymsg 'output * power off'`, `niri msg action power-off-monitors`): lyr
  answered `lyr reload` every 18 s throughout, in 7 – 11 ms on niri, and stayed
  quiet (13 wake-ups and 3 presents over 77 s on sway, 18 and 6 on niri, the
  reloads among them); niri, woken by the pointer, showed the current minute.
  Turning the nested sway's output back on hung that sway, which is the
  nesting's, not lyr's. Not seen: the owner's real outputs under hypridle.
- **Two presenters crash NVIDIA's Wayland driver on teardown: reported to
  NVIDIA?** office-crcbl found (2026-10-22) that on the NVIDIA MX550 (driver
  615.78) under Wayland, a process with two `Presenter`s segfaults in
  `libnvidia-eglcore` when the second's swapchain is torn down; Intel and
  lavapipe are fine, and so is one presenter. lyr hit it at a monitor unplug
  with its bars on the MX550. lyr now opens every surface after the first on the
  first's device (crcbl 0.9.0's `crcbl_app::present::SharedGpu`, in
  `lyr_render::surface`), one the shared device cannot present to getting its
  own. Run on the laptop with the bars on the MX550 on crcbl 0.9.0 (2026-10-23,
  two outputs in a nested Hyprland, `CRCBL_ADAPTER=discrete`): six rounds of
  removing and re-adding an output, both bars back each time, then a clean
  `lyr quit`. Earlier, on the crcbl main commit: a tooltip and the clock panel,
  and the Intel default surviving the unplug. A report to NVIDIA is still open,
  the owner's call.

- **crcbl's `set_panel`: adopted.** crcbl 0.7.0's `Shell::set_panel` resizes a
  panel in place, and lyr uses it where it used to recreate one: the toast stack
  as toasts come and go (`toasts::panel_desc`), the Bluetooth, Wi-Fi and audio
  menus as their content's size changes (`menu_panel::panel_desc`), and the bars
  on a reload that changes `[bar]` (`Bars::reshape`, then `sync` for the
  outputs). Tested on the headless shell, each mutation-checked: the toast panel
  and the Bluetooth menu keep their window and are configured taller, and
  reshaped bars keep theirs with the new anchor and layer. Run on Windows
  (2026-10-20): a reload from a 30 px top bar to a 50 px bottom one moved it in
  place, seen in a screenshot. That run found that D3D12 refuses `ResizeBuffers`
  while frames in flight use the back buffers; crcbl 0.8.0's `Presenter` drains
  before every reconfigure, and lyr's own drain in `Surface::resize` is gone.
  Run again on Windows without it (2026-10-22, crcbl 0.8.1): reloads from a 30
  px top bar to a 50 px bottom one and back moved the bar each time, seen in
  screenshots, and lyr exited cleanly on `lyr quit`.
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

- **The media player card: where it still differs from swaync's.** The card
  draws the player's art (`lyr_core::centre`, `ART_SIZE`), read from a `file://`
  `mpris:artUrl` through `lyr_sources::icons` and cached per URL until no player
  names it (`lyr_sources::mpris::linux::Art`); run in a nested Hyprland
  (2026-10-26) with a stand-in player whose art was a PNG with no extension. Not
  carried from swaync 0.12.6 (`data/ui/mpris_player.blp`, `mpris_player.vala`):
  `http(s)` art (no HTTP client, plan 09) and `data:` URIs (base64, which lyr
  has no decoder for); the generic `audio-x-generic-symbolic` icon swaync shows
  when there is no art (its `show-album-art` defaults to `always`); the art's
  rounded corners (`Item::Image` has no radius); the blurred art behind the card
  (`mpris-background`); and swaync's layout, with the controls centred in a row
  under the art and text, and shuffle and repeat buttons. lyr keeps previous,
  play/pause and next at the card's right. WebP art is left out until lyr takes
  crcbl-image's `webp` feature. swaync is no longer installed on the owner's
  laptop (2026-10-26), so a side-by-side comparison needs it installed again.
- **Hyprland backend, rest of M2.** `lyr-wm::hyprland` is built against Hyprland
  v0.56.2's formats: the event parser, the `Tracker` that folds events into the
  shared `lyr_wm::State`, the JSON seed, and `Hyprland` (socket I/O, `from_env`,
  `subscribe`, `focus`). Its tests run against a fake Hyprland on Unix sockets,
  and one against a real Hyprland
  (`ipc::tests::a_real_hyprland_switches_and_logs_out`, ignored unless
  `LYR_E2E_HYPRLAND` names a Hyprland started for it, since it logs that one
  out; deliberately not `HYPRLAND_INSTANCE_SIGNATURE`). Run against a nested
  Hyprland 0.56.2 with a Lua config on the owner's machine (2026-10-20): the
  dialect read from `j/status`'s `configProvider`, a switch by number, a switch
  to a name holding `]]` and `;` (`name:NAME` as a Lua long string,
  `request::lua_long_string`), and Log out (`hl.dsp.exit()`) ending the event
  stream; with the hyprlang form sent instead, that Hyprland refused it, so the
  test reaches the real thing. No CI job runs it: no runner has Hyprland. Click
  is `focus_request` and scroll is `State::step`, both pure and tested. The bar
  shows the workspaces (`apps/lyr/src/sources.rs` runs the backend and hands its
  `State` to the main loop), and a click on a button or, with
  `workspaces.scroll`, a wheel step sends the switch through the same task
  (`apps/lyr/src/input.rs`, `lyr_core::pointer`); the bar's own click has not
  been made on Hyprland, for want of pointer injection. `lyr --stats` logs the
  idle counts (`apps/lyr/src/stats.rs`); on Windows an idle bar showed 3
  wake-ups, 2 events and 2 presents over 86 s (the start and one per clock
  minute). The idle test on Hyprland (nested, the owner's Arch machine, every
  Linux source running, 2026-10-20): presents now meet plan 12's target, one a
  minute for the clock, since a bar whose new draw list equals the one on screen
  presents nothing (`Window::shown`) and every D-Bus watcher hands on only a
  value that changed (`lyr_sources::bus::publish`); before, UPower's identical
  30 s re-sends and every Wi-Fi or BlueZ change the bar does not show each
  presented a frame. Wake-ups do not meet it yet, though they are down from 13 a
  minute once a Wi-Fi change (NetworkManager's background scans move it every
  few seconds) stopped waking the bar while the menu is closed
  (`Commands::wifi_menu`; the change waits in the channel for the next wake-up,
  which drains it before handling its own events, so a click opening the menu
  still sees it). Logged wake by wake on the owner's laptop (crcbl 0.8.1,
  2026-10-22): each clock minute costs three, the tick, an empty one about 14 ms
  later, and the compositor's `FrameReady` answering the present (crcbl's frame
  gate), so an idle minute is 3; UPower's 30 s re-sends change only `UpdateTime`
  and `Voltage` there, which the bar does not show, and wake nothing. The rest
  come from the desktop's apps while they are busy. A stream starting or ending
  (a notification sound) changed the audio menu's rows and re-sent an unchanged
  volume reading, two wake-ups a stream; the reading now wakes the bar only when
  it changed and the rows only while the menu is open, as the Wi-Fi state does
  (a silent `pacat` stream: two wake-ups before, none after; with the menu open,
  the rows still came at once). A tray item's updates (Spotify's and Discord's,
  a dozen in a minute at times) woke it too; one now wakes the bar only when
  what it draws of the items changed (`lyr_core::tray::faces`, each shown item's
  letter, status and picture), and waits for the next wake-up otherwise (a test
  item flipping its icon every second: 13 wake-ups in 12 s before, none after
  its first showing). A tooltip that changes under a pointer held still shows
  the new text at that next wake-up. The empty wake-up after the tick is Mesa's:
  traced with `WAYLAND_DEBUG` (2026-10-22), the present's commit is answered
  within about 9 ms by `wp_presentation_feedback` and the buffer's release on
  Mesa's own queues and by the frame callback on crcbl's, all on the one
  connection, and any data on the socket ends crcbl's `poll`, so the bar wakes
  once for Mesa's traffic and once for its `FrameReady`. It comes with each
  present on a shared connection, and nothing in lyr drives it.
  `lyr_wm::Compositor` picks it, or sway or niri, at start. Hyprland main's
  changes for the next release are in
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
  clock's tick, or on a source's update. It draws every module whose source the
  platform has (the per-platform entries below). **Run once, on Windows**
  (2026-10-19, with the owner's go-ahead): Windows 11 at 2560 × 1440, D3D12 on
  an AMD Radeon RX 7900 XTX as asked, the bar an appbar reserving 50 px. It drew
  the transparent bar and the clock box with its border, radius and text, as a
  screenshot showed, and the work area came back when the process was killed.
  Hack Nerd Font Propo is not installed on that machine, so it drew in the
  monospace fallback, and lyr said so. It has run on Hyprland 0.56.2 since (the
  entries below). `lyr quit` stops it. It repaints only when something it shows
  changed (plan 04), and presents only when the window system asks for a frame
  (crcbl's frame gate, its own entry above). It logs through `tracing` to
  `~/.local/state/lyr/logs/lyr.log` and to stderr (`apps/lyr/src/log.rs`,
  `RUST_LOG` to filter), capped at `log::MAX_BYTES` by moving it to `lyr.log.1`
  (tested on Windows and Linux, not watched rotating in a running bar). Colours
  go through crcbl-ui's `style::from_srgb8`, and a module's opacity is a
  crcbl-ui opacity group (`DrawList::push_opacity`), faded as one layer as CSS
  `opacity` is (crcbl 0.6.0); neither has been seen on screen since the change.
  crcbl's CI had not verified the opacity layer on Metal when it reported it.
- **Shell, rest of M1.** `lyr-shell` has the bars (`bars::Bars`): one
  transparent layer panel per output (or per named output), stretched along the
  configured edge and reserving its height, created and destroyed as monitors
  come and go and as the config changes, tested against crcbl-shell's
  `HeadlessShell` and mutation-checked. The main loop draws into them through
  crcbl-app (`lyr_render::surface::Surface`), follows resizes and scale changes,
  and hands pointer events to the hit tests (`apps/lyr/src/bar.rs`). Run on a
  real compositor: Hyprland 0.56.2 (the owner's, and nested) and Windows, where
  the same panels are Win32 appbars. A scale change was seen in a nested
  Hyprland (2026-10-20): the output set to 1.5 and back, the bar kept its
  surface and followed both. That run found every text cut short at 1.5 ("D T"
  drawn as "[ ]"): lyr gave crcbl-frame the scale for glyph sizes only, while
  positions and clips stayed in logical pixels. `translate` now records the
  whole list at the window's scale (`DrawList::set_scale`), so text, boxes and
  clips scale together, and the bar and the clock panel drew whole and sharp at
  1.5; tested in `translate` and mutation-checked. Removing an output under a
  running bar (a headless output in a nested Hyprland, added and removed,
  2026-10-21) stopped lyr with "invalid or stale window handle": `Bars`
  destroyed the bar's window (or answered its close request) while its surface
  lived on in `App.windows` until the `WindowDestroyed` that followed, so the
  same loop drew into the dead window, and crcbl wants a window's surface
  destroyed before the window anyway. `Bars::sync` and `Bars::handle` now take a
  `retire` hook that runs while the window is still open, and the app's
  (`App::drop_bar`) drops the bar's surface, tooltip and menus there; the `Bars`
  tests check each retired window is still open (mutation-checked). After it, 60
  rounds of adding and removing the output at random intervals left lyr running
  with its one bar, where it had died within two before, and five reloads
  between one bar and two raised no error.
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
  mutation-checked, and read from the owner's real one by the `system_services`
  test), so a zone change redraws the bar at once. The clock's text takes the
  new zone because chrono 0.4.45's `Local` re-reads `/etc/localtime` when its
  mtime changes, checking at most once a second (`src/offset/local/unix.rs`); a
  change within a second of the last draw shows on the next tick instead. Text
  is cut at its rect's edges (`lyr_render::translate` clips each text item), so
  a label wider than its box no longer runs over its neighbours. A format cannot
  drop the literal text around an empty placeholder; the one case that came up,
  a battery with no power reading, now shows 0 W as waybar 0.15.0 does
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
  (`notifications::linux::CAPABILITIES`: `actions`, `body`, `body-hyperlinks`,
  `body-markup`, `icon-static` and `persistence`), tested against an application
  on a peer-to-peer connection. Claiming the name is
  `notifications::linux::serve`: `DO_NOT_QUEUE`, so a name another daemon holds
  comes back as `Exists` and lyr takes its object down and names the owner and
  its pid, or `REPLACE_EXISTING` when told to replace; it allows being replaced
  in turn. Claiming is tested on a private `dbus-daemon`
  (`lyr_sources::bus::test::Bus`), since zbus 6 reports a taken name as
  `Error::NameTaken` on a bus and never as `Exists`, which the peer-to-peer
  tests could not show (`bus::claim` handles both). On the owner's Hyprland
  (2026-10-20) lyr named swaync (`:1.6`, its pid) and left the name to it.
  `Served::close` and `Served::invoke` are lyr's side, with their signals (an
  invoked action closes a notification that is not `resident`), tested and
  mutation-checked. The toast stack is `lyr_core::toast::stack::Stack` (newest
  on top, timeouts paused under the pointer) and a toast's draw list
  `lyr_core::toast` (swaync's look with the `toast_*` theme keys, its body
  wrapped where UAX #14 allows, through crcbl-ui 0.7.0's public `font::breaks`:
  `paragraphs` for the hard breaks and `soft_breaks` for where a line may end,
  which lyr asked crcbl to export because its bodies mix bold, italic and plain
  spans that crcbl's own `TextLayout` cannot lay out together; whitespace
  between pieces still becomes the one space that starts the next span), both
  tested and mutation-checked. The bar serves it on Linux
  (`apps/lyr/src/sources.rs`) and leaves notifications to another daemon that
  has the name, logging which, unless `notifications.replace = true` (read at
  start; a reload does not hand the name over). Toasts are on screen
  (`apps/lyr/src/toasts.rs`): one Overlay panel at the top right of the output
  the user is on, 16 px from the edges and clear of the bar, resized in place as
  the stack's size changes (`set_panel`, crcbl 0.7.0); a click invokes the
  `default` action when there is one and takes the toast down, a middle click
  dismisses the notification. Tested on crcbl's headless shell and
  mutation-checked. Seen on screen in a nested Hyprland on the owner's Arch
  machine (2026-10-20), lyr the daemon on a private session bus
  (`dbus-run-session`, so swaync kept the real one): a critical toast with its
  red border and a Reply button, a markup body (bold, italic, an entity) wrapped
  over three lines above it, newest on top; the clock panel's centre listing
  them critical first; `lyr dnd` and `lyr clear` acting on them. Clicked there
  through `wlrctl` too (2026-10-21): the Later button of a
  `notify-send -A reply=Reply -A later=Later` toast made `notify-send` print
  `later` and exit, so the action went back over D-Bus; and a toast with a 3 s
  timeout stayed 6 s under the pointer and went within 5 s of the pointer
  leaving. A toast that appears under a pointer that does not move gets no
  pointer enter from Hyprland, so it expires on time; moving the pointer onto it
  pauses it. The bell is drawn from the store while lyr is the daemon. A toast
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
  the styled spans), and `body-markup` is advertised. The panel's Clear All,
  switch and media players work; `lyr dnd` and `lyr clear` work, and on Windows
  answer that lyr is not the daemon, run there. A toast and a centre entry draw
  the notification's image (`image-data`, else `image-path`, else `icon_data`)
  left of the text with the app's icon over its right side, or the app's icon
  alone (`toast::PICTURE_SIZE`): the app's icon is `app_icon`, else the `Icon`
  of its desktop entry, looked for as swaync does
  (`notifications::linux::entry_ids`, `lyr_sources::desktop_entry`). The sizes
  are what swaync 0.12.6 drew with the dotfiles' style in a nested Hyprland
  (2026-10-26): 16 px, since their `* { all: unset }` leaves GTK's default icon
  size. lyr drew the same four notifications the same way there; tested and
  mutation-checked. An id the spec maps to a subdirectory (`kde-foo` as
  `kde/foo.desktop`) is looked for only as the flat file. libnotify's
  `notify-send -i` sends the icon as the `image-path` hint with `app_icon`
  empty, so it is the image, not the app's icon. A click on a body's link opens
  it with `xdg-open` (`exec::open_url`, no shell) when it is `http` or `https`
  (`markup::openable`; the owner's choice, 2026-10-26, with no confirmation),
  and leaves the toast up; any other scheme is logged and not opened. Clicked in
  a nested Hyprland (2026-10-26) with a stand-in `xdg-open` that wrote down its
  argument: an `https` link's URL arrived with its `&amp;` decoded, and a
  `file:///etc/passwd` link was refused. There, a second toast arriving was
  missing from a screenshot taken two seconds later and was up in the next one.
  It did not happen again (2026-10-26, nested Hyprland): two plain toasts sent
  0, 0.3 and 1 s apart were both up in every screenshot, taken each half second
  from 0.5 s to 4 s after the second. If it comes back, the difference from that
  run was the link bodies and a pointer moved by `wlrctl`. Each action but
  `default` gets a button under the body (swaync's style, at the bar's text size
  until the owner rules on swaync's `0.2rem`), in toasts and the panel's
  history; a click invokes it. A transient notification, kept in no history,
  closes as expired when its toast times out.
- **Sources, M4.** Built: the battery source. UPower's display device is read
  over D-Bus (`lyr_sources::battery::linux::UPower`, zbus 6) once and again on
  each `PropertiesChanged`, with no polling, and turned into the view's reading
  (`battery::upower::Device`). It is tested under WSL against a fake UPower on
  zbus's peer-to-peer connection, which needs no bus daemon, and
  mutation-checked; CI's Linux job runs the same test. The bar runs it, with the
  Hyprland workspaces, on a tokio runtime of its own thread
  (`apps/lyr/src/sources.rs`), each update handed to the main loop and the loop
  woken through crcbl's `ShellWaker`. Every Linux source's read also runs
  against the machine's real services in
  `crates/lyr-sources/tests/system_services.rs`, ignored unless asked for (CI
  has none of them): UPower (its battery checked against the kernel's
  `/sys/class/power_supply/BAT0/capacity`, mutation-checked),
  power-profiles-daemon, NetworkManager, BlueZ, timedate1 and the sound server.
  All passed on the owner's Dell Latitude 3440 (Arch, 2026-10-21): BAT0 full at
  100 % on mains (UPower reports a 153-day time to empty there, which the
  reading shows as unknown), the `performance` profile, the wired connection,
  the adapter, Asia/Manila, and the headset as the default sink. A switch of
  power profile through `PowerProfiles::set` ran there too, to power-saver and
  back, read back from the daemon (mutation-checked). Not exercised: a battery
  charging or discharging, since the laptop stayed on mains. The power-profile
  source is built the same way
  (`lyr_sources::power_profile::linux::PowerProfiles`): the daemon found under
  `org.freedesktop.UPower.PowerProfiles` or its older `net.hadess` name,
  followed on `PropertiesChanged`, switched by the `cycle_profile` built-ins,
  and tested against a fake daemon that has only the old name. The bluetooth
  source too (`lyr_sources::bluetooth::linux::BlueZ`): BlueZ's managed objects
  read once and again on each `PropertiesChanged` under `/org/bluez` or object
  added or removed, the rfkill block from the adapter's `PowerState`, and the
  device shown the last to connect, as waybar does
  (`lyr_sources::bluetooth::Tracker`); tested against a fake BlueZ. The volume
  and mic sources are built over crcbl's mixer (`lyr_sources::audio::mixer`),
  after waybar 0.15.0's `AudioBackend` for what the modules show; see "crcbl: an
  audio-control seam" for what was verified. The network source is built over
  NetworkManager (`lyr_sources::network::linux::NetworkManager`): the device of
  `PrimaryConnection` (the default route, as waybar picks it), else of
  `ActivatingConnection`, its IPv4 address, prefix and gateway, and on Wi-Fi the
  access point's SSID and strength, with the radio blocked when
  `WirelessEnabled` or `WirelessHardwareEnabled` is false; the state is waybar's
  `getNetworkState` (`lyr_core::network::Reading::state`), without its carrier
  test, since NetworkManager's chosen device has one. It re-reads on
  `PropertiesChanged` from the objects the reading came from only; a scan's
  access-point updates are drained and dropped. Every D-Bus watcher reads
  through `lyr_sources::bus::Follow`, which keeps draining its stream during a
  read: zbus stops reading a connection while any of its streams is full, and
  the first run against a real NetworkManager (2026-10-20) hung that way on a
  scan's burst, freezing the network module and the Wi-Fi menu (`Follow`'s test
  reproduces it on a peer-to-peer connection; mutation-checked). Tested against
  a fake NetworkManager on a peer-to-peer connection and mutation-checked, and
  run against the owner's NetworkManager on Arch (wired, Wi-Fi idle). Not built:
  the nl80211 fallback without NetworkManager.
- **macOS sources, M10.** Built: the battery (`lyr_sources::battery::macos`):
  IOKit's power sources (`IOPSCopyPowerSourcesInfo`), the first internal
  battery's description turned into the reading by `battery::iops` (tested and
  mutation-checked on every platform), re-read on each
  `IOPSNotificationCreateRunLoopSource` notification on a run-loop thread of its
  own. The `objc2` framework crates it takes are a question above. Checked by
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
  implemented (a question above). The network is built too
  (`lyr_sources::network::macos`): the primary interface, router and first
  address from the dynamic store's global and interface IPv4 entities (keys made
  with the framework's own key functions), Wi-Fi's name, signal (waybar's dBm
  scale, `network::scdynamic::signal`) and power from CoreWLAN, re-read on the
  store's notifications for the global IPv4 entity and every interface's IPv4,
  link and AirPort entities, on the shared run loop thread (`macos_runloop`).
  `lyr_core::network::Link::wireless` keeps a Wi-Fi link whose name macOS
  withholds (no Location permission) showing as Wi-Fi, and the radio being off
  shows as disabled when there is no link. The signal is read only on those
  changes, since no notification follows it; plan 07 has waybar re-reading it on
  an interval. `scdynamic` is tested and mutation-checked everywhere; CI's Mac
  reads the store and starts and stops the watch, and no Mac on Wi-Fi has run
  it. The workspaces are AeroSpace's (`lyr_wm::aerospace`): `list-workspaces`
  and `list-windows --focused` as JSON, read on a thread at start, on each
  `lyr event workspace` (AeroSpace's `on-focus-changed`, plan 06) and after a
  click's `aerospace workspace -- NAME`; the parsing and arguments are tested
  and mutation-checked on every platform, from AeroSpace v0.21.3-Beta's docs and
  its `formatToJson.swift`. No AeroSpace has run it: CI's Mac has none, and its
  process calls run nowhere in the tests. A window's title changing alone tells
  lyr nothing, so the window module can lag it. Bluetooth is built too
  (`lyr_sources::bluetooth::macos`): IOBluetooth's default controller and paired
  devices in BlueZ's shape, so `bluetooth::Tracker` gives the reading, re-read
  on `IOBluetoothHostControllerPoweredOnNotification` and
  `…PoweredOffNotification` and on connect and disconnect notifications sent to
  an Objective-C observer class (`define_class!`), registered on the bar's main
  thread, whose run loop IOBluetooth delivers on. That the main thread's run
  loop under crcbl's AppKit backend runs those deliveries is assumed, not seen.
  CI's Mac has a controller: its test reads it and registers and unregisters the
  observers, and found that `objc2-io-bluetooth` 0.3.2 binds `powerState` as
  unsigned where the runtime declares an `int`, so lyr sends that message
  itself. The Bluetooth menu, whose actions are BlueZ's, opens System Settings'
  Bluetooth pane on macOS instead. The audio menu is built too:
  `audio::macos::nodes` lists every device with output or input streams
  (`kAudioHardwarePropertyDevices`, `kAudioDevicePropertyStreams`), each with
  its volume, mute and default mark; its actions take a device
  (`audio::macos::Which`), and Default sets
  `kAudioHardwarePropertyDefault…Device`. The device list's changes come through
  a system listener, and while the menu is open it reads again each second, as
  on Windows (`MENU_REFRESH`). The stream tabs are empty, since macOS has no
  per-program volume. No Mac has shown the menu. The Wi-Fi and Bluetooth menus
  open System Settings' Wi-Fi and Bluetooth panes instead
  (`settings::Page::command`, as Windows opens `ms-settings:`), tested and
  mutation-checked for the command; native menus are a question above.
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
  seen, this machine being wired. The radio's state is read
  (`WlanEnumInterfaces`, then `WlanQueryInterface`'s
  `wlan_intf_opcode_radio_state`; off when every phy of every interface has its
  software or hardware switch off, `network::win32::radio_off`, tested and
  mutation-checked), but a radio switched off has not been seen either: this
  machine has no Wi-Fi interface, so the read returns false. It is taken at each
  reading, and a switch prompts one: the watch registers the WLAN service's MSM
  source, which carries `wlan_notification_msm_radio_state_change`, and every
  WLAN notification re-reads. The power profile too
  (`lyr_sources::power_profile::windows`): the effective power mode, reported by
  `PowerRegisterForEffectivePowerModeNotifications` (documented, Windows 10 1809
  and later) when registered and at each change, mapped onto
  power-profiles-daemon's three names with Windows' own mode name as the
  tooltip's driver (`power_profile::win32`); run on Windows (2026-10-20), its
  tooltip read "Profile: balanced / Driver: Balanced". Switching it has no
  public API on Windows, so its clicks (`cycle_profile` and
  `cycle_profile_back`) open Settings' Power page (`ms-settings:powersleep`, the
  owner's choice, 2026-10-22, over the undocumented
  `PowerSetActiveOverlayScheme`); run on Windows (2026-10-22), a click on the
  module opened Settings on System > Power, read through UI Automation. Volume
  and mic too, through crcbl's mixer since 2026-10-23 (`crcbl-mixer`'s WASAPI
  backend, read by `lyr_sources::audio::mixer` as on Linux; lyr's own WASAPI
  code before it is gone): the default output and input (`eConsole`), each one's
  scalar volume and mute, friendly name as the tooltip's description, and form
  factor as the icon's port kind. The mixer's thread follows every endpoint and
  session through WASAPI's callbacks; lyr's `windows::audio` thread reads its
  state on each change, and carries out `toggle_mute`, `volume_up` and
  `volume_down` (by `scroll_step`, held to 100 %, since Windows has no boost)
  through the controls Linux uses (`mixer::toggle_mute` and the rest). The audio
  service going away hides the modules until it is back, retried as on Linux;
  not tried here, since stopping it cuts what is heard. Run on Windows
  (2026-10-23): the bar showed the output at 40 % and the input at 100 %; a
  click on the volume module muted the default output and a second unmuted it,
  read back between clicks through crcbl's mixer from another process (this
  machine draws the icons as missing glyphs, so the bar could not show it); a
  forward (+120) wheel event took the output to 35 % and a backward one back to
  40 %. The step went down for a forward event because this machine has Windows'
  reversed mouse scrolling on (`ReverseMouseWheelDirection` = 1 under
  `HKCU\Control Panel\Mouse`), which Windows applies before any program sees the
  event; lyr follows it as every other program does, so it is not a bug. A
  Bluetooth endpoint is told apart by its device's bus enumerator
  (`PKEY_Device_EnumeratorName` among `BTHENUM`, `BTHHFENUM`, `BTHLEDEVICE`,
  crcbl's `bluetooth`); on this machine the USB headset's names `USB`, the
  device's bus rather than the endpoint's own `SWD`, and no Bluetooth endpoint
  has been seen. A handset's form factor reads as `handset`, which none of
  waybar's port kinds match, as on Linux; lyr's own code read it as `phone`. The
  notifications module is plan 09's Windows bell: always the `none` icon
  (`lyr_core::bell::stateless`), since Windows offers no API for its
  notifications or do-not-disturb, with both clicks bound in
  `[platform.windows.notifications.on]` to `start "" ms-actioncenter:`; run on
  Windows (2026-10-20), a click on it opened the notification centre. The Wi-Fi
  and Bluetooth menus open Windows' Settings pages in their place
  (`apps/lyr/src/settings.rs`, `ms-settings:network` and `:bluetooth` through
  `cmd /C start`), from their bindings and from `lyr panel`; run on Windows
  (2026-10-22), `lyr panel bluetooth` opened Settings on Bluetooth & devices and
  `lyr panel wifi` on Network & internet, read from the Settings window through
  UI Automation. The audio menu is native on Windows (`lyr_core::audio_menu`,
  `apps/lyr/src/audio_menu.rs`, its rows from `mixer::nodes`): tabs for playback
  and recording streams (WASAPI's audio sessions on every active endpoint, named
  by the session, else its program's file name, as crcbl names them; every
  endpoint has a system-sounds session, so those off the default endpoint are
  left out, and another stream there names its endpoint, `mixer::nodes_on` with
  `WINDOWS`, tested and mutation-checked on every platform) and output and input
  devices, each row with mute, a step down and up by the module's `scroll_step`,
  and on a device that is not the default a Default button, which makes it the
  default through crcbl's `IPolicyConfig` (undocumented; before crcbl's mixer it
  opened `ms-settings:sound`). A session's changes come through crcbl's
  callbacks, followed only while the menu is open; the once-a-second re-read is
  gone. Run on Windows (2026-10-23): `lyr panel audio` opened it on this
  machine's four output devices, the default in the active colour and no
  Configuration tab, and its Playback tab listed five streams, one of them
  System sounds, with no device buttons. The Default button has not been
  clicked, since it moves what is heard to another device; crcbl's own
  `wasapi_live` test sets a default. Each row's lookup is tested by a 0 % step
  on every device and stream
  (`this_machine_s_audio_reads_and_each_row_finds_its_control`, run on this
  machine). From the runs of 2026-10-20, which tried the menu rather than its
  source: a menu panel stays inside its output (`menu_panel::Place`, from the
  clicked bar's width; `lyr panel` with no bar showing the module opens at the
  left edge), and the audio menu opened ending 16 px from the screen's right
  edge, its Playback tab clicked for real redrawing it there. On the real
  machine (2026-10-20) a click on the steam stream's step down showed 95 % and
  its step up 100 % again, so a click reaches the source and the change comes
  back to the menu; Mute and a device's steps have not been clicked, since they
  change what is heard. Each row also has a slider
  (`audio_menu::Target::Level`): a click sets the volume where it lands
  (`level_at`, from `MenuPanel::picked_across`), up to `MAX_PERCENT`, 100 % for
  Windows; Enter on it does nothing, having no place. Clicked on the real
  machine (2026-10-20): a quarter of the way along steam's slider set 25 %, and
  its right end then a step up set 100 % again. A drag that starts on a slider
  sets it at every move until the button is let go (`MenuPanel::with_drags`),
  dragged on the real machine from a quarter to three quarters of steam's slider
  for 75 %. A right click on the mic module opened it on Input devices, listing
  this machine's two microphones with the default in the active colour. Not on
  Windows, which Linux's menu has: 150 % (Windows' endpoint volume stops at 100
  %), the cards tab (the source lists no cards, so it never shows), and moving a
  stream to another device (no public API). The Bluetooth source is built
  (`lyr_sources::bluetooth::windows`, 2026-10-22): WinRT's default adapter and
  its radio's state (`Windows.Devices.Radios`; disabled reads as BlueZ's
  `off-blocked`), and the paired devices, classic and LE, as association
  endpoints with `System.Devices.Aep.IsConnected` and `DeviceAddress`, a device
  paired over both joined by its address (`bluetooth::win32`, tested and
  mutation-checked on every platform). A `DeviceWatcher` on the adapters and on
  each kind of paired device, and the radio's `StateChanged`, call for a fresh
  read, on a thread in COM's multithreaded apartment. The owner chose
  (2026-10-22) to build it, and the native Wi-Fi menu, although this machine
  cannot show them working: its Bluetooth adapter is in a PnP error state and
  WinRT lists no radios (checked 2026-10-20), and it is on Ethernet. Run here,
  the source finds no adapter, so the module stays hidden, and its watchers
  start; nothing has shown a real adapter or device. Not seen: a radio switched
  on after start-up when no adapter was there at start (its `StateChanged` is
  not followed then). The Wi-Fi menu is native on Windows too (2026-10-22):
  `lyr_sources::network::windows::wlan` reads the first wireless interface, its
  radio and `WlanGetAvailableNetworkList`'s networks (one entry per SSID, so an
  SSID in hex stands for its access point's path), and scans, switches the
  radio, leaves, forgets (`WlanDeleteProfile`) and joins with `WlanConnect`,
  writing an all-user profile (`WlanSetProfile`) for a new network or a new
  password; the profile, the security mapping, the listing and what a failed
  join does with its profile are in `network::win32::wifi`, tested and
  mutation-checked on every platform. The WLAN service's scan and connection
  notifications end a scan and a join (a wrong password,
  `WLAN_REASON_CODE_MSMSEC_PSK_MISMATCH_SUSPECTED`, offers a new one for a saved
  profile and deletes a profile made for the attempt), and the bar is woken for
  the menu's state only while it is open, as on Linux. None of it has run: this
  machine's WLAN service is stopped (`WlanSvc`, manual start) and it has no
  wireless interface, so the source logs that the service is not running and the
  menu's click opens Settings; the read-and-watch test returns early here for
  the same reason. Not seen either: whether `WlanSetInterface`'s software radio
  switch needs elevation. The Bluetooth menu is native on Windows too
  (2026-10-23), with what Windows lets a program do
  (`lyr_core::bluetooth_menu::Abilities::WINDOWS`, the owner's choice): Power
  (`Radio::RequestAccessAsync`, then `SetStateAsync`), Scan (a `DeviceWatcher`
  on each kind of unpaired device while it is on, `bluetooth::windows::Scan`),
  Pair (`DeviceInformationCustomPairing`, its confirm-only, show-PIN, type-PIN
  and PIN-match requests asked as the menu's pairing questions on a thread of
  its own, refused after `PAIRING_ANSWER` unanswered) and Forget (`UnpairAsync`
  on each of the device's endpoints); a paired device's Settings button opens
  Settings, where Windows connects it, since connecting has no public API. A
  request for an adapter other than the one last read is refused. The
  pairing-kind mapping, the endpoint ids and the drawn buttons are tested and
  mutation-checked; none of it has run against an adapter, this machine's being
  broken, so here the click still opens Settings. Workspaces are left out on
  Windows (declined below); macOS's `x-apple.systempreferences:` fallbacks are
  built (the macOS entry).
- **Pointer input.** Built: a click or a wheel step on a bar is hit-tested
  against its last frame's regions (`apps/lyr/src/input.rs`) and resolved to an
  action by the module's bindings (`lyr_core::pointer::action`): a workspace
  button focuses its workspace, `{ exec = … }` runs through `sh -c` or `cmd /C`
  (`apps/lyr/src/exec.rs`), `cycle_profile` and `cycle_profile_back` switch the
  power profile, `open_power_menu` opens the power menu, `toggle_dnd` flips
  do-not-disturb as `lyr dnd` does (the bell's right click; not run end to end,
  since Windows draws no bell and WSL has no compositor), and every other
  built-in is handled; `cycle_profile` opens Settings' Power page on Windows,
  and on macOS says it is not implemented, waiting on a question above. Touchpad
  scrolling makes one step per 50 device pixels (`PIXELS_PER_STEP`, lyr's own
  choice; crcbl leaves it to the application). Scrolling away from the user
  steps to the previous workspace. Run on Windows with injected input
  (`SetCursorPos` and `mouse_event`, not a physical mouse; Windows sources
  entry): a click on the mic toggled its mute, a wheel step on the volume
  stepped it, and a click on the bell opened the notification centre. Hover is
  built: the workspace button under the pointer takes the active colour, as
  waybar's style colours both (`apps/lyr/src/modules.rs`), and moving onto
  another button redraws the bar. Tooltips are built
  (`apps/lyr/src/tooltips.rs`): timed as GTK 3 does (`lyr_core::tooltip`), a
  passthrough popup below the module (above it on a bottom bar) drawn by
  `lyr_core::draw::tooltip`, for every module that has one; tested on crcbl's
  headless shell and mutation-checked. Seen on Windows: the clock's `UTC+08:00`
  and the power button's "Power Button" tooltips opened below them. **Known
  issue on Windows:** the tooltip stays up after the pointer leaves the bar,
  until something else wakes the bar (a clock tick, a control request). Traced
  on crcbl 0.8.1 (2026-10-22, scripted `SetCursorPos`,
  `RUST_LOG=crcbl_shell=trace` and a temporary probe of the events the bar
  receives): crcbl reported the bar's `WM_MOUSELEAVE` 8 ms after the warp, the
  tooltip window never took a pointer move, but the leave reached lyr only 2.3 s
  later, when `lyr event workspace` woke the loop, and the tooltip then closed.
  The cause is crcbl-shell's Win32 `wait`: it drains the message queue first,
  and a message that drain dispatches (the leave, posted just after the warp's
  raw input already woke and pumped the loop) records an event that `wait` then
  sleeps on. Reported to crcbl with the trace; crcbl confirmed it and fixes it
  on its branch `win32-wait-recorded` (`Win32Shell::wait` returns
  `Wake::Recorded` when its drain recorded an event, with a desktop test of
  exactly this leave), for its next release. On that release: run the
  hover-then-warp check again, with no wake-up step. crcbl's traces reach lyr's
  log only since lyr bridges `log` into its subscriber (`tracing-subscriber`'s
  `tracing-log`); before that every crcbl record was dropped, on every platform.
  A tooltip follows its module when the module moves (repositioned where the
  window system can, opened again elsewhere). A tooltip of several lines (a
  format with line breaks) is drawn a row per line.
- **Control socket, rest.** Built: the socket (`apps/lyr/src/control.rs`,
  `interprocess`), the reply line, single-instance detection and `lyr quit`,
  tested on Windows and Linux (WSL), and run end to end on Windows (a second
  `lyr` refused; `lyr quit` stopped the bar, exit 0, the work area restored).
  `lyr reload` reloads the config, keeping the running one when the new one is
  invalid or its font will not load, and reshapes the bars in place when `[bar]`
  changed (`Bars::reshape`; they were recreated before crcbl 0.7.0); run end to
  end on Windows (30 px to 40 px when they were recreated, a 30 px top bar to a
  50 px bottom one in place, then a bad `bar.height` refused with the bar still
  running). `lyr toggle` hides the bars by closing them, so their space is given
  back on every platform, and shows them by creating them again (run on Windows:
  the work area grew by 50 px and shrank back). Run on a nested Hyprland 0.56.2
  too (2026-10-20): a reload from a 30 px top bar to a 50 px bottom one moved it
  in place (no new surface), a reload with `bar.height = -5` was refused with
  the bar kept, and `lyr toggle` took the reserved 50 px to 0 and back.
  `lyr dnd` and `lyr clear` act on the notification daemon on Linux and answer
  that lyr is not the daemon elsewhere. `lyr panel clock`, `lyr panel bluetooth`
  and `lyr panel wifi` open their panels; `lyr panel audio` opens the audio menu
  (M7) on Windows, macOS and Linux, and answers that there is no audio source
  where none is running. `lyr event workspace` has the macOS bar read
  AeroSpace's workspaces again, and answers with an error elsewhere.
- **Config, rest of M3.** Built in `lyr-config`: the schema for `[bar]`,
  `[layout]`, `[workspaces]`, `[clock]`, `[theme]` and every module's table, the
  embedded defaults, `[platform.*]` layering on both the defaults and the user
  file, and validation that names the key (unknown keys and modules, modules the
  platform cannot run, colours, ranges, format placeholders, the clock's
  strftime format), with tests that were each broken and seen to fail. The
  `[<module>.on]` action bindings (`actions.rs`) are checked against the module
  they are bound in, and the main loop runs them (see "Pointer input"). The
  tray's table is `[tray]`: `spacing` and `icon_size` (the dotfiles' 10 px
  between items and 18 px icons, both carried by `lyr import-waybar`). Not
  built: the file watcher that reloads on save (it needs a crate such as
  `notify`, new to lyr, so the owner's call), a finer diff (a reload redraws
  everything, and reshapes the bars only when `[bar]` changed; starting and
  stopping sources is a question above), the control socket (its own entry
  above), and the defaults-parity test on CI (it runs locally from
  `LYR_WAYBAR_CONFIG`; whether CI gets a fixture is a question above). Format
  strings are one `Format` type (`lyr-config`'s `format.rs`), parsed when the
  config loads and rendered with `Format::render`, so validation and drawing
  will share one parser. Every error names its full key (`layout.right[0]: …`,
  `theme.border: …`, the latter through `serde_path_to_error`). `lyr-config`
  keeps its own copy of the table merge, because `hjkl_config`'s `deep_merge`
  and `locate` are private in 0.42.2; if hjkl exports them, lyr drops the copy.
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
  anyone noticing, and now writes them as code points. One deliberate departure,
  at the owner's request (2026-10-21): a battery on mains that is neither
  charging nor discharging shows `format_plugged` (the dotfiles' plug), full or
  held below full, where waybar shows a full or not-charging battery in `format`
  as if on battery; `format_full`, when set, still wins at 100 %
  (`lyr_core::battery::view`). Seen on the owner's laptop, full on mains: the
  plug where the full battery was.

- **Native notification centre and menus, not yet researched**
  ([plan/09](plan/09-notifications.md), [plan/10](plan/10-popups-and-menus.md)).
  Written from memory and the dotfiles, with each item to read when its
  milestone starts: the Windows Wi-Fi and Bluetooth menu APIs (M9; the audio
  menu's WASAPI calls were read in `windows` 0.62.2 and are built) and macOS's
  CoreWLAN and IOBluetooth menus (M10; CoreAudio's were read in
  `objc2-core-audio` 0.3.2 and are built). Both wait on the questions above
  about building menus no machine here can show.
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
  (a setting of the Mac's), Hyprland closes it on a click on the bare desktop,
  as a grabbing popup's outside click does (nested, `wlrctl`, 2026-10-21).
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
  BlueZ, and drawn again as BlueZ changes (resized in place when its size
  changes). Tested on crcbl's headless shell and mutation-checked; never run
  against a real BlueZ. On Windows the menu draws what Windows allows, and opens
  Settings' Bluetooth & devices page on a machine with no adapter (Windows
  sources entry). The arrow keys move an outline through its buttons and
  switches in the order drawn, and Enter acts on the outlined one. The pairing
  agent (`bluetooth::linux::agent`) is built and registered at start as
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
  and the macOS menu (Windows' is built, with what Windows allows; Windows
  sources entry). Rename is built: a paired device's Rename button puts a field
  for its name at the top of the menu (text input `Normal`, the end of a long
  name in sight), OK or Enter sets `Device1.Alias` (empty gives back the
  device's own name), and Cancel or Escape drops it; the agent's question takes
  the top over from it. Tested on the headless shell and against a recording
  device on a peer-to-peer connection, mutation-checked; not run against a real
  BlueZ. The menu's reading and its scan are: the `system_services` test reads
  the owner's laptop's adapter and starts and stops a scan, seen in the
  adapter's `Discovering` (2026-10-21, mutation-checked). Pairing, connecting
  and renaming need a device, and it has none paired.
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
  fake NetworkManager and mutation-checked. Run against the owner's
  NetworkManager in a nested Hyprland (2026-10-20): the menu listed the 33 SSIDs
  in range, and on a stale scan showed "Scanning…" until the scan landed, then
  the list within half a second (about 3 s before `wifi` read each access point
  with one `GetAll`, all in flight at once, instead of a call per property;
  about 120 access points here). The owner joined a network from it on the
  laptop (2026-10-21): two joins with a wrong password failed on
  NetworkManager's "psk mismatch", and lyr deleted the profile each had made, as
  `recovery` says. Its Forget then failed with "Object does not exist at path
  …/Settings/5": the delete had landed between NetworkManager listing the
  profiles and lyr reading them, that read's error ended `watch_wifi`, and the
  menu kept the deleted profile. A profile or access point gone before its read
  is now skipped (`unless_gone`), NetworkManager's answer for such a path
  (`UnknownMethod`) is pinned by the `system_services` test, and a watch that
  ends says so at once rather than when all the Wi-Fi tasks end. Not run: a
  successful join, the radio switch. The owner looked for a Disconnect, and the
  network in use offered only Forget, which deletes its profile; its row now has
  Disconnect in place of Connect, which calls the device's `Disconnect`
  (NetworkManager then joins nothing on its own until asked), tested on the fake
  and mutation-checked at each layer. The real daemon answered `Disconnect` on
  the idle device with "This device is not active", so the call is the right
  one; a disconnect from a network in use was not run, since the laptop was on
  its wired connection. A menu taller than the room beside the bar is held to it
  and scrolls (`MenuPanel`: three lines a wheel step, the arrow keys bringing
  the outlined control into view, the frame kept at the window's size); seen on
  a nested Hyprland whose output was cut to 500 px, the Wi-Fi menu held to 418
  px and scrolled by keys typed with `wtype`. The panel is up
  (`apps/lyr/src/wifi_menu.rs`, its glue in `apps/lyr/src/bar/wifi.rs`): opened
  by `open_wifi_menu` or `lyr panel wifi` under the network module, a rescan
  asked for on every open and "Scanning…" until a newer scan lands or
  `wifi_menu::SCAN_WAIT` (lyr's own 15 s) is up, unless the last scan is under
  30 s old; a password typed with text input `Password`, spaces kept; `recovery`
  applied to a failure by the source task; a second click on the module within
  GTK's default double-click time leaves it open. Tested on crcbl's headless
  shell and mutation-checked. A failure while the menu is closed is a notice
  from lyr (`notifications::linux::notify_session`, to whichever daemon holds
  the name, lyr's own included), its text escaped for markup
  (`lyr_core::markup::escape`); the call is tested against lyr's daemon on a
  peer-to-peer connection, the glue that sends it is not. Two equally strong
  access points of one SSID: the first listed is joined, which either would do.
  The header's buttons take the arrow keys from the right (No before Yes), as in
  the Bluetooth menu. Both menus drop the keyboard's outline when a question
  appears (`MenuPanel::unfocus`), so Enter answers it.
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
  a peer-to-peer connection and mutation-checked; `serve` leaving the name to
  another watcher is tested on a private `dbus-daemon`; `follow_names` and
  `Host::register` are not. Run on the owner's Hyprland next to waybar's watcher
  (2026-10-20): Steam, Discord and Teams showed as letters, Spotify (passive)
  hidden. lyr draws items in the watcher's order; waybar 0.15.0 draws each as
  its properties finish loading (`Tray::onAdd`'s `pack_start`), so its order is
  whichever item answers first and can differ from lyr's. Not a bug; master's
  `Tray::reorderBox` sorts by a configured order. DBusMenu is read too
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
  mutation-checked; the glue joining source and bar has no unit test. It has met
  an item over a real bus (2026-10-21): in a nested Hyprland on a private
  session bus, where lyr was the watcher, a test StatusNotifierItem with a
  one-entry DBusMenu (a Gio script, not in the repo) showed as its title's
  letter, a right click opened its menu 16 px under the bar, and a click on the
  entry reached the item as `clicked`. That run found the menu 14 px right of
  the tray's box, anchored to the item's button inside it; it now opens under
  the module's box as every other menu does. Submenus closed the whole menu
  until 2026-10-26: each step opened the next grab popup before closing the old
  one, and Hyprland 0.56.2 dismisses a grab popup opened while another grabs
  (xdg-shell's `xdg_popup.grab`); `TrayMenu::reopen` now closes the old one
  first (`ListPopup::close_in_place`), and the headless test's `Grabbing` shell
  models that rule, mutation-checked. Then verified in the nested Hyprland: a
  test item's two-level menu by pointer and by keys (a click on a leaf reached
  it as `clicked`), and Teams for Linux's Settings › Theme › Built-in, three
  levels with its check mark, opened and closed without clicking an entry.
  Wayland gives a window no place on the screen, so `Activate` and `ContextMenu`
  get the point under the button on the bar, not on the screen. Each item's
  button has its tooltip, its `ToolTip` title or else its `Title`
  (`Item::tooltip_text`, as waybar falls back), through the bar's tooltips, now
  keyed by the hovered target rather than the module. Icons are drawn: see "Tray
  icons" below.
- **Tray icons (2026-10-25, the owner asked for them).** `crcbl-image` 0.9.0
  came in for them (PNG, SVG, XPM and the theme lookup; the owner had asked
  crcbl for it as the shared asset crate, S28). Each shown item's icon is found
  in waybar's order (`Item::icon_sources`, after waybar master's
  `Item::getIconPixbuf`, read 2026-10-25): while it needs attention its
  attention icon's name, then its pixels; then its icon's name (or a file's
  path), then its pixels; else the theme's `image-missing`; else its letter
  (`lyr_sources::tray::icon::Icons`). Names are looked up in the theme GTK's
  `settings.ini` names (`gtk-icon-theme-name`, GTK 3's then GTK 4's;
  Papirus-Dark here) with the item's `IconThemePath` first, or `hicolor` with no
  setting. A pixmap is the one nearest the host's size (`TRAY_PICTURE_SIZE` in
  `apps/lyr/src/sources.rs`; waybar takes the largest). Each window resizes a
  picture to the device pixels it covers and keeps it on its own atlas until a
  frame stops drawing it (`lyr_render::pictures`), drawn `tray.icon_size` tall
  with its aspect kept, as waybar does. Tested, each mutation-checked: the
  order, the pixel conversion, the size choice, a changed file read again, the
  theme path, the missing-image fallback, the GTK setting, the atlas's keeping
  and giving back, and the device size asked for. Run in a nested Hyprland
  (2026-10-25): Teams' and Discord's pixmaps drawn next to waybar's watcher
  (Discord with its red badge), and on a private bus a test item's
  `dialog-information` from Papirus-Dark flipping to `dialog-warning` on
  `NewIcon`. Not done: waybar's overlay icon and attention movie; `svg-text` (an
  SVG icon with text is refused and the next source tried); reading the theme
  from GSettings, which GNOME keeps there rather than in `settings.ini`. Not
  seen: an output at a scale above 1, a theme icon given by file path, Steam's
  or Spotify's own icons (neither was showing).
- **Clicking lyr in tests.** The owner installed `wlrctl` (2026-10-20), which
  moves and clicks a virtual pointer in the compositor named by
  `WAYLAND_DISPLAY` alone, so a nested Hyprland can be clicked without touching
  the real desktop (`wlrctl pointer move -5000 -5000` then `move X Y` puts the
  cursor at X, Y; `wlrctl pointer click left`). Headless sway still has no
  pointer on its seat. Before crcbl 0.8.1, clicks reached lyr late (a Wayland
  wait slept through events Mesa had read off the shared connection), so a
  toggle test was erratic; on 0.8.1 ten rounds of opening and closing the clock
  panel by clicks all toggled (2026-10-21).
- **How Hyprland treats a panel's keyboard and pointer (surprising, not a
  bug).** A panel takes the exclusive keyboard so its keys work from the moment
  it opens, and Hyprland 0.56 then holds it there while the panel is mapped and
  hands the panel every click, on the bar, a window or the desktop, at a
  position outside its surface. A click outside therefore never reached what it
  was aimed at, and the panel closes on it (`menu_panel::pressed_outside`), as a
  grabbing popup does; clicking another module while a panel is open closes the
  panel without opening the other. Taking the keyboard on demand instead was
  tried and reverted: with `follow_mouse` on (the owner's setting and Hyprland's
  default) focus moved to any window the pointer crossed and closed the panel
  with no click, which the owner reported. Seen in a nested Hyprland with
  `wlrctl` (2026-10-21): hovering a window kept the clock panel open, and a
  click in a window, on the desktop or on the clock closed it. On sway 1.12
  (nested, 2026-10-21) the panel keeps the keyboard too, but crossing a window
  with the pointer made sway send its keyboard a `leave` and at once an `enter`,
  which closed it with no click; a focus that comes back in the same batch is
  now dropped (`keys::settled_focus`), and hovering keeps it open there. sway
  hands a click outside to what is under it, not to the panel, so on sway a
  click in a window or on the desktop does not close a panel; Escape and the
  module's own click do. Not tried on niri. A grabbing dropdown opened from a
  panel (the audio menu's profile list) takes the keyboard from it on Hyprland
  0.56.2: in a nested run with `WAYLAND_DEBUG=1` (2026-10-23) the panel got
  `leave` and the popup `enter` right after `xdg_popup.grab`, and `wtype` keys
  moved the dropdown's selection. The audio menu hands the panel's keys to the
  dropdown anyway, in case a compositor leaves them there.
- **A panel that shrinks can show one stretched frame (seen once, not chased).**
  In the same run, switching the audio menu from Output devices to Configuration
  narrowed the panel, and a screenshot right after the click showed the new,
  narrower panel holding the old content squeezed to fit; a second screenshot
  two seconds later was right. That looks like the compositor scaling the old
  buffer to the new size until lyr's next frame lands. Not timed, and not tried
  on sway or niri.

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
  with the `calendar_*` theme keys; tested and mutation-checked. Weeks start on
  the locale's first weekday (Sunday on the owner's `en_US`, seen in a nested
  Hyprland, numbered `%U`); names stay English until the question above is
  answered (plan 16, 19). The panel is up too (`apps/lyr/src/clock_panel.rs`):
  opened by `open_clock_panel` (a click on the clock or the bell) or
  `lyr panel clock`, it opens under the module that opened it by the rule every
  menu follows (`menu_panel::Place`), and is as big as the calendar view it
  shows, the year or a month, resized in place when the view switches; with the
  notification centre below it is at least swaync's 500 px wide, and as tall as
  it holds up to 16 px short of the output's far edge. Seen in a nested Hyprland
  (2026-10-20): the year view 662 × 564 under the clock, ended 16 px from the
  right edge, at the same 16 px below the bar as the Wi-Fi and Bluetooth menus.
  The month view and the resize between them are tested on the headless shell
  only (no pointer injection here for the right click). It takes the keyboard;
  Escape, losing focus and the same click or binding again close it; a right
  click switches year and month mode and scrolling moves by one of either.
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
  control. Not built: the day and month names from the locale (the calendar
  locale question above), Hyprland's click on the bare desktop while it is open
  closes it (nested, `wlrctl`, 2026-10-21; "How Hyprland treats a panel's
  keyboard and pointer").
- **macOS cannot reserve screen space.** The bar overlaps windows unless the
  menu bar is set to auto-hide.
- **No tray and no notification state on Windows or macOS.** Neither platform
  offers an API ([plan/08](plan/08-tray.md),
  [plan/09](plan/09-notifications.md)).

## Ideas considered and declined

- **Windows workspaces through virtual desktops**
  (`IVirtualDesktopManagerInternal`, plan 06): declined by the owner on
  2026-10-22, so the workspaces module is left out on Windows. The interface is
  undocumented, its IIDs change with Windows builds, and a wrong IID calls
  through the wrong vtable rather than failing; the development machine's
  Insider build 26300 is in no public IID table. Reopen with komorebi's or
  GlazeWM's IPC if one of them is used.
- **Switching the Windows power mode through `PowerSetActiveOverlayScheme`**:
  declined by the owner on 2026-10-22 for opening Settings' Power page. The
  export is undocumented and bound by neither `windows-sys` nor `windows`, and a
  per-build table would refuse on this machine's Insider build, so it could not
  be seen working.

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

- **Window title module: built, run on Windows and on Linux's three
  compositors.** `window` (`lyr_core::window`, and
  `lyr_wm::State::active_window`) follows Hyprland's `activewindow>>CLASS,TITLE`
  event, seeded at start from `j/activeworkspace`'s `lastwindowtitle` (what
  waybar reads) and the client's class; `{title}` and `{class}`, `max_length`
  cut with an ellipsis, the tooltip the text unless `tooltip_format` is set. Not
  in the default layout. Tested and mutation-checked against recorded lines, and
  run on Hyprland 0.56 (nested on the owner's laptop, 2026-10-22, `window` in
  the centre zone): it showed a new alacritty's title, followed the focus to
  another window, and a lyr started with that window focused showed its title at
  once. On sway it is the focused window in `GET_TREE` (its app id, or its class
  under Xwayland, and its name), re-read on focus and title events; on niri the
  window niri reports focused, by app id and title. Both are tested against fake
  sockets, and both ran the same day, sway 1.12 and niri 26.04 nested in a
  nested Hyprland: two alacritty windows with set titles, the module following
  the focus as `swaymsg focus left` and `niri msg action focus-column-left`
  moved it. CI's real sway opens no window. waybar's `rewrite` rules,
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
- **komorebi or GlazeWM on Windows**: unbuilt; Windows workspaces are left out
  (declined above), and a tiling window manager's IPC is the way back in if the
  owner uses one.

## Not yet verified

- First run on the owner's Hyprland 0.56.2 (Arch, 2026-10-20, Intel RPL-U GPU,
  one 2560 × 1440 output, next to the running waybar): the layer panel reserved
  its 50 px under waybar's, and drew the power button, battery, power profile,
  network, Bluetooth, the workspaces (matching waybar's, the active one
  coloured) and the clock, as a screenshot showed. Since, in a nested Hyprland
  on the same machine (2026-10-21): tooltips (the battery's, from the real
  UPower), clicks through `wlrctl` opening and closing the power menu, the clock
  panel and the Bluetooth menu, toasts on a private session bus (the
  notification daemon's entry), and the idle counts with `--stats`: after
  start-up, 3 – 4 wake-ups and 1 – 2 presents a minute over three idle minutes,
  the clock's minute tick among them. Not exercised: a click on a toast or a
  tray item.
- The sway backend runs against a real sway only in CI's `sway e2e` job
  (headless, two outputs; first green 2026-10-20, run 39371299842): the first
  state with a workspace on each output, a switch to a name with a space and a
  `;`, a refused `$` name, and Log out. Its first runs found that sway quits
  without answering `exit` and sends events after its socket is gone, both
  handled since. Not exercised there: the focused window's title (a headless
  sway has no window open) and urgency. The niri backend is tested against a
  fake socket written from niri v26.04's `niri-ipc`, and against a real niri
  26.04 by `a_real_niri_switches_and_quits` (ignored; `LYR_E2E_NIRI` names the
  socket of a niri started for it, which it quits): run on the owner's laptop
  (2026-10-21) with niri nested in a nested Hyprland, where a niri on a hidden
  special workspace stalls without frame callbacks. It switched workspace, saw
  the switch, and quit niri, which closed the socket without answering, as read
  from its source (mutation-checked: without that handling the quit fails with
  `Closed`). The bar ran in that niri too, the workspace module following
  switches made with `niri msg`, its panel reserving its height. The sway
  backend ran on a desktop too, sway 1.12 nested on the owner's laptop
  (2026-10-21): the bar showed its workspaces and followed a switch to a name
  with a space made with `swaymsg`.
- The GPU choice (the owner's rule: `CRCBL_ADAPTER`, else the adapter driving
  the bar's display when it is discrete, else low power) is crcbl-app's
  `adapter::choose` inside the `Presenter`, with `PowerPreference::LowPower`
  named in `lyr_render::surface::Surface::open`. Seen on the owner's Windows
  machine, where the display's discrete RX 7900 XTX was chosen (crcbl 0.6.0,
  2026-10-20), and on the owner's hybrid laptop, a Dell Latitude 3440 (Intel
  Raptor Lake UHD and an NVIDIA GeForce MX550 with the proprietary driver, both
  outputs on the Intel card), in a nested Hyprland (2026-10-21): with nothing
  set the bar drew on the Intel GPU, `CRCBL_ADAPTER=discrete` drew it on the
  MX550 (presented across GPUs, seen in a screenshot) and `integrated` on the
  Intel one. `CRCBL_ADAPTER` takes a device class, not a name:
  `CRCBL_ADAPTER=NVIDIA` is refused with the classes listed. On Wayland that
  Intel choice came from low power, not from the display rule: crcbl 0.7.0's
  `Instance::display_adapter` did not answer there. crcbl 0.8.0's does (the
  compositor's dmabuf main device), and its `Presenter` ranks the adapters by
  it; on 0.8.0 the bar still drew on the Intel GPU in a nested Hyprland
  (2026-10-21), as both rules give on this laptop. Not seen: a hybrid laptop
  whose panel is wired to the discrete GPU, which is the case that tells them
  apart; that is the run that would show the display rule at work on Wayland.
- The release pipeline's publish jobs (`publish-crates`, `aur-bin`, `brew-tap`,
  `scoop-bucket`, [plan/14](plan/14-ci-and-release.md)) run only on a tag; the
  `build` and `package` jobs run on every push to main. v0.1.0's tag run
  published the GitHub release, the AUR package, the formula and the Scoop
  manifest, with hashes matching the release's `.sha256` files. Its
  `publish-crates` stopped at `lyr-core`: the crates.io index lagged the uploads
  (it took no update from anyone for about three and a half hours, 04:46 to
  08:18 UTC), cargo's own wait gave up after a minute, and `lyr-core` could not
  resolve `lyr-config`. Re-running the failed jobs once the index had both
  crates published the rest; all seven crates are at 0.1.0. The step now waits
  for each crate in the sparse index (`wait_for_index`) and retries an
  unresolvable dependency. On v0.2.0's tag run it did its job: cargo's wait for
  `lyr-core` timed out, `wait_for_index` waited a further 210 s until the index
  had it, and every channel published in one run, the AUR, formula and Scoop
  hashes matching the release's `.sha256` files. The Linux aarch64 and macOS
  x86_64 binaries are built but run by no job: only x86_64 Linux, Apple silicon
  macOS and Windows get a `--version` smoke run.
