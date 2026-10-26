//! Where the bar keeps the GPU it ran on ([`lyr_render::gpu`]): `gpu-choice`
//! in hjkl-xdg's state directory, beside the logs. A file that is missing
//! or cannot be read is no saved choice, so the bar opens as crcbl chooses
//! and saves what it opened on.

use std::io;
use std::path::{Path, PathBuf};

use lyr_render::gpu::GpuChoice;

/// The file's name in the state directory.
const FILE: &str = "gpu-choice";

/// Opens the first surface on the GPU the last run saved, and saves the
/// one it opens on.
pub fn remember() {
    let path = match hjkl_xdg::state_dir("lyr") {
        Ok(dir) => dir.join(FILE),
        Err(error) => {
            tracing::warn!("no state directory to keep the GPU in: {error}");
            return;
        }
    };
    let saved = read(&path);
    lyr_render::gpu::remember(saved, move |choice| match save(&path, choice) {
        Ok(()) => tracing::info!("saved the GPU for the next start in {}", path.display()),
        Err(error) => tracing::warn!("could not save the GPU in {}: {error}", path.display()),
    });
}

/// The choice saved at `path`, if there is one this build can read.
fn read(path: &Path) -> Option<GpuChoice> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return None,
        Err(error) => {
            tracing::warn!("could not read {}: {error}", path.display());
            return None;
        }
    };
    text.parse()
        .inspect_err(|error| tracing::info!("{}: {error}; choosing the GPU again", path.display()))
        .ok()
}

/// Saves `choice` at `path` whole: written beside it, then moved over it,
/// so a bar starting meanwhile never reads half of one.
fn save(path: &Path, choice: &GpuChoice) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let partial = PathBuf::from(format!("{}.partial", path.display()));
    std::fs::write(&partial, choice.to_string())?;
    std::fs::rename(&partial, path)
}

#[cfg(test)]
mod tests {
    use super::{read, save};

    const SAVED: &str = "crcbl-gpu-choice 1\nbackend vk\nvendor 0x8086\ndevice 0xa7a1\nname Intel(R) Graphics (RPL-U)\n";

    #[test]
    fn a_saved_choice_is_read_back() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = dir.path().join("state/gpu-choice");
        let choice = SAVED.parse().expect("a choice");
        save(&path, &choice).expect("saved");
        assert_eq!(read(&path), Some(choice));
        assert!(
            !dir.path().join("state/gpu-choice.partial").exists(),
            "moved into place"
        );
    }

    #[test]
    fn a_missing_or_unreadable_file_is_no_choice() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = dir.path().join("gpu-choice");
        assert_eq!(read(&path), None);
        std::fs::write(&path, "not a choice").expect("written");
        assert_eq!(read(&path), None);
    }
}
