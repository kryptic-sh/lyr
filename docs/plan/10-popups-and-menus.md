# 10 — Popups, panels and menus

Everything that opens out of the bar is lyr's own: tooltips, the clock panel
(calendar and notifications), the power menu, the Wi-Fi, Bluetooth and audio
menus, and tray menus. None of them hands off to another program. The owner
decided on 2026-10-16 that the calendar, the notification centre and the
network, Bluetooth and audio menus are native to lyr, so swaync, `.menu-wifi`,
bluetui and wiremix all drop out of the bar's setup
([05](05-layout-and-config.md#migration-from-waybar)).

These are a platform feature that the windowing layer has to provide on every
platform ([03](03-surfaces-and-input.md#window-kinds)), and so they are
milestones of their own ([15](15-milestones.md)).

## Two kinds of window

- **Popups** are anchored to a module's rectangle on a bar, and only ever open
  from the pointer: tooltips, the power menu and tray menus. A popup opens below
  the bar (above it for a bottom bar), is clamped to its output, and closes when
  the pointer leaves it (tooltips), when a click lands outside it, or on
  `Escape`.
- **Panels** can also open from a key binding through the control socket
  (`lyr panel clock`, `lyr panel wifi`, …), when there is no click to anchor to.
  The clock panel and the Wi-Fi, Bluetooth and audio menus are panels, and so
  are the notification toasts. A panel opens on the focused output, under its
  module when that module is on the bar, and takes the keyboard. On Wayland it
  cannot be an `xdg_popup`, because a popup's grab needs the serial of the input
  event that opened it, and a key binding sent over the control socket has none.
  Opening a panel by click or by binding closes any other panel or popup.

All of them share one renderer and theme, and draw the same `DrawList`
([04](04-rendering-and-theme.md)) as the bar.

How a panel closes on an outside click is not settled. It closes on `Escape`, on
a second click on its module, and on the same binding again. Closing when it
loses keyboard focus (S17) covers a click on another window, but not a click on
the empty desktop. M5 tries that first and records what Hyprland does.

## Tooltips

Every module has one except notifications, which waybar's config turns off
(`"tooltip": false`). The text comes from the module's tooltip format, and it
shows after the platform's hover delay. The theme matches waybar's `tooltip`
block: background `#1e1e2e`, text `#cdd6f4`.

The clock's tooltip is no longer the calendar, which moved into the clock panel.
It names the time zone and its offset (`Asia/Manila, UTC+08:00`).

## Clock panel

Opened by a click on the clock or on the bell, or by `lyr panel clock` (which
`mod+N` runs). A second click or the same binding closes it. It is laid out the
way GNOME's date menu is: the calendar on top, notifications below.

From the top:

1. **Calendar**, carried from waybar's clock tooltip:
   - Opens in **year** mode (`mode: "year"`): twelve months in 3 columns
     (`mode-mon-col: 3`). Right click switches between year and month mode
     (`on-click-right: "mode"`). Scrolling over it shifts it by one unit of the
     current mode (`on-scroll: 1`): a year in year mode, a month in month mode.
   - ISO week numbers in a right-hand column (`weeks-pos: right`), prefixed `W`
     (`<b>W{}</b>`).
   - Monospace and smaller than the bar's text, as waybar's `<tt><small>` makes
     it. The monospace family is the theme's `calendar_font`, defaulting to the
     bar font, which is proportional (Hack Nerd Font **Propo**), so the default
     needs checking against waybar's look in M5.
   - Colours come from the waybar calendar format: months `#ffead3`, days
     `#ecc6d9`, weeks `#99ffdd`, weekdays `#ffcc66`, today `#ff6699` underlined.
     All bold.
   - The first day of the week follows the system locale.
   - Whether the year grid fits swaync's 500 px width in that font is measured
     in M5. If it does not, the panel grows to the grid's width; the grid is not
     shrunk.
2. **Title** ("Notification Center") with **Clear All**.
3. **Do Not Disturb** switch.
4. **Media players** (MPRIS).
5. **Notifications**, the history.

Items 2 – 5 are swaync's widgets in swaync's order ([09](09-notifications.md)).
On Windows and macOS, where lyr is not the notification daemon, the panel is the
calendar alone.

## Power menu

Opened from the power button, closed by a second click on it. Entries, in the
order and with the labels `.menu-power` uses today: Lock, Sleep, Hibernate,
Shutdown, Logout, Reboot. Each entry is a configured command or a built-in
action.

| Action    | Linux (logind over D-Bus)          | Windows                       | macOS                                  |
| --------- | ---------------------------------- | ----------------------------- | -------------------------------------- |
| Lock      | `loginctl lock-session` (hyprlock) | `LockWorkStation`             | `pmset displaysleepnow` (lock on wake) |
| Log out   | compositor exit (`hl.dsp.exit()`)  | `ExitWindowsEx(EWX_LOGOFF)`   | System Events `log out` (AppleScript)  |
| Sleep     | what `systemctl sleep` does        | `SetSuspendState(FALSE, ...)` | `pmset sleepnow`                       |
| Hibernate | `Manager.Hibernate`                | `SetSuspendState(TRUE, ...)`  | not offered                            |
| Reboot    | `Manager.Reboot`                   | `ExitWindowsEx(EWX_REBOOT)`   | System Events `restart`                |
| Power off | `Manager.PowerOff`                 | `ExitWindowsEx(EWX_POWEROFF)` | System Events `shut down`              |

Windows reboot and power-off need the `SE_SHUTDOWN_NAME` privilege enabled on
the process token first.

`.menu-power` sleeps with `systemctl sleep`, not `systemctl suspend`. That verb
(systemd 256 and later) picks suspend, hibernate or suspend-then-hibernate from
`sleep.conf`, so lyr's Sleep must do the same rather than call
`Manager.Suspend`. The logind method behind it is confirmed against the
installed systemd's introspection when M5 starts, not taken from memory.

## Wi-Fi menu

Opened by a click on the network module or by `lyr panel wifi` (`mod+W`). It
replaces `.local/bin/.menu-wifi` and its pikr picker, and carries what that
script learned, through NetworkManager's D-Bus API rather than `nmcli`. The
script drives `nmcli`, which settles that the daemon is NetworkManager. The NM
method, property and reason-code names are read from the installed
NetworkManager's introspection when M7 starts.

- **A header switch** turns the Wi-Fi radio on and off. That is new: today the
  script only reports that the radio is off.
- **When the device cannot scan** (unmanaged, or unavailable while
  wpa_supplicant is down), the menu says so instead of showing an empty list.
- **Scanning:** NM's scan cache is used only when its last scan is under 30 s
  old, the cutoff `nmcli --rescan auto` uses. Otherwise the menu shows
  "Scanning…" until a fresh scan lands, and the wait is capped, because NM can
  accept a scan request and never run it. A rescan is requested on every open,
  so the next one is current.
- **One row per SSID,** strongest first: the strongest access point's signal,
  and the **most demanding** security among its access points, so a spoofed open
  twin cannot downgrade the prompt. Hidden networks have no SSID and are not
  listed. The network in use is marked with U+F00C.
- **Joining a known network** uses its saved profiles, matched by SSID rather
  than by profile name, and only Wi-Fi client profiles (a hotspot or ad-hoc
  profile would start a network, not join one).
- **Joining a new network** takes its security from the scan. WPA3-only access
  points use SAE. A transition-mode access point's open side joins as open. OWE
  needs no password. 802.1X (enterprise) and WEP networks are refused with a
  message, as the script refuses them.
- **Passwords** are typed into a masked field in the menu. WPA-PSK takes 8 – 63
  bytes or 64 hex digits, as NetworkManager counts them; SAE has no limit.
  Leading and trailing spaces are kept, which pikr could not do.
- **A failed password on a saved profile** may be a wrong password or an
  association timeout, which NM reports the same way. The profile is kept and
  the menu offers to replace the password. A **new** profile that fails on its
  secrets is deleted so the next attempt asks again, as is any failed open
  profile. A PSK or OWE profile that failed for another reason (a DHCP timeout)
  is kept.
- **Forget** removes a network's profiles, after a confirmation.
- **Errors** show in the menu while it is open, and as a notification once it is
  closed. SSIDs are escaped before they go into notification markup.
- **One action at a time:** while a connection is being made, the rows are
  disabled, so a second click cannot act on the same network.
- **A click within the double-click interval** of the menu opening is ignored,
  so a double click does not open and close it again. The script needed the same
  guard.

## Bluetooth menu

Opened by a click on the bluetooth module or by `lyr panel bluetooth` (`mod+U`).
It replaces bluetui in a floating terminal, over BlueZ's D-Bus API
(`org.bluez.Adapter1`, `Device1`, `Battery1`).

- The adapter's power, pairable and discoverable switches, and a scan toggle.
- Paired devices first, then new ones found by the scan, each with its battery
  where `Battery1` reports one.
- Connect, disconnect, pair, trust, forget and rename.
- Pairing registers an `org.bluez.Agent1` from lyr, so passkey confirmation and
  PIN entry happen in the menu.
- `Escape` closes it, as the dotfiles set bluetui's `esc_quit` to make every
  menu close the same way.

bluetui's feature set is read from its source when M7 starts. The list above is
the provisional scope until then.

## Audio menu

Opened by a right click on volume or mic, or by `lyr panel audio` (`mod+A`). It
replaces wiremix in a floating terminal, over the same PulseAudio protocol
connection as the volume module ([07](07-modules.md#audio-sources)).

- Tabs: output devices, input devices, playback streams, recording streams.
  Right click on volume opens on output devices, as `wiremix --tab output` does.
  Right click on mic opens on input devices, which is new.
- Each device row has a volume slider, mute, and "make default". Each stream row
  has a volume slider, mute, and the device it plays to or records from, which
  can be changed.
- A card's profile and port are chosen from its device row.
- `Escape` closes it, as the dotfiles' `wiremix.toml` binds it.

wiremix's views are read from its source when M7 starts. The list above is the
provisional scope until then.

## Windows and macOS

The native menus are planned for Windows and macOS too, wherever the platform
has a public API, and fall back to opening the matching Settings page where it
does not (`ms-settings:network`, `ms-settings:bluetooth`, `ms-settings:sound`;
on macOS the `x-apple.systempreferences:` URLs). The APIs below are from memory
and are checked when M9 and M10 start ([16](16-open-questions.md)):

| Menu      | Windows                                                                                                                           | macOS                                                                      |
| --------- | --------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------- |
| Wi-Fi     | Native Wifi (`WlanScan`, `WlanGetAvailableNetworkList`, `WlanConnect`)                                                            | CoreWLAN; SSIDs need Location Services                                     |
| Bluetooth | WinRT `Windows.Devices.Enumeration` pairing; connecting a paired device has no public API                                         | `IOBluetooth`                                                              |
| Audio     | WASAPI devices and per-app sessions (`IAudioSessionManager2`); setting the default device has no public API, so it opens Settings | CoreAudio devices and the default device; per-app volume has no public API |
