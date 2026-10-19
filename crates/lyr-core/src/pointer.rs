//! What a click or a scroll on the bar asks for
//! (`docs/plan/02-architecture.md#data-flow-of-a-click`).
//!
//! The pointer lands on a [`Target`] of the last layout; the module's
//! `[<module>.on]` bindings, or the workspace buttons' own behaviour, turn it
//! into an [`Action`] for the caller to carry out.

use lyr_config::{Binding, Builtin, Config, Module, On};

use crate::draw::Target;

/// One pointer input on the bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Press {
    Left,
    Right,
    Middle,
    /// One step of the wheel away from the user.
    ScrollUp,
    /// One step of the wheel towards the user.
    ScrollDown,
}

/// Which way a scroll steps through the workspaces.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    Next,
    Previous,
}

/// What the bar is asked to do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    Builtin(Builtin),
    /// Run this through the platform shell.
    Exec(String),
    /// Switch to the workspace of the button at this index in the row.
    FocusWorkspace(usize),
    /// Step to the next or previous workspace.
    StepWorkspace(Direction),
}

/// The action `press` on `target` asks for, or `None` when nothing is bound.
#[must_use]
pub fn action(target: &Target<Module>, press: Press, config: &Config) -> Option<Action> {
    match (target, press) {
        (Target::Button(Module::Workspaces, index), Press::Left) => {
            Some(Action::FocusWorkspace(*index))
        }
        (Target::Button(Module::Workspaces, _) | Target::Module(Module::Workspaces), scroll) => {
            // Scrolling away from the user goes back, as a list scrolls.
            let direction = match scroll {
                Press::ScrollUp => Direction::Previous,
                Press::ScrollDown => Direction::Next,
                Press::Left | Press::Right | Press::Middle => return None,
            };
            config
                .workspaces
                .scroll
                .then_some(Action::StepWorkspace(direction))
        }
        (Target::Module(module) | Target::Button(module, _), press) => {
            let on = bindings(*module, config)?;
            let binding = match press {
                Press::Left => &on.click,
                Press::Right => &on.click_right,
                Press::Middle => &on.click_middle,
                Press::ScrollUp => &on.scroll_up,
                Press::ScrollDown => &on.scroll_down,
            };
            match binding.as_ref()? {
                Binding::Builtin(builtin) => Some(Action::Builtin(*builtin)),
                Binding::Exec { exec } => Some(Action::Exec(exec.clone())),
            }
        }
    }
}

/// A module's `[<module>.on]` table; the workspaces and the tray have none.
const fn bindings(module: Module, config: &Config) -> Option<&On> {
    Some(match module {
        Module::Power => &config.power.on,
        Module::Battery => &config.battery.on,
        Module::PowerProfile => &config.power_profile.on,
        Module::Network => &config.network.on,
        Module::Bluetooth => &config.bluetooth.on,
        Module::Volume => &config.volume.on,
        Module::Mic => &config.mic.on,
        Module::Clock => &config.clock.on,
        Module::Notifications => &config.notifications.on,
        Module::Workspaces | Module::Tray => return None,
    })
}

#[cfg(test)]
mod tests {
    use lyr_config::{Builtin, Config, Module, Platform, from_sources};

    use super::{Action, Direction, Press, action};
    use crate::draw::Target;

    fn defaults() -> Config {
        from_sources(None, Platform::Linux).expect("defaults")
    }

    #[test]
    fn a_workspace_button_focuses_its_workspace() {
        let config = defaults();
        assert_eq!(
            action(&Target::Button(Module::Workspaces, 2), Press::Left, &config),
            Some(Action::FocusWorkspace(2))
        );
        assert_eq!(
            action(
                &Target::Button(Module::Workspaces, 2),
                Press::Right,
                &config
            ),
            None
        );
    }

    #[test]
    fn scrolling_the_workspaces_needs_the_switch() {
        let mut config = defaults();
        let workspaces = Target::Module(Module::Workspaces);
        assert_eq!(action(&workspaces, Press::ScrollDown, &config), None);
        config.workspaces.scroll = true;
        assert_eq!(
            action(&workspaces, Press::ScrollDown, &config),
            Some(Action::StepWorkspace(Direction::Next))
        );
        assert_eq!(
            action(
                &Target::Button(Module::Workspaces, 0),
                Press::ScrollUp,
                &config
            ),
            Some(Action::StepWorkspace(Direction::Previous))
        );
    }

    #[test]
    fn a_module_follows_its_bindings() {
        let config = defaults();
        let volume = Target::Module(Module::Volume);
        assert_eq!(
            action(&volume, Press::Left, &config),
            Some(Action::Builtin(Builtin::ToggleMute))
        );
        assert_eq!(
            action(&volume, Press::Right, &config),
            Some(Action::Builtin(Builtin::OpenAudioMenu))
        );
        assert_eq!(
            action(&volume, Press::ScrollUp, &config),
            Some(Action::Builtin(Builtin::VolumeUp))
        );
        assert_eq!(action(&volume, Press::Middle, &config), None);
        assert_eq!(
            action(&Target::Module(Module::Battery), Press::Left, &config),
            None
        );
    }

    #[test]
    fn an_exec_binding_carries_its_command() {
        let config = from_sources(
            Some((
                std::path::Path::new("config.toml"),
                "[clock.on]\nclick_middle = { exec = \"date\" }\n",
            )),
            Platform::Linux,
        )
        .expect("valid");
        assert_eq!(
            action(&Target::Module(Module::Clock), Press::Middle, &config),
            Some(Action::Exec("date".to_owned()))
        );
    }
}
