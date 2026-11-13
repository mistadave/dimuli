//! Desktop implementation used for development: images come from the command line.

use std::path::PathBuf;

use super::{Platform, PlatformEvent, post_event};
use crate::texts;

/// Desktop platform. "Picking" returns the paths given on the command line
/// (`cargo run -p dimuli-app -- a.jpg b.jpg`); there is no file dialog.
pub struct DesktopPlatform {
    paths: Vec<PathBuf>,
}

impl DesktopPlatform {
    /// Creates the platform with the image paths passed at startup.
    pub fn new(paths: Vec<PathBuf>) -> Self {
        Self { paths }
    }
}

/// The event answering a pick request.
fn pick_event(paths: &[PathBuf], max: usize) -> PlatformEvent {
    if paths.is_empty() {
        PlatformEvent::PickFailed(texts::NO_IMAGE_ARGUMENTS.to_owned())
    } else {
        PlatformEvent::ImagesPicked(paths.iter().take(max).cloned().collect())
    }
}

impl Platform for DesktopPlatform {
    fn pick_images(&self, max: usize) {
        post_event(pick_event(&self.paths, max));
    }

    // Exporting arrives with milestone 4.
    fn save_to_gallery(&self, _jpeg: Vec<u8>) {
        post_event(PlatformEvent::Saved(Err(
            texts::EXPORT_UNAVAILABLE.to_owned()
        )));
    }

    fn share(&self, _jpeg: Vec<u8>) {
        post_event(PlatformEvent::Shared(Err(
            texts::EXPORT_UNAVAILABLE.to_owned()
        )));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths(count: usize) -> Vec<PathBuf> {
        (0..count)
            .map(|i| PathBuf::from(format!("{i}.jpg")))
            .collect()
    }

    #[test]
    fn pick_returns_given_paths() {
        assert_eq!(
            pick_event(&paths(2), 9),
            PlatformEvent::ImagesPicked(paths(2))
        );
    }

    #[test]
    fn pick_returns_at_most_max_paths() {
        assert_eq!(
            pick_event(&paths(12), 9),
            PlatformEvent::ImagesPicked(paths(9))
        );
        assert_eq!(
            pick_event(&paths(3), 1),
            PlatformEvent::ImagesPicked(paths(1))
        );
    }

    #[test]
    fn pick_without_arguments_explains_what_to_do() {
        assert!(matches!(
            pick_event(&[], 9),
            PlatformEvent::PickFailed(message) if message.contains("cargo run")
        ));
    }
}
