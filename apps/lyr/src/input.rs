//! Pointer input on a bar: where the pointer is, what a click or a wheel step
//! lands on, and carrying out the action it asks for.

use crcbl_shell::{ButtonState, PointerButton, ScrollDelta, ShellEvent};
use lyr_config::{Builtin, Config, Module};
use lyr_core::draw::Target;
use lyr_core::hit::Regions;
use lyr_core::pointer::{self, Action, Press};

use crate::exec;
use crate::modules::Data;
use crate::sources::Commands;

/// Device pixels of touchpad scrolling that make one wheel step. A policy
/// choice: crcbl leaves the conversion to the application.
const PIXELS_PER_STEP: f64 = 50.0;

/// One bar's pointer state.
#[derive(Debug, Default)]
pub struct Pointer {
    /// Where the pointer is over the bar, in logical pixels, or `None` when
    /// it is elsewhere.
    position: Option<(f32, f32)>,
    /// Touchpad scrolling not yet amounting to a whole step.
    pixels: f64,
}

impl Pointer {
    /// Applies a pointer event for this bar at `scale` and returns the
    /// presses it makes, in order.
    pub fn handle(&mut self, event: &ShellEvent, scale: f32) -> Vec<Press> {
        let logical = |x: f64, y: f64| (x as f32 / scale, y as f32 / scale);
        match *event {
            ShellEvent::PointerMotion { abs: Some(at), .. } => {
                self.position = Some(logical(at.x, at.y));
                Vec::new()
            }
            ShellEvent::PointerFocus {
                entered, position, ..
            } => {
                self.position = position.filter(|_| entered).map(|at| logical(at.x, at.y));
                Vec::new()
            }
            ShellEvent::Button {
                button,
                state: ButtonState::Pressed,
                ..
            } => match button {
                PointerButton::Left => vec![Press::Left],
                PointerButton::Right => vec![Press::Right],
                PointerButton::Middle => vec![Press::Middle],
                _ => Vec::new(),
            },
            ShellEvent::Wheel {
                delta, position, ..
            } => {
                if let Some(at) = position {
                    self.position = Some(logical(at.x, at.y));
                }
                self.steps(delta)
            }
            _ => Vec::new(),
        }
    }

    /// The wheel steps `delta` makes: one per detent, or one per
    /// [`PIXELS_PER_STEP`] of touchpad travel.
    fn steps(&mut self, delta: ScrollDelta) -> Vec<Press> {
        let step = |up: bool| {
            if up {
                Press::ScrollUp
            } else {
                Press::ScrollDown
            }
        };
        match delta {
            ScrollDelta::Lines { y, .. } if y != 0.0 => vec![step(y > 0.0)],
            ScrollDelta::Lines { .. } => Vec::new(),
            ScrollDelta::Pixels { y, .. } => {
                self.pixels += y;
                let mut presses = Vec::new();
                while self.pixels.abs() >= PIXELS_PER_STEP {
                    presses.push(step(self.pixels > 0.0));
                    self.pixels -= PIXELS_PER_STEP.copysign(self.pixels);
                }
                presses
            }
        }
    }

    /// Where the pointer is, in logical pixels, or `None` when it is
    /// elsewhere.
    pub const fn position(&self) -> Option<(f32, f32)> {
        self.position
    }

    /// What the pointer is over in `regions`.
    pub fn target<'a, T>(&self, regions: &'a Regions<T>) -> Option<&'a T> {
        let (x, y) = self.position?;
        regions.hit(x, y)
    }
}

