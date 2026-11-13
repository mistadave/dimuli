//! The only place that changes the [`AppState`]: reacts to UI callbacks and platform events, then refreshes the
//! UI from the new state.
//!
//! Rules: the state is borrowed only for the duration of a single transition, never across a call into Slint,
//! another callback or `invoke_from_event_loop`.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::{Rc, Weak};

use collage_core::MAX_IMAGES;
use slint::ComponentHandle;

use crate::images;
use crate::platform::{Platform, PlatformEvent};
use crate::state::{AppState, BackOutcome, Insets};
use crate::texts;
use crate::ui::{AppWindow, Backend, Theme};
use crate::view::Models;

thread_local! {
    /// The controller of this (UI) thread, so events from other threads can find it.
    static CURRENT: RefCell<Weak<Controller>> = const { RefCell::new(Weak::new()) };
}

/// Runs `action` with the controller in the UI thread. May be called from any thread.
/// Silently dropped (with a log entry) if the event loop or controller is gone, e.g. during shutdown.
pub(crate) fn post(action: impl FnOnce(&Controller) + Send + 'static) {
    let result = slint::invoke_from_event_loop(move || {
        let controller = CURRENT.with(|current| current.borrow().upgrade());
        match controller {
            Some(controller) => action(&controller),
            None => log::warn!("event dropped: no controller"),
        }
    });
    if let Err(error) = result {
        log::error!("cannot post event to the UI thread: {error}");
    }
}

pub(crate) struct Controller {
    window: slint::Weak<AppWindow>,
    platform: Rc<dyn Platform>,
    state: RefCell<AppState>,
    models: Models,
}

impl Controller {
    /// Creates the controller and connects all callbacks of `window` to it.
    pub(crate) fn install(window: &AppWindow, platform: Rc<dyn Platform>) -> Rc<Self> {
        let controller = Rc::new(Self {
            window: window.as_weak(),
            platform,
            state: RefCell::new(AppState::new()),
            models: Models::new(),
        });
        CURRENT.with(|current| *current.borrow_mut() = Rc::downgrade(&controller));
        // One source for the touch-target size: the theme. Rust needs it for divider hit-testing.
        controller.state.borrow_mut().touch_target = window.global::<Theme>().get_touch_target();
        controller.models.attach(window);
        controller.connect(&window.global::<Backend>());
        controller.refresh();
        controller
    }

    fn connect(self: &Rc<Self>, backend: &Backend<'_>) {
        // Each closure owns a clone of the controller; the controller only holds a weak window handle,
        // so there is no reference cycle.
        macro_rules! on {
            ($register:ident, |$c:ident $(, $arg:ident)*| $body:expr) => {{
                let $c = self.clone();
                backend.$register(move |$($arg),*| $body);
            }};
        }

        on!(on_pick_images, |c| c.pick_images());
        on!(on_back_requested, |c| c.update(|s| {
            // The in-app back button exists only in the editor, so the outcome does not matter.
            s.request_back();
        }));
        on!(on_back_confirmed, |c| c.update(AppState::confirm_back));
        on!(on_back_cancelled, |c| c.update(AppState::cancel_back));
        on!(on_save_requested, |c| c
            .update(|s| s.show_notice(texts::EXPORT_UNAVAILABLE)));
        on!(on_share_requested, |c| c
            .update(|s| s.show_notice(texts::EXPORT_UNAVAILABLE)));
        on!(on_dismiss_message, |c| c.update(AppState::dismiss_message));

        on!(on_canvas_resized, |c, w, h| c
            .update(|s| s.set_canvas(w, h)));
        on!(on_canvas_pressed, |c, x, y| c.update(|s| s.press(x, y)));
        on!(on_canvas_moved, |c, x, y| c.update(|s| s.move_to(x, y)));
        on!(on_canvas_released, |c, x, y| c.update(|s| s.release(x, y)));
        on!(on_canvas_cancelled, |c| c.update(AppState::cancel_gesture));
        on!(on_canvas_long_pressed, |c| c.update(AppState::long_press));
        on!(on_canvas_scrolled, |c, delta, x, y| c
            .update(|s| s.scroll(delta, x, y)));
        on!(on_canvas_pinch_started, |c| c
            .update(AppState::pinch_started));
        on!(on_canvas_pinched, |c, scale, x, y| c
            .update(|s| s.pinch(scale, x, y)));

        on!(on_template_selected, |c, i| c
            .update(|s| s.select_template(i)));
        on!(on_aspect_selected, |c, i| c.update(|s| s.select_aspect(i)));
        on!(on_spacing_changed, |c, v| c.update(|s| s.set_spacing(v)));
        on!(on_padding_changed, |c, v| c.update(|s| s.set_padding(v)));
        on!(on_corner_radius_changed, |c, v| c
            .update(|s| s.set_corner_radius(v)));
        on!(on_background_selected, |c, i| c
            .update(|s| s.select_background(i)));
    }

    /// Applies one state transition and refreshes the UI.
    fn update(&self, transition: impl FnOnce(&mut AppState)) {
        transition(&mut self.state.borrow_mut());
        self.refresh();
    }

    fn refresh(&self) {
        let Some(window) = self.window.upgrade() else {
            return;
        };
        let snapshot = self.models.snapshot(&self.state.borrow());
        self.models.apply(&window, snapshot);
    }

    fn pick_images(&self) {
        self.update(|s| s.dismiss_message());
        self.platform.pick_images(MAX_IMAGES);
    }

    /// Reacts to something the platform reported.
    pub(crate) fn handle_platform_event(&self, event: PlatformEvent) {
        match event {
            PlatformEvent::ImagesPicked(paths) => self.start_loading(paths),
            PlatformEvent::PickFailed(message) => {
                log::error!("picking images failed: {message}");
                self.update(|s| s.show_error(message));
            }
            PlatformEvent::Saved(result) => self.show_result(result, texts::SAVED),
            PlatformEvent::Shared(result) => self.show_result(result, texts::SHARED),
            PlatformEvent::InsetsChanged {
                top,
                bottom,
                left,
                right,
            } => self.update(|s| {
                s.set_insets(Insets {
                    top,
                    bottom,
                    left,
                    right,
                })
            }),
            PlatformEvent::BackPressed => {
                let outcome = self.state.borrow_mut().request_back();
                self.refresh();
                if outcome == BackOutcome::NotHandled {
                    self.platform.close_app();
                }
            }
        }
    }

    fn show_result(&self, result: Result<(), String>, success: &'static str) {
        match result {
            Ok(()) => self.update(|s| s.show_notice(success)),
            Err(message) => {
                log::error!("platform action failed: {message}");
                self.update(|s| s.show_error(message));
            }
        }
    }

    /// Decodes the picked images in the background, then builds the collage.
    fn start_loading(&self, mut paths: Vec<PathBuf>) {
        if paths.is_empty() {
            return; // cancelled by the user
        }
        let truncated = paths.len() > MAX_IMAGES;
        paths.truncate(MAX_IMAGES);

        if !self.state.borrow_mut().begin_loading() {
            return;
        }
        self.refresh();

        let spawned = images::load_in_background(paths, move |results| {
            post(move |controller| {
                controller.update(|s| s.finish_loading(results, truncated));
            });
        });
        if let Err(error) = spawned {
            log::error!("cannot start image loader: {error}");
            self.update(AppState::loading_failed);
        }
    }
}
