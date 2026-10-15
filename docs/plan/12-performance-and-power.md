# 12 — Performance and power

A bar is on screen all day, so its idle cost is its real cost.

## Targets

| Measure                                | Target                                           | How it is measured                                                                                                                         |
| -------------------------------------- | ------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------ |
| Wakeups while nothing changes          | the clock's one per minute, and nothing else     | Linux: `perf trace -s -p <pid>` or `powertop` over 5 minutes; Windows: ETW / Process Monitor; macOS: `powermetrics --show-process-wakeups` |
| CPU while idle                         | 0 % at the precision `top` shows, over 5 minutes | the same run                                                                                                                               |
| GPU while idle                         | no submissions                                   | no presents logged by lyr between events (a debug counter, see below)                                                                      |
| Event → pixels (e.g. workspace switch) | presented in the next compositor frame           | timestamps logged at event receive and at present, at `debug` level                                                                        |
| Resident memory                        | measured in M2 and then held; not guessed now    | `/proc/<pid>/status` `VmRSS`, Task Manager, Activity Monitor                                                                               |

These are the targets the draft's open question (Q13) asked for. The memory
figure is left to be measured because GPU driver overhead dominates it and
differs per driver.

## How the design meets them

- **No polling.** Every source pushes ([07](07-modules.md)). The single
  exception is the WiFi signal fallback on Linux without NetworkManager, every
  30 s while connected to WiFi.
- **The clock ticks per minute**, aligned to the minute boundary, because its
  format has no seconds ([07](07-modules.md#clock)).
- **No frame loop.** A window repaints only when its `DrawList` changes, paced
  by frame callbacks ([04](04-rendering-and-theme.md#repaint-on-change-only)).
- **The main loop blocks** in crcbl-shell's `wait_events` with no timeout. The
  wake proxy (S6) is what makes that safe with tokio tasks producing events.
  This is exactly the defect in wayr's 50 ms cap, which wakes 20 times a second
  while idle.
- **tokio's timers are not a wakeup source while idle.** Sources wait on fds and
  channels. The clock is a single `sleep_until` per minute.
- **Coalescing:** events that arrive together are applied together, then each
  dirty window repaints once.
- **Panels cost nothing while closed.** A toast's timeout is a timer that exists
  only while the toast is shown, its 200 ms slide draws frames only during the
  slide, and the Wi-Fi menu scans only while it is open.

## Checking it

`lyr --stats` (debug builds) logs, once a minute, the number of main-loop
wakeups, events applied and presents per window. The idle test in M2's exit
check reads that line and requires one wakeup per minute (the clock) and no
presents apart from the clock's.
