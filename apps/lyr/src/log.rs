//! The bar's log (`docs/plan/02-architecture.md`): `lyr.log` in
//! `~/.local/state/lyr/logs/` (hjkl-xdg's state directory, the same on every
//! platform), and a copy on stderr for a bar started from a terminal.
//! `RUST_LOG` filters it, `info` by default.

use std::fs::{File, OpenOptions};
use std::io;
use std::path::PathBuf;
use std::sync::Mutex;

use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt;
use tracing_subscriber::layer::SubscriberExt as _;
use tracing_subscriber::util::SubscriberInitExt as _;

/// Opens the log file and starts logging to it and to stderr. Returns the
/// file's path.
///
/// # Errors
///
/// When the log directory or file cannot be created; logging to stderr is
/// still set up then.
pub fn init() -> io::Result<PathBuf> {
    let filter = || EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let stderr = fmt::layer().with_writer(io::stderr).with_target(false);
    match open() {
        Ok((path, file)) => {
            let file = fmt::layer()
                .with_writer(Mutex::new(file))
                .with_ansi(false)
                .with_target(false);
            tracing_subscriber::registry()
                .with(filter())
                .with(stderr)
                .with(file)
                .init();
            Ok(path)
        }
        Err(error) => {
            tracing_subscriber::registry()
                .with(filter())
                .with(stderr)
                .init();
            Err(error)
        }
    }
}

/// The log file, opened for appending.
fn open() -> io::Result<(PathBuf, File)> {
    let dir = hjkl_xdg::state_dir("lyr")
        .map_err(io::Error::other)?
        .join("logs");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("lyr.log");
    let file = OpenOptions::new().create(true).append(true).open(&path)?;
    Ok((path, file))
}
