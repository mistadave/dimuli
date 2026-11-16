//! The only place that changes the [`AppState`]: reacts to UI callbacks and platform events, then refreshes the
//! UI from the new state.
//!
//! Rules: the state is borrowed only for the duration of a single transition, never across a call into Slint,
//! another callback or `invoke_from_event_loop`.

use std::cell::{Cell, RefCell};
use std::panic::AssertUnwindSafe;
use std::path::PathBuf;
use std::rc::{Rc, Weak};
use std::time::Duration;

use collage_core::{CoreError, ExportOptions, MAX_IMAGES, render};
use slint::ComponentHandle;

use crate::images;
use crate::platform::{Platform, PlatformEvent};
use crate::state::{AppState, BackOutcome, Busy, ExportAction, ExportJob, Screen};
use crate::ui::{AppWindow, Backend, Theme};
use crate::view::Models;

/// How long copying and decoding picked images may take before the busy overlay appears; shorter waits
/// finish without any flash.
const BUSY_OVERLAY_DELAY: Duration = Duration::from_millis(250);

/// How long a success message stays visible. Errors and notices stay until dismissed.
const MESSAGE_HIDE_AFTER: Duration = Duration::from_secs(3);

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
    /// Hides success messages; restarted when a new success message appears.
    message_timer: slint::Timer,
    /// Reveals the busy overlay when copying/loading takes longer than [`BUSY_OVERLAY_DELAY`].
    overlay_timer: slint::Timer,
    /// Sequence number of the success message the timer was started for.
    armed_seq: Cell<Option<u64>>,
    collage_open_pushed: Cell<Option<bool>>,
}

