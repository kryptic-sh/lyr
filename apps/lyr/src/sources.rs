//! Running the module sources (`docs/plan/02-architecture.md`): each on a
//! tokio runtime of its own thread, handing its state to the main loop over a
//! channel and waking it, so the main loop never blocks on a source and no
//! source is polled.
//!
//! Linux has the battery (UPower), the power profile
//! (power-profiles-daemon) and the workspaces (Hyprland). Other platforms
//! have no sources yet, and their modules are left out of the bar.

use std::sync::mpsc;

use crcbl_shell::ShellWaker;

/// A source's new state. Empty on a platform with no sources.
#[derive(Debug)]
pub enum Update {
    /// The battery, or `None` when there is none to show.
    #[cfg(target_os = "linux")]
    Battery(Option<lyr_core::battery::Reading>),
    /// The power profiles, or `None` when the daemon lists none.
    #[cfg(target_os = "linux")]
    PowerProfile(Option<lyr_sources::power_profile::Profiles>),
    /// Hyprland's workspaces.
    #[cfg(target_os = "linux")]
    Workspaces(lyr_wm::hyprland::State),
}

/// Where the bar sends what it asks of a source. A source that has stopped
/// drops what it is sent; it already logged why it stopped.
#[derive(Debug)]
pub struct Commands {
    #[cfg(target_os = "linux")]
    focus: tokio::sync::mpsc::UnboundedSender<lyr_wm::hyprland::Workspace>,
    #[cfg(target_os = "linux")]
    profile: tokio::sync::mpsc::UnboundedSender<String>,
}

#[cfg(target_os = "linux")]
impl Commands {
    /// Switches to `workspace`.
    pub fn focus(&self, workspace: lyr_wm::hyprland::Workspace) {
        let _stopped = self.focus.send(workspace);
    }

    /// Switches to the power profile `name`.
    pub fn set_profile(&self, name: String) {
        let _stopped = self.profile.send(name);
    }
}

/// Starts every source this platform has. A source that cannot start, or
/// stops, logs why and its module stays out of the bar.
pub fn start(waker: Option<ShellWaker>) -> std::io::Result<(mpsc::Receiver<Update>, Commands)> {
    let (sender, receiver) = mpsc::channel();
    #[cfg(target_os = "linux")]
    {
        let (focus, focus_requests) = tokio::sync::mpsc::unbounded_channel();
        let (profile, profile_requests) = tokio::sync::mpsc::unbounded_channel();
        linux::start(sender, focus_requests, profile_requests, waker)?;
        Ok((receiver, Commands { focus, profile }))
    }
    #[cfg(not(target_os = "linux"))]
    {
        drop((sender, waker));
        Ok((receiver, Commands {}))
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use std::sync::mpsc;

    use crcbl_shell::ShellWaker;
    use lyr_sources::battery::linux::UPower;
    use lyr_sources::power_profile::linux::PowerProfiles;
    use lyr_wm::hyprland::{Hyprland, Workspace};
    use tokio::sync::mpsc::UnboundedReceiver;
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
        focus: UnboundedReceiver<Workspace>,
        profile: UnboundedReceiver<String>,
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
                        power_profile(sender.clone(), profile, waker.clone()),
                        workspaces(sender, focus, waker),
                    );
                });
            })?;
        Ok(())
    }

    async fn battery(sender: mpsc::Sender<Update>, waker: Option<ShellWaker>) {
        let upower = match UPower::system().await {
            Ok(upower) => upower,
            Err(error) => {
                tracing::warn!("battery: UPower is not reachable: {error}");
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
            tracing::warn!("battery: UPower stopped answering: {error}");
        }
    }

    async fn power_profile(
        sender: mpsc::Sender<Update>,
        mut requests: UnboundedReceiver<String>,
        waker: Option<ShellWaker>,
    ) {
        let daemon = match PowerProfiles::system().await {
            Ok(daemon) => daemon,
            Err(error) => {
                tracing::warn!("power profile: power-profiles-daemon is not reachable: {error}");
                return;
            }
        };
        let (states, mut changed) = watch::channel(None);
        let forward = async {
            while changed.changed().await.is_ok() {
                let state = changed.borrow_and_update().clone();
                if !deliver(&sender, waker.as_ref(), Update::PowerProfile(state)) {
                    break;
                }
            }
        };
        let switch = async {
            while let Some(name) = requests.recv().await {
                // The new profile comes back as a change; a refusal changes
                // nothing on screen and is logged.
                if let Err(error) = daemon.set(&name).await {
                    tracing::warn!("power profile: switching to {name}: {error}");
                }
            }
        };
        let (watched, (), ()) = tokio::join!(daemon.watch(&states), forward, switch);
        if let Err(error) = watched {
            tracing::warn!("power profile: the daemon stopped answering: {error}");
        }
    }

    async fn workspaces(
        sender: mpsc::Sender<Update>,
        mut focus: UnboundedReceiver<Workspace>,
        waker: Option<ShellWaker>,
    ) {
        let run = async {
            let hyprland = Hyprland::from_env().await?;
            let (mut state, mut events) = hyprland.subscribe().await?;
            if !deliver(&sender, waker.as_ref(), Update::Workspaces(state.clone())) {
                return Ok(());
            }
            loop {
                tokio::select! {
                    event = events.next() => {
                        let Some(event) = event? else { break };
                        if state.apply(event)
                            && !deliver(&sender, waker.as_ref(), Update::Workspaces(state.clone()))
                        {
                            break;
                        }
                    }
                    Some(workspace) = focus.recv() => {
                        // A refused switch changes nothing on screen, which
                        // is the truth; it is only logged.
                        if let Err(error) = hyprland.focus(&workspace).await {
                            tracing::warn!("workspaces: {error}");
                        }
                    }
                }
            }
            Ok::<_, lyr_wm::hyprland::Error>(())
        };
        if let Err(error) = run.await {
            tracing::warn!("workspaces: {error}");
        }
    }
}
