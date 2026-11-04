//! The runner against real scripts through the platform shell: `cmd /C` on
//! Windows, `sh -c` on Unix.

use std::path::Path;
use std::time::{Duration, Instant};

use lyr_config::{Config, Platform, from_sources};

use super::{Runner, Shown};

/// How long a test waits for a script before it fails.
const PATIENCE: Duration = Duration::from_secs(20);

/// A config showing `custom/test` with `body` in its table.
fn config(body: &str) -> Config {
    let text = format!("[layout]\nleft = [\"custom/test\"]\n[custom.test]\n{body}");
    from_sources(Some((Path::new("config.toml"), &text)), Platform::current()).expect("valid")
}

/// `windows` on Windows, else `unix`, as a TOML multi-line literal string,
/// which keeps quotes and backslashes as they are.
fn command(windows: &str, unix: &str) -> String {
    let command = if cfg!(windows) { windows } else { unix };
    assert!(!command.contains("'''"), "{command} cannot be written so");
    format!("'''{command}'''")
}

/// The next update, waiting for it.
fn next(runner: &Runner) -> Shown {
    let deadline = Instant::now() + PATIENCE;
    loop {
        if let Some(shown) = runner.recv() {
            return shown;
        }
        assert!(Instant::now() < deadline, "no update came");
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// The text of the next update, `None` when it leaves the module out.
fn next_text(runner: &Runner) -> Option<String> {
    next(runner).shown.map(|output| output.text)
}

#[test]
fn an_interval_script_runs_again_with_its_bar_s_output_name() {
    let exec = command(
        "echo %WAYBAR_OUTPUT_NAME% %LYR_OUTPUT_NAME%",
        "echo \"$WAYBAR_OUTPUT_NAME $LYR_OUTPUT_NAME\"",
    );
    let config = config(&format!("exec = {exec}\ninterval = 0.05\n"));
    let mut runner = Runner::new(None);
    runner.sync(&config, &[Some("DP-1")]);
    let first = next(&runner);
    assert_eq!((first.module, first.output.as_deref()), (0, Some("DP-1")));
    assert_eq!(
        first.shown.map(|output| output.text).as_deref(),
        Some("DP-1 DP-1")
    );
    assert_eq!(
        next_text(&runner).as_deref(),
        Some("DP-1 DP-1"),
        "and again"
    );
}

#[test]
fn a_json_script_s_object_is_read() {
    let exec = command(
        r#"echo {"text": "x", "class": ["warn"], "percentage": 50}"#,
        r#"echo '{"text": "x", "class": ["warn"], "percentage": 50}'"#,
    );
    let config = config(&format!(
        "exec = {exec}\nreturn_type = \"json\"\ninterval = \"once\"\n"
    ));
    let mut runner = Runner::new(None);
    runner.sync(&config, &[None]);
    let output = next(&runner).shown.expect("shown");
    assert_eq!(output.text, "x");
    assert_eq!(output.class, ["warn"]);
    assert_eq!(output.percentage, 50);
}

#[test]
fn a_continuous_script_s_lines_are_each_an_update() {
    let exec = command("echo one& echo two", "printf 'one\\ntwo\\n'");
    let config = config(&format!("exec = {exec}\n"));
    let mut runner = Runner::new(None);
    runner.sync(&config, &[None]);
    assert_eq!(next_text(&runner).as_deref(), Some("one"));
    assert_eq!(next_text(&runner).as_deref(), Some("two"));
}

#[test]
fn a_continuous_script_restarts_after_its_interval() {
    let exec = command("echo again", "echo again");
    let config = config(&format!("exec = {exec}\nrestart_interval = 0.05\n"));
    let mut runner = Runner::new(None);
    runner.sync(&config, &[None]);
    assert_eq!(next_text(&runner).as_deref(), Some("again"));
    assert_eq!(next_text(&runner).as_deref(), Some("again"), "run again");
}

#[test]
fn a_failing_exec_if_or_exec_leaves_the_module_out() {
    for body in [
        format!(
            "exec = {}\nexec_if = {}\ninterval = \"once\"\n",
            command("echo shown", "echo shown"),
            command("exit 1", "exit 1")
        ),
        format!(
            "exec = {}\ninterval = \"once\"\n",
            command("echo shown& exit 3", "echo shown; exit 3")
        ),
    ] {
        let mut runner = Runner::new(None);
        runner.sync(&config(&body), &[None]);
        assert_eq!(next_text(&runner), None, "{body}");
    }
    let passing = format!(
        "exec = {}\nexec_if = {}\ninterval = \"once\"\n",
        command("echo shown", "echo shown"),
        command("exit 0", "exit 0")
    );
    let mut runner = Runner::new(None);
    runner.sync(&config(&passing), &[None]);
    assert_eq!(next_text(&runner).as_deref(), Some("shown"));
}

#[test]
fn a_signal_and_a_click_run_a_one_off_script_again() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let count = dir.path().join("count");
    let path = count.display();
    // Each run adds a line to the file and prints how many there are.
    let exec = command(
        // By its path: a PATH with Git for Windows in it has GNU find first.
        &format!("echo x>>\"{path}\"& %SystemRoot%\\System32\\find.exe /c \"x\" < \"{path}\""),
        &format!("echo x >> '{path}'; wc -l < '{path}' | tr -d ' '"),
    );
    let config = config(&format!("exec = {exec}\nsignal = 8\n"));
    let mut runner = Runner::new(None);
    runner.sync(&config, &[None]);
    assert_eq!(
        next_text(&runner).as_deref(),
        Some("1"),
        "once at the start"
    );
    assert_eq!(runner.signal(9), 0, "no module has signal 9");
    assert_eq!(runner.signal(8), 1);
    assert_eq!(next_text(&runner).as_deref(), Some("2"));
    runner.run(0, None);
    assert_eq!(next_text(&runner).as_deref(), Some("3"));
    runner.run(0, Some("elsewhere"));
    std::thread::sleep(Duration::from_millis(500));
    assert!(runner.recv().is_none(), "no script runs on that output");
}

#[test]
fn a_script_runs_once_per_bar_and_stops_with_its_bar() {
    let exec = command("echo %LYR_OUTPUT_NAME%", "echo \"$LYR_OUTPUT_NAME\"");
    let config = config(&format!("exec = {exec}\ninterval = \"once\"\n"));
    let mut runner = Runner::new(None);
    runner.sync(&config, &[Some("A"), Some("B"), Some("A")]);
    let mut outputs = vec![next(&runner).output, next(&runner).output];
    outputs.sort();
    assert_eq!(outputs, [Some("A".to_owned()), Some("B".to_owned())]);
    assert_eq!(runner.scripts.len(), 2);
    runner.sync(&config, &[Some("B")]);
    assert_eq!(runner.scripts.len(), 1);
    assert_eq!(runner.scripts[0].output.as_deref(), Some("B"));
    runner.sync(
        &lyr_config::from_sources(None, Platform::current()).expect("defaults"),
        &[Some("B")],
    );
    assert!(runner.scripts.is_empty(), "no custom module shown");
}

#[test]
fn a_continuous_script_that_fails_leaves_the_module_out() {
    let exec = command("echo up& exit 3", "echo up; exit 3");
    let config = config(&format!("exec = {exec}\n"));
    let mut runner = Runner::new(None);
    runner.sync(&config, &[None]);
    assert_eq!(next_text(&runner).as_deref(), Some("up"));
    assert_eq!(next_text(&runner), None, "its exit status");
}

#[test]
fn stopping_a_script_stops_what_it_started() {
    // The shell starts a child that prints a line each second and outlives
    // the shell's own kill: only stopping the whole tree ends its output. On
    // Unix the loop runs in a subshell the shell waits for, since a shell
    // runs a lone command in its own process.
    let exec = command(
        "ping -n 60 127.0.0.1",
        "sh -c '(while :; do echo tick; sleep 1; done) & wait'",
    );
    let config = config(&format!("exec = {exec}\n"));
    let mut runner = Runner::new(None);
    runner.sync(&config, &[None]);
    next(&runner);
    runner.restart();
    // A line already on its way may still come.
    std::thread::sleep(Duration::from_millis(500));
    while runner.receiver.try_recv().is_ok() {}
    std::thread::sleep(Duration::from_secs(3));
    assert!(
        runner.receiver.try_recv().is_err(),
        "the script's child still prints"
    );
}

#[test]
fn an_update_from_before_a_reload_is_dropped() {
    let mut runner = Runner::new(None);
    let stale = |generation| super::Message {
        generation,
        shown: Shown {
            module: 0,
            output: None,
            shown: None,
        },
    };
    runner.sender.send(stale(0)).expect("sent");
    runner.restart();
    runner.sender.send(stale(0)).expect("sent");
    assert!(runner.recv().is_none(), "the old config's script");
    runner.sender.send(stale(1)).expect("sent");
    assert!(runner.recv().is_some(), "the new config's");
}
