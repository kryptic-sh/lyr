//! The sway backend against a running sway, which CI's `sway e2e` job starts
//! headless with two outputs. Ignored elsewhere: without `SWAYSOCK` it fails
//! rather than passes, so it runs only where a sway was started for it.
#![cfg(unix)]

use std::time::Duration;

use lyr_wm::Workspace;
use lyr_wm::sway::{Error, Sway};

/// How long sway gets to report a change before the test fails.
const CHANGE_TIMEOUT: Duration = Duration::from_secs(10);

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
async fn a_real_sway_switches_workspaces_and_logs_out() {
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
