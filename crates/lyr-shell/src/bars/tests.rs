use crcbl_shell::{
    HeadlessShell, MonitorId, MonitorInfo, PanelAnchor, PanelLayer, PhysicalRect, Reserve, Shell,
    ShellError, ShellEvent, WindowId, WindowRole,
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

/// A `retire` hook that records the bars it is handed, each checked to be
/// still open then, as the bar's surface needs.
fn retire_into(
    retired: &mut Vec<WindowId>,
) -> impl FnMut(&mut dyn Shell, WindowId) -> Result<(), ShellError> + '_ {
    |shell, window| {
        assert!(
            shell.window_state(window).is_ok(),
            "{window:?} retired after its window closed"
        );
        retired.push(window);
        Ok(())
    }
}

/// Hands every queued shell event to `bars`, as the main loop does, and
/// returns the bars retired.
fn drain(shell: &mut HeadlessShell, bars: &mut Bars, config: &Bar) -> Vec<WindowId> {
    let mut events = Vec::new();
    shell.pump(&mut |event| events.push(event));
    let mut retired = Vec::new();
    for event in &events {
        bars.handle(shell, config, event, retire_into(&mut retired))
            .expect("handled");
    }
    retired
}

#[test]
fn a_reshape_moves_the_bars_in_place() {
    let mut shell = two_monitors();
    let mut bars = Bars::default();
    bars.sync(&mut shell, &bar(&[]), retire_into(&mut Vec::new()))
        .expect("sync");
    let before: Vec<_> = bars.windows().collect();
    let mut config = bar(&[]);
    config.position = Position::Bottom;
    config.layer = Layer::Overlay;
    bars.reshape(&mut shell, &config).expect("reshaped");
    assert_eq!(
        bars.windows().collect::<Vec<_>>(),
        before,
        "the same windows"
    );
    for (_, window) in bars.windows() {
        let panel = shell.panel(window).expect("live").expect("a panel");
        assert_eq!(
            panel.anchor,
            PanelAnchor::BOTTOM | PanelAnchor::LEFT | PanelAnchor::RIGHT
        );
        assert_eq!(panel.layer, PanelLayer::Overlay);
    }
}

#[test]
fn every_output_gets_one_bar() {
    let mut shell = two_monitors();
    let mut bars = Bars::default();
    assert!(
        bars.sync(&mut shell, &bar(&[]), retire_into(&mut Vec::new()))
            .expect("sync")
    );
    assert_eq!(monitors_with_bars(&bars), [1, 2]);
    assert!(
        !bars
            .sync(&mut shell, &bar(&[]), retire_into(&mut Vec::new()))
            .expect("sync"),
        "no change"
    );
    assert_eq!(monitors_with_bars(&bars), [1, 2]);
}

#[test]
fn named_outputs_limit_the_bars() {
    let mut shell = two_monitors();
    let mut bars = Bars::default();
    bars.sync(
        &mut shell,
        &bar(&["HDMI-A-1"]),
        retire_into(&mut Vec::new()),
    )
    .expect("sync");
    assert_eq!(monitors_with_bars(&bars), [2]);
}

#[test]
fn a_config_change_removes_bars_no_longer_wanted() {
    let mut shell = two_monitors();
    let mut bars = Bars::default();
    bars.sync(&mut shell, &bar(&[]), retire_into(&mut Vec::new()))
        .expect("sync");
    let dropped = bars
        .windows()
        .find(|(monitor, _)| monitor.0 == 1)
        .map(|(_, window)| window)
        .expect("a bar on DP-1");

    let mut retired = Vec::new();
    assert!(
        bars.sync(&mut shell, &bar(&["HDMI-A-1"]), retire_into(&mut retired))
            .expect("sync")
    );
    assert_eq!(monitors_with_bars(&bars), [2]);
    assert_eq!(retired, [dropped], "retired before it went");
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
    bars.sync(&mut shell, &config, retire_into(&mut Vec::new()))
        .expect("sync");
    drain(&mut shell, &mut bars, &config);

    let third = shell.plug_monitor(monitor(0, "DP-3"));
    drain(&mut shell, &mut bars, &config);
    assert_eq!(monitors_with_bars(&bars), [1, 2, third.0]);

    let unplugged = bars
        .windows()
        .find(|(monitor, _)| monitor.0 == 1)
        .map(|(_, window)| window)
        .expect("a bar on DP-1");
    shell.unplug_monitor(MonitorId(1)).expect("connected");
    assert_eq!(drain(&mut shell, &mut bars, &config), [unplugged]);
    assert_eq!(monitors_with_bars(&bars), [2, third.0]);
}

#[test]
fn a_bar_the_compositor_closes_comes_back_on_the_next_output_change() {
    let mut shell = two_monitors();
    let mut bars = Bars::default();
    let config = bar(&[]);
    bars.sync(&mut shell, &config, retire_into(&mut Vec::new()))
        .expect("sync");
    drain(&mut shell, &mut bars, &config);
    let (_, window) = bars.windows().next().expect("a bar");

    shell.request_close(window).expect("open");
    assert_eq!(drain(&mut shell, &mut bars, &config), [window]);
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
