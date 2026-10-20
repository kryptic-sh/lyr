//! What the D-Bus sources share about the bus itself.

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
