//! Talking to a running sway over its IPC socket.

use std::io;
use std::path::PathBuf;
use std::time::Duration;

use serde::Deserialize;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

use super::{
    EXIT_COMMAND, GET_TREE, GET_WORKSPACES, HEADER_LENGTH, Header, OUTPUT_EVENT, RUN_COMMAND,
    SHUTDOWN_EVENT, SUBSCRIBE, SUBSCRIPTION, Unquotable, WINDOW_EVENT, WORKSPACE_EVENT,
    focus_command, frame, state, window_event_moves_focus,
};
use crate::{State, Workspace};

/// How long a request may take before lyr gives up on it, as for Hyprland.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

/// What went wrong talking to sway.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("sway is not running here: SWAYSOCK is not set")]
    NotRunning,
    #[error("{what} {path}: {source}")]
    Io {
        what: &'static str,
        path: PathBuf,
        source: io::Error,
    },
    #[error("sway did not answer `{request}` within {REQUEST_TIMEOUT:?}")]
    Timeout { request: String },
    #[error("a message from sway on {path} does not start with `i3-ipc`")]
    Framing { path: PathBuf },
    #[error("sway's reply to `{request}` is not the JSON lyr expects: {source}")]
    Json {
        request: String,
        source: serde_json::Error,
    },
    #[error("sway refused `{request}`: {reply}")]
    Refused { request: String, reply: String },
    #[error(transparent)]
    Unquotable(#[from] Unquotable),
}

/// A connection point to one sway instance.
#[derive(Clone, Debug)]
pub struct Sway {
    socket: PathBuf,
}

impl Sway {
    /// Finds the sway this process runs under, from `SWAYSOCK`.
    ///
    /// # Errors
    ///
    /// [`Error::NotRunning`] when the variable is not set.
    pub fn from_env() -> Result<Self, Error> {
        let socket = std::env::var_os("SWAYSOCK").ok_or(Error::NotRunning)?;
        Ok(Self {
            socket: socket.into(),
        })
    }

    /// Subscribes to events and reads the current state.
    ///
    /// The subscription is made first and the state read after, so that no
    /// change between the two is lost: an event that arrives meanwhile only
    /// makes [`Events::next`] read a state it may already have.
    ///
    /// # Errors
    ///
    /// When sway cannot be reached, or refuses the subscription.
    pub async fn subscribe(&self) -> Result<(State, Events), Error> {
        let mut stream = self.connect().await?;
        let request = format!("subscribe {SUBSCRIPTION}");
        let reply = tokio::time::timeout(REQUEST_TIMEOUT, async {
            write(&mut stream, &frame(SUBSCRIBE, SUBSCRIPTION)).await?;
            read(&mut stream).await
        })
        .await
        .map_err(|_| Error::Timeout {
            request: request.clone(),
        })?
        .map_err(|error| self.error("subscribing over", error))?;
        let Some((_, reply)) = reply else {
            return Err(self.closed());
        };
        let success: Success = parse(&request, &reply)?;
        if !success.success {
            return Err(Error::Refused { request, reply });
        }
        let state = self.state().await?;
        let events = Events {
            stream,
            sway: self.clone(),
            last: state.clone(),
        };
        Ok((state, events))
    }

    /// Switches to `workspace`, by name.
    ///
    /// # Errors
    ///
    /// When sway cannot be reached or refuses, or when the name is one sway
    /// would read as something else ([`Unquotable`]), which is never sent.
    pub async fn focus(&self, workspace: &Workspace) -> Result<(), Error> {
        self.command(&focus_command(workspace)?).await
    }

    /// Ends the sway session: the power menu's Log out.
    ///
    /// # Errors
    ///
    /// When sway cannot be reached or refuses.
    pub async fn exit(&self) -> Result<(), Error> {
        self.command(EXIT_COMMAND).await
    }

    /// Reads the workspaces and the tree, and builds the state from them.
    async fn state(&self) -> Result<State, Error> {
        let workspaces = self.request(GET_WORKSPACES, "").await?;
        let tree = self.request(GET_TREE, "").await?;
        state(&workspaces, &tree).map_err(|source| Error::Json {
            request: "GET_WORKSPACES or GET_TREE".to_owned(),
            source,
        })
    }

