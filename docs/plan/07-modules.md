# 07 — Modules

Every module is a **source** (in `lyr-sources` or `lyr-wm`) that pushes state
changes as events, plus a **view** (in `lyr-core`) that turns that state into
glyphs, text and a tooltip. Sources never poll unless a row below says so and
names the interval. Defaults copy `dotfiles/.config/waybar/config.jsonc`:
formats, icons, click and scroll actions. Where waybar's behaviour comes from
its own defaults rather than the config, it was read in waybar 0.15.0's source.

Icons are Nerd Font glyphs, written here as code points (`U+F011`) because the
glyphs themselves are private-use characters that tools strip without a trace.

A module whose source cannot run on the current platform fails **loudly at
config load**: lyr logs `module '<name>' is not supported on <os>: <reason>` and
leaves it out. The embedded defaults list each platform's supported set, so a
fresh install never hits this. A user config that asks for an unsupported module
does.

Platform sources are summarised in [11-platforms.md](11-platforms.md). The tray
and notifications are big enough to get their own sections ([08](08-tray.md),
[09](09-notifications.md)).

Where waybar ran another program from a click (swaync, `krypt menu wifi`,
`krypt menu bluetooth`, `krypt menu audio`), lyr opens its own panel instead
([10](10-popups-and-menus.md)). That is the owner's decision of 2026-10-16.

## Left zone

### Power button

