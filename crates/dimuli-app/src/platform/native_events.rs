//! Maps results reported by the Kotlin side to [`PlatformEvent`]s.
//!
//! Pure functions, so the mapping is covered by host tests; the JNI glue in `android.rs` stays thin.

use std::path::PathBuf;

use super::PlatformEvent;

/// Builds the event for the paths Kotlin copied into the cache. An empty list means the user
/// cancelled; empty strings are dropped so they never reach the decoder as a path.
pub(super) fn images_picked_event(paths: Vec<String>) -> PlatformEvent {
    PlatformEvent::ImagesPicked(
        paths
            .into_iter()
            .filter(|path| !path.is_empty())
            .map(PathBuf::from)
            .collect(),
    )
}

/// Builds the event for a photo picker that could not be used (not started or no copy succeeded).
pub(super) fn pick_failed_event() -> PlatformEvent {
    PlatformEvent::PickFailed(crate::texts::PICK_FAILED.to_owned())
}

/// Builds the event for the result of a MediaStore save reported by Kotlin.
pub(super) fn saved_event(ok: bool) -> PlatformEvent {
    PlatformEvent::Saved(if ok {
        Ok(())
    } else {
        Err(crate::texts::SAVE_FAILED.to_owned())
    })
}

/// Builds the event for the result of opening the share sheet reported by Kotlin.
pub(super) fn shared_event(ok: bool) -> PlatformEvent {
    PlatformEvent::Shared(if ok {
        Ok(())
    } else {
        Err(crate::texts::SHARE_FAILED.to_owned())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picked_paths_become_images_picked_event() {
        let event = images_picked_event(vec![
            "/c/picked/img-0".to_owned(),
            "/c/picked/img-1".to_owned(),
            "/c/picked/img-2".to_owned(),
        ]);
        assert_eq!(
            event,
            PlatformEvent::ImagesPicked(vec![
                PathBuf::from("/c/picked/img-0"),
                PathBuf::from("/c/picked/img-1"),
                PathBuf::from("/c/picked/img-2"),
            ])
        );
    }

    #[test]
    fn picked_empty_list_means_cancelled() {
        assert_eq!(
            images_picked_event(Vec::new()),
            PlatformEvent::ImagesPicked(Vec::new())
        );
    }

    #[test]
    fn picked_empty_strings_are_dropped() {
        let event = images_picked_event(vec![String::new(), "/c/picked/img-0".to_owned()]);
        assert_eq!(
            event,
            PlatformEvent::ImagesPicked(vec![PathBuf::from("/c/picked/img-0")])
        );
    }

    #[test]
    fn picked_failure_carries_pick_failed_text() {
        assert_eq!(
            pick_failed_event(),
            PlatformEvent::PickFailed(crate::texts::PICK_FAILED.to_owned())
        );
    }

    #[test]
    fn saved_ok_maps_to_saved_ok() {
        assert_eq!(saved_event(true), PlatformEvent::Saved(Ok(())));
    }

    #[test]
    fn saved_failure_carries_save_failed_text() {
        assert_eq!(
            saved_event(false),
            PlatformEvent::Saved(Err(crate::texts::SAVE_FAILED.to_owned()))
        );
    }

    #[test]
    fn shared_ok_maps_to_shared_ok() {
        assert_eq!(shared_event(true), PlatformEvent::Shared(Ok(())));
    }

    #[test]
    fn shared_failure_carries_share_failed_text() {
        assert_eq!(
            shared_event(false),
            PlatformEvent::Shared(Err(crate::texts::SHARE_FAILED.to_owned()))
        );
    }
}
