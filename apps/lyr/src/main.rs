//! `lyr` — a desktop status bar.
//!
//! With no command it runs the bar ([`bar::run`]); any other command is sent
//! to the running bar over its control socket ([`control`]). See `docs/plan/`
//! for the design and `docs/plan/15-milestones.md` for the build order.

mod audio_menu;
mod bar;
mod bluetooth_menu;
mod clock_panel;
mod control;
mod exec;
mod input;
mod list_popup;
mod log;
mod menu_panel;
mod modules;
mod power_menu;
#[cfg(any(windows, test))]
mod settings;
mod sources;
mod stats;
mod toasts;
mod tooltips;
mod tray_menu;
mod wifi_menu;

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

/// What `lyr` was asked to do.
#[derive(Debug, Subcommand, PartialEq, Eq)]
enum Command {
    #[command(flatten)]
    Bar(BarCommand),
    /// Print a waybar config.jsonc as a lyr config.toml. What lyr cannot
    /// carry is listed at the top; waybar's style.css is not read.
    ImportWaybar {
        /// The waybar config, such as ~/.config/waybar/config.jsonc.
        #[arg(value_name = "FILE")]
        config: PathBuf,
    },
}

/// Commands sent to a running bar over its control socket.
#[derive(Debug, Subcommand, PartialEq, Eq)]
enum BarCommand {
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

impl BarCommand {
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

/// `lyr import-waybar`: prints `config` converted, or says why it cannot.
fn import_waybar(config: &std::path::Path) -> ExitCode {
    let converted = std::fs::read_to_string(config)
        .map_err(|error| error.to_string())
        .and_then(|text| lyr_config::waybar::import(&text).map_err(|error| error.to_string()));
    match converted {
        Ok(imported) => {
            print!("{}", imported.toml);
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("lyr: import-waybar: {}: {error}", config.display());
            ExitCode::FAILURE
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
    let command = match command {
        Command::Bar(command) => command,
        Command::ImportWaybar { config } => return import_waybar(&config),
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
        assert_eq!(cli.command, Some(Command::Bar(BarCommand::Reload)));
        let cli = Cli::parse_from(["lyr", "import-waybar", "config.jsonc"]);
        assert_eq!(
            cli.command,
            Some(Command::ImportWaybar {
                config: PathBuf::from("config.jsonc")
            })
        );
    }

    /// The request a command line sends to the running bar, if it sends one.
    fn request_of(args: &[&str]) -> Option<Request> {
        match Cli::parse_from(args).command? {
            Command::Bar(command) => Some(command.request()),
            Command::ImportWaybar { .. } => None,
        }
    }

    #[test]
    fn each_command_sends_its_request() {
        let request = |args: &[&str]| request_of(args).map(|request| request.to_string());
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
            assert_eq!(
                request_of(&["lyr", "panel", panel.name()]),
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
