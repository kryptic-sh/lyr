//! Running a binding's `exec` command through the platform shell
//! (`docs/plan/05-layout-and-config.md`). A config file is code: the command
//! runs as the user (SECURITY.md).

use std::io;
use std::process::{Command, Stdio};

/// Starts `command` and returns without waiting for it. A thread waits for it
/// instead, so a finished command does not linger as a zombie.
///
/// # Errors
///
/// When the shell cannot be started.
pub fn spawn(command: &str) -> io::Result<()> {
    let mut child = shell(command)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    std::thread::Builder::new()
        .name("lyr exec".to_owned())
        .spawn(move || {
            let _status = child.wait();
        })?;
    Ok(())
}

/// `sh -c` on Unix.
#[cfg(unix)]
fn shell(command: &str) -> Command {
    let mut shell = Command::new("sh");
    shell.arg("-c").arg(command);
    shell
}

/// `cmd /C` on Windows, with no console window. The command goes to `cmd`
/// as written: `cmd` parses its own command line, so the usual argument
/// quoting would change what it runs.
#[cfg(windows)]
fn shell(command: &str) -> Command {
    use std::os::windows::process::CommandExt as _;
    /// `CREATE_NO_WINDOW` from the Win32 process creation flags.
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let mut shell = Command::new("cmd");
    shell
        .arg("/C")
        .raw_arg(command)
        .creation_flags(CREATE_NO_WINDOW);
    shell
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::spawn;

    #[test]
    fn a_command_runs_through_the_shell() {
        let marker = std::env::temp_dir().join(format!("lyr-exec-{}", std::process::id()));
        let _ = std::fs::remove_file(&marker);
        let command = if cfg!(windows) {
            format!("echo ran > \"{}\"", marker.display())
        } else {
            format!("echo ran > '{}'", marker.display())
        };
        spawn(&command).expect("started");
        let deadline = Instant::now() + Duration::from_secs(10);
        while !marker.exists() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(50));
        }
        assert!(marker.exists(), "the command wrote its marker");
        let _ = std::fs::remove_file(&marker);
    }
}
