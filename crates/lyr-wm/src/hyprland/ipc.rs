//! Talking to a running Hyprland over its two Unix sockets.

use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, Lines};
use tokio::net::UnixStream;

use super::{Dialect, Event, ParseError, State, Workspace, focus_request, parse_line, snapshot};

/// How long a request may take before lyr gives up on it. `hyprctl` uses
/// the same receive timeout.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

/// A Hyprland reply that means the request was carried out.
const OK_REPLY: &str = "ok";

/// What went wrong talking to Hyprland.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Hyprland is not running here: HYPRLAND_INSTANCE_SIGNATURE is not set")]
    NotRunning,
    #[error("XDG_RUNTIME_DIR is not set, so Hyprland's sockets cannot be found")]
    NoRuntimeDir,
    #[error("{what} {path}: {source}")]
    Io {
        what: &'static str,
        path: PathBuf,
        source: io::Error,
    },
    #[error("Hyprland did not answer `{request}` within {REQUEST_TIMEOUT:?}")]
    Timeout { request: String },
    #[error("Hyprland's reply to `{request}` is not the JSON lyr expects: {source}")]
    Json {
        request: String,
        source: serde_json::Error,
    },
    #[error("Hyprland refused `{request}`: {reply}")]
    Refused { request: String, reply: String },
    #[error("lyr cannot name workspace {name:?} in a Lua dispatch")]
    Unaddressable { name: String },
    #[error(transparent)]
    Parse(#[from] ParseError),
}

/// A connection point to one Hyprland instance.
#[derive(Clone, Debug)]
pub struct Hyprland {
    socket_dir: PathBuf,
    dialect: Dialect,
}

impl Hyprland {
    /// Finds the Hyprland this process runs under, from its environment.
    pub fn from_env() -> Result<Self, Error> {
        Self::from_vars(|name| std::env::var_os(name))
    }

    /// [`Hyprland::from_env`] with the environment passed in.
    ///
    /// The sockets are in `$XDG_RUNTIME_DIR/hypr/$HYPRLAND_INSTANCE_SIGNATURE`.
    /// The config is Lua when `hypr/hyprland.lua` exists in
    /// `$XDG_CONFIG_HOME`, or in `$HOME/.config` when that is unset.
    fn from_vars(var: impl Fn(&str) -> Option<OsString>) -> Result<Self, Error> {
        let signature = var("HYPRLAND_INSTANCE_SIGNATURE").ok_or(Error::NotRunning)?;
        let runtime = var("XDG_RUNTIME_DIR").ok_or(Error::NoRuntimeDir)?;
        let config = var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| var("HOME").map(|home| Path::new(&home).join(".config")));
        let lua = config.is_some_and(|config| config.join("hypr/hyprland.lua").is_file());
        Ok(Self {
            socket_dir: Path::new(&runtime).join("hypr").join(signature),
            dialect: if lua { Dialect::Lua } else { Dialect::Hyprlang },
        })
    }

    /// The dispatcher syntax requests are written in.
    #[must_use]
    pub const fn dialect(&self) -> Dialect {
        self.dialect
    }

    /// Subscribes to events and reads the current state.
    ///
    /// The event socket is connected first and the state read after, so
    /// that no change between the two is lost: events that arrive meanwhile
    /// wait in the socket and are applied on top of a state that may already
    /// include them, which [`State::apply`] tolerates.
    pub async fn subscribe(&self) -> Result<(State, Events), Error> {
        let path = self.socket_dir.join(".socket2.sock");
        let stream = UnixStream::connect(&path)
            .await
            .map_err(|source| Error::Io {
                what: "connecting to",
                path,
                source,
            })?;
        let events = Events {
            lines: BufReader::new(stream).lines(),
        };
        let workspaces = self.request("j/workspaces").await?;
        let active = self.request("j/activeworkspace").await?;
        let clients = self.request("j/clients").await?;
        let state =
            snapshot::parse(&workspaces, &active, &clients).map_err(|source| Error::Json {
                request: "j/workspaces, j/activeworkspace or j/clients".to_owned(),
                source,
            })?;
        Ok((state, events))
    }

    /// Switches to `workspace`.
    pub async fn focus(&self, workspace: &Workspace) -> Result<(), Error> {
        let request =
            focus_request(workspace, self.dialect).ok_or_else(|| Error::Unaddressable {
                name: workspace.name.clone(),
            })?;
        let reply = self.request(&request).await?;
        if reply.trim() == OK_REPLY {
            Ok(())
        } else {
            Err(Error::Refused { request, reply })
        }
    }

    /// Sends one request and returns the whole reply. Hyprland takes one
    /// request per connection and closes it after replying.
    async fn request(&self, request: &str) -> Result<String, Error> {
        let path = self.socket_dir.join(".socket.sock");
        let exchange = async {
            let mut stream = UnixStream::connect(&path).await?;
            stream.write_all(request.as_bytes()).await?;
            let mut reply = String::new();
            stream.read_to_string(&mut reply).await?;
            Ok::<_, io::Error>(reply)
        };
        match tokio::time::timeout(REQUEST_TIMEOUT, exchange).await {
            Ok(Ok(reply)) => Ok(reply),
            Ok(Err(source)) => Err(Error::Io {
                what: "requesting over",
                path,
                source,
            }),
            Err(_) => Err(Error::Timeout {
                request: request.to_owned(),
            }),
        }
    }
}

