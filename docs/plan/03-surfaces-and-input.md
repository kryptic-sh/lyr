# 03 — Surfaces and input

lyr's windows come from **crcbl-shell**. `lyr-shell` is the bar policy on top of
it: which windows exist, where they go, and what a pointer event means. It has
no platform code of its own. Everything platform-specific below is work that
belongs in crcbl-shell, and it is listed as a requirement at the end of this
section.

## Window kinds

lyr needs three kinds of window, on every platform:

- **Bar**: a thin strip along one edge of one output that reserves its space so
  maximised windows stop at it. It never takes keyboard focus. It shows on every
  workspace or Space and above normal windows.
- **Popup**: anchored to a rectangle on a bar (the module that opened it), above
  everything else, closed on an outside click or `Escape`. Tooltips, the power
  menu and tray menus ([10](10-popups-and-menus.md)). Menus take the pointer and
  the keyboard while open.
- **Panel**: a surface on an output, not parented to a bar, above everything
  else, which can open from a key binding with no click behind it. The clock
  panel, the Wi-Fi, Bluetooth and audio menus, and notification toasts
  ([10](10-popups-and-menus.md#two-kinds-of-window)). Panels take the keyboard,
  including text entry (Wi-Fi passwords, Bluetooth PINs); toasts do not.

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
- **Panel:** a `zwlr_layer_surface_v1` on layer `overlay`, anchored top + right
  (top + left for a panel opened from the left zone) with the 16 px margins
  swaync uses, exclusive zone 0, keyboard interactivity `on-demand` (`none` for
  toasts), on the output it was opened for. That is a layer surface the bar's
  own requirement S1 already covers.
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
- **Panel:** the same window kind as a popup, placed against the monitor's work
  area instead of a module's rectangle.
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
- **Panel:** a key-capable `NSPanel` (it has to become key for text entry) at
  the same level as the bar.
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
Menus and panels need key presses for `Escape`, arrows and `Enter`, dragging for
the audio menu's sliders, and text entry with the user's keymap, compose and
input method for Wi-Fi passwords.

## What crcbl must provide

This is the joint list for **lyr** and **pikr**, and it is the single list for
both: the owner has decided that pikr migrates to crcbl too, dropping floem and
its forks. S1 – S15 were sent to crcbl's development on 2026-10-15. S16 – S27
are pikr's further needs (its P1 – P12), which pikr's session sent to crcbl the
same day. lyr took on S9, S12, S17, S19 and S22 as well on 2026-10-16, when its
notification centre and menus became native: text entry, notification markup and
images, panel focus, caret blink, and emoji in notification text. S28, one image
and asset crate for both apps, was asked for on 2026-10-16 at the owner's
request, so neither app adds its own decoders. This table records what was asked
and why. Where each row stands on each backend is in the
[backend matrix](#backend-matrix) below, and the work still open in
[docs/backlog.md](../backlog.md).

| #   | Requirement                                                                                                                                                                                                                                                                        | lyr | pikr |
| --- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --- | ---- |
| S1  | Wayland layer surfaces: layer, anchors, exclusive zone, margins, keyboard interactivity (none / exclusive / on-demand)                                                                                                                                                             | ✓   | ✓    |
| S2  | Create a surface on a **chosen output**                                                                                                                                                                                                                                            | ✓   |      |
| S3  | Popups parented to a surface: `xdg_popup` (including on layer surfaces), positioned screen windows on Win32 and AppKit                                                                                                                                                             | ✓   |      |
| S4  | Win32 appbar windows (`SHAppBarMessage`), tool, topmost and no-activate styles                                                                                                                                                                                                     | ✓   |      |
| S5  | AppKit non-activating `NSPanel` with a window level and Spaces behaviour                                                                                                                                                                                                           | ✓   |      |
| S6  | Cross-thread **wake proxy** for `wait_events` (other threads post an event and the loop wakes)                                                                                                                                                                                     | ✓   | ✓    |
| S7  | Output enumeration and hotplug events on every backend, with name, scale and work area                                                                                                                                                                                             | ✓   |      |
| S8  | Centred, borderless, focused, topmost toplevel on Win32 and AppKit                                                                                                                                                                                                                 |     | ✓    |
| S9  | Keyboard: XKB keymaps, modifiers, repeat, compose; text-input-v3 IME preedit and commit                                                                                                                                                                                            | ✓   | ✓    |
| S10 | The UI draw pass usable **without** the 3D renderer (`UiRenderer` pulls in water, sprite and shader crates today)                                                                                                                                                                  | ✓   | ✓    |
| S11 | Fonts by family name from the system, or accepting font bytes the app found itself                                                                                                                                                                                                 | ✓   | ✓    |
| S12 | Rich text spans, images, and a virtualised list in `crcbl-ui`                                                                                                                                                                                                                      | ✓   | ✓    |
| S13 | Cold start to the first presented frame under 500 ms                                                                                                                                                                                                                               |     | ✓    |
| S14 | Accessibility (AccessKit)                                                                                                                                                                                                                                                          |     | ✓    |
| S15 | Published to crates.io with a declared `rust-version`                                                                                                                                                                                                                              | ✓   | ✓    |
| S16 | Transparent, per-pixel-alpha surfaces on layer-shell, Win32 and AppKit (the bar background is transparent)                                                                                                                                                                         | ✓   | ✓    |
| S17 | Focus gained / lost events on layer surfaces, and reliable first-key delivery                                                                                                                                                                                                      | ✓   | ✓    |
| S18 | FreeBSD Wayland build                                                                                                                                                                                                                                                              |     | ✓    |
| S19 | Deadline / timer wake of `wait_events` (caret blink)                                                                                                                                                                                                                               | ✓   | ✓    |
| S20 | Pointer input for lists and modules: hover, click, cursor shape, discrete and high-resolution wheel; scrollbar handle styling                                                                                                                                                      | ✓   | ✓    |
| S21 | `wp_fractional_scale_v1` on **layer** surfaces, not only toplevels                                                                                                                                                                                                                 | ✓   | ✓    |
| S22 | Colour emoji (COLR / CBDT / sbix) with per-glyph font fallback, and text measurement                                                                                                                                                                                               | ✓   | ✓    |
| S23 | The CSS subset pikr's `default.css` uses, through crcbl-ui's stylesheet or an `hjkl-css` mapping (lyr: only if [16](16-open-questions.md) item 6 picks CSS)                                                                                                                        |     | ✓    |
| S24 | RGBA image upload of bitmaps the app rasterised itself (tray icons, pikr's SVG icons)                                                                                                                                                                                              | ✓   | ✓    |
| S25 | A clean exit from the event loop                                                                                                                                                                                                                                                   | ✓   | ✓    |
| S26 | No usable GPU: a software fallback, or a documented, clear failure                                                                                                                                                                                                                 | ✓   | ✓    |
| S27 | Win32 tool window kept out of the taskbar and Alt-Tab, taking foreground focus on a hotkey launch                                                                                                                                                                                  |     | ✓    |
| S28 | An image and asset crate shared by both apps: PNG, JPEG and SVG (at a size and scale) to straight RGBA8; raw-buffer conversions (SNI ARGB32, notification `image-data`, Win32 BGRA); resizing; XDG icon-theme lookup with a cache; limits for untrusted input; CPU only and `Send` | ✓   | ✓    |

When the list was sent, crcbl's development session made these recommendations,
which are the owner's calls and not commitments:

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

The joint list is recorded in crcbl's `docs/backlog.md`.

**If crcbl declines a lyr requirement** (S1 – S7, S10), the fallback is the
stack lyr was first drafted on. On Linux that is wayr, which already does S1,
once wayr fixes its own gaps (backlog). On Windows and macOS it is native
windows on `windows-sys` and `objc2`. Rendering would then be wgpu (buffr's
precedent) instead of crcbl-hal. [16](16-open-questions.md) records this. The
fallback is lyr's alone: pikr no longer considers wayr.

### Backend matrix

The owner asked on 2026-10-17 that every row work on every backend. crcbl's
session built this matrix from crcbl's code, e2e tests and CI logs at main
`4d48eaf4` (CI run 38988977910), and `d3f7e280` for the transparency rows, and
reported it on 2026-10-17. It is crcbl's report; lyr has not checked it. Since
then, 0.3.0 fixed `GpuContext`'s and crcbl-mtl's forced opacity (S16); 0.4.0
made `TEXT_IME` honest (S9), added the fall-through to D3D12 (S16) and
cursor-shape-v1 on Wayland (S20, the `CURSOR_SHAPE` cap); and X11 cursor shapes
are on main after 0.5.0. The cells below are not yet updated for those.

- **CI**: an automated test runs it in that CI job.
- **unit**: pure tests only.
- **untested**: the code exists and nothing runs it.
- **refused**: fails loudly. On X11 the capability is never set, so
  `role::check` returns `Unsupported` (tested headlessly only).
- **SILENT**: missing and accepted silently. crcbl treats each as a bug and
  fixes these first.
- **n/a**: does not involve that backend. Shell-only rows leave the GPU columns
  n/a.

| #   | Wayland                                                                                                   | X11                                                                                                          | Win32                                                    | AppKit                                                    | Vulkan                                                                                                                                                                              | D3D12                                        | Metal                       |
| --- | --------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------- | --------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------- | --------------------------- |
| S1  | CI                                                                                                        | refused                                                                                                      | CI                                                       | CI                                                        | n/a                                                                                                                                                                                 | n/a                                          | n/a                         |
| S2  | CI                                                                                                        | refused                                                                                                      | CI                                                       | CI                                                        | n/a                                                                                                                                                                                 | n/a                                          | n/a                         |
| S3  | CI; the reposition success path never runs (CI's sway has xdg_wm_base < 3, so only the refusal is tested) | refused                                                                                                      | CI                                                       | CI                                                        | n/a                                                                                                                                                                                 | n/a                                          | n/a                         |
| S4  | CI                                                                                                        | refused                                                                                                      | CI; the `FullScreenApp` test is flaky                    | refused (`PANEL_RESERVE`; impossible on macOS)            | n/a                                                                                                                                                                                 | n/a                                          | n/a                         |
| S5  | CI                                                                                                        | refused                                                                                                      | CI                                                       | CI                                                        | n/a                                                                                                                                                                                 | n/a                                          | n/a                         |
| S6  | CI                                                                                                        | CI                                                                                                           | CI                                                       | CI                                                        | n/a                                                                                                                                                                                 | n/a                                          | n/a                         |
| S7  | CI                                                                                                        | enumeration CI; hotplug untested                                                                             | enumeration CI; hotplug untested                         | enumeration CI; hotplug untested                          | n/a                                                                                                                                                                                 | n/a                                          | n/a                         |
| S8  | CI                                                                                                        | refused                                                                                                      | CI                                                       | CI                                                        | n/a                                                                                                                                                                                 | n/a                                          | n/a                         |
| S9  | CI                                                                                                        | keys CI; compose unit; IME **SILENT** (the `TEXT_IME` cap claims an IME, and there is no XIM or IBus client) | keys, dead keys and context switch CI; pre-edit untested | keys CI; IME on/off, pre-edit and secure input **SILENT** | n/a                                                                                                                                                                                 | n/a                                          | n/a                         |
| S11 | CI                                                                                                        | CI                                                                                                           | CI                                                       | CI                                                        | n/a                                                                                                                                                                                 | n/a                                          | n/a                         |
| S12 | n/a                                                                                                       | n/a                                                                                                          | n/a                                                      | n/a                                                       | text golden CI; runs, weight and underline unit                                                                                                                                     | same as Vulkan                               | same as Vulkan              |
| S13 | not measured                                                                                              | not measured                                                                                                 | not measured (one manual pikr run on D3D12: about 1.1 s) | not measured                                              | n/a                                                                                                                                                                                 | n/a                                          | n/a                         |
| S14 | refused                                                                                                   | refused                                                                                                      | CI                                                       | refused                                                   | n/a                                                                                                                                                                                 | n/a                                          | n/a                         |
| S16 | CI                                                                                                        | refused                                                                                                      | CI                                                       | CI                                                        | composite alpha not validated by the HAL (**SILENT** for raw-HAL callers; `GpuContext` refuses a surface without premultiplied); refuses transparent Win32 (CI) and AppKit surfaces | CI (premultiplied through DirectComposition) | CI (macos-26 only)          |
| S17 | CI under sway; Hyprland untested                                                                          | refused                                                                                                      | untested; `Exclusive` does not hold focus as documented  | untested; `Exclusive` does not hold focus as documented   | n/a                                                                                                                                                                                 | n/a                                          | n/a                         |
| S18 | FreeBSD: clippy only, nothing runs                                                                        | n/a                                                                                                          | n/a                                                      | n/a                                                       | n/a                                                                                                                                                                                 | n/a                                          | n/a                         |
| S19 | CI                                                                                                        | CI                                                                                                           | CI                                                       | CI                                                        | n/a                                                                                                                                                                                 | n/a                                          | n/a                         |
| S20 | events CI; cursor shape **SILENT**                                                                        | events CI; cursor shape **SILENT**; wheel in lines only                                                      | CI                                                       | CI                                                        | n/a                                                                                                                                                                                 | n/a                                          | n/a                         |
| S21 | untested on panels                                                                                        | refused                                                                                                      | CI at the runner's scale                                 | n/a                                                       | n/a                                                                                                                                                                                 | n/a                                          | n/a                         |
| S22 | n/a                                                                                                       | n/a                                                                                                          | n/a                                                      | n/a                                                       | unit                                                                                                                                                                                | unit                                         | unit                        |
| S24 | n/a                                                                                                       | n/a                                                                                                          | n/a                                                      | n/a                                                       | golden CI                                                                                                                                                                           | golden CI                                    | golden CI                   |
| S25 | untested                                                                                                  | untested                                                                                                     | untested                                                 | untested                                                  | n/a                                                                                                                                                                                 | n/a                                          | n/a                         |
| S26 | n/a                                                                                                       | n/a                                                                                                          | n/a                                                      | n/a                                                       | loud `NoBackend`; lavapipe is the CPU adapter in CI                                                                                                                                 | WARP CI                                      | refused (no software Metal) |
| S27 | n/a                                                                                                       | refused                                                                                                      | style CI; foreground focus untested                      | CI                                                        | n/a                                                                                                                                                                                 | n/a                                          | n/a                         |

S10, S15, S23 and S28 do not involve a backend.

Refused because they are impossible: D3D12 off Windows, Metal off Apple, a
software Metal, and screen reservation on AppKit. crcbl declined a transparent
Vulkan window on Windows (Vulkan presenting through DirectComposition), as large
when D3D12 is always there on Windows. Instead, a transparent Win32 window made
through `GpuContext` will fall through to D3D12.

Cells that matter to lyr's milestones:

- **S21 untested on Wayland panels**: the bar is a layer panel at fractional
  scale, so M1 checks it.
- **S20 cursor shape silent on Wayland**: the link and hover cursors in the
  clock panel (M5, M6) wait on crcbl's cursor-shape-v1 fix.
- **S17 on Hyprland**: untested in crcbl's CI. M5 measures it.
- **S25 untested everywhere**: M1 tests lyr's own clean exit on `lyr quit`.
- **S17 and S9 on Win32 and AppKit**: M9 and M10 depend on them.
