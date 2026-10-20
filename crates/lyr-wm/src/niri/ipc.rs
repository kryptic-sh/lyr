//! Talking to a running niri over its IPC socket.

use std::ffi::OsString;
use std::io;
use std::path::PathBuf;
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines};
use tokio::net::UnixStream;

use super::{EVENT_STREAM_REQUEST, QUIT_REQUEST, Tracker, focus_request};
use crate::{State, Workspace};

/// How long a request, or the event stream's first state, may take before
/// lyr gives up on it, as for Hyprland.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

/// What went wrong talking to niri.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("niri is not running here: NIRI_SOCKET is not set")]
    NotRunning,
    #[error("{what} {path}: {source}")]
    Io {
        what: &'static str,
        path: PathBuf,
        source: io::Error,
    },
    #[error("niri did not answer `{request}` within {REQUEST_TIMEOUT:?}")]
    Timeout { request: String },
    #[error("niri's reply to `{request}` is not the JSON lyr expects: {source}")]
    Json {
        request: String,
        source: serde_json::Error,
    },
    #[error("niri refused `{request}`: {reply}")]
    Refused { request: String, reply: String },
    #[error("niri closed {path} before replying")]
    Closed { path: PathBuf },
}

/// A connection point to one niri instance.
#[derive(Clone, Debug)]
pub struct Niri {
    socket: PathBuf,
}

impl Niri {
    /// Finds the niri this process runs under, from `NIRI_SOCKET`.
    ///
    /// # Errors
    ///
    /// [`Error::NotRunning`] when the variable is not set.
    pub fn from_env() -> Result<Self, Error> {
        Self::from_vars(|name| std::env::var_os(name))
    }

    /// [`Niri::from_env`], reading the variable through `var`.
    pub(crate) fn from_vars(var: impl Fn(&str) -> Option<OsString>) -> Result<Self, Error> {
        let socket = var("NIRI_SOCKET").ok_or(Error::NotRunning)?;
        Ok(Self {
            socket: socket.into(),
        })
    }

    /// Opens the event stream and reads the state it starts with.
    ///
    /// # Errors
    ///
    /// When niri cannot be reached, refuses the stream, or does not send
    /// its first workspace and window lists in time.
    pub async fn subscribe(&self) -> Result<(State, Events), Error> {
        let stream = self.connect().await?;
        let mut lines = BufReader::new(stream).lines();
        let mut tracker = Tracker::default();
        let start = async {
            self.exchange(&mut lines, EVENT_STREAM_REQUEST).await?;
            while !tracker.is_whole() {
                let line = self.line(&mut lines).await?;
                tracker.apply(&line).map_err(|source| Error::Json {
                    request: EVENT_STREAM_REQUEST.to_owned(),
                    source,
                })?;
            }
            Ok::<_, Error>(())
        };
        tokio::time::timeout(REQUEST_TIMEOUT, start)
            .await
            .map_err(|_| Error::Timeout {
                request: EVENT_STREAM_REQUEST.to_owned(),
            })??;
        let state = tracker.state();
        let events = Events {
            niri: self.clone(),
            lines,
            tracker,
            last: state.clone(),
        };
        Ok((state, events))
    }

    /// Switches to `workspace`, by niri's id for it.
    ///
    /// # Errors
    ///
    /// When niri cannot be reached or refuses.
    pub async fn focus(&self, workspace: &Workspace) -> Result<(), Error> {
        self.request(&focus_request(workspace)).await
    }

    /// Ends the niri session: the power menu's Log out.
    ///
    /// niri carries out `Quit` by stopping its event loop, which also runs
    /// the task that would write the reply (`src/ipc/server.rs` and
    /// `src/input/mod.rs` at v26.04), so a socket closed before the reply
    /// is taken as done, as on sway. Read from the source, not seen.
    ///
    /// # Errors
    ///
    /// When niri cannot be reached or refuses.
    pub async fn exit(&self) -> Result<(), Error> {
        match self.request(QUIT_REQUEST).await {
            Err(Error::Closed { .. }) => Ok(()),
            outcome => outcome,
        }
    }