impl Controller {
    /// Creates the controller and connects all callbacks of `window` to it.
    pub(crate) fn install(window: &AppWindow, platform: Rc<dyn Platform>) -> Rc<Self> {
        let controller = Rc::new(Self {
            window: window.as_weak(),
            platform,
            state: RefCell::new(AppState::new()),
            models: Models::new(),
            message_timer: slint::Timer::default(),
            overlay_timer: slint::Timer::default(),
            armed_seq: Cell::new(None),
            collage_open_pushed: Cell::new(None),
        });
        CURRENT.with(|current| *current.borrow_mut() = Rc::downgrade(&controller));
        // One source for the touch-target size: the theme. Rust needs it for divider hit-testing.
        let touch_target = window.global::<Theme>().get_touch_target();
        let share_supported = controller.platform.share_supported();
        let restored = controller.platform.restored_after_close();
        {
            let mut state = controller.state.borrow_mut();
            state.touch_target = touch_target;
            state.share_supported = share_supported;
            if restored {
                state.show_restart_notice();
            }
        }
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
        on!(on_save_requested, |c| c.start_export(ExportAction::Save));
        on!(on_share_requested, |c| c.start_export(ExportAction::Share));
        on!(on_dismiss_message, |c| c.update(AppState::dismiss_message));

        on!(on_canvas_area_resized, |c, w, h| c
            .update(|s| s.set_canvas_area(w, h)));
        on!(on_canvas_pressed, |c, x, y| c.update(|s| s.press(x, y)));
        on!(on_canvas_moved, |c, x, y| c
            .update_gesture(|s| s.move_to(x, y)));
        on!(on_canvas_released, |c, x, y| c.update(|s| s.release(x, y)));
        on!(on_canvas_cancelled, |c| c.update(AppState::cancel_gesture));
        on!(on_canvas_long_pressed, |c| c.update(AppState::long_press));
        on!(on_canvas_scrolled, |c, delta, x, y| c
            .update_gesture(|s| s.scroll(delta, x, y)));
        on!(on_canvas_pinch_started, |c| c
            .update(AppState::pinch_started));
        on!(on_canvas_pinched, |c, scale, x, y| c
            .update_gesture(|s| s.pinch(scale, x, y)));

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

    /// Fast path for pan, pinch and wheel events: rewrites only the changed cell rows. Falls back to a full
    /// refresh when the transition raised a message or changed the screen, so those still reach the UI.
    fn update_gesture(&self, transition: impl FnOnce(&mut AppState)) {
        let (seq, screen) = {
            let state = self.state.borrow();
            (state.message_seq, state.screen)
        };
        let changed = {
            let mut state = self.state.borrow_mut();
            transition(&mut state);
            state.message_seq != seq || state.screen != screen
        };
        if changed {
            self.refresh();
            return;
        }
        self.models.fill_gesture_rows(&self.state.borrow());
        self.models.apply_gesture_rows();
    }

    fn refresh(&self) {
        let Some(window) = self.window.upgrade() else {
            return;
        };
        let (snapshot, success_seq, open, awaiting_reveal) = {
            let state = self.state.borrow();
            (
                self.models.snapshot(&state),
                state.success_seq(),
                state.screen == Screen::Editor,
                matches!(state.busy, Some(Busy::Copying | Busy::Decoding)),
            )
        };
        // The reveal timer is only useful while the overlay is still hidden and a wait is going on.
        if !awaiting_reveal {
            self.overlay_timer.stop();
        }
        self.models.apply(&window, snapshot);
        if self.collage_open_pushed.get() != Some(open) {
            self.collage_open_pushed.set(Some(open));
            self.platform.set_collage_open(open);
        }
        self.arm_message_timer(success_seq);
    }

    /// Starts the hide timer for a new success message; stops it when there is none.
    fn arm_message_timer(&self, success_seq: Option<u64>) {
        match success_seq {
            Some(seq) if self.armed_seq.get() != Some(seq) => {
                self.armed_seq.set(Some(seq));
                // The closure holds no `Rc`: it finds the controller through `CURRENT` (UI thread).
                self.message_timer.start(
                    slint::TimerMode::SingleShot,
                    MESSAGE_HIDE_AFTER,
                    move || {
                        let controller = CURRENT.with(|current| current.borrow().upgrade());
                        if let Some(controller) = controller {
                            controller.update(|s| s.expire_message(seq));
                        }
                    },
                );
            }
            Some(_) => {}
            None => {
                self.message_timer.stop();
                self.armed_seq.set(None);
            }
        }
    }

    fn pick_images(&self) {
        // The borrow ends with this statement; the platform call below runs without it.
        let started = self.state.borrow_mut().begin_picking();
        self.refresh();
        if started {
            self.platform.pick_images(MAX_IMAGES);
        }
    }

    /// Reacts to something the platform reported.
    pub(crate) fn handle_platform_event(&self, event: PlatformEvent) {
        match event {
            PlatformEvent::ImagesPicked { paths, failed } => self.start_loading(paths, failed),
            PlatformEvent::PickCopying => {
                let seq = self.state.borrow_mut().pick_copying();
                if let Some(seq) = seq {
                    // The closure holds no `Rc`: it finds the controller through `CURRENT` (UI thread).
                    self.overlay_timer.start(
                        slint::TimerMode::SingleShot,
                        BUSY_OVERLAY_DELAY,
                        move || {
                            let controller = CURRENT.with(|current| current.borrow().upgrade());
                            if let Some(controller) = controller {
                                controller.update(|s| s.reveal_busy(seq));
                            }
                        },
                    );
                }
            }
            PlatformEvent::PickFailed(message) => {
                log::error!("picking images failed: {message}");
                self.update(|s| s.pick_failed(message));
            }
            PlatformEvent::Saved(result) => self.update(|s| s.save_finished(result)),
            PlatformEvent::Shared(result) => self.update(|s| s.share_finished(result)),
            PlatformEvent::BackPressed => {
                let outcome = self.state.borrow_mut().request_back();
                self.refresh();
                if outcome == BackOutcome::NotHandled {
                    self.platform.close_app();
                }
            }
        }
    }

    /// Decodes the picked images in the background, then builds the collage.
    fn start_loading(&self, mut paths: Vec<PathBuf>, failed: usize) {
        if paths.is_empty() {
            // Cancelled by the user: release the waiting overlay.
            self.update(AppState::pick_cancelled);
            return;
        }
        let truncated = paths.len() > MAX_IMAGES;
        paths.truncate(MAX_IMAGES);

        if !self.state.borrow_mut().begin_loading() {
            return;
        }
        self.refresh();

        let spawned = images::load_in_background(paths, move |results| {
            post(move |controller| {
                controller.update(|s| s.finish_loading(results, truncated, failed));
            });
        });
        if let Err(error) = spawned {
            log::error!("cannot start image loader: {error}");
            self.update(AppState::loading_failed);
        }
    }

    /// Renders the collage in the background, then hands the JPEG to the platform. The busy overlay ends when
    /// the render finishes; the platform's answer arrives later: `Saved` shows a notice, `Shared` only
    /// shows a message on failure (D-11).
    fn start_export(&self, action: ExportAction) {
        let Some(job) = self.state.borrow_mut().begin_export(action) else {
            return;
        };
        self.refresh();

        let action = job.action;
        let spawned = std::thread::Builder::new()
            .name("exporter".into())
            .spawn(move || {
                let result = render_job(&job);
                post(move |controller| controller.finish_export(action, result));
            });
        if let Err(error) = spawned {
            log::error!("cannot start export thread: {error}");
            self.update(AppState::export_failed);
        }
    }

    fn finish_export(&self, action: ExportAction, result: Result<Vec<u8>, CoreError>) {
        let jpeg = self.state.borrow_mut().finish_export(result);
        self.refresh();
        if let Some(jpeg) = jpeg {
            match action {
                ExportAction::Save => self.platform.save_to_gallery(jpeg),
                ExportAction::Share => self.platform.share(jpeg),
            }
        }
    }
}

/// Renders on the export thread. A panic (e.g. in a decoder) becomes an error, so the busy state always ends.
fn render_job(job: &ExportJob) -> Result<Vec<u8>, CoreError> {
    std::panic::catch_unwind(AssertUnwindSafe(|| {
        render(&job.collage, &job.sources, ExportOptions::default())
    }))
    .unwrap_or_else(|_| Err(CoreError::Encode("renderer panicked".to_owned())))
}
