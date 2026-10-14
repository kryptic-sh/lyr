# 03 — Surfaces and input

lyr's windows come from **crcbl-shell**. `lyr-shell` is the bar policy on top of
it: which windows exist, where they go, and what a pointer event means. It has
no platform code of its own. Everything platform-specific below is work that
belongs in crcbl-shell, and it is listed as a requirement at the end of this
section.

## Window kinds

lyr needs two kinds of window, on every platform:

- **Bar**: a thin strip along one edge of one output that reserves its space so
  maximised windows stop at it. It never takes keyboard focus. It shows on every
  workspace or Space and above normal windows.
- **Popup**: anchored to a rectangle on a bar (the module that opened it), above
  everything else, closed on an outside click or `Escape`. Tooltips, calendar,
  power menu and tray menus ([10](10-popups-and-menus.md)). The calendar and
  menus take the pointer, and menus take the keyboard while open.

## Linux — layer-shell

- **Bar:** a `zwlr_layer_surface_v1` on layer `top`, anchored top + left +
  right, height 50 (`height` in config), exclusive zone equal to the height,
  keyboard interactivity `none`, namespace `lyr`. This is waybar's `layer: top`,
  `exclusive: true`, `passthrough: false`.
- **Popup:** an `xdg_popup` parented to the bar's layer surface with
  `zwlr_layer_surface_v1.get_popup`, positioned with an `xdg_positioner` at the
  module's rectangle. The positioner flips and slides the popup to keep it on
  screen. A menu takes an explicit grab (`xdg_popup.grab`) so that clicking
  outside closes it.
- Scale: `wp_fractional_scale_v1` with a viewport. crcbl-shell already has both.

## Windows — appbar