    /// Sends one request on a connection of its own and checks the reply.
    async fn request(&self, request: &str) -> Result<(), Error> {
        let exchange = async {
            let stream = self.connect().await?;
            let mut lines = BufReader::new(stream).lines();
            self.exchange(&mut lines, request).await
        };
        tokio::time::timeout(REQUEST_TIMEOUT, exchange)
            .await
            .map_err(|_| Error::Timeout {
                request: request.to_owned(),
            })?
    }

    /// Writes `request` as one line and reads its reply, which must be
    /// `{"Ok": …}`.
    async fn exchange(
        &self,
        lines: &mut Lines<BufReader<UnixStream>>,
        request: &str,
    ) -> Result<(), Error> {
        let stream = lines.get_mut().get_mut();
        stream
            .write_all(format!("{request}\n").as_bytes())
            .await
            .map_err(|source| self.io("requesting over", source))?;
        let reply = self.line(lines).await?;
        let outcome: Result<serde_json::Value, String> =
            serde_json::from_str(&reply).map_err(|source| Error::Json {
                request: request.to_owned(),
                source,
            })?;
        outcome.map(drop).map_err(|_| Error::Refused {
            request: request.to_owned(),
            reply,
        })
    }

    /// The next line, or an error when niri closed the socket first.
    async fn line(&self, lines: &mut Lines<BufReader<UnixStream>>) -> Result<String, Error> {
        lines
            .next_line()
            .await
            .map_err(|source| self.io("reading from", source))?
            .ok_or_else(|| Error::Closed {
                path: self.socket.clone(),
            })
    }

    async fn connect(&self) -> Result<UnixStream, Error> {
        UnixStream::connect(&self.socket)
            .await
            .map_err(|source| self.io("connecting to", source))
    }

    fn io(&self, what: &'static str, source: io::Error) -> Error {
        Error::Io {
            what,
            path: self.socket.clone(),
            source,
        }
    }
}

/// The event stream, read for changes.
#[derive(Debug)]
pub struct Events {
    niri: Niri,
    lines: Lines<BufReader<UnixStream>>,
    tracker: Tracker,
    /// The state last handed on, so an event that changes nothing the bar
    /// draws is not handed on again.
    last: State,
}

