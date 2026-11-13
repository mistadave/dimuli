//! The boundary to the operating system: image picking, saving, sharing, insets and the back gesture.
//!
//! Everything platform-specific lives behind [`Platform`]; results come back asynchronously as
//! [`PlatformEvent`]s in the UI thread. `cfg(target_os = "android")` code belongs in this directory only.

mod desktop;

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
    /// Opens the system share dialog for a JPEG. Answers with [`PlatformEvent::Shared`].
    fn share(&self, jpeg: Vec<u8>);
    /// Closes the app, e.g. after a back gesture on the start screen that the app did not use.
    fn close_app(&self);
}

/// Something that happened outside the app's own UI.
#[derive(Debug, Clone, PartialEq)]
pub enum PlatformEvent {
    /// Images chosen by the user (readable copies in the app cache on Android). Empty if cancelled.
    ImagesPicked(Vec<PathBuf>),
    /// Picking was not possible; carries a message for the user.
    PickFailed(String),
    /// Result of saving to the gallery; the error is a message for the user.
    Saved(Result<(), String>),
    /// Result of sharing; the error is a message for the user.
    Shared(Result<(), String>),
    /// New safe-area insets in logical pixels.
    InsetsChanged {
        /// Top inset.
        top: f32,
        /// Bottom inset.
        bottom: f32,
        /// Left inset.
        left: f32,
        /// Right inset.
        right: f32,
    },
    /// The system back button or gesture was used.
    BackPressed,
}

/// Delivers `event` to the controller in the UI thread. May be called from any thread.
pub(crate) fn post_event(event: PlatformEvent) {
    crate::controller::post(move |controller| controller.handle_platform_event(event));
}
