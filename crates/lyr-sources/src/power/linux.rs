//! The power menu's actions on Linux: logind over the system bus, called as
//! systemd v262.1's `systemctl` and `loginctl` call it
//! (`src/systemctl/systemctl-logind.c`, `src/login/loginctl.c`).
//!
//! Sleep is `Manager.Sleep`, which picks suspend, hibernate or
//! suspend-then-hibernate from `sleep.conf` as `systemctl sleep` does; it is
//! new in systemd 256 and has no fallback. The others are
//! `<Method>WithFlags`, falling back to `<Method>(interactive)` on a logind
//! that does not know it, with `interactive` true, systemctl's default. Lock
//! is `Session.Lock` on the caller's own session, as `loginctl lock-session`
//! with no id does.

use std::os::unix::fs::MetadataExt as _;

use zbus::Connection;

use super::reboot_flags;

#[zbus::proxy(
    interface = "org.freedesktop.login1.Manager",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1"
)]
trait Manager {
    fn sleep(&self, flags: u64) -> zbus::Result<()>;
    fn hibernate_with_flags(&self, flags: u64) -> zbus::Result<()>;
    fn hibernate(&self, interactive: bool) -> zbus::Result<()>;
    fn reboot_with_flags(&self, flags: u64) -> zbus::Result<()>;
    fn reboot(&self, interactive: bool) -> zbus::Result<()>;
    fn power_off_with_flags(&self, flags: u64) -> zbus::Result<()>;
    fn power_off(&self, interactive: bool) -> zbus::Result<()>;
}

#[zbus::proxy(
    interface = "org.freedesktop.login1.Session",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1/session/auto"
)]
trait Session {
    fn lock(&self) -> zbus::Result<()>;
}

/// A failed power action.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Sleep needs systemd 256 or later")]
    SleepNeedsSystemd256,
    #[error(transparent)]
    Bus(#[from] zbus::Error),
}

/// Whether `error` is logind answering that it has no such method.
fn unknown_method(error: &zbus::Error) -> bool {
    matches!(error, zbus::Error::FDO(fdo) if matches!(**fdo, zbus::fdo::Error::UnknownMethod(_)))
}

/// logind on one connection.
#[derive(Clone, Debug)]
pub struct Logind {
    manager: ManagerProxy<'static>,
    session: SessionProxy<'static>,
}

impl Logind {
    /// logind on the system bus.
    ///
    /// # Errors
    ///
    /// When the system bus cannot be reached.
    pub async fn system() -> zbus::Result<Self> {
        Self::on(&Connection::system().await?).await
    }

    /// logind on `connection`.
    ///
    /// # Errors
    ///
    /// When the proxies cannot be built.
    pub async fn on(connection: &Connection) -> zbus::Result<Self> {
        Ok(Self {
            manager: ManagerProxy::new(connection).await?,
            session: SessionProxy::new(connection).await?,
        })
    }

    /// Locks this session's screen.
    ///
    /// # Errors
    ///
    /// When logind refuses.
    pub async fn lock(&self) -> Result<(), Error> {
        Ok(self.session.lock().await?)
    }

    /// Sleeps as `sleep.conf` says.
    ///
    /// # Errors
    ///
    /// [`Error::SleepNeedsSystemd256`] on an older logind, or when it
    /// refuses.
    pub async fn sleep(&self) -> Result<(), Error> {
        match self.manager.sleep(0).await {
            Err(error) if unknown_method(&error) => Err(Error::SleepNeedsSystemd256),
            other => Ok(other?),
        }
    }

    /// Hibernates.
    ///
    /// # Errors
    ///
    /// When logind refuses.
    pub async fn hibernate(&self) -> Result<(), Error> {
        match self.manager.hibernate_with_flags(0).await {
            Err(error) if unknown_method(&error) => Ok(self.manager.hibernate(true).await?),
            other => Ok(other?),
        }
    }

    /// Reboots, with the flags `systemctl reboot` would pass here.
    ///
    /// # Errors
    ///
    /// When logind refuses.
    pub async fn reboot(&self) -> Result<(), Error> {
        let flags = reboot_flags(
            std::env::var("SYSTEMCTL_SKIP_AUTO_KEXEC").ok().as_deref(),
            std::env::var("SYSTEMCTL_SKIP_AUTO_SOFT_REBOOT")
                .ok()
                .as_deref(),
            nextroot_mounted(),
        );
        self.reboot_with(flags).await
    }

    /// Reboots with `flags`. A logind that does not take the soft-reboot
    /// flag answers `InvalidArgs`, and systemctl asks again without it.
    async fn reboot_with(&self, flags: u64) -> Result<(), Error> {
        let result = match self.manager.reboot_with_flags(flags).await {
            Err(zbus::Error::FDO(fdo))
                if flags & super::SOFT_REBOOT_IF_NEXTROOT_SET_UP != 0
                    && matches!(*fdo, zbus::fdo::Error::InvalidArgs(_)) =>
            {
                self.manager
                    .reboot_with_flags(flags & !super::SOFT_REBOOT_IF_NEXTROOT_SET_UP)
                    .await
            }
            other => other,
        };
        match result {
            Err(error) if unknown_method(&error) => Ok(self.manager.reboot(true).await?),
            other => Ok(other?),
        }
    }

    /// Powers off.
    ///
    /// # Errors
    ///
    /// When logind refuses.
    pub async fn power_off(&self) -> Result<(), Error> {
        match self.manager.power_off_with_flags(0).await {
            Err(error) if unknown_method(&error) => Ok(self.manager.power_off(true).await?),
            other => Ok(other?),
        }
    }
}

/// Whether `/run/nextroot` is a mount point: on another device than `/run`.
/// systemd's `path_is_mount_point` also catches a bind mount from the same
/// device, which this does not.
fn nextroot_mounted() -> bool {
    match (
        std::fs::metadata("/run/nextroot"),
        std::fs::metadata("/run"),
    ) {
        (Ok(nextroot), Ok(run)) => nextroot.dev() != run.dev(),
        _ => false,
    }
}

#[cfg(test)]
mod tests;