    /// Runs one command and checks that sway carried it out.
    async fn command(&self, command: &str) -> Result<(), Error> {
        #[derive(Deserialize)]
        struct Outcome {
            success: bool,
        }
        let reply = self.request(RUN_COMMAND, command).await?;
        let outcomes: Vec<Outcome> = parse(command, &reply)?;
        if !outcomes.is_empty() && outcomes.iter().all(|outcome| outcome.success) {
            Ok(())
        } else {
            Err(Error::Refused {
                request: command.to_owned(),
                reply,
            })
        }
    }

    /// Sends one message on a connection of its own and returns the reply's
    /// payload.
    async fn request(&self, kind: u32, payload: &str) -> Result<String, Error> {
        let exchange = async {
            let mut stream = self.connect().await?;
            write(&mut stream, &frame(kind, payload))
                .await
                .map_err(|error| self.error("requesting over", error))?;
            read(&mut stream)
                .await
                .map_err(|error| self.error("requesting over", error))?
                .map(|(_, reply)| reply)
                .ok_or_else(|| self.closed())
        };
        tokio::time::timeout(REQUEST_TIMEOUT, exchange)
            .await
            .map_err(|_| Error::Timeout {
                request: format!("type {kind} {payload}"),
            })?
    }

    async fn connect(&self) -> Result<UnixStream, Error> {
        UnixStream::connect(&self.socket)
            .await
            .map_err(|source| Error::Io {
                what: "connecting to",
                path: self.socket.clone(),
                source,
            })
    }

    /// The error a failed read or write turns into: a framing error as
    /// itself, any other as I/O on this socket.
    fn error(&self, what: &'static str, error: ReadError) -> Error {
        match error {
            ReadError::Framing => Error::Framing {
                path: self.socket.clone(),
            },
            ReadError::Io(source) => Error::Io {
                what,
                path: self.socket.clone(),
                source,
            },
        }
    }

    /// The error for a socket sway closed before replying.
    fn closed(&self) -> Error {
        Error::Io {
            what: "reading a reply from",
            path: self.socket.clone(),
            source: io::ErrorKind::UnexpectedEof.into(),
        }
    }
}

/// `SUBSCRIBE`'s reply.
#[derive(Deserialize)]
struct Success {
    success: bool,
}

/// Parses `reply` to `request` as `T`.
fn parse<'a, T: Deserialize<'a>>(request: &str, reply: &'a str) -> Result<T, Error> {
    serde_json::from_str(reply).map_err(|source| Error::Json {
        request: request.to_owned(),
        source,
    })
}

/// Why a message could not be read.
enum ReadError {
    Framing,
    Io(io::Error),
}

impl From<io::Error> for ReadError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

async fn write(stream: &mut UnixStream, message: &[u8]) -> Result<(), ReadError> {
    Ok(stream.write_all(message).await?)
}

/// Reads one message: its type and payload. `None` when sway closed the
/// socket before a new message began.
async fn read(stream: &mut UnixStream) -> Result<Option<(u32, String)>, ReadError> {
    let mut header = [0; HEADER_LENGTH];
    match stream.read_exact(&mut header).await {
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(error) => return Err(error.into()),
    }
    let header = Header::parse(&header).ok_or(ReadError::Framing)?;
    let length = usize::try_from(header.length).map_err(|_| ReadError::Framing)?;
    let mut payload = vec![0; length];
    stream.read_exact(&mut payload).await?;
    let payload = String::from_utf8(payload)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    Ok(Some((header.kind, payload)))
}

/// The subscribed connection, read for changes.
#[derive(Debug)]
pub struct Events {
    stream: UnixStream,
    sway: Sway,
    /// The state last handed on, so an event that changes nothing the bar
    /// draws is not handed on again.
    last: State,
}