- Glyph U+F011 (waybar `custom/power`), tooltip "Power Button".
- Click opens the power menu ([10](10-popups-and-menus.md#power-menu)); a second
  click closes it, as `krypt menu power` does today.

### Battery

Shown in the current waybar between the power button and the power profile, so
it is in the default layout there. On a machine with no battery the module hides
itself; that is not an error.

- Icon from the 5-step ramp U+F244, U+F243, U+F242, U+F241, U+F240 (empty to
  full) by capacity. Charging shows `{icon} U+F140B`. Plugged in below 100 %
  (for example held at a charge threshold) shows U+EE9E alone. At 100 % waybar
  reports the battery as full, and with no `format-full` in the config that
  shows the ramp's top icon.
- States `warning` at ≤ 30 % and `critical` at ≤ 10 % (waybar `states`), which
  colour the module ([04](04-rendering-and-theme.md#theme)).
- Tooltip `{capacity}% {power}W {timeTo}`.
- Linux: UPower over D-Bus (`org.freedesktop.UPower`, `PropertiesChanged`).
  Windows: `GetSystemPowerStatus` plus `RegisterPowerSettingNotification` for
  percentage and AC/DC changes. macOS: `IOPSNotificationCreateRunLoopSource`.
  The power draw in watts is read when the platform reports it (UPower
  `EnergyRate`, IOKit `AppleSmartBattery`), otherwise it is left out of the
  tooltip.

### Power profile

- Icons per waybar's `power-profiles-daemon`: performance and `default` U+F04C5,
  balanced U+F24E, power-saver U+F06C. The tooltip reads
  `Profile: {profile}\nDriver: {driver}`.
- Left click cycles to the next profile, any other button to the previous one.
  That is waybar's built-in click, which applies because the config sets no
  `on-click`.
- Linux: power-profiles-daemon over D-Bus
  (`org.freedesktop.UPower.PowerProfiles`, with the older
  `net.hadess.PowerProfiles` name as a fallback). Profile names are
  `performance` / `balanced` / `power-saver`.
- Windows: the power-mode slider.
  `PowerRegisterForEffectivePowerModeNotifications` is documented and read-only.
  **Setting** the mode needs the undocumented `PowerSetActiveOverlayScheme`.
  Provisional default: read through the documented call and set through the
  undocumented one, and say so in the tooltip if setting fails.
- macOS: Low Power Mode. It is readable (`NSProcessInfo.isLowPowerModeEnabled`
  and its change notification), but setting it needs `pmset` as root. Read-only
  on macOS. A click shows a tooltip saying so.

### Network

- Icons: wired U+EF09, wifi U+F1EB, disconnected U+F015B. Waybar's key for the
  first is `default`, which covers every connected state that is not WiFi.
- Tooltips: wired `{ifname} {ipaddr}/{cidr}`, wifi
  `{ifname} {ipaddr}/{cidr} {essid} ({signalStrength}%)`, disconnected
  `YOU IN THE WOODS?`. These are the current strings.
- Click opens the Wi-Fi menu ([10](10-popups-and-menus.md#wi-fi-menu)), where
  waybar ran `krypt menu wifi`.
- Linux: link and address changes come from rtnetlink (pushed). **WiFi signal
  strength is not pushed by rtnetlink.** NetworkManager's D-Bus API pushes
  `Strength` changes. The dotfiles' `.menu-wifi` drives `nmcli`, so the daemon
  is NetworkManager, which the Wi-Fi menu needs anyway. On a machine without it,
  lyr falls back to an nl80211 read **every 30 s** while connected to WiFi (the
  only interval in the plan), and the Wi-Fi menu is left out.
- Windows: `NotifyIpInterfaceChange` for links and addresses,
  `WlanRegisterNotification` (signal-quality changes) for WiFi.
- macOS: `SCDynamicStore` notifications for links and addresses, CoreWLAN for
  WiFi. Since macOS 14 the SSID needs **Location Services** permission. Without
  it, the tooltip shows the interface and signal and leaves the name out.

### Bluetooth

- Icons: on U+F00AF, off or disabled U+F00B2. When a device is connected and
  reports its battery: `{icon} {device_alias} {device_battery_percentage}%`.
- With no Bluetooth controller, waybar still shows the on icon. lyr hides the
  module instead, as battery does on a machine with no battery.
- Tooltip `Bluetooth {status}`.
- Click opens the Bluetooth menu ([10](10-popups-and-menus.md#bluetooth-menu)),
  where waybar ran `krypt menu bluetooth`.
- Linux: BlueZ over D-Bus (`org.bluez.Adapter1` `Powered`, `org.bluez.Device1`
  `Connected`, `org.bluez.Battery1` `Percentage`).
- Windows: radio state through WinRT `Windows.Devices.Radios`, connected devices
  through `Windows.Devices.Enumeration` watchers. Device battery is **not**
  exposed by a public API for classic Bluetooth, so it is left out.
- macOS: `IOBluetooth` power state and connection notifications. Device battery
  is likewise left out.

## Centre zone

Workspaces, see [06-workspaces.md](06-workspaces.md).

## Right zone

### Tray

See [08-tray.md](08-tray.md).

### Volume

- Format `{icon} {volume}%`. When muted, `U+EEE8 {volume}%`. Icons by output
  port: headphone, hands-free and headset U+F025; phone and portable U+F095; car
  U+F1B9; otherwise the ramp U+F026, U+F027, U+F028.
- Click toggles output mute. Scroll up raises and scroll down lowers the volume
  by **5 %**, with no ceiling at 100 %, as `pactl set-sink-volume ... +5%` has
  none. Right-click opens the audio menu on its output devices
  ([10](10-popups-and-menus.md#audio-menu)), where waybar ran
  `krypt menu audio`.
- Tooltip: the default sink's description, which is waybar's default when no
  `tooltip-format` is set.
- Colour `#89b4fa` (waybar `#pulseaudio`).

### Mic

- Format `U+F130 {volume}%`, muted `U+F131 {volume}%`. The same click and scroll
  as volume, applied to the default source. Right-click opens the audio menu on
  its input devices; waybar opened the same output tab from both.
- Tooltip: the default source's description. Waybar shows the sink's here too,
  because both modules are its one `pulseaudio` module; that is not carried.
- Colour `#cba6f7` (waybar `#pulseaudio.microphone`).

### Mute linkage

The draft plan said "muting output also mutes the mic". The current waybar does
**not** do that: its click is a plain
`pactl set-sink-mute @DEFAULT_SINK@ toggle`. The linkage would be new behaviour,
and what unmute should restore was never specified. Provisional default
([16](16-open-questions.md)): an opt-in `audio.mute_links_mic = false`. When it
is on, muting output mutes the mic and **remembers** whether the mic was already
muted, and unmuting output restores that state.

### Audio sources

- Linux: the **PulseAudio protocol** (`libpulse` bindings), which PipeWire
  serves through `pipewire-pulse`. It pushes sink, source and server changes
  through its subscribe API, and follows `@DEFAULT_SINK@` / `@DEFAULT_SOURCE@`
  the way `pactl` does. Native PipeWire would need WirePlumber metadata to find
  the default node, which is more code for no visible gain. See
  [16](16-open-questions.md).
- Windows: WASAPI `IAudioEndpointVolume` with an `IAudioEndpointVolumeCallback`
  for changes, and `IMMNotificationClient` for default-device changes.
- macOS: CoreAudio `AudioObjectAddPropertyListener` on the default output and
  input devices' volume and mute, plus the default-device properties.

### Clock

- Format `{:%Y/%m/%d %H:%M}`, exactly the current one. The finest unit is a
  **minute**, so the timer fires once a minute, aligned to the minute boundary.
  The draft plan's 1 Hz timer was 59 wasted wakeups a minute. A format with `%S`
  switches it to 1 Hz. The timer interval is derived from the format, not
  configured separately.
- Click opens the clock panel ([10](10-popups-and-menus.md#clock-panel)): the
  calendar on top and the notification centre below. Waybar showed the calendar
  as the clock's tooltip and used clicks and scroll to change its mode and
  range. Those now act on the calendar inside the panel.
- Tooltip: the time zone and its offset.
- Time zone: the system's. On Linux it is re-read on a `timedate1`
  `PropertiesChanged` signal, so `krypt menu time` changing the zone takes
  effect immediately.

### Notifications

See [09-notifications.md](09-notifications.md).

## Not carried over

The waybar config also defines `cpu`, `memory` and `disk`. None of them is in
any `modules-*` list, so they never render. They are not planned.
