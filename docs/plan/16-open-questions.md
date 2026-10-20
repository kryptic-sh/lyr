# 16 — Open questions

Decisions the plan made provisionally that the owner may want to overrule. Each
has the default the implementation follows until told otherwise.

The draft plan's questions are answered here or in the section named: Q1 battery
(in, after the power button: [07](07-modules.md#battery)), Q2 and Q3 (the
platform layer and rendering are crcbl: [03](03-surfaces-and-input.md),
[04](04-rendering-and-theme.md)), Q5 (TOML: [05](05-layout-and-config.md)), Q11
(one bar per output, all-outputs workspaces:
[03](03-surfaces-and-input.md#outputs)), Q12 (the clock ticks per minute:
[07](07-modules.md#clock)), Q13 (targets: [12](12-performance-and-power.md)).

1. **Platform layer.** crcbl-shell + crcbl-hal, with the bar's window kinds
   added to crcbl ([03](03-surfaces-and-input.md#what-crcbl-must-provide)). The
   alternative, if crcbl declines: wayr on Linux plus native `windows-sys` and
   `objc2` windows elsewhere, rendered with wgpu.
2. **Mute linkage** (draft Q9). The draft said muting output also mutes the mic,
   but today's waybar does not do that. Default: off, behind
   `audio.mute_links_mic`. When it is on, unmuting output restores the mic's
   earlier state ([07](07-modules.md#mute-linkage)).
3. **Notifications** (draft Q6). **Decided by the owner (2026-10-16): lyr is the
   notification daemon** ([09](09-notifications.md)). Critical notifications
   still show during do-not-disturb, as swaync's do (read in its source).
   Provisional details: the history is not kept across restarts; when another
   daemon owns the name, lyr steps aside unless `notifications.replace = true`;
   MPRIS art is read from `file://` URLs only.
4. **Menus** (draft Q7). **Decided by the owner (2026-10-16): the calendar, the
   notification centre and the Wi-Fi, Bluetooth and audio menus are lyr's own,
   built before the switch from waybar** ([10](10-popups-and-menus.md)). The
   power menu was already built in. Provisional details: the Wi-Fi menu still
   refuses enterprise and WEP networks and does not list hidden ones, as
   `.menu-wifi` does; on Windows and macOS each menu is native where a public
   API exists and opens Settings where not
   ([10](10-popups-and-menus.md#windows-and-macos)); whether `wiremix` and
   `bluetui` stay installed as standalone tools is the dotfiles' call.
5. **Linux audio API** (draft Q8). Default: the PulseAudio protocol through
   `pipewire-pulse` with `libpulse-binding`, which is a new dependency. The
   alternative is native PipeWire with WirePlumber metadata
   ([07](07-modules.md#audio-sources)).
6. **Theme format** (draft Q10). Default: a `[theme]` TOML table. The
   alternative is crcbl-ui's CSS stylesheet, which would let `style.css` values
   be carried more directly. Icons stay Nerd Font glyphs.
7. **Theme palette.** The current CSS mixes Tokyo Night and Catppuccin colours.
   Default: copy it exactly. Say if it should become pure Tokyo Night (the rest
   of the desktop) ([04](04-rendering-and-theme.md#theme)).
8. **Compositors** (draft Q4). Answered 2026-10-20: Hyprland, sway and niri on
   Linux, each one backend in `lyr-wm` ([06](06-workspaces.md#linux--sway)).
   Other compositors leave the workspace module out.
9. **Windows workspaces.** Default: native virtual desktops through the
   undocumented COM interfaces, with a per-build table. The alternative is
   targeting komorebi or GlazeWM, if one of them gets used
   ([06](06-workspaces.md#windows--virtual-desktops)).
10. **macOS workspaces.** Default: AeroSpace only. Without it, the module is
    left out ([06](06-workspaces.md#macos--spaces)).
11. **macOS bar placement.** Default: in the menu-bar strip, with the menu bar
    set to auto-hide, because macOS cannot reserve space
    ([03](03-surfaces-and-input.md#macos--status-panel)).
12. **Windows taskbar.** Default: lyr stacks with it and never hides it. The
    tray stays there ([03](03-surfaces-and-input.md#windows--appbar)).
13. **Power profile setting on Windows.** Default: use the undocumented
    `PowerSetActiveOverlayScheme`. The alternative is read-only, like macOS
    ([07](07-modules.md#power-profile)).
14. **Linux network daemon.** Answered by the dotfiles: `.menu-wifi` drives
    `nmcli`, so it is NetworkManager, which the Wi-Fi menu needs too. The 30 s
    nl80211 read stays as the signal fallback on machines without it
    ([07](07-modules.md#network)).
15. **Window title module.** It is in the draft but not in today's bar. Default:
    not in MVP (backlog).
16. **New dependencies.** `libpulse-binding` and an rtnetlink crate are new to
    the org and need the owner's approval before M4
    ([02](02-architecture.md#dependencies-initial)). Images and icons are not on
    this list: the owner asked crcbl for a shared asset crate (S28) instead.
17. **Where lyr differs from waybar on purpose.** Everything else in the
    dotfiles' waybar setup is carried exactly (reviewed against `config.jsonc`
    and `style.css` on 2026-10-16). Defaults that differ: with no Bluetooth
    controller the module is hidden (waybar shows the on icon), and the mic's
    tooltip names the source (waybar names the sink there too). Say if either
    should match waybar instead ([07](07-modules.md#bluetooth),
    [07](07-modules.md#mic)). The clock panel changes more, by the owner's
    choice: see 18.
18. **The clock panel** ([10](10-popups-and-menus.md#clock-panel)). Decided by
    the owner (2026-10-16): the calendar and the notification centre share one
    panel, opened by a click on the clock or the bell. Provisional details: the
    calendar keeps waybar's year mode, three columns and colours, and the panel
    widens if the year grid does not fit 500 px; right click and scroll act on
    the calendar inside the panel rather than on the clock; the clock's tooltip
    names the time zone; mic's right click opens the audio menu on input devices
    rather than outputs.
19. **The calendar's first weekday and names** (backlog). waybar reads both from
    the system locale. The first weekday is read on every platform
    (`lyr_sources::locale`; on Linux from `locale -k LC_TIME`, no `libc`), and
    week numbers follow it as waybar's do (`%W` from Monday, `%U` otherwise).
    Default until the names are decided: the month and weekday names are
    English.