impl Events {
    /// The next state that differs from the last one handed on. `Ok(None)`
    /// means sway is shutting down or closed the socket.
    ///
    /// # Errors
    ///
    /// When the socket fails, a message is malformed, or re-reading the
    /// state fails.
    pub async fn next(&mut self) -> Result<Option<State>, Error> {
        loop {
            let message = read(&mut self.stream)
                .await
                .map_err(|error| self.sway.error("reading events from", error))?;
            let Some((kind, payload)) = message else {
                return Ok(None);
            };
            let reread = match kind {
                WORKSPACE_EVENT | OUTPUT_EVENT => true,
                WINDOW_EVENT => {
                    window_event_moves_focus(&payload).map_err(|source| Error::Json {
                        request: "a window event".to_owned(),
                        source,
                    })?
                }
                SHUTDOWN_EVENT => return Ok(None),
                _ => false,
            };
            if !reread {
                continue;
            }
            let state = self.sway.state().await?;
            if state != self.last {
                self.last = state.clone();
                return Ok(Some(state));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{UnixListener, UnixStream};

    use super::super::{
        GET_TREE, GET_WORKSPACES, HEADER_LENGTH, Header, RUN_COMMAND, SHUTDOWN_EVENT, SUBSCRIBE,
        Unquotable, WINDOW_EVENT, WORKSPACE_EVENT, frame,
    };
    use super::{Error, Sway};
    use crate::Workspace;

    /// Workspace 1 focused, before any event.
    const BEFORE: &str = r#"[
        {"id": 4, "name": "1", "num": 1, "output": "eDP-1", "focused": true, "urgent": false},
        {"id": 6, "name": "2", "num": 2, "output": "eDP-1", "focused": false, "urgent": false}
    ]"#;
    /// Workspace 2 focused, after one.
    const AFTER: &str = r#"[
        {"id": 4, "name": "1", "num": 1, "output": "eDP-1", "focused": false, "urgent": false},
        {"id": 6, "name": "2", "num": 2, "output": "eDP-1", "focused": true, "urgent": false}
    ]"#;
    const TREE: &str = r#"{"id": 1, "type": "root", "focused": false, "nodes": []}"#;

    async fn read_message(stream: &mut UnixStream) -> Option<(u32, String)> {
        let mut header = [0; HEADER_LENGTH];
        stream.read_exact(&mut header).await.ok()?;
        let header = Header::parse(&header).expect("framed");
        let mut payload = vec![0; header.length as usize];
        stream.read_exact(&mut payload).await.expect("payload");
        Some((header.kind, String::from_utf8(payload).expect("text")))
    }

    /// A sway on `path`: `GET_WORKSPACES` answers [`BEFORE`] the first time
    /// and [`AFTER`] from then on, a subscription is answered with
    /// `subscribed` as its `success` and, when it succeeds, followed by
    /// `events` on a connection kept open, and every command gets
    /// `command_reply`. Each message is handed out before it is answered.
    fn fake_sway(
        path: &Path,
        subscribed: bool,
        events: Vec<(u32, &'static str)>,
        command_reply: &'static str,
    ) -> tokio::sync::mpsc::UnboundedReceiver<(u32, String)> {
        let listener = UnixListener::bind(path).expect("bind");
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let workspace_reads = Arc::new(AtomicUsize::new(0));
        tokio::spawn(async move {
            loop {
                let (mut stream, _) = listener.accept().await.expect("accept");
                let tx = tx.clone();
                let events = events.clone();
                let workspace_reads = workspace_reads.clone();
                tokio::spawn(async move {
                    while let Some((kind, payload)) = read_message(&mut stream).await {
                        let reply = match kind {
                            GET_WORKSPACES => {
                                if workspace_reads.fetch_add(1, Ordering::SeqCst) == 0 {
                                    BEFORE
                                } else {
                                    AFTER
                                }
                            }
                            GET_TREE => TREE,
                            SUBSCRIBE if subscribed => r#"{"success": true}"#,
                            SUBSCRIBE => r#"{"success": false}"#,
                            RUN_COMMAND => command_reply,
                            _ => "{}",
                        };
                        tx.send((kind, payload)).expect("send");
                        stream.write_all(&frame(kind, reply)).await.expect("reply");
                        if kind == SUBSCRIBE && subscribed {
                            for (kind, event) in &events {
                                stream.write_all(&frame(*kind, event)).await.expect("event");
                            }
                            // Held open, so only a shutdown event ends
                            // the stream.
                            std::future::pending::<()>().await;
                        }
                    }
                });
            }
        });
        rx
    }

    fn at(path: &Path) -> Sway {
        Sway {
            socket: path.to_owned(),
        }
    }

    #[tokio::test]
    async fn events_hand_on_each_changed_state_until_shutdown() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("sway.sock");
        let mut requests = fake_sway(
            &path,
            true,
            vec![
                (WORKSPACE_EVENT, r#"{"change": "focus"}"#),
                (WINDOW_EVENT, r#"{"change": "move", "container": {}}"#),
                (WORKSPACE_EVENT, r#"{"change": "focus"}"#),
                (SHUTDOWN_EVENT, r#"{"change": "exit"}"#),
            ],
            "[]",
        );
        let (state, mut events) = at(&path).subscribe().await.expect("subscribed");
        assert_eq!(
            requests.recv().await,
            Some((
                SUBSCRIBE,
                r#"["workspace","window","output","shutdown"]"#.to_owned()
            ))
        );
        assert_eq!(state.active(), Some(4));

        let next = events.next().await.expect("read").expect("a change");
        assert_eq!(next.active(), Some(6));
        // The connection stays open, so only the shutdown event ends it.
        let end = tokio::time::timeout(Duration::from_secs(5), events.next()).await;
        assert!(
            matches!(end, Ok(Ok(None))),
            "the second focus changed nothing, then sway shut down"
        );
        // The first state and each focus event read the workspaces; the
        // move did not.
        let reads = std::iter::from_fn(|| requests.try_recv().ok())
            .filter(|(kind, _)| *kind == GET_WORKSPACES)
            .count();
        assert_eq!(reads, 3);
    }

    #[tokio::test]
    async fn focus_runs_the_command_and_checks_every_outcome() {
        let dir = tempfile::tempdir().expect("tempdir");
        let ok = dir.path().join("ok.sock");
        let mut requests = fake_sway(&ok, true, Vec::new(), r#"[{"success": true}]"#);
        let two = Workspace {
            id: 6,
            name: "2:web".into(),
            monitor: None,
            urgent: false,
        };
        at(&ok).focus(&two).await.expect("carried out");
        assert_eq!(
            requests.recv().await,
            Some((
                RUN_COMMAND,
                r#"workspace --no-auto-back-and-forth "2:web""#.to_owned()
            ))
        );

        let refused = dir.path().join("refused.sock");
        let _requests = fake_sway(
            &refused,
            true,
            Vec::new(),
            r#"[{"success": true}, {"success": false, "error": "No workspace to switch to"}]"#,
        );
        assert!(matches!(
            at(&refused).focus(&two).await,
            Err(Error::Refused { reply, .. }) if reply.contains("No workspace")
        ));
        assert!(matches!(
            at(&refused).exit().await,
            Err(Error::Refused { .. })
        ));

        let silent = dir.path().join("silent.sock");
        let _requests = fake_sway(&silent, true, Vec::new(), "[]");
        assert!(
            matches!(at(&silent).exit().await, Err(Error::Refused { .. })),
            "no outcome is not success"
        );
    }

    #[tokio::test]
    async fn a_name_sway_would_misread_is_never_sent() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("sway.sock");
        let mut requests = fake_sway(&path, true, Vec::new(), r#"[{"success": true}]"#);
        let odd = Workspace {
            id: 6,
            name: "$ws".into(),
            monitor: None,
            urgent: false,
        };
        assert!(matches!(
            at(&path).focus(&odd).await,
            Err(Error::Unquotable(Unquotable::Character(_)))
        ));
        assert!(requests.try_recv().is_err(), "nothing reached sway");
    }

    #[tokio::test]
    async fn a_refused_subscription_is_an_error() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("sway.sock");
        let _requests = fake_sway(&path, false, Vec::new(), "[]");
        assert!(matches!(
            at(&path).subscribe().await,
            Err(Error::Refused { reply, .. }) if reply.contains("false")
        ));
    }

    #[test]
    fn without_swaysock_sway_is_not_running() {
        // The variable is read, not set, so the test does not race others.
        if std::env::var_os("SWAYSOCK").is_none() {
            assert!(matches!(Sway::from_env(), Err(Error::NotRunning)));
        }
    }
}