/// The event stream from `.socket2.sock`.
#[derive(Debug)]
pub struct Events {
    lines: Lines<BufReader<UnixStream>>,
}

impl Events {
    /// The next event lyr acts on, skipping the others. `Ok(None)` means
    /// Hyprland closed the socket. An [`Error::Parse`] is about one line
    /// only, and the stream can be read on after it.
    pub async fn next(&mut self) -> Result<Option<Event>, Error> {
        loop {
            let line = self.lines.next_line().await.map_err(|source| Error::Io {
                what: "reading events from",
                path: PathBuf::from(".socket2.sock"),
                source,
            })?;
            let Some(line) = line else {
                return Ok(None);
            };
            if let Some(event) = parse_line(&line)? {
                return Ok(Some(event));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::Path;

    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::UnixListener;

    use super::{Error, Hyprland};
    use crate::hyprland::{Dialect, Event};

    fn vars<'a>(pairs: &'a [(&'a str, &'a Path)]) -> impl Fn(&str) -> Option<OsString> + 'a {
        move |name| {
            pairs
                .iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| value.as_os_str().to_owned())
        }
    }

    #[test]
    fn the_environment_names_the_sockets_and_the_dialect() {
        let config = tempfile::tempdir().expect("tempdir");
        let runtime = Path::new("/run/user/1000");
        let signature = Path::new("abc_123");
        let env = [
            ("HYPRLAND_INSTANCE_SIGNATURE", signature),
            ("XDG_RUNTIME_DIR", runtime),
            ("XDG_CONFIG_HOME", config.path()),
        ];

        let hyprlang = Hyprland::from_vars(vars(&env)).expect("found");
        assert_eq!(
            hyprlang.socket_dir,
            Path::new("/run/user/1000/hypr/abc_123")
        );
        assert_eq!(hyprlang.dialect(), Dialect::Hyprlang);

        std::fs::create_dir(config.path().join("hypr")).expect("mkdir");
        std::fs::write(config.path().join("hypr/hyprland.lua"), "").expect("write");
        assert_eq!(
            Hyprland::from_vars(vars(&env)).expect("found").dialect(),
            Dialect::Lua
        );

        assert!(matches!(
            Hyprland::from_vars(vars(&env[1..])),
            Err(Error::NotRunning)
        ));
        assert!(matches!(
            Hyprland::from_vars(vars(&env[..1])),
            Err(Error::NoRuntimeDir)
        ));
    }

