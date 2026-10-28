//! The power menu's actions on macOS: each entry's command
//! ([`super::macos_command`]), run and waited for.

use std::io;
use std::process::Command;

use lyr_core::power_menu::Entry;

use super::macos_command;

/// Carries out the power menu's `entry`, waiting for its command.
///
/// # Errors
///
/// When the entry has no macOS command (Hibernate), the command cannot run,
/// or it fails, as `osascript` does when System Events may not be
/// controlled.
pub fn carry_out(entry: Entry) -> io::Result<()> {
    let [program, args @ ..] = macos_command(entry).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::Unsupported,
            format!("{} is not offered on macOS", entry.label()),
        )
    })?
    else {
        return Err(io::Error::other("an empty command"));
    };
    #[expect(
        clippy::disallowed_methods,
        reason = "macOS, where a child never opens a console window"
    )]
    let status = Command::new(program).args(args).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!("`{program}` failed: {status}")))
    }
}

#[cfg(test)]
mod tests {
    use lyr_core::power_menu::Entry;

    use super::carry_out;

    #[test]
    fn hibernate_is_refused_without_running_anything() {
        // Every other entry would lock, sleep or end the session, so only
        // the refusal is run.
        let error = carry_out(Entry::Hibernate).expect_err("not offered");
        assert_eq!(error.kind(), std::io::ErrorKind::Unsupported);
    }
}
