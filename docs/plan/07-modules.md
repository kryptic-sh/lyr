# 07 — Modules

Every module is a **source** (in `lyr-sources` or `lyr-wm`) that pushes state
changes as events, plus a **view** (in `lyr-core`) that turns that state into
glyphs, text and a tooltip. Sources never poll unless a row below says so and
names the interval. Defaults copy `dotfiles/.config/waybar/config.jsonc`:
formats, icons, click and scroll actions.

A module whose source cannot run on the current platform fails **loudly at
config load**: lyr logs `module '<name>' is not supported on <os>: <reason>` and
leaves it out. The embedded defaults list each platform's supported set, so a
fresh install never hits this. A user config that asks for an unsupported module
does.

Platform sources are summarised in [11-platforms.md](11-platforms.md). The tray
and notifications are big enough to get their own sections ([08](08-tray.md),
[09](09-notifications.md)).

## Left zone

### Power button

- Glyph `` (waybar `custom/power`), tooltip "Power Button".
- Click opens the power menu ([10](10-popups-and-menus.md#power-menu)).

### Battery

Shown in the current waybar between the power button and the power profile, so
it is in the default layout there. On a machine with no battery the module hides
itself; that is not an error.

- Icon from the 5-step ramp `      ` by capacity. Charging adds `󱐋`. Plugged in
  and full shows ``.
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

- Icons per waybar's `power-profiles-daemon`: performance `󰓅`, balanced
  `, power-saver `. The tooltip reads `Profile: {profile}\nDriver: {driver}`.
- Click cycles to the next profile. That is new: waybar's module only cycles
  when its own default `on-click` is left in place, which the current config
  does.
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

- Icons: wired `, wifi `, disconnected `󰅛`.
- Tooltips: wired `{ifname} {ipaddr}/{cidr}`, wifi
  `{ifname} {ipaddr}/{cidr} {essid} ({signalStrength}%)`, disconnected
  `YOU IN THE WOODS?`. These are the current strings.
- Click runs the network action ([10](10-popups-and-menus.md#menus)).
- Linux: link and address changes come from rtnetlink (pushed). **WiFi signal
  strength is not pushed by rtnetlink.** NetworkManager's D-Bus API pushes
  `Strength` changes, and iwd pushes them through its signal-level agent. The
  dotfiles' wifi menu decides which daemon is in use; until that is checked, the
  plan assumes NetworkManager and falls back to an nl80211 read **every 30 s**
  while connected to WiFi (the only interval in the plan). See
  [16](16-open-questions.md).
- Windows: `NotifyIpInterfaceChange` for links and addresses,
  `WlanRegisterNotification` (signal-quality changes) for WiFi.
- macOS: `SCDynamicStore` notifications for links and addresses, CoreWLAN for
  WiFi. Since macOS 14 the SSID needs **Location Services** permission. Without
  it, the tooltip shows the interface and signal and leaves the name out.

### Bluetooth

- Icons: on `󰂯`, off or disabled `󰂲`. When a device is connected and reports its
  battery: `{icon} {device_alias} {device_battery_percentage}%`.
- Tooltip `Bluetooth {status}`.
- Click runs the bluetooth action ([10](10-popups-and-menus.md#menus)).
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

- Format `{icon} {volume}%`. When muted, ` {volume}%`. Icons by output port:
  headphone `, hands-free and headset `, phone `, portable `, car ``, otherwise
  the ramp `  `.
- Click toggles output mute. Scroll changes the volume by **5 %**. Right-click
  runs the audio action ([10](10-popups-and-menus.md#menus)).
- Colour `#89b4fa` (waybar `#pulseaudio`).

### Mic

- Format ` {volume}%`, muted ` {volume}%`. The same click, scroll and
  right-click as volume, applied to the default source.
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
- Click opens the calendar popup ([10](10-popups-and-menus.md#calendar)). That
  changes how it behaves today: waybar shows a year-mode **tooltip**,
  right-click switches between month and year mode, and scroll moves the range.
  lyr's popup keeps scroll for the range and adds `<` / `>` buttons for the
  month and the year.
- Time zone: the system's. On Linux it is re-read on a `timedate1`
  `PropertiesChanged` signal, so `krypt menu time` changing the zone takes
  effect immediately.

### Notifications

See [09-notifications.md](09-notifications.md).

## Not carried over

The waybar config also defines `cpu`, `memory` and `disk`. None of them is in
any `modules-*` list, so they never render. They are not planned.