    /// A stand-in Hyprland: answers each request on `.socket.sock` from
    /// `reply`, records what it was asked, and writes `events` to the first
    /// `.socket2.sock` client.
    async fn fake_hyprland(
        dir: &Path,
        reply: fn(&str) -> &'static str,
        events: &'static str,
    ) -> tokio::sync::mpsc::UnboundedReceiver<String> {
        let requests = UnixListener::bind(dir.join(".socket.sock")).expect("bind");
        let event_socket = UnixListener::bind(dir.join(".socket2.sock")).expect("bind");
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        tokio::spawn(async move {
            let (mut stream, _) = event_socket.accept().await.expect("accept");
            stream.write_all(events.as_bytes()).await.expect("write");
        });
        tokio::spawn(async move {
            loop {
                let (mut stream, _) = requests.accept().await.expect("accept");
                let mut buf = vec![0; 4096];
                let n = stream.read(&mut buf).await.expect("read");
                let request = String::from_utf8_lossy(&buf[..n]).into_owned();
                stream
                    .write_all(reply(&request).as_bytes())
                    .await
                    .expect("write");
                tx.send(request).expect("send");
            }
        });
        rx
    }

    fn replies(request: &str) -> &'static str {
        match request {
            "j/workspaces" => {
                r#"[{"id": 1, "name": "1", "monitor": "DP-1"},
                                  {"id": 2, "name": "2", "monitor": "DP-1"}]"#
            }
            "j/activeworkspace" => r#"{"id": 1, "name": "1", "monitor": "DP-1"}"#,
            "j/clients" => "[]",
            "/dispatch hl.dsp.focus({ workspace = 2 })" => "ok",
            _ => "unknown request",
        }
    }

    #[tokio::test]
    async fn subscribe_reads_the_state_then_streams_events() {
        let dir = tempfile::tempdir().expect("tempdir");
        let _requests = fake_hyprland(
            dir.path(),
            replies,
            "activewindowv2>>5f1e\nworkspacev2>>2,2\nworkspacev2>>x\n",
        )
        .await;
        let hyprland = Hyprland {
            socket_dir: dir.path().to_owned(),
            dialect: Dialect::Lua,
        };

        let (mut state, mut events) = hyprland.subscribe().await.expect("subscribed");
        assert_eq!(state.active(), Some(1));
        assert_eq!(state.workspaces().count(), 2);

        let event = events.next().await.expect("an event").expect("open");
        assert_eq!(
            event,
            Event::Workspace {
                id: 2,
                name: "2".into()
            },
            "the line lyr does not act on is skipped"
        );
        assert!(state.apply(event));
        assert_eq!(state.active(), Some(2));

        assert!(matches!(events.next().await, Err(Error::Parse(_))));
        assert!(matches!(events.next().await, Ok(None)), "socket closed");
    }

    #[tokio::test]
    async fn focus_writes_the_dispatch_and_checks_the_reply() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut requests = fake_hyprland(dir.path(), replies, "").await;
        let lua = Hyprland {
            socket_dir: dir.path().to_owned(),
            dialect: Dialect::Lua,
        };
        let (state, _events) = lua.subscribe().await.expect("subscribed");
        for _ in 0..3 {
            requests.recv().await.expect("a state request");
        }
        let two = state.workspaces().find(|w| w.id == 2).expect("2").clone();

        lua.focus(&two).await.expect("ok reply");
        assert_eq!(
            requests.recv().await.as_deref(),
            Some("/dispatch hl.dsp.focus({ workspace = 2 })")
        );

        let hyprlang = Hyprland {
            dialect: Dialect::Hyprlang,
            ..lua
        };
        assert!(matches!(
            hyprlang.focus(&two).await,
            Err(Error::Refused { reply, .. }) if reply == "unknown request"
        ));
    }
}
