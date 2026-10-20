//! The control socket (`docs/plan/02-architecture.md#control-socket`): how
//! `lyr quit`, `lyr panel wifi` and the rest reach the running bar, and how a
//! second `lyr` finds that one is running.
//!
//! One connection carries one request line and gets one reply line: `ok`, or
//! `error: …`.
//!
//! Where it lives, and who can reach it:
//! - Unix: `lyr.sock` in `$XDG_RUNTIME_DIR`, which the XDG spec makes private
//!   to the user, or else `$TMPDIR` (macOS gives each user their own). Neither
//!   set is an error rather than a fall back to a shared `/tmp`. The socket is
//!   made `0600` after it is bound, so only its owner can connect.
//! - Windows: the named pipe `\\.\pipe\lyr-<user>`. Windows' default security
//!   for a pipe gives other users read access only, which cannot send a
//!   request.

use std::io::{self, BufRead as _, BufReader, Write as _};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

use crcbl_shell::ShellWaker;
use interprocess::local_socket::traits::{ListenerExt as _, Stream as _};
use interprocess::local_socket::{GenericFilePath, ListenerOptions, Stream, ToFsName as _};
use lyr_core::control::Request;

/// The reply to a request the bar carried out.
pub const OK: &str = "ok";

/// How long a connection waits for the main loop to answer.
const REPLY_TIMEOUT: Duration = Duration::from_secs(5);

/// Where the socket is.
///
/// # Errors
///
/// On Unix, when neither `XDG_RUNTIME_DIR` nor `TMPDIR` is set.
pub fn path() -> io::Result<PathBuf> {
    #[cfg(unix)]
    {
        let dir = ["XDG_RUNTIME_DIR", "TMPDIR"]
            .iter()
            .filter_map(std::env::var_os)
            .find(|dir| !dir.is_empty())
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    "neither XDG_RUNTIME_DIR nor TMPDIR is set, so lyr has no private \
                     directory for its control socket",
                )
            })?;
        Ok(PathBuf::from(dir).join("lyr.sock"))
    }
    #[cfg(windows)]
    {
        // A backslash is the one character a pipe name cannot contain.
        let user = std::env::var("USERNAME")
            .unwrap_or_else(|_| "user".to_owned())
            .replace('\\', "_");
        Ok(PathBuf::from(format!(r"\\.\pipe\lyr-{user}")))
    }
}

/// A request from a client, waiting for the main loop's answer.
#[derive(Debug)]
pub struct Incoming {
    pub request: Request,
    reply: mpsc::Sender<String>,
}

impl Incoming {
    /// Answers the client. A client that already gave up is not an error.
    pub fn reply(self, text: impl Into<String>) {
        let _gone = self.reply.send(text.into());
    }
}

/// Why the socket could not be bound.
#[derive(Debug, thiserror::Error)]
pub enum ListenError {
    #[error("lyr is already running (its control socket {0} answers)")]
    AlreadyRunning(PathBuf),
    #[error("cannot open the control socket {path}: {source}")]
    Io { path: PathBuf, source: io::Error },
}

/// Binds the socket at `path` and serves it on a thread of its own, handing
/// each request to the returned receiver and waking the main loop through
/// `waker`.
///
/// # Errors
///
/// When another lyr already answers there, or the socket cannot be bound.
pub fn listen(
    path: PathBuf,
    waker: Option<ShellWaker>,
) -> Result<mpsc::Receiver<Incoming>, ListenError> {
    let io = |source| ListenError::Io {
        path: path.clone(),
        source,
    };
    let name = || path.clone().to_fs_name::<GenericFilePath>();
    let bind = || {
        ListenerOptions::new()
            .name(name()?)
            .reclaim_name(true)
            .create_sync()
    };
    // Asked first: a Windows pipe name takes a second listener without
    // complaint, so only a bar answering tells that one is running.
    if Stream::connect(name().map_err(io)?).is_ok() {
        return Err(ListenError::AlreadyRunning(path));
    }
    let listener = match bind() {
        // A socket file left by a lyr that died: nobody answered it above.
        #[cfg(unix)]
        Err(error) if error.kind() == io::ErrorKind::AddrInUse => {
            std::fs::remove_file(&path).map_err(io)?;
            bind().map_err(io)?
        }
        other => other.map_err(io)?,
    };
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).map_err(io)?;
    }
    let (sender, receiver) = mpsc::channel();
    std::thread::Builder::new()
        .name("lyr control".to_owned())
        .spawn(move || {
            for stream in listener.incoming().filter_map(Result::ok) {
                let sender = sender.clone();
                let waker = waker.clone();
                // A thread per connection, so a client that never sends its
                // line cannot hold up the next one.
                let _detached = std::thread::Builder::new()
                    .name("lyr control client".to_owned())
                    .spawn(move || serve(stream, &sender, waker.as_ref()));
            }
        })
        .map_err(io)?;
    Ok(receiver)
}

