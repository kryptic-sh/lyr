//! The bar's log (`docs/plan/02-architecture.md`): `lyr.log` in
//! `~/.local/state/lyr/logs/` (hjkl-xdg's state directory, the same on every
//! platform), and a copy on stderr for a bar started from a terminal.
//! `RUST_LOG` filters it, `info` by default.
//!
//! A bar runs for months, so the file is capped: when a line would take it
//! past [`MAX_BYTES`] it becomes `lyr.log.1`, replacing the one before, and a
//! new `lyr.log` starts.

use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt;
use tracing_subscriber::layer::SubscriberExt as _;
use tracing_subscriber::util::SubscriberInitExt as _;

/// The most `lyr.log` holds before it is rotated.
const MAX_BYTES: u64 = 10 * 1024 * 1024;

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
fn open() -> io::Result<(PathBuf, Capped)> {
    let dir = hjkl_xdg::state_dir("lyr")
        .map_err(io::Error::other)?
        .join("logs");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("lyr.log");
    let file = Capped::open(path.clone(), MAX_BYTES)?;
    Ok((path, file))
}

/// A log file kept under `max` bytes by moving it to `<path>.1` when a write
/// would take it past.
#[derive(Debug)]
struct Capped {
    path: PathBuf,
    max: u64,
    file: File,
    /// The file's length.
    len: u64,
}

impl Capped {
    fn open(path: PathBuf, max: u64) -> io::Result<Self> {
        let file = append(&path)?;
        let len = file.metadata()?.len();
        Ok(Self {
            path,
            max,
            file,
            len,
        })
    }

    /// `<path>.1`, where the full file goes.
    fn previous(&self) -> PathBuf {
        let mut name = self.path.clone().into_os_string();
        name.push(".1");
        name.into()
    }

    fn rotate(&mut self) -> io::Result<()> {
        // Moved while still open, and over an existing `.1`; the tests do
        // both, on Windows too.
        std::fs::rename(&self.path, self.previous())?;
        self.file = append(&self.path)?;
        self.len = 0;
        Ok(())
    }
}

fn append(path: &Path) -> io::Result<File> {
    OpenOptions::new().create(true).append(true).open(path)
}

impl Write for Capped {
    /// Each call is one formatted event, so rotating before it keeps lines
    /// whole.
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.len > 0
            && self.len + buf.len() as u64 > self.max
            && let Err(error) = self.rotate()
        {
            // Better a log past its cap than no log; try again after
            // another cap's worth.
            eprintln!(
                "lyr: could not rotate {}: {error}; still appending",
                self.path.display()
            );
            self.len = 0;
        }
        let written = self.file.write(buf)?;
        self.len += written as u64;
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write as _;

    use super::Capped;

    #[test]
    fn a_full_log_moves_to_dot_one_and_a_new_one_starts() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("lyr.log");
        std::fs::write(&path, "old\n").expect("written");
        let mut log = Capped::open(path.clone(), 11).expect("opened");
        log.write_all(b"line 1\n").expect("written");
        assert_eq!(
            std::fs::read_to_string(&path).expect("read"),
            "old\nline 1\n",
            "fills the cap exactly with what was there"
        );
        log.write_all(b"line 2\n").expect("written");
        assert_eq!(std::fs::read_to_string(&path).expect("read"), "line 2\n");
        assert_eq!(
            std::fs::read_to_string(log.previous()).expect("read"),
            "old\nline 1\n"
        );
        log.write_all(b"line 3\n").expect("written");
        assert_eq!(std::fs::read_to_string(&path).expect("read"), "line 3\n");
        assert_eq!(
            std::fs::read_to_string(log.previous()).expect("read"),
            "line 2\n",
            "the one before is replaced"
        );
        log.write_all(b"4\n").expect("written");
        assert_eq!(
            std::fs::read_to_string(&path).expect("read"),
            "line 3\n4\n",
            "the new file is counted from empty"
        );
    }

    #[test]
    fn a_line_longer_than_the_cap_is_still_written() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("lyr.log");
        let mut log = Capped::open(path.clone(), 4).expect("opened");
        log.write_all(b"a long line\n").expect("written");
        assert_eq!(
            std::fs::read_to_string(&path).expect("read"),
            "a long line\n"
        );
        assert!(!log.previous().exists(), "an empty log is not rotated");
    }
}
