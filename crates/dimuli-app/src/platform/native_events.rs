//! Maps results reported by the Kotlin side to [`PlatformEvent`]s.
//!
//! Pure functions, so the mapping is covered by host tests; the JNI glue in `android.rs` stays thin.

use std::path::PathBuf;

use super::PlatformEvent;

/// Builds the event for the paths Kotlin copied into the cache. An empty list means the user
/// cancelled when no copies failed; empty strings count as failed copies.
pub(super) fn images_picked_event(paths: Vec<String>, failed: i32) -> PlatformEvent {
    let failed =
        usize::try_from(failed).unwrap_or(0) + paths.iter().filter(|path| path.is_empty()).count();
    let paths: Vec<PathBuf> = paths
        .into_iter()
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .collect();
    if paths.is_empty() && failed > 0 {
        PlatformEvent::PickFailed(crate::texts::NO_IMAGES_LOADED.to_owned())
    } else {
        PlatformEvent::ImagesPicked { paths, failed }
    }
}

/// Builds the event for a photo picker that could not be started.
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
    fn picked_paths_carry_the_failed_copy_count() {
        assert_eq!(
            images_picked_event(vec!["/c/img-0".into()], 2),
            PlatformEvent::ImagesPicked {
                paths: vec![PathBuf::from("/c/img-0")],
                failed: 2
            }
        );
    }

    #[test]
    fn all_copies_failed_maps_to_no_images_loaded() {
        assert_eq!(
            images_picked_event(vec![], 3),
            PlatformEvent::PickFailed(crate::texts::NO_IMAGES_LOADED.to_owned())
        );
    }

    #[test]
    fn negative_failed_count_counts_as_zero() {
        assert_eq!(
            images_picked_event(vec!["/c/img-0".into()], -5),
            PlatformEvent::ImagesPicked {
                paths: vec![PathBuf::from("/c/img-0")],
                failed: 0
            }
        );
    }

    #[test]
    fn picked_paths_become_images_picked_event() {
        let event = images_picked_event(
            vec![
                "/c/picked/img-0".to_owned(),
                "/c/picked/img-1".to_owned(),
                "/c/picked/img-2".to_owned(),
            ],
            0,
        );
        assert_eq!(
            event,
            PlatformEvent::ImagesPicked {
                paths: vec![
                    PathBuf::from("/c/picked/img-0"),
                    PathBuf::from("/c/picked/img-1"),
                    PathBuf::from("/c/picked/img-2"),
                ],
                failed: 0
            }
        );
    }

    #[test]
    fn picked_empty_list_means_cancelled() {
        assert_eq!(
            images_picked_event(Vec::new(), 0),
            PlatformEvent::ImagesPicked {
                paths: Vec::new(),
                failed: 0
            }
        );
    }

    #[test]
    fn picked_empty_strings_are_dropped() {
        let event = images_picked_event(vec![String::new(), "/c/picked/img-0".to_owned()], 0);
        assert_eq!(
            event,
            PlatformEvent::ImagesPicked {
                paths: vec![PathBuf::from("/c/picked/img-0")],
                failed: 1
            }
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