/// Carries out the action `press` on `target` asks for. `workspaces` are the
/// workspaces the bar's buttons stand for, and `monitor` the bar's output.
/// Returns the built-ins the main loop carries out itself: opening a
/// window, and do-not-disturb, which lives with the notification daemon.
#[cfg_attr(not(target_os = "linux"), allow(unused_variables))]
pub fn act(
    target: &Target<Module>,
    press: Press,
    config: &Config,
    data: &Data,
    workspaces: &[i64],
    monitor: Option<&str>,
    commands: &Commands,
) -> Option<Builtin> {
    let action = pointer::action(target, press, config)?;
    match action {
        Action::Exec(command) => {
            if let Err(error) = exec::spawn(&command) {
                tracing::warn!("`{command}` did not start: {error}");
            }
        }
        #[cfg(target_os = "linux")]
        Action::Builtin(
            builtin @ (lyr_config::Builtin::CycleProfile | lyr_config::Builtin::CycleProfileBack),
        ) => {
            use lyr_core::power_profile::{Direction, cycle};
            let profiles = data.power_profile.as_ref()?;
            let direction = if builtin == lyr_config::Builtin::CycleProfile {
                Direction::Next
            } else {
                Direction::Previous
            };
            if let Some(next) = cycle(&profiles.available, &profiles.active.name, direction) {
                commands.set_profile(next.to_owned());
            }
        }
        Action::Builtin(
            builtin @ (Builtin::OpenAudioMenu
            | Builtin::OpenBluetoothMenu
            | Builtin::OpenClockPanel
            | Builtin::OpenPowerMenu
            | Builtin::OpenWifiMenu
            | Builtin::ToggleDnd),
        ) => {
            return Some(builtin);
        }
        #[cfg(windows)]
        Action::Builtin(
            builtin @ (Builtin::ToggleMute | Builtin::VolumeUp | Builtin::VolumeDown),
        ) => {
            use lyr_sources::audio::windows::{Flow, Which};
            // The mic's bindings act on the input, the volume's on the output.
            let input = matches!(
                target,
                Target::Module(Module::Mic) | Target::Button(Module::Mic, _)
            );
            let step = if input {
                config.mic.scroll_step
            } else {
                config.volume.scroll_step
            };
            let step = i32::try_from(step).unwrap_or(i32::MAX);
            let which = Which::Default(if input { Flow::Input } else { Flow::Output });
            match builtin {
                Builtin::ToggleMute => commands.toggle_mute(which),
                Builtin::VolumeUp => commands.step_volume(which, step),
                _ => commands.step_volume(which, -step),
            }
        }
        Action::Builtin(builtin) => {
            tracing::warn!("`{}` is not implemented yet", builtin.name());
        }
        #[cfg(target_os = "linux")]
        Action::FocusWorkspace(index) => {
            let workspace = workspaces.get(index).and_then(|id| {
                data.workspaces
                    .as_ref()?
                    .workspaces()
                    .find(|workspace| workspace.id == *id)
            });
            if let Some(workspace) = workspace {
                commands.focus(workspace.clone());
            }
        }
        #[cfg(target_os = "linux")]
        Action::StepWorkspace(direction) => {
            use lyr_core::pointer::Direction;
            use lyr_wm::hyprland::Step;
            let state = data.workspaces.as_ref()?;
            let step = match direction {
                Direction::Next => Step::Next,
                Direction::Previous => Step::Previous,
            };
            let own = (!config.workspaces.all_outputs)
                .then_some(monitor)
                .flatten();
            if let Some(workspace) = state.step(step, own) {
                commands.focus(workspace.clone());
            }
        }
        #[cfg(not(target_os = "linux"))]
        Action::FocusWorkspace(_) | Action::StepWorkspace(_) => {}
    }
    None
}

#[cfg(test)]
mod tests {
    use crcbl_shell::ScrollDelta;
    use lyr_core::pointer::Press;

    use super::{PIXELS_PER_STEP, Pointer};

    #[test]
    fn a_detent_is_one_step_and_its_sign_is_the_direction() {
        let mut pointer = Pointer::default();
        assert_eq!(
            pointer.steps(ScrollDelta::Lines { x: 0.0, y: 1.0 }),
            [Press::ScrollUp]
        );
        assert_eq!(
            pointer.steps(ScrollDelta::Lines { x: 0.0, y: -1.0 }),
            [Press::ScrollDown]
        );
        assert!(
            pointer
                .steps(ScrollDelta::Lines { x: 3.0, y: 0.0 })
                .is_empty()
        );
    }

    #[test]
    fn touchpad_travel_adds_up_to_steps() {
        let mut pointer = Pointer::default();
        let pixels = |y| ScrollDelta::Pixels { x: 0.0, y };
        assert!(pointer.steps(pixels(PIXELS_PER_STEP * 0.6)).is_empty());
        assert_eq!(
            pointer.steps(pixels(PIXELS_PER_STEP * 0.6)),
            [Press::ScrollUp]
        );
        assert_eq!(
            pointer.steps(pixels(-PIXELS_PER_STEP * 2.3)),
            [Press::ScrollDown, Press::ScrollDown]
        );
    }
}
