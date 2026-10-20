//! What the D-Bus sources share about the bus itself.

use std::pin::pin;

use futures_util::future::{Either, select};
use futures_util::{FutureExt, Stream, StreamExt};
use zbus::Connection;
use zbus::fdo::{RequestNameFlags, RequestNameReply};

/// Asks the bus for `name` with `flags`, and says whether lyr now owns it.
///
/// Another owner keeping the name is `false`, however the bus says so:
/// zbus 6 turns the bus's `Exists` reply into [`zbus::Error::NameTaken`]
/// rather than returning it, and a queued request does not own the name
/// either.
///
/// # Errors
///
/// When the bus does not answer.
pub async fn claim(
    connection: &Connection,
    name: &str,
    flags: RequestNameFlags,
) -> zbus::Result<bool> {
    match connection.request_name_with_flags(name, flags).await {
        Ok(RequestNameReply::PrimaryOwner | RequestNameReply::AlreadyOwner) => Ok(true),
        Ok(RequestNameReply::InQueue | RequestNameReply::Exists) | Err(zbus::Error::NameTaken) => {
            Ok(false)
        }
        Err(error) => Err(error),
    }
}

/// A stream of changes a source follows, drained while the source reads.
///
/// zbus stops reading a connection while any of its streams is full, so a
/// stream left unpolled through a read that awaits replies on the same
/// connection never gets them: a Wi-Fi scan's burst of access-point
/// signals hung NetworkManager's watch that way. So each read runs through
/// [`Follow::during`], and the changes it accepts meanwhile make one more
/// read, not one each.
pub struct Follow<S> {
    changes: S,
    /// A wanted change came during the last read.
    due: bool,
    ended: bool,
}

impl<S: Stream + Unpin> Follow<S> {
    /// Follows `changes`: subscribe them before the first read, so a change
    /// during it is not lost.
    pub const fn new(changes: S) -> Self {
        Self {
            changes,
            due: false,
            ended: false,
        }
    }

    /// Runs `read` to its end, draining the changes meanwhile; one that
    /// `wanted` accepts makes [`Follow::changed`] answer at once.
    ///
    /// # Errors
    ///
    /// The first error `wanted` returns.
    pub async fn during<F: Future>(
        &mut self,
        read: F,
        mut wanted: impl FnMut(S::Item) -> zbus::Result<bool>,
    ) -> zbus::Result<F::Output> {
        let mut read = pin!(read);
        while !self.ended {
            match select(read.as_mut(), self.changes.next()).await {
                Either::Left((output, _)) => return Ok(output),
                Either::Right((Some(change), _)) => self.due |= wanted(change)?,
                Either::Right((None, _)) => self.ended = true,
            }
        }
        Ok(read.await)
    }

    /// Waits for a change `wanted` accepts, or says there was one during
    /// the last read; `false` once the changes have ended. The changes
    /// already queued are taken too, since the read this asks for covers
    /// them.
    ///
    /// # Errors
    ///
    /// The first error `wanted` returns.
    pub async fn changed(
        &mut self,
        mut wanted: impl FnMut(S::Item) -> zbus::Result<bool>,
    ) -> zbus::Result<bool> {
        self.take_queued(&mut wanted)?;
        while !std::mem::take(&mut self.due) {
            if self.ended {
                return Ok(false);
            }
            match self.changes.next().await {
                Some(change) => self.due = wanted(change)?,
                None => self.ended = true,
            }
            self.take_queued(&mut wanted)?;
        }
        Ok(true)
    }

    /// Takes the changes that are already queued, without waiting.
    fn take_queued(
        &mut self,
        wanted: &mut impl FnMut(S::Item) -> zbus::Result<bool>,
    ) -> zbus::Result<()> {
        while !self.ended {
            match self.changes.next().now_or_never() {
                Some(Some(change)) => self.due |= wanted(change)?,
                Some(None) => self.ended = true,
                None => break,
            }
        }
        Ok(())
    }
}

/// A private message bus for tests that need names, which the
/// peer-to-peer connections the other tests use do not have.
#[cfg(test)]
pub mod test {
    use std::io::{BufRead, BufReader};
    use std::process::{Child, Command, Stdio};

    use zbus::Connection;
    use zbus::connection::Builder;

    /// A `dbus-daemon` of the test's own, stopped when dropped.
    pub struct Bus {
        daemon: Child,
        address: String,
    }

