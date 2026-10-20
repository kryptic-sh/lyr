//! The Linux compositor lyr runs under, whichever backend speaks to it.

use std::ffi::OsString;

use crate::{State, Workspace, hyprland, niri, sway};

/// What went wrong finding or talking to the compositor.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(
        "no compositor lyr knows is running here: none of \
         HYPRLAND_INSTANCE_SIGNATURE, SWAYSOCK or NIRI_SOCKET is set"
    )]
    NotRunning,
    #[error(transparent)]
    Hyprland(#[from] hyprland::Error),
    #[error(transparent)]
    Sway(#[from] sway::Error),
    #[error(transparent)]
    Niri(#[from] niri::Error),
}

/// A running Hyprland, sway or niri.
#[derive(Clone, Debug)]
pub enum Compositor {
    Hyprland(hyprland::Hyprland),
    Sway(sway::Sway),
    Niri(niri::Niri),
}

impl Compositor {
    /// Finds the compositor this process runs under: the first of Hyprland,
    /// sway and niri whose environment variable is set.
    ///
    /// # Errors
    ///
    /// [`Error::NotRunning`] when none is, or the found compositor's own
    /// error when it cannot be reached.
    pub async fn from_env() -> Result<Self, Error> {
        Self::from_vars(|name| std::env::var_os(name)).await
    }

    /// [`Compositor::from_env`], reading the variables through `var`.
    async fn from_vars(var: impl Fn(&str) -> Option<OsString>) -> Result<Self, Error> {
        match hyprland::Hyprland::from_vars(&var).await {
            Err(hyprland::Error::NotRunning) => {}
            found => return Ok(Self::Hyprland(found?)),
        }
        match sway::Sway::from_vars(&var) {
            Err(sway::Error::NotRunning) => {}
            found => return Ok(Self::Sway(found?)),
        }
        match niri::Niri::from_vars(&var) {
            Err(niri::Error::NotRunning) => Err(Error::NotRunning),
            found => Ok(Self::Niri(found?)),
        }
    }

    /// The compositor's name, for the log.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Hyprland(_) => "Hyprland",
            Self::Sway(_) => "sway",
            Self::Niri(_) => "niri",
        }
    }

    /// Subscribes to changes and reads the current state.
    ///
    /// # Errors
    ///
    /// When the compositor cannot be reached or refuses.
    pub async fn subscribe(&self) -> Result<(State, Changes), Error> {
        Ok(match self {
            Self::Hyprland(hyprland) => {
                let (tracker, events) = hyprland.subscribe().await?;
                let state = tracker.state().clone();
                (state, Changes::Hyprland(Box::new((tracker, events))))
            }
            Self::Sway(sway) => {
                let (state, events) = sway.subscribe().await?;
                (state, Changes::Sway(events))
            }
            Self::Niri(niri) => {
                let (state, events) = niri.subscribe().await?;
                (state, Changes::Niri(Box::new(events)))
            }
        })
    }

    /// Switches to `workspace`.
    ///
    /// # Errors
    ///
    /// When the compositor cannot be reached or refuses.
    pub async fn focus(&self, workspace: &Workspace) -> Result<(), Error> {
        match self {
            Self::Hyprland(hyprland) => hyprland.focus(workspace).await?,
            Self::Sway(sway) => sway.focus(workspace).await?,
            Self::Niri(niri) => niri.focus(workspace).await?,
        }
        Ok(())
    }

    /// Ends the session: the power menu's Log out.
    ///
    /// # Errors
    ///
    /// When the compositor cannot be reached or refuses.
    pub async fn exit(&self) -> Result<(), Error> {
        match self {
            Self::Hyprland(hyprland) => hyprland.exit().await?,
            Self::Sway(sway) => sway.exit().await?,
            Self::Niri(niri) => niri.exit().await?,
        }
        Ok(())
    }
}

/// The changes a subscription delivers.
#[derive(Debug)]
pub enum Changes {
    /// Hyprland's events and the tracker they are folded into.
    Hyprland(Box<(hyprland::Tracker, hyprland::Events)>),
    Sway(sway::Events),
    Niri(Box<niri::Events>),
}

impl Changes {
    /// The next state that differs from the last one handed on. `Ok(None)`
    /// means the compositor closed the connection.
    ///
    /// # Errors
    ///
    /// When the connection fails or a message is malformed.
    pub async fn next(&mut self) -> Result<Option<State>, Error> {
        match self {
            Self::Hyprland(hyprland) => {
                let (tracker, events) = &mut **hyprland;
                while let Some(event) = events.next().await? {
                    if tracker.apply(event) {
                        return Ok(Some(tracker.state().clone()));
                    }
                }
                Ok(None)
            }
            Self::Sway(events) => Ok(events.next().await?),
            Self::Niri(events) => Ok(events.next().await?),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::{Compositor, Error};

    /// The variables `pairs` set, and no others.
    fn vars(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<OsString> {
        let pairs: Vec<(String, OsString)> = pairs
            .iter()
            .map(|&(name, value)| (name.to_owned(), value.into()))
            .collect();
        move |name| {
            pairs
                .iter()
                .find(|(key, _)| key == name)
                .map(|(_, value)| value.clone())
        }
    }

    async fn found(pairs: &[(&str, &str)]) -> Result<&'static str, Error> {
        Compositor::from_vars(vars(pairs))
            .await
            .map(|compositor| compositor.name())
    }

    #[tokio::test]
    async fn the_first_compositor_whose_variable_is_set_wins() {
        assert!(matches!(found(&[]).await, Err(Error::NotRunning)));
        assert_eq!(found(&[("NIRI_SOCKET", "/n")]).await.ok(), Some("niri"));
        assert_eq!(
            found(&[("NIRI_SOCKET", "/n"), ("SWAYSOCK", "/s")])
                .await
                .ok(),
            Some("sway")
        );
        // Hyprland is asked for its config dialect when found, so a
        // signature with no socket behind it is Hyprland's own error, not a
        // reason to try sway.
        let missing = tempfile::tempdir().expect("tempdir");
        let runtime = missing.path().to_str().expect("a UTF-8 path");
        assert!(matches!(
            found(&[
                ("HYPRLAND_INSTANCE_SIGNATURE", "gone"),
                ("XDG_RUNTIME_DIR", runtime),
                ("SWAYSOCK", "/s"),
            ])
            .await,
            Err(Error::Hyprland(_))
        ));
    }
}
