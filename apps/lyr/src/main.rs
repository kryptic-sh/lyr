//! `lyr` — a desktop status bar.
//!
//! With no command it runs the bar ([`bar::run`]); any other command is sent
//! to the running bar over its control socket ([`control`]). See `docs/plan/`
//! for the design and `docs/plan/15-milestones.md` for the build order.

mod bar;
mod control;
mod exec;
mod input;
mod log;
mod modules;
mod sources;
mod stats;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};
use lyr_core::control::{Panel, Request};

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

    /// Log the bar's wake-ups, events and presents once a minute.
    #[arg(long)]
    stats: bool,

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
    /// Open a panel on the focused output, or close it if it is open.
    Panel {
        #[arg(value_enum)]
        panel: PanelArg,
    },
    /// Turn do-not-disturb on or off.
    Dnd,
    /// Clear the notification history.
    Clear,
}

impl Command {
    /// The request this command sends to the running bar.
    const fn request(&self) -> Request {
        match self {
            Self::Reload => Request::Reload,
            Self::Toggle => Request::Toggle,
            Self::Quit => Request::Quit,
            Self::Panel { panel } => Request::Panel(panel.panel()),
            Self::Dnd => Request::Dnd,
            Self::Clear => Request::Clear,
        }
    }
}

/// The panels `lyr panel` takes, as command-line values.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
enum PanelArg {
    Clock,
    Wifi,
    Bluetooth,
    Audio,
}

impl PanelArg {
    const fn panel(self) -> Panel {
        match self {
            Self::Clock => Panel::Clock,
            Self::Wifi => Panel::Wifi,
            Self::Bluetooth => Panel::Bluetooth,
            Self::Audio => Panel::Audio,
        }
    }
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let Some(command) = cli.command else {
        if let Err(error) = log::init() {
            eprintln!("lyr: logging to stderr only: {error}");
        }
        return match bar::run(cli.config.as_deref(), cli.stats) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                tracing::error!("{error:#}");
                ExitCode::FAILURE
            }
        };
    };
    let request = command.request();
    let reply = control::path().and_then(|path| control::send(path, request));
    match reply {
        Ok(reply) if reply == control::OK => ExitCode::SUCCESS,
        Ok(reply) => {
            eprintln!("lyr: {request}: {reply}");
            ExitCode::FAILURE
        }
        Err(error) => {
            eprintln!("lyr: {request}: no running bar answered ({error})");
            ExitCode::FAILURE
        }
    }
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
        assert!(!cli.stats);
        assert!(Cli::parse_from(["lyr", "--stats"]).stats);
    }

    #[test]
    fn control_commands_parse() {
        let cli = Cli::parse_from(["lyr", "reload"]);
        assert_eq!(cli.command, Some(Command::Reload));
    }

    #[test]
    fn each_command_sends_its_request() {
        let request = |args: &[&str]| {
            Cli::parse_from(args)
                .command
                .map(|command| command.request().to_string())
        };
        assert_eq!(
            request(&["lyr", "panel", "wifi"]).as_deref(),
            Some("panel wifi")
        );
        assert_eq!(request(&["lyr", "dnd"]).as_deref(), Some("dnd"));
        assert_eq!(request(&["lyr", "clear"]).as_deref(), Some("clear"));
        assert_eq!(request(&["lyr", "quit"]).as_deref(), Some("quit"));
    }

    #[test]
    fn every_panel_can_be_named_on_the_command_line() {
        for panel in Panel::ALL {
            let cli = Cli::parse_from(["lyr", "panel", panel.name()]);
            assert_eq!(
                cli.command.map(|c| c.request()),
                Some(Request::Panel(panel))
            );
        }
        assert!(Cli::try_parse_from(["lyr", "panel", "tray"]).is_err());
    }

    #[test]
    fn unknown_command_is_rejected() {
        let err = Cli::try_parse_from(["lyr", "frobnicate"]).unwrap_err();
        assert_eq!(err.kind(), clap::error::ErrorKind::InvalidSubcommand);
    }
}
