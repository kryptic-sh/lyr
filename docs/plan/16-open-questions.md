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
3. **Notifications** (draft Q6). Default: a swaync client. The alternative is an
   own `org.freedesktop.Notifications` daemon (toasts, history, DND, MPRIS),
   which is the size of the rest of lyr ([09](09-notifications.md)).
4. **Menus** (draft Q7). Default: power menu and calendar built in; network,
   bluetooth and audio as `exec` actions (`krypt menu …` on Linux, Settings
   pages on Windows and macOS). This changes the power button from
   `krypt menu power` to the built-in menu. Say if it should stay on krypt
   ([10](10-popups-and-menus.md#menus)).
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
8. **Compositors** (draft Q4). Default: Hyprland only on Linux, behind the
   `WorkspaceBackend` trait. sway (i3 IPC) and niri (JSON socket) are each one
   backend.
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
14. **Linux network daemon.** Default: NetworkManager for WiFi signal, with a 30
    s nl80211 read as the fallback. Confirm which daemon `krypt menu wifi`
    drives ([07](07-modules.md#network)).
15. **Window title module.** It is in the draft but not in today's bar. Default:
    not in MVP (backlog).
16. **New dependencies.** `libpulse-binding` and an rtnetlink crate are new to
    the org and need the owner's approval before M4
    ([02](02-architecture.md#dependencies-initial)).
