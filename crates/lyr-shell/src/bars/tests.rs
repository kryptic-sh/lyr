use crcbl_shell::{
    HeadlessShell, MonitorId, MonitorInfo, PanelAnchor, PhysicalRect, Reserve, Shell, ShellEvent,
    WindowRole,
};
use lyr_config::{Bar, Layer, Position};

use super::{Bars, window_desc};

fn bar(output: &[&str]) -> Bar {
    Bar {
        position: Position::Top,
        height: 50,
        output: output.iter().map(|&name| name.to_owned()).collect(),
        layer: Layer::Top,
    }
}

fn monitor(id: u32, name: &str) -> MonitorInfo {
    let x = i32::try_from(id).expect("small id") * 1920;
    MonitorInfo {
        id: MonitorId(id),
        name: name.to_owned(),
        bounds: PhysicalRect::new(x, 0, 1920, 1080),
        work_area: PhysicalRect::new(x, 0, 1920, 1080),
        scale_factor: 1.0,
        refresh_millihertz: 60_000,
        is_primary: id == 1,
    }
}

fn two_monitors() -> HeadlessShell {
    HeadlessShell::new().with_monitors(vec![monitor(1, "DP-1"), monitor(2, "HDMI-A-1")])
}

fn monitors_with_bars(bars: &Bars) -> Vec<u32> {
    let mut ids: Vec<u32> = bars.windows().map(|(monitor, _)| monitor.0).collect();
    ids.sort_unstable();
    ids
}

/// Hands every queued shell event to `bars`, as the main loop does.
fn drain(shell: &mut HeadlessShell, bars: &mut Bars, config: &Bar) {
    let mut events = Vec::new();
    shell.pump(&mut |event| events.push(event));
    for event in &events {
        bars.handle(shell, config, event).expect("handled");
    }
}

#[test]
fn every_output_gets_one_bar() {
    let mut shell = two_monitors();
    let mut bars = Bars::default();
    assert!(bars.sync(&mut shell, &bar(&[])).expect("sync"));
    assert_eq!(monitors_with_bars(&bars), [1, 2]);
    assert!(
        !bars.sync(&mut shell, &bar(&[])).expect("sync"),
        "no change"
    );
    assert_eq!(monitors_with_bars(&bars), [1, 2]);
}

#[test]
fn named_outputs_limit_the_bars() {
    let mut shell = two_monitors();
    let mut bars = Bars::default();
    bars.sync(&mut shell, &bar(&["HDMI-A-1"])).expect("sync");
    assert_eq!(monitors_with_bars(&bars), [2]);
}

#[test]
fn a_config_change_removes_bars_no_longer_wanted() {
    let mut shell = two_monitors();
    let mut bars = Bars::default();
    bars.sync(&mut shell, &bar(&[])).expect("sync");
    let dropped = bars
        .windows()
        .find(|(monitor, _)| monitor.0 == 1)
        .map(|(_, window)| window)
        .expect("a bar on DP-1");

    assert!(bars.sync(&mut shell, &bar(&["HDMI-A-1"])).expect("sync"));
    assert_eq!(monitors_with_bars(&bars), [2]);
    assert!(
        shell.window_state(dropped).is_err(),
        "its window was destroyed"
    );
}

#[test]
fn a_plugged_output_gets_a_bar_and_an_unplugged_one_loses_it() {
    let mut shell = two_monitors();
    let mut bars = Bars::default();
    let config = bar(&[]);
    bars.sync(&mut shell, &config).expect("sync");
    drain(&mut shell, &mut bars, &config);

    let third = shell.plug_monitor(monitor(0, "DP-3"));
    drain(&mut shell, &mut bars, &config);
    assert_eq!(monitors_with_bars(&bars), [1, 2, third.0]);

    shell.unplug_monitor(MonitorId(1)).expect("connected");
    drain(&mut shell, &mut bars, &config);
    assert_eq!(monitors_with_bars(&bars), [2, third.0]);
}

#[test]
fn a_bar_the_compositor_closes_comes_back_on_the_next_output_change() {
    let mut shell = two_monitors();
    let mut bars = Bars::default();
    let config = bar(&[]);
    bars.sync(&mut shell, &config).expect("sync");
    drain(&mut shell, &mut bars, &config);
    let (_, window) = bars.windows().next().expect("a bar");

    shell.request_close(window).expect("open");
    drain(&mut shell, &mut bars, &config);
    assert!(!bars.contains(window));

    shell.inject(ShellEvent::MonitorsChanged);
    drain(&mut shell, &mut bars, &config);
    assert_eq!(monitors_with_bars(&bars), [1, 2]);
}

#[test]
fn a_bar_is_a_stretched_panel_that_reserves_its_height() {
    let mut config = bar(&[]);
    config.position = Position::Bottom;
    let desc = window_desc(&config, MonitorId(7));
    assert!((desc.size.height - 50.0).abs() < f64::EPSILON);
    assert!(desc.transparent);
    let WindowRole::Panel(panel) = desc.role else {
        panic!("a bar is a panel");
    };
    assert_eq!(panel.monitor, Some(MonitorId(7)));
    assert_eq!(
        panel.anchor,
        PanelAnchor::BOTTOM | PanelAnchor::LEFT | PanelAnchor::RIGHT
    );
    assert_eq!(panel.reserve, Reserve::OwnExtent);
    assert_eq!(panel.namespace, "lyr");
}
