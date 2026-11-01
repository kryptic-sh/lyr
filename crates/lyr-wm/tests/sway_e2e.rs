//! The sway backend against a running sway, which CI's `sway e2e` job starts
//! headless with two outputs. Ignored elsewhere: without `SWAYSOCK` it fails
//! rather than passes, so it runs only where a sway was started for it.
#![cfg(unix)]

use std::process::Command;
use std::time::Duration;

use lyr_wm::sway::{Error, Events, Sway};
use lyr_wm::{State, Workspace};

/// How long sway gets to report a change before the test fails.
const CHANGE_TIMEOUT: Duration = Duration::from_secs(10);

/// The app id of the window the test opens with foot, which CI's job
/// installs for it.
const WINDOW_APP: &str = "lyr-e2e-window";

/// Runs `command` through `swaymsg`, which reads the same `SWAYSOCK`.
#[expect(
    clippy::disallowed_methods,
    reason = "a Unix test, where a child never opens a console window"
)]
fn swaymsg(command: &str) {
    let status = Command::new("swaymsg")
        .args(["-q", command])
        .status()
        .expect("swaymsg runs");
    assert!(status.success(), "swaymsg {command:?}: {status}");
}

/// The first state from `events`, within [`CHANGE_TIMEOUT`], that `done`
/// accepts.
async fn until(events: &mut Events, what: &str, done: impl Fn(&State) -> bool) -> State {
    tokio::time::timeout(CHANGE_TIMEOUT, async {
        loop {
            let state = events.next().await.expect("read").expect("still running");
            if done(&state) {
                return state;
            }
        }
    })
    .await
    .unwrap_or_else(|_| panic!("{what} in time"))
}

/// The name of the workspace `state` shows on the focused output.
fn active_name(state: &State) -> Option<&str> {
    state
        .workspaces()
        .find(|workspace| Some(workspace.id) == state.active())
        .map(|workspace| workspace.name.as_str())
}

fn named(name: &str) -> Workspace {
    Workspace {
        id: 0,
        name: name.to_owned(),
        monitor: None,
        urgent: false,
    }
}

#[tokio::test]
#[ignore = "needs a running sway; CI's sway e2e job starts one"]
async fn a_real_sway_switches_workspaces_focuses_an_app_and_logs_out() {
    let sway = Sway::from_env().expect("SWAYSOCK is set by the job");
    let (state, mut events) = sway.subscribe().await.expect("subscribed");

    let active = state
        .workspaces()
        .find(|workspace| Some(workspace.id) == state.active())
        .expect("a focused workspace");
    assert_eq!(active.name, "1", "the config ends on workspace 1");
    let outputs: Vec<_> = state
        .workspaces()
        .map(|workspace| workspace.monitor.clone().expect("on an output"))
        .collect();
    assert!(
        outputs.contains(&"HEADLESS-1".to_owned()) && outputs.contains(&"HEADLESS-2".to_owned()),
        "a workspace on each output: {outputs:?}"
    );

    // A name with a space and a `;`, which only the quoting keeps whole.
    sway.focus(&named("lyr e2e; one"))
        .await
        .expect("sway switched");
    let changed = tokio::time::timeout(CHANGE_TIMEOUT, events.next())
        .await
        .expect("a change in time")
        .expect("read")
        .expect("still running");
    let shown = changed
        .workspaces()
        .find(|workspace| Some(workspace.id) == changed.active())
        .expect("a focused workspace");
    assert_eq!(shown.name, "lyr e2e; one");

    assert!(matches!(
        sway.focus(&named("$nope")).await,
        Err(Error::Unquotable(_))
    ));

    // A window on workspace 2, then workspace 1 shown: focusing the window's
    // app has to switch back to 2.
    swaymsg(&format!("workspace 2; exec foot --app-id {WINDOW_APP}"));
    until(&mut events, "the window focused on 2", |state| {
        active_name(state) == Some("2")
            && state.active_window().map(|(app, _)| app) == Some(WINDOW_APP)
    })
    .await;
    sway.focus(&named("1")).await.expect("sway switched");
    until(&mut events, "workspace 1 shown", |state| {
        active_name(state) == Some("1")
    })
    .await;
    assert!(
        sway.focus_app(&[WINDOW_APP]).await.expect("sway focused"),
        "the window was found"
    );
    until(&mut events, "the window focused again", |state| {
        active_name(state) == Some("2")
            && state.active_window().map(|(app, _)| app) == Some(WINDOW_APP)
    })
    .await;
    assert!(
        !sway
            .focus_app(&["lyr-e2e-no-such-app"])
            .await
            .expect("sway answered"),
        "no window for an app that has none"
    );

    sway.exit()
        .await
        .expect("sway answered exit before quitting");
    let end = tokio::time::timeout(CHANGE_TIMEOUT, async {
        loop {
            match events.next().await {
                Ok(Some(_)) => {}
                ended => return ended,
            }
        }
    })
    .await
    .expect("the stream ended in time");
    assert!(matches!(end, Ok(None)), "{end:?}");
}