- **Bar:** a `WS_POPUP` window with
  `WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE`, registered as an
  **appbar** (`SHAppBarMessage`: `ABM_NEW`, then `ABM_QUERYPOS` and `ABM_SETPOS`
  for the monitor's top edge). The shell then shrinks the work area, so
  maximised windows stop under the bar. It handles `ABN_POSCHANGED` and
  `ABN_FULLSCREENAPP` (hide while an app is full screen), and `ABM_REMOVE` on
  exit. A crash that skips `ABM_REMOVE` leaves the work area reserved until
  Explorer restarts, so the exit path also runs from a panic hook.
- The Windows taskbar stays. It is also an appbar, so the two stack. Hiding the
  taskbar is the user's setting ("automatically hide the taskbar"), and lyr does
  not touch it. The tray lives there anyway ([08](08-tray.md)).
- **Popup:** a `WS_POPUP` + `WS_EX_TOOLWINDOW | WS_EX_TOPMOST` window at screen
  coordinates computed from the bar's rectangle, clamped to the monitor's work
  area. For menus, the window activates so it can receive keys and close on
  `WM_ACTIVATE(WA_INACTIVE)`.
- Scale: per-monitor-v2 DPI awareness (crcbl-shell already sets it) and
  `WM_DPICHANGED`.

## macOS — status panel

macOS has no API to reserve screen space: the menu bar and the Dock are the only
things that do. Provisional default ([16](16-open-questions.md)): lyr's bar
**sits in the menu-bar strip**, the way SketchyBar does, and the user turns on
"Automatically hide and show the menu bar". Windows then maximise up to the top
edge, beneath the bar. That is the same overlap SketchyBar users accept.

- **Bar:** a borderless, non-activating `NSPanel` per `NSScreen`, at a window
  level above the menu bar's, with `collectionBehavior`
  `canJoinAllSpaces | stationary | ignoresCycle`, and
  `hidesOnDeactivate = false`.
- **Popup:** a child `NSPanel`, positioned in screen coordinates (AppKit's
  origin is bottom-left, so conversion happens once in crcbl-shell).
- Scale: `backingScaleFactor` per screen and
  `NSWindowDidChangeBackingPropertiesNotification`.

## Outputs

One bar per output, which is what the current waybar does (its config has no
`output` key). `output = ["DP-1"]` in config limits the bars to the named
outputs. Names are the platform's: `wl_output.name` on Wayland, the
`MONITORINFOEX.szDevice` display name on Windows, `NSScreen.localizedName` on
macOS.

Outputs come and go. A new output gets a bar, and a removed output's bar and
popups are destroyed. On Windows `WM_DISPLAYCHANGE` triggers a re-query; on
macOS `NSApplicationDidChangeScreenParametersNotification`.

## Input

The bar needs pointer input only: enter, leave, motion (for hover and tooltips),
buttons (left, middle, right) and scroll (discrete and high-resolution, with a 5
% step per notch as today). `lyr-core` hit-tests positions in logical pixels.
Menus need key presses for `Escape`, arrows and `Enter`.

## What crcbl must provide

This is the joint list for **lyr** and **pikr** (pikr's needs are from
[pikr/docs/gui-backend-research.md](https://github.com/kryptic-sh/pikr/blob/main/docs/gui-backend-research.md)).
It was sent to crcbl's development on 2026-10-15. Each row is open until crcbl
confirms it. The live state is kept in [docs/backlog.md](../backlog.md), and
this table records what was asked and why.

| #   | Requirement                                                                                                            | lyr | pikr | crcbl today (at `1db97a5b`)                                                                                                    |
| --- | ---------------------------------------------------------------------------------------------------------------------- | --- | ---- | ------------------------------------------------------------------------------------------------------------------------------ |
| S1  | Wayland layer surfaces: layer, anchors, exclusive zone, margins, keyboard interactivity (none / exclusive / on-demand) | ✓   | ✓    | No layer-shell protocol vendored                                                                                               |
| S2  | Create a surface on a **chosen output**                                                                                | ✓   |      | Monitors are enumerated on every backend (`MonitorId`/`MonitorInfo`); a window can only be put on one as borderless-fullscreen |
| S3  | Popups parented to a surface: `xdg_popup` (including on layer surfaces), positioned screen windows on Win32 and AppKit | ✓   |      | `xdg_popup` generated but unused; `WindowDesc` has no position or parent                                                       |
| S4  | Win32 appbar windows (`SHAppBarMessage`), tool, topmost and no-activate styles                                         | ✓   |      | Not present                                                                                                                    |
| S5  | AppKit non-activating `NSPanel` with a window level and Spaces behaviour                                               | ✓   |      | Not present                                                                                                                    |
| S6  | Cross-thread **wake proxy** for `wait_events` (other threads post an event and the loop wakes)                         | ✓   | ✓    | Not present; only the shell's own clipboard fds are polled                                                                     |
| S7  | Output enumeration and hotplug events on every backend, with name, scale and work area                                 | ✓   |      | Enumeration exists on every backend; hotplug events not confirmed on all of them                                               |
| S8  | Centred, borderless, focused, topmost toplevel on Win32 and AppKit                                                     |     | ✓    | Borderless only as fullscreen-on-a-monitor                                                                                     |
| S9  | Keyboard: XKB keymaps, modifiers, repeat, compose; text-input-v3 IME preedit and commit                                |     | ✓    | XKB keymaps on Wayland; IME preedit on AppKit only; no compose, no text-input-v3                                               |
| S10 | The UI draw pass usable **without** the 3D renderer (`UiRenderer` pulls in water, sprite and shader crates today)      | ✓   | ✓    | `UiRenderer` lives in `crcbl-render`                                                                                           |
| S11 | Fonts by family name from the system, or accepting font bytes the app found itself                                     | ✓   | ✓    | `Font::parse(&'static [u8])`; no system lookup                                                                                 |
| S12 | Rich text spans, images, and a virtualised list in `crcbl-ui`                                                          |     | ✓    | Virtualised fixed-height list, caret and selection, images exist; per-span styling not confirmed                               |
| S13 | Cold start to the first presented frame under 500 ms                                                                   |     | ✓    | Not measured                                                                                                                   |
| S14 | Accessibility (AccessKit)                                                                                              |     | ✓    | Not present and not planned                                                                                                    |
| S15 | Published to crates.io with a declared `rust-version`                                                                  | ✓   | ✓    | Never tagged, and no release scheduled; toolchain pinned to 1.98.1, no `rust-version`                                          |

The "crcbl today" column comes from a read-only survey at the commit named in
the heading, corrected by crcbl's own development session on 2026-10-15. That
session's recommendations, which are the owner's calls and not commitments:

- S1 – S7 belong in **crcbl-shell as general window kinds**, not in a lyr
  platform layer. crcbl's editor and menus need popups and overlays too, and a
  second set of Wayland, Win32 and AppKit layers would duplicate the configure
  handshakes, DPI handling and message pumps.
- S10: split the UI pass out of `crcbl-render` into a small leaf crate
  (`crcbl-ui-render`: hal + ui + shaders).
- S15: publish the engine's leaf crates (core, hal, shell, ui, input and the
  backends), and declare `rust-version` equal to the pinned toolchain.
- Exact version pins in lyr are fine. Every breaking change is a minor bump
  below 1.0, and there will be many while these window kinds land.
- crcbl's shell clipboard already works on every backend, so pikr could drop
  `hjkl-clipboard`.

None of this is scheduled in crcbl yet. The joint list is recorded in crcbl's
`docs/backlog.md`.

**If crcbl declines a lyr requirement** (S1 – S7, S10), the fallback is the
stack lyr was first drafted on. On Linux that is wayr, which already does S1,
once wayr fixes its own gaps (backlog). On Windows and macOS it is native
windows on `windows-sys` and `objc2`. Rendering would then be wgpu (buffr's
precedent) instead of crcbl-hal. [16](16-open-questions.md) records this.
