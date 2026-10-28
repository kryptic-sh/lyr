# Manual checks

What automation cannot see ([plan 13](plan/13-testing.md#3-manual-checklists)):
the real compositor, Explorer or WindowServer, real hardware and real
applications. Each check says what to do and what should happen. When one is
run, record the result and the date in its `docs/backlog.md` entry, and take the
check out of "Not yet verified" there.

## Linux (Hyprland, on the desktop)

- [ ] **Space reservation.** Maximise a window. It stops under the bar; after
      `lyr toggle` it takes the bar's space, and after another it gives it back.
- [ ] **Monitor hotplug.** Plug in a second monitor, then unplug it, a few
      times. A bar comes and goes with it, the other bar stays, and lyr keeps
      running.
- [ ] **Fractional scaling.** Set an output to 1.25 and then 1.5. The bar, a
      tooltip, the clock panel and a menu are sharp and whole at each.
- [ ] **Outputs turned off.** Let hypridle turn the outputs off for a minute.
      `lyr reload` answers at once meanwhile; when they come back the clock
      shows the current minute.
- [ ] **Tray applications.** With Steam, Teams, Discord and Spotify running:
      each shows its own icon (Steam's and Spotify's not yet seen), and a right
      click opens its menu; Teams' three-level menu opens and closes.
- [ ] **Battery.** Unplug and plug in the charger. The battery module's icon and
      format follow, and the tooltip's time to empty or full.
- [ ] **Wi-Fi.** From the Wi-Fi menu: join a network with its password, then
      Disconnect from it, then switch the radio off and on.
- [ ] **Bluetooth.** From the Bluetooth menu: pair a device (a confirmation and,
      with an older device, a PIN typed with the input method), connect,
      disconnect, rename it, then Forget it.
- [ ] **Audio.** In the audio menu: make another output the default, switch a
      device between two ports, and use a Bluetooth headset as the output. With
      PulseAudio instead of PipeWire, the volume and mic modules work.
- [ ] **Power menu.** Lock, Sleep, Hibernate, Reboot and Power off each do what
      they say, where running them is fine.
- [ ] **Notifications as the daemon.** With `notifications.replace = true`, real
      applications' notifications show as toasts and in the clock panel, with
      their images, actions and links.
- [ ] **Idle cost** ([plan 12](plan/12-performance-and-power.md)). Over five
      idle minutes, `lyr --stats` logs about three wake-ups and one present a
      minute, and `perf trace -s -p <pid>` agrees.
- [ ] **Time zone.** Change the system's zone. The clock shows the new zone's
      time within about a second.

## Linux (sway and niri)

- [ ] **sway on the desktop.** The bar reserves its space, the workspaces follow
      `swaymsg` switches and urgency, and the window title follows the focus.
- [ ] **niri on the desktop.** The same, and a click outside an open panel
      leaves it open while Escape closes it.

## Windows

- [ ] **Full screen.** Play a video full screen, or a game. The bar hides on
      that monitor and comes back after.
- [ ] **Battery.** On a laptop, unplug and plug in the charger; the module
      follows.
- [ ] **Wi-Fi.** On a machine with Wi-Fi: the menu lists networks, joins one,
      offers a new password after a wrong one, forgets one, and switches the
      radio (note whether it needs elevation).
- [ ] **Bluetooth.** With a working adapter: switch it on after lyr started,
      scan, pair (each pairing kind a device asks for), and Forget.
- [ ] **Audio.** The audio menu's Default button moves the default output; a
      Bluetooth headset is told apart as Bluetooth.
- [ ] **Power menu.** Lock, Sleep, Reboot and Power off each do what they say.

## macOS

- [ ] **Placement.** The bar sits in the menu-bar strip, and overlaps windows
      unless the menu bar auto-hides.
- [ ] **Battery and Low Power Mode.** The battery module follows the charger;
      turning Low Power Mode on and off changes the power profile module.
- [ ] **Network.** On Wi-Fi: the network's name (after granting Location),
      signal and power.
- [ ] **Workspaces.** With AeroSpace and its `on-focus-changed` running
      `lyr event workspace`: the module follows switches, and a click switches.
- [ ] **Bluetooth.** Connecting and disconnecting a device updates the module
      (IOBluetooth's notifications on crcbl's AppKit run loop).
- [ ] **Audio.** The volume and mic modules follow the system; the audio menu
      lists devices and makes one the default.
- [ ] **Power menu.** Lock asks for the password afterwards (a setting of the
      Mac's); Sleep, Restart, Shut down and Log out do what they say.
