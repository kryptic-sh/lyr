# 17 — Bar placement on every edge

`[bar] position` takes `top` and `bottom` today. This phase adds `left` and
`right`: a vertical bar down one side of each output, as waybar's `position`
allows. Once it ships, close it out per [00](00-index.md): leftovers go to
`docs/backlog.md`, and this file is deleted.

## What crcbl already gives

Nothing is needed from crcbl (read at crcbl-shell 0.17.1):

- **Anchoring.** `PanelAnchor` has `LEFT` and `RIGHT`, and `Reserve::OwnExtent`
  reserves whichever single edge a panel names (`LEFT | TOP | BOTTOM` reserves
  the left edge). Every backend maps it:
  - **Wayland:** layer-shell anchors with an exclusive zone.
  - **X11:** strut.
  - **Win32:** an AppBar on `ABE_LEFT` / `ABE_RIGHT` (`win32/panel.rs`).
  - **AppKit:** the panel overlaps, since macOS reserves nothing.
- **Popups.** `PopupPlacement` has `Left` and `Right` beside `Below` and
  `Above`.

## Config

```toml
[bar]
position = "left"   # top | bottom | left | right
height = 50         # a top or bottom bar's height
width = 50          # a left or right bar's width (new)
```

- **`width` is new.** It is a side bar's thickness in logical pixels, and is
  ignored on a top or bottom bar, as `height` is on a side bar. This follows
  waybar, which sizes a vertical bar with `width`. Its default matches
  `height`'s.
- **A side bar spans its output's whole height**, as a top bar spans its width.
  On a waybar side bar, `height` sets the bar's length; lyr has no such key, so
  `import-waybar` lists that `height` as not carried.
- **The zone keys stay `left`, `center` and `right`.** On a side bar, `left` is
  the top zone and `right` the bottom one, as waybar's `modules-left` and
  `modules-right` are on a vertical bar. Renaming them would break every config.
- **Reload.** A reload that changes `position` moves the bars, recreating their
  windows where the backend cannot re-anchor them in place.

## Drawing a side bar

- **Zones run top to bottom.** The centre zone is centred on the bar's length,
  as it is on a horizontal bar. `lyr_core::layout` is one-dimensional already,
  over slot extents, so it is reused along the bar's length. Where zones meet,
  the existing rule applies: modules nearest the centre go compact first, then
  are dropped.
- **Module boxes span the bar's width**, less the theme's margins. Each box is
  one line tall: the font's line height plus the theme's vertical padding, the
  same box a horizontal bar draws, but stacked.
- **Text stays horizontal.** waybar's `rotate` is declined (backlog). A label
  wider than its box shows its compact, icon-only form. If even that does not
  fit, it is cut with an ellipsis, measured in pixels.
- **Button rows become columns.** The workspaces and the tray stack one button
  per row, `gap` apart, each centred across the bar.
- **A separator becomes a horizontal rule**, and a `{ spacer = N }` becomes a
  vertical gap.
- **Hit-testing needs no change:** regions are rectangles. Wheel steps work as
  on a horizontal bar.

This lands in `lyr_core::draw::build`, which takes the bar's orientation. The
horizontal path's output must not change: the existing draw and reference-image
tests guard it.

## Popups, menus and panels beside a side bar

Each one opens away from the bar's edge, beside the module it belongs to:

- **Tooltips:** `PopupPlacement::Right` from a left bar, `Left` from a right
  one.
- **Panels** (the clock panel and the Wi-Fi, Bluetooth and audio menus) are
  anchored to the bar's edge:
  - one `GAP` past the bar;
  - their top level with the module's box, moved up as far as it takes to stay
    on the output;
  - their room is the output's width less the bar's.

  `menu_panel::Place` gains the bar's edge on both axes, in place of a left
  offset and a fixed `Top`/`Bottom` edge.

- **Popups** (the power menu, list popups and tray menus) are anchored to the
  module's box stretched `GAP` past the bar's edge, as
  `menu_panel::popup_anchor` does vertically today.
- **Toasts** stay at the top right of the output. A right bar's reserved space
  keeps them clear, since their panel reserves nothing.

## Slices

Each slice is pushed once its gate is green.

1. **Config and drawing** (this Windows session):
   - `Position::Left` / `Right` and `bar.width` in `lyr-config`;
   - the vertical layout in `lyr-core`;
   - anchoring in `lyr-shell::bars`;
   - the bar's size in `apps/lyr`.

   Every popup site handles the new edges in this slice, even if only roughly,
   since the compiler insists; slice 2 makes them right. Run on Windows as an
   AppBar on each edge.

2. **Popups beside a side bar** (this session): tooltips, panels and popups as
   above, with placement unit tests for all four edges. Run on Windows.

3. **Import, end to end, and other platforms** (the office-lyr session, after
   slice 1 lands):
   - `lyr import-waybar` maps `position: left/right` and `width`;
   - CI's sway e2e gains a side-bar case;
   - each pushed slice is checked on Linux (Hyprland, sway, niri) and on macOS
     where available.

4. **Close out:** `docs/plan/05-layout-and-config.md` and the README document
   the keys; the changelog entry is written alongside each slice; then this file
   is deleted and a release is cut.

## Found in slice 1, for slice 2

- **Labels lose their text at the default width.** On a side bar `width` 50
  wide, volume, mic and the clock have no compact form, so they show as a cut
  "…" (run on Windows, 2026-11-04). Give the modules with an icon a compact form
  (the icon alone), and decide whether a side bar's default `width` should be
  wider.
- **Zones meeting drop modules rather than compacting them.** Compacting saves
  no length on a side bar, so the modules nearest the centre are dropped, unlike
  the horizontal rule above.
- **A button wider than the bar** (a long workspace name) is not cut.
- **A panel's top margin is measured from the output's top.** crcbl places a
  panel that reserves nothing inside the work area, so a taskbar or bar at the
  top would push it down by its height. Check this when the popups are redone.
- **Popups, panels and tooltips from a side bar** open beside it in slice 1
  (`menu_panel::placement`, `Place::top`). They are checked by unit tests only
  and have not been opened on screen.
- **No `DrawList` snapshot** of a side bar yet
  (`apps/lyr/src/modules/snapshots.rs` has the horizontal ones).

## Not in this phase

- **Rotated text** (waybar's `rotate`): declined in the backlog.
- **Several bars on one output** (waybar's list of bar configs): one bar per
  output stays the rule ([03](03-surfaces-and-input.md#outputs)).

## Checks

- **Unit tests:**
  - `lyr-config`: `left` and `right` and `width` read; unknown positions are
    refused.
  - `lyr-core`: a side bar's layout (zones top to bottom, the centre centred,
    compact then drop where zones meet, buttons stacked, a separator as a rule).
  - `lyr-shell`: the anchor and size for each edge.
  - `apps/lyr`: popup and panel placement for each edge.

  Each one must be shown to fail with what it guards removed.

- **Real runs:**
  - Windows, here: an AppBar on each edge, with maximised windows stopping at
    it.
  - Linux on Hyprland, sway and niri, through office-lyr (nested, never on the
    owner's desktop): the exclusive zone on each edge.
  - macOS: no machine runs it. CI only checks that it builds and starts, so the
    AppKit side stays a backlog gap.
  - Popups on each edge, on both platforms.
