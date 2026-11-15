//! The boundary to the operating system: image picking, saving, sharing and the back gesture.
//!
//! Everything platform-specific lives behind [`Platform`]; results come back asynchronously as
//! [`PlatformEvent`]s in the UI thread. `cfg(target_os = "android")` code belongs in this directory only.

#[cfg(target_os = "android")]
mod android;
mod desktop;
#[cfg(any(target_os = "android", test))]
mod native_events;

use std::path::PathBuf;

pub use desktop::DesktopPlatform;

/// Platform services. All methods return immediately; the outcome arrives as a [`PlatformEvent`].
pub trait Platform {
    /// Lets the user choose images; `max` is a hint for the picker's selection limit. Answers with
    /// [`PlatformEvent::ImagesPicked`] (an empty list means "cancelled") or [`PlatformEvent::PickFailed`].
    /// The answer may contain more than `max` paths: the controller cuts the list and tells the user.
    fn pick_images(&self, max: usize);
    /// Stores a JPEG in the gallery. Answers with [`PlatformEvent::Saved`].
    fn save_to_gallery(&self, jpeg: Vec<u8>);
    /// Whether [`Platform::share`] can open a share dialog. Checked before rendering, so a platform without
    /// one answers at once instead of after an export.
    fn share_supported(&self) -> bool;
    /// Whether the system closed the previous process while a collage was open.
    fn restored_after_close(&self) -> bool;
    /// Reports whether a collage is currently open for saved-state detection.
    fn set_collage_open(&self, open: bool);
    /// Opens the system share dialog for a JPEG. Answers with [`PlatformEvent::Shared`].
    fn share(&self, jpeg: Vec<u8>);
    /// Closes the app, e.g. after a back gesture on the start screen that the app did not use.
    fn close_app(&self);
}

/// Something that happened outside the app's own UI.
#[derive(Debug, Clone, PartialEq)]
pub enum PlatformEvent {
    /// Readable copies and the count of failed copies; empty paths and zero failures means cancelled.
    ImagesPicked { paths: Vec<PathBuf>, failed: usize },
    /// Picking was not possible; carries a message for the user.
    PickFailed(String),
    /// Result of saving to the gallery; the error is a message for the user.
    Saved(Result<(), String>),
    /// Result of sharing; the error is a message for the user.
    Shared(Result<(), String>),
    /// The system back button or gesture was used.
    BackPressed,
}

/// Delivers `event` to the controller in the UI thread. May be called from any thread.
pub(crate) fn post_event(event: PlatformEvent) {
    crate::controller::post(move |controller| controller.handle_platform_event(event));
}
