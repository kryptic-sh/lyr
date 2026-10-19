use std::os::unix::net::UnixStream;
use std::sync::{Arc, Mutex};

use zbus::connection::Builder;
use zbus::{Connection, Guid};

use super::{Error, Logind};
use crate::power::{REBOOT_VIA_KEXEC, SOFT_REBOOT_IF_NEXTROOT_SET_UP};

type Calls = Arc<Mutex<Vec<String>>>;

/// A logind of systemd 256 or later. With `refuse_soft_reboot`, it answers
/// `InvalidArgs` to the soft-reboot flag, as one that does not know it does.
struct NewManager {
    calls: Calls,
    refuse_soft_reboot: bool,
}

#[zbus::interface(name = "org.freedesktop.login1.Manager")]
impl NewManager {
    fn sleep(&self, flags: u64) {
        self.calls
            .lock()
            .expect("calls")
            .push(format!("Sleep({flags})"));
    }
    fn hibernate_with_flags(&self, flags: u64) {
        let mut calls = self.calls.lock().expect("calls");
        calls.push(format!("HibernateWithFlags({flags})"));
    }
    fn reboot_with_flags(&self, flags: u64) -> zbus::fdo::Result<()> {
        let mut calls = self.calls.lock().expect("calls");
        calls.push(format!("RebootWithFlags({flags})"));
        if self.refuse_soft_reboot && flags & SOFT_REBOOT_IF_NEXTROOT_SET_UP != 0 {
            return Err(zbus::fdo::Error::InvalidArgs("unknown flag".to_owned()));
        }
        Ok(())
    }
    fn power_off_with_flags(&self, flags: u64) {
        let mut calls = self.calls.lock().expect("calls");
        calls.push(format!("PowerOffWithFlags({flags})"));
    }
}

/// A logind from before the `WithFlags` methods and `Sleep`.
struct OldManager {
    calls: Calls,
}

#[zbus::interface(name = "org.freedesktop.login1.Manager")]
impl OldManager {
    fn hibernate(&self, interactive: bool) {
        let mut calls = self.calls.lock().expect("calls");
        calls.push(format!("Hibernate({interactive})"));
    }
    fn reboot(&self, interactive: bool) {
        let mut calls = self.calls.lock().expect("calls");
        calls.push(format!("Reboot({interactive})"));
    }
    fn power_off(&self, interactive: bool) {
        let mut calls = self.calls.lock().expect("calls");
        calls.push(format!("PowerOff({interactive})"));
    }
}

struct FakeSession {
    calls: Calls,
}

#[zbus::interface(name = "org.freedesktop.login1.Session")]
impl FakeSession {
    fn lock(&self) {
        self.calls.lock().expect("calls").push("Lock".to_owned());
    }
}

async fn logind(
    manager: impl zbus::object_server::Interface,
    calls: &Calls,
) -> (Logind, Connection) {
    let (server, client) = UnixStream::pair().expect("socket pair");
    let server = Builder::unix_stream(server)
        .server(Guid::generate())
        .p2p()
        .serve_at("/org/freedesktop/login1", manager)
        .serve_at(
            "/org/freedesktop/login1/session/auto",
            FakeSession {
                calls: calls.clone(),
            },
        )
        .build();
    let client = Builder::unix_stream(client).p2p().build();
    let (server, client) = tokio::try_join!(server, client).expect("connected");
    (Logind::on(&client).await.expect("logind"), server)
}

fn taken(calls: &Calls) -> Vec<String> {
    std::mem::take(&mut *calls.lock().expect("calls"))
}

#[tokio::test]
async fn a_new_logind_gets_systemctl_s_calls() {
    let calls = Calls::default();
    let manager = NewManager {
        calls: calls.clone(),
        refuse_soft_reboot: false,
    };
    let (logind, _server) = logind(manager, &calls).await;
    logind.lock().await.expect("locked");
    logind.sleep().await.expect("slept");
    logind.hibernate().await.expect("hibernated");
    logind
        .reboot_with(REBOOT_VIA_KEXEC)
        .await
        .expect("rebooted");
    logind.power_off().await.expect("powered off");
    assert_eq!(
        taken(&calls),
        [
            "Lock",
            "Sleep(0)",
            "HibernateWithFlags(0)",
            "RebootWithFlags(2)",
            "PowerOffWithFlags(0)",
        ]
    );
}

#[tokio::test]
async fn an_old_logind_gets_the_interactive_methods_and_no_sleep() {
    let calls = Calls::default();
    let manager = OldManager {
        calls: calls.clone(),
    };
    let (logind, _server) = logind(manager, &calls).await;
    assert!(matches!(
        logind.sleep().await,
        Err(Error::SleepNeedsSystemd256)
    ));
    logind.hibernate().await.expect("hibernated");
    logind
        .reboot_with(REBOOT_VIA_KEXEC)
        .await
        .expect("rebooted");
    logind.power_off().await.expect("powered off");
    assert_eq!(
        taken(&calls),
        ["Hibernate(true)", "Reboot(true)", "PowerOff(true)"]
    );
}

#[tokio::test]
async fn a_refused_soft_reboot_flag_is_dropped_and_asked_again() {
    let calls = Calls::default();
    let manager = NewManager {
        calls: calls.clone(),
        refuse_soft_reboot: true,
    };
    let (logind, _server) = logind(manager, &calls).await;
    logind
        .reboot_with(REBOOT_VIA_KEXEC | SOFT_REBOOT_IF_NEXTROOT_SET_UP)
        .await
        .expect("rebooted");
    assert_eq!(taken(&calls), ["RebootWithFlags(10)", "RebootWithFlags(2)"]);
}
