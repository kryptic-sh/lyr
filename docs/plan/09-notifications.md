# 09 — Notifications

On Linux, lyr **is the notification daemon**. It owns
`org.freedesktop.Notifications` on the session bus, draws the toasts, keeps the
history, and shows that history in the clock panel
([10](10-popups-and-menus.md#clock-panel)) under the calendar. swaync goes
(decided by the owner on 2026-10-16). Windows and macOS keep their own
notification centres, because neither lets another program replace them.

## What swaync does today

The baseline is `dotfiles/.config/swaync/config.json`, the waybar
`custom/notification` module, and the three Hyprland bindings in
`dotfiles/.config/hypr/hyprland.lua` (`mod+N` toggles the centre, `mod+shift+N`
toggles do-not-disturb, `mod+alt+N` clears everything, each through
`apps.notify`, which is `swaync-client`).

| swaync setting                                      | Value                                       | In lyr                                                                               |
| --------------------------------------------------- | ------------------------------------------- | ------------------------------------------------------------------------------------ |
| `positionX`, `positionY`                            | right, top                                  | toasts and the clock panel at the top right                                          |
| `control-center-margin-*`                           | 16 px on every side                         | the same margins                                                                     |
| `control-center-width`, `notification-window-width` | 500 px                                      | panel and toast width                                                                |
| `fit-to-screen`                                     | true                                        | the panel runs the output's height, less the margins                                 |
| `timeout`, `timeout-low`, `timeout-critical`        | 10 s, 5 s, never                            | the same, unless the app sends its own `expire_timeout`                              |
| `notification-body-image-width`, `-height`          | 200 × 100 px                                | the largest a body image is drawn                                                    |
| `image-visibility`                                  | `when-available`                            | an image is shown only when the notification has one                                 |
| `transition-time`                                   | 200 ms                                      | toasts slide in and out over 200 ms; frames are drawn only during it                 |
| `hide-on-clear`                                     | false                                       | Clear All leaves the panel open                                                      |
| `hide-on-action`                                    | true                                        | invoking an action closes the panel                                                  |
| `keyboard-shortcuts`                                | true                                        | the panel takes keys: `Escape` closes, arrows move, `Enter` acts, `Delete` dismisses |
| `widgets`                                           | `title`, `dnd`, `mpris`, `notifications`    | the same widgets, in that order, below the calendar                                  |
| `widget-config.title`                               | "Notification Center", a "Clear All" button | the same text                                                                        |
| `widget-config.dnd`                                 | "Do Not Disturb"                            | the same text, as a switch                                                           |
| `script-fail-notify`                                | true                                        | not carried: the config defines no scripts                                           |

The look comes from `dotfiles/.config/swaync/style.css` (349 lines, read on
2026-10-18). M6 ports these values into the theme the way
[04](04-rendering-and-theme.md) ported waybar's. The palette is Tokyo Night:
background `#1a1b26`, light background `#24283b`, text `#c0caf5`, comment
`#565f89`, red `#f7768e`, orange `#ff9e64`, teal `#73daca`, cyan `#7dcfff`, blue
`#7aa2f7`, hover `#2a2c3e`; every radius is 8 px, and every transition is 200
ms.

| Element                        | Values                                                                                                                                     |
| ------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------ |
| Text                           | 14 px                                                                                                                                      |
| Panel (`.control-center`)      | background `#1a1b26`, text `#c0caf5`, padding 16 px, border 2 px `#73daca`                                                                 |
| Toast and history entry        | background `#24283b`, text `#c0caf5`, padding 8 px, border 2 px `#73daca`; `#f7768e` when critical, `#7aa2f7` when low                     |
| Spacing                        | a toast has an 18 px margin; a history entry 4 px above and below; groups 8 px apart; content inset 7 px                                   |
| Summary, body, time            | `#c0caf5`, `#c0caf5`, `#565f89`                                                                                                            |
| Action button                  | background `#24283b` (hover `#2a2c3e`, pressed `#565f89`), border 2 px `#73daca`, padding 8 px, margin 4 px                                |
| Close button                   | background `#f7768e` (pressed `#ff9e64`), glyph `#1a1b26`, padding 2 px; in the panel 10 px from the top and 6 px from the right           |
| Title (`widget-title`)         | label `#7dcfff` at 2 rem; Clear All button background `#24283b` (hover `#2a2c3e`, pressed `#565f89`), border 2 px `#73daca`, padding 4 × 8 |
| Do Not Disturb (`widget-dnd`)  | label at 2 rem, 16 px above; switch `#24283b`, `#ff9e64` when on, `#f7768e` on hover, border 2 px `#73daca`; knob `#73daca` inset 4 px     |
| Media player (`widget-mpris`)  | background `#24283b`, border 2 px `#73daca`, 16 px above; art 100 px; title 1.2 rem `#c0caf5`, artist 1 rem `#565f89`; buttons 5 px apart  |
| Progress (a notification hint) | `#f7768e` when critical, `#7aa2f7` otherwise                                                                                               |
| Sliders (`scale`)              | trough `#24283b`, 8 px tall, at least 70 px wide; filled part `#c0caf5`; knob `#7aa2f7` with a 1 px `#565f89` border (`#7aa2f7` on hover)  |

Two of its values look like slips, and M6 asks the owner before copying them:

- `font-family: 'Hadk Nerd Font'` names no installed font, so swaync draws in a
  fallback today. lyr uses the bar's font (`Hack Nerd Font Propo`) unless told
  otherwise.
- Action buttons have `font-size: 0.2rem`, which is about 3 px.

The rules for widgets the config does not enable (`widget-menubar`,
`widget-buttons-grid`, `widget-volume`, `widget-backlight`, `widget-label`) are
not carried.

`.local/bin/.swaync-test` fires seven notifications that cover normal, low and
critical urgency, actions, an image, a long body, and an image with actions.
That set becomes M6's manual check, and each case gets an automated test.

## The daemon

The interface is the freedesktop Desktop Notifications Specification. The
details below are from memory of version 1.2 of that spec and are confirmed
against its text when M6 starts.

- **Methods:** `Notify` (with `replaces_id`, so a repeated notification updates
  in place), `CloseNotification`, `GetCapabilities` and `GetServerInformation`.
- **Signals:** `NotificationClosed` with its reason (expired, dismissed, closed
  by a call), `ActionInvoked`, and `ActivationToken` so the app that is raised
  can take focus under Wayland.
- **Capabilities:** `actions`, `body`, `body-markup`, `icon-static` and
  `persistence`. Each is advertised only once it works. `body-images` is not
  (see below).
- **Hints:** `urgency`, `category`, `desktop-entry`, `image-data`, `image-path`,
  `transient` (shown but not kept in the history) and `resident` (not closed by
  an action). Sound hints are ignored and not advertised.
- **Body markup:** the spec's subset (`<b>`, `<i>`, `<u>`, `<a>`). Anything else
  is shown as text, never interpreted. `.menu-wifi` escapes `&`, `<` and `>`
  because swaync renders markup, so lyr must too.
- **Images:** `image-data` is raw pixels. `image-path` and `app_icon` are a file
  path, a `file://` URL or an icon name looked up in the icon theme as the tray
  does ([08](08-tray.md)). Inline `<img>` in the body is not supported. An icon
  that cannot be found or decoded (crcbl's SVG subset refuses what it cannot
  draw) falls back to a generic icon. An image that fails is left out, and the
  notification is still shown.

### Toasts

- One toast per notification, newest on top, in a column at the top right of the
  **focused output** (Hyprland's, from [06](06-workspaces.md)).
- Clicking the body invokes the `default` action when the app sent one, then
  closes the toast. Action buttons invoke their action. The close button and a
  middle click dismiss.
- While the pointer is over a toast, its timeout is paused.
- With do-not-disturb on, toasts are not shown but still go to the history.
  Critical notifications still show (provisional, [16](16-open-questions.md)).

### History, do-not-disturb and Clear All

- The history lives in memory and is lost when lyr exits (provisional,
  [16](16-open-questions.md)). It is shown in the clock panel, newest first and
  grouped by app.
- Do-not-disturb and Clear All are in the panel (the `dnd` and `title` widgets)
  and on the control socket, so the bindings stay:
  - `mod+N` runs `lyr panel clock` (toggle the clock panel);
  - `mod+shift+N` runs `lyr dnd` (toggle do-not-disturb);
  - `mod+alt+N` runs `lyr clear` (clear all).

### Media players (the `mpris` widget)

Every `org.mpris.MediaPlayer2.*` name on the session bus gets a row with its
title, artist and art, and previous, play/pause and next buttons. State comes
from `PropertiesChanged` on `org.mpris.MediaPlayer2.Player`, so nothing is
polled. Art is read from `file://` URLs only. An `http(s)` art URL is not
fetched, because lyr has no HTTP client and is not getting one for this.

### When another daemon owns the name

swaync, mako or dunst may already own `org.freedesktop.Notifications`, for
example while the dotfiles still start swaync. lyr then logs the owner's name
and PID and leaves notifications out. It does not take the name over unless
`notifications.replace = true` is set. This keeps the switch from swaync to lyr
a dotfiles change rather than a race at start-up.

## The bell module

The waybar module shows one of three glyphs:

| State                               | Glyph  |
| ----------------------------------- | ------ |
| notifications in the history        | U+F071 |
| no notifications                    | U+F0A2 |
| no notifications, do-not-disturb on | U+F1F7 |

These are waybar's icons for swaync's eight states, which collapse to three. The
`inhibited-*` states come from swaync's inhibitors, which nothing in the
dotfiles uses, so lyr has no inhibitors and no inhibited state.

- No tooltip, as waybar's config has `"tooltip": false`.
- Left click toggles the clock panel. Right click toggles do-not-disturb.
- Waybar ran both clicks as `sleep 0.1 && swaync-client ...`, most likely so
  that swaync did not see the click that opened its panel as an outside click.
  lyr's panel and click are in one process, so the delay has no reason to exist.

## Windows

The notification centre belongs to the shell. Reading notifications needs WinRT
`UserNotificationListener`, which requires package identity and a user
permission prompt. There is no public API to toggle Do Not Disturb or Focus.
Provisional default: the module shows a static bell, and a click opens the
notification centre with the `ms-actioncenter:` URI. Count and DND state are
**not** shown on Windows.

## macOS

There are no public APIs to read notifications, open Notification Center or
toggle Focus. The module is left out on macOS.
