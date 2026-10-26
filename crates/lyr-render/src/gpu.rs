//! The GPU the bar ran on, remembered between runs through crcbl-app's
//! `gpu_choice`: the first surface opens on the GPU an earlier run saved,
//! and on Vulkan loads only that GPU's driver, which spares a hybrid
//! laptop's other driver at every start. The program keeps the saved text
//! where it likes ([`remember`]); what to save and when is decided here.

use std::sync::{Arc, Mutex, PoisonError};

use crcbl_app::backend::GpuBackend;
use crcbl_app::gpu_choice::ChoiceOutcome;
pub use crcbl_app::gpu_choice::GpuChoice;
use crcbl_app::present::Presenter;

/// Saves a choice where the program keeps it.
type Save = Arc<dyn Fn(&GpuChoice) + Send + Sync>;

/// The choice an earlier run saved, and how to save a new one.
static REMEMBERED: Mutex<Option<(Option<GpuChoice>, Save)>> = Mutex::new(None);

/// Has the first surface open on `saved`, the choice an earlier run saved
/// (`None` for none or one that could not be read), and hands `save` the
/// choice to keep for the next run whenever there is a new one. Without
/// it, surfaces open as crcbl chooses and nothing is saved.
pub fn remember(saved: Option<GpuChoice>, save: impl Fn(&GpuChoice) + Send + Sync + 'static) {
    *REMEMBERED.lock().unwrap_or_else(PoisonError::into_inner) = Some((saved, Arc::new(save)));
}

/// The saved choice to open the first surface on.
pub(crate) fn saved() -> Option<GpuChoice> {
    REMEMBERED
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .as_ref()
        .and_then(|(saved, _)| saved.clone())
}

/// What to do once the first surface has opened.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Next {
    Nothing,
    /// Save the presenter's choice.
    Save,
    /// Find its Vulkan driver, off the start-up path, and save it with it.
    Locate,
    /// Both: save it now, and again with its driver once found.
    SaveThenLocate,
}

/// What follows an open that did `outcome` with the saved choice and
/// opened on `chosen`: a choice not given or no longer holding is replaced,
/// and a Vulkan choice without its driver gets one, as crcbl-app's
/// `gpu_choice` module asks. One overridden (`CRCBL_ADAPTER`, the loader's
/// own variables) or for another backend is left as it was saved.
fn next(outcome: &ChoiceOutcome, chosen: &GpuChoice) -> Next {
    let locate = chosen.backend() == GpuBackend::Vulkan && chosen.vulkan_driver().is_none();
    match (outcome, locate) {
        (ChoiceOutcome::NotGiven | ChoiceOutcome::Stale(_), false) => Next::Save,
        (ChoiceOutcome::NotGiven | ChoiceOutcome::Stale(_), true) => Next::SaveThenLocate,
        (ChoiceOutcome::Used, true) => Next::Locate,
        (ChoiceOutcome::Used, false) | (ChoiceOutcome::Ignored(_), _) => Next::Nothing,
    }
}

/// Follows the first surface's open on `presenter`: saves its choice, and
/// finds its Vulkan driver on a thread of its own, as [`next`] decides.
pub(crate) fn opened(presenter: &Presenter) {
    let Some(save) = REMEMBERED
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .as_ref()
        .map(|(_, save)| save.clone())
    else {
        return;
    };
    let chosen = presenter.gpu_choice().clone();
    let next = next(presenter.gpu_choice_outcome(), &chosen);
    if matches!(next, Next::Save | Next::SaveThenLocate) {
        save(&chosen);
    }
    if matches!(next, Next::Locate | Next::SaveThenLocate) {
        // It loads every installed driver: slow, and the reason the next
        // start does not have to.
        std::thread::spawn(move || {
            let mut located = chosen;
            if located.locate_vulkan_driver() {
                save(&located);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use crcbl_app::backend::GpuBackend;
    use crcbl_app::gpu_choice::{ChoiceOutcome, GpuChoice, Ignored};
    use crcbl_hal::{AdapterId, AdapterInfo, BackendKind, DeviceType};

    use super::{Next, next};

    fn choice(backend: GpuBackend) -> GpuChoice {
        GpuChoice::of(
            backend,
            &AdapterInfo {
                id: AdapterId(0),
                name: "Intel(R) Graphics (RPL-U)".to_owned(),
                vendor_id: 0x8086,
                device_id: 0xa7a1,
                device_type: DeviceType::Integrated,
                driver: String::new(),
                backend: BackendKind::Vulkan,
                luid: None,
            },
        )
    }

    #[test]
    fn a_new_or_stale_choice_is_saved_and_vulkan_s_driver_found() {
        let vulkan = choice(GpuBackend::Vulkan);
        let located = vulkan
            .clone()
            .with_vulkan_driver("/usr/share/vulkan/icd.d/intel_icd.x86_64.json".into());
        let stale = ChoiceOutcome::Stale("its adapter is gone".to_owned());
        assert_eq!(
            next(&ChoiceOutcome::NotGiven, &vulkan),
            Next::SaveThenLocate
        );
        assert_eq!(next(&stale, &vulkan), Next::SaveThenLocate);
        assert_eq!(next(&stale, &located), Next::Save);
        assert_eq!(
            next(&ChoiceOutcome::NotGiven, &choice(GpuBackend::Dx12)),
            Next::Save,
            "only Vulkan has a driver to find"
        );
    }

    #[test]
    fn a_used_choice_is_kept_and_given_its_driver_once() {
        let vulkan = choice(GpuBackend::Vulkan);
        let located = vulkan
            .clone()
            .with_vulkan_driver("/usr/share/vulkan/icd.d/intel_icd.x86_64.json".into());
        assert_eq!(next(&ChoiceOutcome::Used, &located), Next::Nothing);
        assert_eq!(next(&ChoiceOutcome::Used, &vulkan), Next::Locate);
    }

    #[test]
    fn an_ignored_choice_is_left_as_it_was_saved() {
        let overridden =
            ChoiceOutcome::Ignored(Ignored::Overridden("the adapter pin (CRCBL_ADAPTER)"));
        assert_eq!(
            next(&overridden, &choice(GpuBackend::Vulkan)),
            Next::Nothing
        );
    }
}
