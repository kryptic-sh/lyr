# 04 — Rendering and theme

## Pipeline

```
state + config + theme ──► lyr-core: layout ──► DrawList + hit regions
                                                     │
                                lyr-render ◄─────────┘
                                   │  translate to crcbl's UI draw commands
                                   ▼
                    crcbl UI pass (rects, rounded rects, glyphs, images)
                                   │
                                   ▼
                    crcbl-hal: Vulkan (Linux, Windows) · D3D12 (Windows fallback) · Metal (macOS)
```

- `lyr-core` builds lyr's **own** `DrawList` (`Rect`,
  `RoundedRect { radius, border }`, `Glyphs { run, colour }`, `Image { id }`,
  `Clip`). Tests can assert on it without a GPU. `lyr-render` translates it into
  crcbl's UI draw commands (`DrawCommand::RoundedRect`, `Glyphs`, `Image`, …),
  which have the same shape. Keeping lyr's own type means a crcbl API break
  touches one translation function, not the layout code.
- **One device, many surfaces.** A single crcbl-hal device is shared by every
  bar and popup, with one swapchain per window.
- **Backend choice:** crcbl's runtime selection. lyr prefers Vulkan on Linux and
  Windows, falls back to D3D12 on Windows, and uses Metal on macOS. There is no
  software fallback: a machine with no working GPU backend gets a clear error at
  start-up. pikr's research argued for a software renderer for start-up time. A
  bar starts once per session, so that matters less here.

## Repaint on change only

- No frame loop. A window repaints when its `DrawList` changed, when it is
  resized or rescaled, or when the compositor asks (exposure).
- A zone is rebuilt only when one of its modules' state changed, and a window is
  redrawn only when a zone on it changed.
- Wayland: a repaint waits for the surface's **frame callback**. Win32 and
  AppKit: repaint directly, at most once per event batch.
- **Present mode:** FIFO with frame-callback pacing, never a blocking present
  loop. With FIFO, `vkQueuePresentKHR` can block indefinitely on a Wayland
  output that is off (hypridle turns DPMS off with
  `hl.dsp.dpms({ action = "off" })`). lyr only presents after a frame callback,
  and the compositor does not send one for an output that is off. Whether
  crcbl's Wayland present path already works this way is unverified (backlog).

## Text

- **Font lookup:** crcbl's `crcbl-font` finds the configured family
  (`Hack Nerd Font Propo`) in the system fonts (`SystemFonts::find`, matched by
  CSS rules), and crcbl-ui's `system-fonts` feature loads it
  (`Ui::register_system_font`) into crcbl-ui's `Font` (skrifa parsing, crcbl's
  rasteriser and shelf-packed glyph atlas). The owner chose it over `fontdb` on
  2026-10-17, as shared crcbl code, the way S28 replaced the image crates. If
  the family is missing, lyr logs which family was asked for and falls back to a
  monospace system font. It never draws "tofu" squares without saying why.
- **Glyph fallback:** `SystemFonts::fallbacks` gives the system's fallback
  order, which lyr passes to `Ui::set_fallback_fonts`, so a character the bar
  font lacks (colour emoji in a notification, S22) comes from the next font that
  has it.
- **No shaping.** Bar text is short left-to-right labels plus Nerd Font
  private-use glyphs, and "Propo" means proportional-width icons that need no
  ligatures. crcbl-ui's unshaped, cmap-driven text is enough. Whether Hack Nerd
  Font Propo's private-use glyphs resolve through crcbl-ui's cmap lookup is
  **not yet tested** (backlog). M1 tests it first.
- Glyphs are rasterised at the output's scale. Each scale gets its own atlas
  pages, so a 1× and a 1.5× monitor both get crisp text.

## Theme

The defaults are the values in `dotfiles/.config/waybar/style.css`, so the day
lyr replaces waybar the bar looks the same.

| Element                    | Value                                                                                     |
| -------------------------- | ----------------------------------------------------------------------------------------- |
| Font                       | `Hack Nerd Font Propo`, 16 px, normal weight                                              |
| Bar background             | transparent                                                                               |
| Text                       | `#cdd6f4`                                                                                 |
| Module box                 | background `#1a1b26`, border 2 px `#2bc5e3`, radius 8 px; the whole box at opacity 0.8    |
| Module box spacing         | padding 0 12 px, margin-top 10 px, margin-bottom 2 px, margin-left 10 px, min-width 20 px |
| Gap between modules        | 0: waybar's config sets no `spacing`, so only the margins separate the boxes              |
| Last module (notification) | margin-right 10 px                                                                        |
| Workspace button           | padding 0 10 px; active or hovered text `#2bc5e3`                                         |
| Battery warning / critical | `#fc4c02` / `#e92d4d`                                                                     |
| Volume / mic               | `#89b4fa` / `#cba6f7`                                                                     |
| Tooltip                    | background `#1e1e2e`, text `#cdd6f4`                                                      |
| Calendar                   | see [10](10-popups-and-menus.md#clock-panel)                                              |
| Panels and toasts          | ported from `dotfiles/.config/swaync/style.css` in M6 (not read yet)                      |

The palette mixes Tokyo Night (`#1a1b26`) with Catppuccin Mocha (`#cdd6f4`,
`#1e1e2e`, `#89b4fa`, `#cba6f7`), exactly as the current CSS does. lyr copies it
as it is. Whether to normalise it is a question for the owner
([16](16-open-questions.md)).

The theme lives in the config file's `[theme]` table
([05](05-layout-and-config.md)). It is not CSS. crcbl-ui does have a CSS
stylesheet (cssparser + taffy), and using it for lyr's theme is an open
question.

## Icons

Module icons are Nerd Font glyphs from the configured font, exactly as in
waybar's formats. Tray icons are images: pixmaps from StatusNotifierItem, or
theme icons rasterised by crcbl's asset crate ([08](08-tray.md)), uploaded to an
atlas once per size.
