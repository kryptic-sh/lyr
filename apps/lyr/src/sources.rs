//! Running the module sources (`docs/plan/02-architecture.md`): each on a
//! tokio runtime of its own thread, handing its state to the main loop over a
//! channel and waking it, so the main loop never blocks on a source and no
//! source is polled.
//!
//! Linux has the battery (UPower) and the workspaces (Hyprland). Other
//! platforms have no sources yet, and their modules are left out of the bar.

use std::sync::mpsc;

use crcbl_shell::ShellWaker;

/// A source's new state. Empty on a platform with no sources.
#[derive(Debug)]
pub enum Update {
    /// The battery, or `None` when there is none to show.
    #[cfg(target_os = "linux")]
    Battery(Option<lyr_core::battery::Reading>),
    /// Hyprland's workspaces.
    #[cfg(target_os = "linux")]
    Workspaces(lyr_wm::hyprland::State),
}

/// Starts every source this platform has. A source that cannot start, or
/// stops, says why on stderr and its module stays out of the bar.
pub fn start(waker: Option<ShellWaker>) -> std::io::Result<mpsc::Receiver<Update>> {
    let (sender, receiver) = mpsc::channel();
    #[cfg(target_os = "linux")]
    linux::start(sender, waker)?;
    #[cfg(not(target_os = "linux"))]
    drop((sender, waker));
    Ok(receiver)
}

#[cfg(target_os = "linux")]
mod linux {
    use std::sync::mpsc;

    use crcbl_shell::ShellWaker;
    use lyr_sources::battery::linux::UPower;
    use lyr_wm::hyprland::Hyprland;
    use tokio::sync::watch;

    use super::Update;

    /// Hands `update` to the main loop and wakes it. `false` once the main
    /// loop has gone.
    fn deliver(sender: &mpsc::Sender<Update>, waker: Option<&ShellWaker>, update: Update) -> bool {
        let delivered = sender.send(update).is_ok();
        if let Some(waker) = waker {
            waker.wake();
        }
        delivered
    }

    pub(super) fn start(
        sender: mpsc::Sender<Update>,
        waker: Option<ShellWaker>,
    ) -> std::io::Result<()> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        std::thread::Builder::new()
            .name("lyr sources".to_owned())
            .spawn(move || {
                runtime.block_on(async move {
                    tokio::join!(
                        battery(sender.clone(), waker.clone()),
                        workspaces(sender, waker),
                    );
                });
            })?;
        Ok(())
    }

    async fn battery(sender: mpsc::Sender<Update>, waker: Option<ShellWaker>) {
        let upower = match UPower::system().await {
            Ok(upower) => upower,
            Err(error) => {
                eprintln!("lyr: battery: UPower is not reachable: {error}");
                return;
            }
        };
        let (readings, mut changed) = watch::channel(None);
        let forward = async {
            while changed.changed().await.is_ok() {
                let reading = *changed.borrow_and_update();
                if !deliver(&sender, waker.as_ref(), Update::Battery(reading)) {
                    break;
                }
            }
        };
        let (watched, ()) = tokio::join!(upower.watch(&readings), forward);
        if let Err(error) = watched {
            eprintln!("lyr: battery: UPower stopped answering: {error}");
        }
    }

    async fn workspaces(sender: mpsc::Sender<Update>, waker: Option<ShellWaker>) {
        let run = async {
            let hyprland = Hyprland::from_env().await?;
            let (mut state, mut events) = hyprland.subscribe().await?;
            if !deliver(&sender, waker.as_ref(), Update::Workspaces(state.clone())) {
                return Ok(());
            }
            while let Some(event) = events.next().await? {
                if state.apply(event)
                    && !deliver(&sender, waker.as_ref(), Update::Workspaces(state.clone()))
                {
                    break;
                }
            }
            Ok::<_, lyr_wm::hyprland::Error>(())
        };
        if let Err(error) = run.await {
            eprintln!("lyr: workspaces: {error}");
        }
    }
}
