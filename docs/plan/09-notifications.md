# 09 — Notifications

## What the current bar does

The waybar `custom/notification` module runs `swaync-client -swb` for its state
and maps it onto eight icons: notification, none, DND, inhibited, and the
combinations of those. Its clicks are:

| Input       | Command                | Meaning                           |
| ----------- | ---------------------- | --------------------------------- |
| left-click  | `swaync-client -t -sw` | toggle the notification **panel** |
| right-click | `swaync-client -d -sw` | toggle **do-not-disturb**         |

The draft plan had these backwards ("click toggles DND, right-click dismisses
all"), and contradicted itself with "click opens a drawer". The table above is
the source of truth. Hyprland binds the same three actions to `mod+N` (panel),
`mod+shift+N` (DND) and `mod+alt+N` (`-C`, clear all) in
`dotfiles/.config/hypr/hyprland.lua`.

swaync's config (`dotfiles/.config/swaync/config.json`) sets timeouts of 10 s
normal, 5 s low and never for critical, a 500 px panel at the top right with 16
px margins, and the widgets `title` (with Clear All), `dnd`, `mpris` and
`notifications`.

## Linux — a swaync client for MVP

Provisional default ([16](16-open-questions.md)): lyr is a **client of swaync**,
not a notification daemon.

- State: swaync's D-Bus control interface (`org.erikreider.swaync.cc`). The
  `Subscribe`/`SubscribeV2` signals push notification count, DND and inhibited
  state, which is what `swaync-client -swb` itself listens to. No subprocess.
- Clicks call the same interface methods that `-t` (`ToggleVisibility`) and `-d`
  (`ToggleDnd`) do. The exact method names are confirmed against the installed
  swaync's introspection XML when M4 starts. They are not re-derived from
  memory.
- When swaync is not running, the module shows the "none" icon greyed out and
  its tooltip says swaync is not running. It does not fail at start, because
  swaync may start after lyr does (they are both started from
  `hl.on("hyprland.start")`).

Becoming the `org.freedesktop.Notifications` daemon (as ashell does) would
replace swaync, but it means owning **toast popups** with their stacking,
timeouts, actions and images, the history panel, DND, inhibition and the MPRIS
widget. That is a project the size of the rest of lyr. It is recorded as a
post-MVP option in the backlog, not planned here.

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