    impl Bus {
        /// Starts the daemon with the session bus's configuration.
        ///
        /// # Panics
        ///
        /// When `dbus-daemon` is not installed or does not say where it
        /// listens: the test cannot run, and passing would hide that.
        pub fn start() -> Self {
            let mut daemon = Command::new("dbus-daemon")
                .args(["--session", "--nofork", "--print-address"])
                .stdout(Stdio::piped())
                .spawn()
                .expect("dbus-daemon runs (the tests that need a bus need it installed)");
            let stdout = daemon.stdout.take().expect("piped stdout");
            let mut address = String::new();
            BufReader::new(stdout)
                .read_line(&mut address)
                .expect("dbus-daemon prints its address");
            let address = address.trim().to_owned();
            assert!(!address.is_empty(), "dbus-daemon printed no address");
            Self { daemon, address }
        }

        /// A new connection to this bus.
        pub async fn connect(&self) -> Connection {
            Builder::address(self.address.as_str())
                .build()
                .await
                .expect("the test bus accepts a connection")
        }
    }

    impl Drop for Bus {
        fn drop(&mut self) {
            // Already gone is fine: there is nothing left to stop.
            let _gone = self.daemon.kill();
            let _reaped = self.daemon.wait();
        }
    }
}

#[cfg(test)]
mod tests {
    use zbus::fdo::RequestNameFlags;

    use super::claim;
    use super::test::Bus;

    const NAME: &str = "sh.kryptic.lyr.Test";

    #[tokio::test]
    async fn a_name_another_connection_holds_is_not_claimed() {
        let bus = Bus::start();
        let first = bus.connect().await;
        let second = bus.connect().await;

        assert!(
            claim(&first, NAME, RequestNameFlags::DO_NOT_QUEUE)
                .await
                .unwrap()
        );
        assert!(
            !claim(&second, NAME, RequestNameFlags::DO_NOT_QUEUE)
                .await
                .unwrap()
        );
        // A queued request does not own the name either.
        assert!(
            !claim(&second, NAME, RequestNameFlags::empty())
                .await
                .unwrap()
        );
        assert!(
            claim(&first, NAME, RequestNameFlags::DO_NOT_QUEUE)
                .await
                .unwrap()
        );
    }
}

#[cfg(test)]
mod follow_tests {
    use std::os::unix::net::UnixStream;
    use std::time::Duration;

    use zbus::connection::Builder;
    use zbus::message::Type;
    use zbus::object_server::SignalEmitter;
    use zbus::{Guid, MatchRule, MessageStream};

    use super::Follow;

    /// More signals than a stream queues by default (zbus's 64).
    const BURST: u32 = 500;

    /// A service whose every read sends a burst of change signals before it
    /// answers, as NetworkManager's access points do during a scan.
    struct Bursty;

    #[zbus::interface(name = "sh.kryptic.lyr.Bursty")]
    impl Bursty {
        async fn read(
            &self,
            #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
        ) -> zbus::fdo::Result<u32> {
            for _ in 0..BURST {
                Self::changed(&emitter).await?;
            }
            Ok(BURST)
        }

        #[zbus(signal)]
        async fn changed(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;
    }

    #[zbus::proxy(
        interface = "sh.kryptic.lyr.Bursty",
        default_service = "sh.kryptic.lyr.Bursty",
        default_path = "/sh/kryptic/lyr/Bursty"
    )]
    trait BurstyClient {
        fn read(&self) -> zbus::Result<u32>;
    }

    #[tokio::test]
    async fn a_burst_during_a_read_does_not_stall_the_connection() {
        let (server, client) = UnixStream::pair().expect("socket pair");
        let server = Builder::unix_stream(server)
            .server(Guid::generate())
            .p2p()
            .serve_at("/sh/kryptic/lyr/Bursty", Bursty)
            .build();
        let client = Builder::unix_stream(client).p2p().build();
        let (_server, client) = tokio::try_join!(server, client).expect("connected");
        let proxy = BurstyClientProxy::new(&client).await.expect("proxy");
        let rule = MatchRule::builder()
            .msg_type(Type::Signal)
            .member("Changed")
            .build()
            .expect("a rule");
        let changes = MessageStream::for_match_rule(rule, &client, None)
            .await
            .expect("subscribed");
        assert!(changes.max_queued().get() < BURST as usize);

        let mut changes = Follow::new(changes);
        let wanted = |change: zbus::Result<zbus::Message>| change.map(|_| true);
        let read = changes.during(proxy.read(), wanted);
        let read = tokio::time::timeout(Duration::from_secs(10), read)
            .await
            .expect("the read was answered, burst and all")
            .expect("drained");
        assert_eq!(read.expect("read"), BURST);
        // The whole burst asks for one more read, not one each.
        assert!(changes.changed(wanted).await.expect("changed"));
        let more = tokio::time::timeout(Duration::from_millis(200), changes.changed(wanted)).await;
        assert!(more.is_err(), "nothing changed since");
    }
}