/// Reads one request from `stream`, hands it to the main loop and writes the
/// answer back.
fn serve(stream: Stream, requests: &mpsc::Sender<Incoming>, waker: Option<&ShellWaker>) {
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    if reader.read_line(&mut line).is_err() {
        return;
    }
    let answer = match line.trim_end_matches(['\r', '\n']).parse::<Request>() {
        Err(error) => format!("error: {error}"),
        Ok(request) => {
            let (reply, answer) = mpsc::channel();
            if requests.send(Incoming { request, reply }).is_err() {
                return;
            }
            if let Some(waker) = waker {
                waker.wake();
            }
            answer
                .recv_timeout(REPLY_TIMEOUT)
                .unwrap_or_else(|_| "error: the bar did not answer".to_owned())
        }
    };
    let _gone = writeln!(reader.get_mut(), "{answer}");
}

/// Sends `request` to the bar listening at `path` and returns its reply line.
///
/// # Errors
///
/// When no bar answers there, or the exchange fails.
pub fn send(path: PathBuf, request: Request) -> io::Result<String> {
    let mut stream = Stream::connect(path.to_fs_name::<GenericFilePath>()?)?;
    writeln!(stream, "{request}")?;
    let mut reply = String::new();
    BufReader::new(stream).read_line(&mut reply)?;
    Ok(reply.trim_end().to_owned())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use lyr_core::control::{Panel, Request};

    use super::{ListenError, OK, listen, send};

    /// A socket path no other test or lyr uses.
    fn unique(name: &str) -> PathBuf {
        let tag = format!("lyr-test-{name}-{}", std::process::id());
        #[cfg(unix)]
        {
            std::env::temp_dir().join(tag)
        }
        #[cfg(windows)]
        {
            PathBuf::from(format!(r"\\.\pipe\{tag}"))
        }
    }

    #[test]
    fn a_request_reaches_the_main_loop_and_its_answer_comes_back() {
        let path = unique("round-trip");
        let requests = listen(path.clone(), None).expect("listening");
        let main_loop = std::thread::spawn(move || {
            let incoming = requests.recv().expect("a request");
            assert_eq!(incoming.request, Request::Panel(Panel::Wifi));
            incoming.reply(OK);
        });
        assert_eq!(send(path, Request::Panel(Panel::Wifi)).expect("sent"), OK);
        main_loop.join().expect("the loop answered");
    }

    #[test]
    fn a_second_bar_finds_the_first() {
        let path = unique("single");
        let _first = listen(path.clone(), None).expect("listening");
        assert!(matches!(
            listen(path, None),
            Err(ListenError::AlreadyRunning(_))
        ));
    }

    #[cfg(unix)]
    #[test]
    fn a_socket_left_by_a_dead_bar_is_replaced() {
        let path = unique("stale");
        // A socket file nobody answers on, as a lyr that died leaves. Bound
        // as a datagram socket, never listening: a listener dropped here
        // could still be held open by a child another test forks before its
        // exec, and answer the liveness check.
        drop(std::os::unix::net::UnixDatagram::bind(&path).expect("bound"));
        assert!(path.exists());
        let requests = listen(path.clone(), None).expect("the stale socket replaced");
        let main_loop = std::thread::spawn(move || {
            requests.recv().expect("a request").reply(OK);
        });
        assert_eq!(send(path, Request::Quit).expect("sent"), OK);
        main_loop.join().expect("answered");
    }

    #[test]
    fn with_no_bar_sending_fails() {
        assert!(send(unique("nobody"), Request::Quit).is_err());
    }
}