impl Events {
    /// The next state that differs from the last one handed on. `Ok(None)`
    /// means niri closed the socket.
    ///
    /// # Errors
    ///
    /// When the socket fails or an event lyr acts on is malformed.
    pub async fn next(&mut self) -> Result<Option<State>, Error> {
        loop {
            let line = self
                .lines
                .next_line()
                .await
                .map_err(|source| self.niri.io("reading events from", source))?;
            let Some(line) = line else {
                return Ok(None);
            };
            self.tracker.apply(&line).map_err(|source| Error::Json {
                request: EVENT_STREAM_REQUEST.to_owned(),
                source,
            })?;
            let state = self.tracker.state();
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
    use std::time::Duration;

    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::UnixListener;

    use super::{Error, Niri};
    use crate::Workspace;

    const WORKSPACES: &str = r#"{"WorkspacesChanged":{"workspaces":[
        {"id":1,"idx":1,"name":null,"output":"DP-1","is_urgent":false,"is_active":true,"is_focused":true},
        {"id":2,"idx":2,"name":null,"output":"DP-1","is_urgent":false,"is_active":false,"is_focused":false}]}}"#;
    const WINDOWS: &str = r#"{"WindowsChanged":{"windows":[]}}"#;

    /// A reply that stands for niri closing the socket unanswered.
    const QUITS: &str = "";

    /// A niri on `path`: each connection's requests are handed out and
    /// answered with `reply`, and once one asks for the event stream it is
    /// sent `events`, each followed by a newline, and held open.
    fn fake_niri(
        path: &Path,
        reply: &'static str,
        events: Vec<String>,
    ) -> tokio::sync::mpsc::UnboundedReceiver<String> {
        let listener = UnixListener::bind(path).expect("bind");
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        tokio::spawn(async move {
            loop {
                let (stream, _) = listener.accept().await.expect("accept");
                let tx = tx.clone();
                let events = events.clone();
                tokio::spawn(async move {
                    let mut lines = BufReader::new(stream).lines();
                    while let Ok(Some(request)) = lines.next_line().await {
                        tx.send(request.clone()).expect("send");
                        if reply == QUITS {
                            return;
                        }
                        let stream = lines.get_mut().get_mut();
                        stream
                            .write_all(format!("{reply}\n").as_bytes())
                            .await
                            .expect("reply");
                        if request == r#""EventStream""# {
                            for event in &events {
                                stream
                                    .write_all(format!("{event}\n").as_bytes())
                                    .await
                                    .expect("event");
                            }
                            std::future::pending::<()>().await;
                        }
                    }
                });
            }
        });
        rx
    }

    /// One line, as niri writes JSON: the multi-line constants above flattened.
    fn line(json: &str) -> String {
        json.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    fn at(path: &Path) -> Niri {
        Niri {
            socket: path.to_owned(),
        }
    }

    #[tokio::test]
    async fn the_stream_starts_whole_and_hands_on_each_change() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("niri.sock");
        let mut requests = fake_niri(
            &path,
            r#"{"Ok":"Handled"}"#,
            vec![
                line(WORKSPACES),
                r#"{"ConfigLoaded":{"failed":false}}"#.to_owned(),
                line(WINDOWS),
                r#"{"KeyboardLayoutSwitched":{"idx":1}}"#.to_owned(),
                r#"{"WorkspaceActivated":{"id":2,"focused":true}}"#.to_owned(),
                r#"{"WorkspaceActivated":{"id":2,"focused":true}}"#.to_owned(),
                r#"{"WorkspaceActivated":{"id":1,"focused":true}}"#.to_owned(),
            ],
        );
        let (state, mut events) = at(&path).subscribe().await.expect("subscribed");
        assert_eq!(requests.recv().await.as_deref(), Some(r#""EventStream""#));
        assert_eq!(state.active(), Some(1));
        assert_eq!(state.workspaces().count(), 2);

        let next = events.next().await.expect("read").expect("a change");
        assert_eq!(next.active(), Some(2));
        let next = events.next().await.expect("read").expect("a change");
        assert_eq!(
            next.active(),
            Some(1),
            "the repeated activation changed nothing and was not handed on"
        );
        let quiet = tokio::time::timeout(Duration::from_millis(100), events.next()).await;
        assert!(quiet.is_err(), "nothing more while niri says nothing");
    }

    #[tokio::test]
    async fn focus_and_quit_send_their_actions_and_check_the_reply() {
        let dir = tempfile::tempdir().expect("tempdir");
        let ok = dir.path().join("ok.sock");
        let mut requests = fake_niri(&ok, r#"{"Ok":"Handled"}"#, Vec::new());
        let two = Workspace {
            id: 2,
            name: "2".into(),
            monitor: None,
            urgent: false,
        };
        at(&ok).focus(&two).await.expect("handled");
        assert_eq!(
            requests.recv().await.as_deref(),
            Some(r#"{"Action":{"FocusWorkspace":{"reference":{"Id":2}}}}"#)
        );
        at(&ok).exit().await.expect("handled");
        assert_eq!(
            requests.recv().await.as_deref(),
            Some(r#"{"Action":{"Quit":{"skip_confirmation":true}}}"#)
        );

        let refused = dir.path().join("refused.sock");
        let _requests = fake_niri(&refused, r#"{"Err":"workspace not found"}"#, Vec::new());
        assert!(matches!(
            at(&refused).focus(&two).await,
            Err(Error::Refused { reply, .. }) if reply.contains("not found")
        ));
        assert!(matches!(
            at(&refused).subscribe().await,
            Err(Error::Refused { .. })
        ));
    }

    #[tokio::test]
    async fn quit_is_done_when_niri_closes_unanswered_and_only_quit_is() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("niri.sock");
        let _requests = fake_niri(&path, QUITS, Vec::new());
        at(&path).exit().await.expect("closing is the answer");
        let two = Workspace {
            id: 2,
            name: "2".into(),
            monitor: None,
            urgent: false,
        };
        assert!(matches!(
            at(&path).focus(&two).await,
            Err(Error::Closed { .. })
        ));
    }

    #[test]
    fn without_niri_socket_niri_is_not_running() {
        // The variable is read, not set, so the test does not race others.
        if std::env::var_os("NIRI_SOCKET").is_none() {
            assert!(matches!(Niri::from_env(), Err(Error::NotRunning)));
        }
    }
}
