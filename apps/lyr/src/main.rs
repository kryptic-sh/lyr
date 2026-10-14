//! `lyr` — a desktop status bar.
//!
//! Scaffold: the CLI surface is parsed, but no bar exists yet, so every
//! invocation reports that and exits non-zero. See `docs/plan/` for the design
//! and `docs/plan/15-milestones.md` for the build order.

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

/// Command-line interface: `lyr [-c config] [command]`.
#[derive(Debug, Parser)]
#[command(
    name = "lyr",
    version,
    about = "A desktop status bar for Linux (Wayland), Windows and macOS",
    after_help = "With no command, lyr starts the bar."
)]
struct Cli {
    /// Config file to use instead of ~/.config/lyr/config.toml.
    #[arg(short = 'c', long = "config", value_name = "FILE")]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Command>,
}

/// Commands sent to a running bar over its control socket.
#[derive(Debug, Subcommand, PartialEq, Eq)]
enum Command {
    /// Reload the config file.
    Reload,
    /// Hide or show the bars.
    Toggle,
    /// Stop the running bar.
    Quit,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let what = match cli.command {
        None => "the bar",
        Some(Command::Reload) => "reload",
        Some(Command::Toggle) => "toggle",
        Some(Command::Quit) => "quit",
    };
    eprintln!("lyr: {what}: not implemented yet (the bar lands in milestone M1)");
    ExitCode::FAILURE
}

#[cfg(test)]
mod cli_tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn no_command_starts_the_bar() {
        let cli = Cli::parse_from(["lyr", "-c", "/tmp/lyr.toml"]);
        assert_eq!(cli.config, Some(PathBuf::from("/tmp/lyr.toml")));
        assert_eq!(cli.command, None);
    }

    #[test]
    fn control_commands_parse() {
        let cli = Cli::parse_from(["lyr", "reload"]);
        assert_eq!(cli.command, Some(Command::Reload));
    }

    #[test]
    fn unknown_command_is_rejected() {
        let err = Cli::try_parse_from(["lyr", "frobnicate"]).unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::InvalidSubcommand);
    }
}
