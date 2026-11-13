//! The single app state and its transitions.
//!
//! Only the controller owns and mutates an [`AppState`], always in the UI thread. All methods here are plain
//! Rust without any window, so the behavior can be unit-tested.

use collage_core::{
    AspectRatio, CellId, CellTransform, Collage, CoreError, DividerId, ImageSource, MAX_IMAGES,
    Size, Style, Template, Vec2, templates_for,
};

use crate::images::{ImageStore, LoadedImage};
use crate::texts;
use crate::view::{self, BACKGROUND_PRESETS, SwapHighlight};

/// Movement in pixels after which a press becomes a drag instead of a (long) press.
const DRAG_SLOP_PX: f32 = 10.0;

/// Which screen is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Screen {
    Start,
    Editor,
}

/// Severity of a [`Message`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MessageKind {
    Error,
    Notice,
}

/// A message for the user, shown in a dismissible banner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Message {
    pub(crate) kind: MessageKind,
    pub(crate) text: String,
}

/// Safe-area insets in logical pixels (all zero on the desktop).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct Insets {
    pub(crate) top: f32,
    pub(crate) bottom: f32,
    pub(crate) left: f32,
    pub(crate) right: f32,
}

/// What the finger or mouse on the canvas is currently doing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Gesture {
    Idle,
    /// Pointer is down; may turn into a pan or (after the long-press time) a swap.
    Pressed {
        cell: Option<CellId>,
        start: (f32, f32),
    },
    /// Dragging the image inside a cell. `origin` is the framing before the pan, so a pinch that starts
    /// with this finger can undo it.
    Panning {
        cell: CellId,
        last: (f32, f32),
        origin: CellTransform,
    },
    /// Long-press picked up `source`; releasing over another cell swaps them.
    Swapping {
        source: CellId,
        target: Option<CellId>,
    },
    /// Dragging a divider by its handle. `grab` is the pointer position minus the divider center (pixels),
    /// so the divider follows the finger exactly and does not drift when the core stops it at a limit.
    DraggingDivider {
        id: DividerId,
        grab: (f32, f32),
    },
    /// Two-finger gesture in progress; single-pointer moves are ignored. `cell` is the zoomed cell, chosen
    /// at the first pinch update and kept for the whole gesture.
    Pinching {
        cell: Option<CellId>,
    },
}

/// Whether the system back gesture was used by the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BackOutcome {
    /// The app reacted (dialog opened or closed).
    Handled,
    /// Nothing to go back to: the platform should close the app.
    NotHandled,
}

/// What an export does with the rendered JPEG.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExportAction {
    /// Store it in the gallery.
    Save,
    /// Open the system share dialog.
    Share,
}

/// Why the UI is blocked by the busy overlay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Busy {
    /// Images are being decoded.
    Loading,
    /// The collage is being rendered.
    Exporting,
}

impl Busy {
    /// Text shown below the progress indicator.
    pub(crate) fn text(self) -> &'static str {
        match self {
            Busy::Loading => texts::LOADING,
            Busy::Exporting => texts::EXPORTING,
        }
    }
}

/// Everything the background thread needs to render the collage; owns its data (no shared state).
#[derive(Debug)]
pub(crate) struct ExportJob {
    pub(crate) action: ExportAction,
    pub(crate) collage: Collage,
    pub(crate) sources: Vec<ImageSource>,
}

/// Everything the UI shows.
pub(crate) struct AppState {
    pub(crate) screen: Screen,
    pub(crate) collage: Option<Collage>,
    pub(crate) images: ImageStore,
    /// `Some` while a background task runs; the overlay then blocks all input.
    pub(crate) busy: Option<Busy>,
    pub(crate) message: Option<Message>,
    pub(crate) templates: Vec<Template>,
    pub(crate) template_index: Option<usize>,
    /// Canvas size in logical pixels as reported by the UI; zero until the first layout.
    pub(crate) canvas: Size,
    /// Side of the square divider handles in pixels (the UI theme's touch-target size); set by the controller.
    pub(crate) touch_target: f32,
    pub(crate) insets: Insets,
    pub(crate) confirm_back: bool,
    pub(crate) gesture: Gesture,
}

impl AppState {
    pub(crate) fn new() -> Self {
        Self {
            screen: Screen::Start,
            collage: None,
            images: ImageStore::default(),
            busy: None,
            message: None,
            templates: Vec::new(),
            template_index: None,
            canvas: Size::default(),
            touch_target: 0.0,
            insets: Insets::default(),
            confirm_back: false,
            gesture: Gesture::Idle,
        }
    }

    // --- messages ---

    pub(crate) fn show_error(&mut self, text: impl Into<String>) {
        self.message = Some(Message {
            kind: MessageKind::Error,
            text: text.into(),
        });
    }

    pub(crate) fn show_notice(&mut self, text: impl Into<String>) {
        self.message = Some(Message {
            kind: MessageKind::Notice,
            text: text.into(),
        });
    }

    pub(crate) fn dismiss_message(&mut self) {
        self.message = None;
    }

    /// Logs a core error and tells the user something went wrong.
    fn core_failed(&mut self, action: &str, error: CoreError) {
        log::error!("{action} failed: {error}");
        self.show_error(texts::GENERIC_ERROR);
    }

    // --- loading ---

    /// Marks loading as started. `false` if loading is already running or the editor is open.
    /// Only possible on the start screen: a late pick result must not wipe an open collage.
    pub(crate) fn begin_loading(&mut self) -> bool {
        if self.busy.is_some() || self.screen != Screen::Start {
            return false;
        }
        self.busy = Some(Busy::Loading);
        self.message = None;
        self.images.clear();
        true
    }

    /// Ends loading without a result (e.g. the loader thread could not start).
    pub(crate) fn loading_failed(&mut self) {
        self.busy = None;
        self.images.clear();
        self.show_error(texts::GENERIC_ERROR);
    }

    /// Builds the collage from the loaded images; unreadable ones are skipped with a notice.
    /// `truncated` tells that more images were chosen than [`MAX_IMAGES`].
    pub(crate) fn finish_loading(
        &mut self,
        results: Vec<Result<LoadedImage, CoreError>>,
        truncated: bool,
    ) {
        self.busy = None;
        let total = results.len();
        let metas: Vec<_> = results
            .into_iter()
            .filter_map(Result::ok)
            .filter_map(|loaded| self.images.insert(loaded))
            .collect();
        let skipped = total - metas.len();

        let templates = templates_for(metas.len());
        let collage = templates
            .first()
            .ok_or(CoreError::NoImages)
            .and_then(|template| Collage::new(&metas, template));
        match collage {
            Ok(collage) => {
                self.collage = Some(collage);
                self.templates = templates;
                self.template_index = Some(0);
                self.screen = Screen::Editor;
                self.gesture = Gesture::Idle;
                self.message = load_notice(skipped, truncated);
            }
            Err(error) => {
                self.images.clear();
                if !metas.is_empty() {
                    log::error!("cannot create collage: {error}");
                }
                self.show_error(if metas.is_empty() {
                    texts::NO_IMAGES_LOADED
                } else {
                    texts::GENERIC_ERROR
                });
            }
        }
    }

    // --- export ---

    /// Starts an export: only in the editor and when nothing else is running. Returns the data for the
    /// render thread, or `None` if the export must not start. The collage is cloned, so the render does not
    /// depend on what happens in the UI afterwards.
    pub(crate) fn begin_export(&mut self, action: ExportAction) -> Option<ExportJob> {
        if self.busy.is_some() || self.screen != Screen::Editor {
            return None;
        }
        let collage = self.collage.clone()?;
        let sources = self.images.sources(&collage);
        self.busy = Some(Busy::Exporting);
        self.message = None;
        self.confirm_back = false;
        // A finger that is still down must not keep editing once the overlay appears.
        self.gesture = Gesture::Idle;
        Some(ExportJob {
            action,
            collage,
            sources,
        })
    }

    /// Ends the export. On success returns the JPEG for the platform; on failure shows an error for the user.
    /// The busy state ends here, not when the platform answers: saving and sharing report back through
    /// [`PlatformEvent`](crate::platform::PlatformEvent)s and must not block the editor.
    pub(crate) fn finish_export(&mut self, result: Result<Vec<u8>, CoreError>) -> Option<Vec<u8>> {
        self.busy = None;
        match result {
            Ok(jpeg) => Some(jpeg),
            Err(error) => {
                log::error!("export failed: {error}");
                self.show_error(texts::export_error(&error));
                None
            }
        }
    }

    /// The export could not start (e.g. the render thread could not be spawned).
    pub(crate) fn export_failed(&mut self) {
        self.busy = None;
        self.show_error(texts::GENERIC_ERROR);
    }

    // --- navigation ---

    /// The back button or system back gesture: closes the confirmation dialog if open, otherwise asks for
    /// confirmation in the editor. On the start screen (also while loading) there is nothing to go back to.
    /// While exporting, back is swallowed: neither the app closes nor the collage is discarded mid-render.
    pub(crate) fn request_back(&mut self) -> BackOutcome {
        if self.busy == Some(Busy::Exporting) {
            BackOutcome::Handled
        } else if self.confirm_back {
            self.confirm_back = false;
            BackOutcome::Handled
        } else if self.screen == Screen::Editor {
            self.confirm_back = true;
            BackOutcome::Handled
        } else {
            BackOutcome::NotHandled
        }
    }

    pub(crate) fn cancel_back(&mut self) {
        self.confirm_back = false;
    }

    /// Discards the collage and returns to the start screen.
    pub(crate) fn confirm_back(&mut self) {
        self.confirm_back = false;
        self.screen = Screen::Start;
        self.collage = None;
        self.templates.clear();
        self.template_index = None;
        self.images.clear();
        self.gesture = Gesture::Idle;
        // Messages refer to the discarded collage.
        self.message = None;
    }

    pub(crate) fn set_insets(&mut self, insets: Insets) {
        let sane = |v: f32| if v.is_finite() { v.max(0.0) } else { 0.0 };
        self.insets = Insets {
            top: sane(insets.top),
            bottom: sane(insets.bottom),
            left: sane(insets.left),
            right: sane(insets.right),
        };
    }

    pub(crate) fn set_canvas(&mut self, width: f32, height: f32) {
        if width.is_finite() && height.is_finite() && width >= 0.0 && height >= 0.0 {
            self.canvas = Size::new(width, height);
        }
    }

    // --- gestures ---

    // Every edit entry point below starts with `if self.busy.is_some() { return; }`: while a background task
    // runs, the overlay blocks the touches, but the state must not depend on that alone (wheel, late timers).

    fn scale(&self) -> f32 {
        view::pixels_per_unit(self.canvas)
    }

    /// The cell under a canvas pixel position.
    fn cell_at_pixels(&self, x: f32, y: f32) -> Option<CellId> {
        let point = view::point_to_units(x, y, self.scale())?;
        self.collage.as_ref()?.cell_at(point)
    }

    /// Cells to highlight for the current gesture.
    pub(crate) fn swap_highlight(&self) -> SwapHighlight {
        match self.gesture {
            Gesture::Swapping { source, target } => SwapHighlight {
                source: Some(source),
                target,
            },
            _ => SwapHighlight::default(),
        }
    }

    /// The divider whose handle is under a canvas pixel position.
    fn divider_at_pixels(&self, x: f32, y: f32) -> Option<DividerId> {
        let collage = self.collage.as_ref()?;
        let handles = view::divider_handles(collage, self.canvas, self.touch_target);
        view::pick_handle(&handles, Vec2::new(x, y))
    }

    /// Center of a divider in canvas pixels.
    fn divider_center_pixels(&self, id: DividerId) -> Option<(f32, f32)> {
        let divider = self
            .collage
            .as_ref()?
            .dividers()
            .into_iter()
            .find(|d| d.id == id)?;
        let center = view::rect_to_pixels(divider.rect, self.scale()).center();
        Some((center.x, center.y))
    }

    /// Current framing of a cell.
    fn cell_transform(&self, cell: CellId) -> Option<CellTransform> {
        let index = usize::try_from(cell.0).ok()?;
        Some(self.collage.as_ref()?.cells().get(index)?.transform)
    }

    pub(crate) fn press(&mut self, x: f32, y: f32) {
        if self.busy.is_some() {
            return;
        }
        // Handles take precedence over the cells below them.
        if let Some(id) = self.divider_at_pixels(x, y) {
            let (cx, cy) = self.divider_center_pixels(id).unwrap_or((x, y));
            self.gesture = Gesture::DraggingDivider {
                id,
                grab: (x - cx, y - cy),
            };
            return;
        }
        self.gesture = Gesture::Pressed {
            cell: self.cell_at_pixels(x, y),
            start: (x, y),
        };
    }

    pub(crate) fn move_to(&mut self, x: f32, y: f32) {
        if self.busy.is_some() {
            return;
        }
        match self.gesture {
            Gesture::Pressed {
                cell: Some(cell),
                start,
            } if distance(start, (x, y)) > DRAG_SLOP_PX => {
                if let Some(origin) = self.cell_transform(cell) {
                    self.pan(cell, origin, start, (x, y));
                }
            }
            Gesture::Panning { cell, last, origin } => self.pan(cell, origin, last, (x, y)),
            Gesture::DraggingDivider { id, grab } => {
                // Move the divider center to where the finger wants it, measured from its current position.
                if let Some((cx, cy)) = self.divider_center_pixels(id) {
                    self.drag_divider(id, (x - grab.0 - cx, y - grab.1 - cy));
                }
            }
            Gesture::Swapping { source, .. } => {
                let target = self.cell_at_pixels(x, y).filter(|&c| c != source);
                self.gesture = Gesture::Swapping { source, target };
            }
            _ => {}
        }
    }

    /// Moves the image in `cell` by the pointer movement `from -> to` and remembers `to` as the last position.
    fn pan(&mut self, cell: CellId, origin: CellTransform, from: (f32, f32), to: (f32, f32)) {
        self.gesture = Gesture::Panning {
            cell,
            last: to,
            origin,
        };
        let scale = self.scale();
        let (Some(dx), Some(dy)) = (
            view::pixels_to_units(to.0 - from.0, scale),
            view::pixels_to_units(to.1 - from.1, scale),
        ) else {
            return;
        };
        let Some(collage) = self.collage.as_mut() else {
            return;
        };
        if let Err(error) = collage.pan(cell, Vec2::new(dx, dy)) {
            self.core_failed("pan", error);
        }
    }

    /// The long-press timer fired: picks up the pressed cell unless the pointer already moved.
    pub(crate) fn long_press(&mut self) {
        if self.busy.is_some() {
            return;
        }
        if let Gesture::Pressed {
            cell: Some(source), ..
        } = self.gesture
        {
            self.gesture = Gesture::Swapping {
                source,
                target: None,
            };
        }
    }

    pub(crate) fn release(&mut self, x: f32, y: f32) {
        if self.busy.is_some() {
            return;
        }
        let gesture = std::mem::replace(&mut self.gesture, Gesture::Idle);
        if let Gesture::Swapping { source, .. } = gesture {
            // Releasing outside every cell cancels the swap (no fallback to the last hover target).
            if let Some(target) = self.cell_at_pixels(x, y).filter(|&t| t != source) {
                self.swap(source, target);
            }
        }
    }

    pub(crate) fn cancel_gesture(&mut self) {
        self.gesture = Gesture::Idle;
    }

    fn swap(&mut self, a: CellId, b: CellId) {
        let Some(collage) = self.collage.as_mut() else {
            return;
        };
        if let Err(error) = collage.swap(a, b) {
            self.core_failed("swap", error);
        }
    }

    /// Mouse wheel: zooms the cell under the pointer. `delta` is the vertical scroll in pixels.
    pub(crate) fn scroll(&mut self, delta: f32, x: f32, y: f32) {
        if self.busy.is_some() {
            return;
        }
        if let Some(cell) = self.cell_at_pixels(x, y) {
            self.zoom_cell(cell, view::wheel_zoom_factor(delta), x, y);
        }
    }

    /// A second finger came down. If the first finger already started a pan, that pan is undone: the user
    /// meant to zoom, not to move the image.
    pub(crate) fn pinch_started(&mut self) {
        if self.busy.is_some() {
            return;
        }
        if let Gesture::Panning { cell, origin, .. } = self.gesture
            && let Some(collage) = self.collage.as_mut()
            && let Err(error) = collage.set_transform(cell, origin)
        {
            self.core_failed("undo pan", error);
        }
        self.gesture = Gesture::Pinching { cell: None };
    }

    /// Pinch: zooms by the relative `scale` the cell that was under the gesture center at the first update.
    /// The center may later drift into another cell or a gap; the zoomed cell stays the same.
    pub(crate) fn pinch(&mut self, scale: f32, x: f32, y: f32) {
        if self.busy.is_some() {
            return;
        }
        let Gesture::Pinching { cell } = self.gesture else {
            return;
        };
        let Some(cell) = cell.or_else(|| self.cell_at_pixels(x, y)) else {
            return;
        };
        self.gesture = Gesture::Pinching { cell: Some(cell) };
        self.zoom_cell(cell, scale, x, y);
    }

    /// Zooms `cell` by `factor`, keeping the image point under the canvas pixel position fixed (clamped to
    /// the cell if the position lies outside it).
    fn zoom_cell(&mut self, cell: CellId, factor: f32, x: f32, y: f32) {
        let Some(point) = view::point_to_units(x, y, self.scale()) else {
            return;
        };
        let Some(collage) = self.collage.as_mut() else {
            return;
        };
        let Some(placement) = collage.placements().into_iter().find(|p| p.cell == cell) else {
            return;
        };
        let focus = view::focus_in_rect(point, placement.rect);
        if let Err(error) = collage.zoom(placement.cell, factor, focus) {
            self.core_failed("zoom", error);
        }
    }

    /// Moves a divider by a pointer movement in pixels.
    fn drag_divider(&mut self, id: DividerId, delta: (f32, f32)) {
        let scale = self.scale();
        let (Some(dx), Some(dy)) = (
            view::pixels_to_units(delta.0, scale),
            view::pixels_to_units(delta.1, scale),
        ) else {
            return;
        };
        let Some(collage) = self.collage.as_mut() else {
            return;
        };
        if let Err(error) = collage.drag_divider(id, Vec2::new(dx, dy)) {
            self.core_failed("drag divider", error);
        }
    }

    // --- templates and style ---

    pub(crate) fn select_template(&mut self, index: i32) {
        if self.busy.is_some() {
            return;
        }
        let Some(template) = usize::try_from(index)
            .ok()
            .and_then(|i| self.templates.get(i).map(|t| (i, t.clone())))
        else {
            return;
        };
        let Some(collage) = self.collage.as_mut() else {
            return;
        };
        match collage.apply_template(&template.1) {
            Ok(()) => self.template_index = Some(template.0),
            Err(error) => self.core_failed("apply template", error),
        }
    }

    pub(crate) fn select_aspect(&mut self, index: i32) {
        if self.busy.is_some() {
            return;
        }
        let aspect = usize::try_from(index)
            .ok()
            .and_then(|i| AspectRatio::ALL.get(i).copied());
        if let (Some(aspect), Some(collage)) = (aspect, self.collage.as_mut()) {
            collage.set_aspect(aspect);
        }
    }

    fn update_style(&mut self, change: impl FnOnce(&mut Style)) {
        if let Some(collage) = self.collage.as_mut() {
            let mut style = collage.style();
            change(&mut style);
            collage.set_style(style);
        }
    }

    pub(crate) fn set_spacing(&mut self, value: f32) {
        if self.busy.is_some() {
            return;
        }
        self.update_style(|s| s.spacing = value);
    }

    pub(crate) fn set_padding(&mut self, value: f32) {
        if self.busy.is_some() {
            return;
        }
        self.update_style(|s| s.padding = value);
    }

    pub(crate) fn set_corner_radius(&mut self, value: f32) {
        if self.busy.is_some() {
            return;
        }
        self.update_style(|s| s.corner_radius = value);
    }

    pub(crate) fn select_background(&mut self, index: i32) {
        if self.busy.is_some() {
            return;
        }
        let color = usize::try_from(index)
            .ok()
            .and_then(|i| BACKGROUND_PRESETS.get(i).copied());
        if let Some(color) = color {
            self.update_style(|s| s.background = color);
        }
    }
}

fn distance(a: (f32, f32), b: (f32, f32)) -> f32 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

/// Notice after loading, if something was skipped or cut off.
fn load_notice(skipped: usize, truncated: bool) -> Option<Message> {
    let mut parts = Vec::new();
    if skipped > 0 {
        parts.push(texts::images_skipped(skipped));
    }
    if truncated {
        parts.push(texts::too_many_images(MAX_IMAGES));
    }
    (!parts.is_empty()).then(|| Message {
        kind: MessageKind::Notice,
        text: parts.join(" "),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use collage_core::collage::MAX_SPACING;
    use slint::{Rgba8Pixel, SharedPixelBuffer};

    fn loaded(width: u32, height: u32) -> LoadedImage {
        LoadedImage {
            path: "test.png".into(),
            width,
            height,
            preview: SharedPixelBuffer::<Rgba8Pixel>::new(width.min(64), height.min(64)),
        }
    }

    fn broken() -> Result<LoadedImage, CoreError> {
        Err(CoreError::InvalidImageSize {
            width: 0,
            height: 0,
        })
    }

    /// Editor state with `sizes.len()` images and a 300x300 px canvas.
    fn editor(sizes: &[(u32, u32)]) -> AppState {
        let mut state = AppState::new();
        assert!(state.begin_loading());
        state.finish_loading(
            sizes.iter().map(|&(w, h)| Ok(loaded(w, h))).collect(),
            false,
        );
        state.set_canvas(300.0, 300.0);
        state.touch_target = 48.0;
        state
    }

    fn collage(state: &AppState) -> &Collage {
        state.collage.as_ref().unwrap()
    }

    fn image_ids(state: &AppState) -> Vec<u32> {
        collage(state)
            .cells()
            .iter()
            .map(|c| c.image.id().0)
            .collect()
    }

    fn center_of_cell(state: &AppState, cell: u32) -> (f32, f32) {
        let rect = collage(state)
            .placements()
            .into_iter()
            .find(|p| p.cell == CellId(cell))
            .unwrap()
            .rect;
        let c = rect.center();
        (c.x * 300.0, c.y * 300.0)
    }

    #[test]
    fn loading_images_opens_the_editor_with_first_template() {
        let state = editor(&[(100, 50), (50, 100), (80, 80)]);
        assert_eq!(state.screen, Screen::Editor);
        assert_eq!(collage(&state).cells().len(), 3);
        assert_eq!(state.template_index, Some(0));
        assert!(!state.templates.is_empty());
        assert!(state.busy.is_none());
        assert_eq!(state.message, None);
    }

    #[test]
    fn loading_one_and_max_images_works() {
        assert_eq!(collage(&editor(&[(10, 10)])).cells().len(), 1);
        let many = vec![(10, 10); MAX_IMAGES];
        assert_eq!(collage(&editor(&many)).cells().len(), MAX_IMAGES);
    }

    #[test]
    fn loading_skips_broken_images_with_a_notice() {
        let mut state = AppState::new();
        state.begin_loading();
        state.finish_loading(
            vec![Ok(loaded(10, 10)), broken(), Ok(loaded(20, 10))],
            false,
        );
        assert_eq!(state.screen, Screen::Editor);
        assert_eq!(collage(&state).cells().len(), 2);
        let message = state.message.unwrap();
        assert_eq!(message.kind, MessageKind::Notice);
        assert!(message.text.contains("übersprungen"));
    }

    #[test]
    fn loading_only_broken_images_stays_on_start_with_error() {
        let mut state = AppState::new();
        state.begin_loading();
        state.finish_loading(vec![broken(), broken()], false);
        assert_eq!(state.screen, Screen::Start);
        assert!(state.collage.is_none());
        assert!(state.busy.is_none());
        assert_eq!(state.message.unwrap().kind, MessageKind::Error);
    }

    #[test]
    fn loading_nothing_shows_error() {
        let mut state = AppState::new();
        state.begin_loading();
        state.finish_loading(Vec::new(), false);
        assert_eq!(state.screen, Screen::Start);
        assert_eq!(state.message.unwrap().kind, MessageKind::Error);
    }

    #[test]
    fn truncation_is_reported() {
        let mut state = AppState::new();
        state.begin_loading();
        state.finish_loading(vec![Ok(loaded(10, 10))], true);
        assert!(
            state
                .message
                .unwrap()
                .text
                .contains(&MAX_IMAGES.to_string())
        );
    }

    #[test]
    fn loading_cannot_start_twice() {
        let mut state = AppState::new();
        assert!(state.begin_loading());
        assert!(!state.begin_loading());
        state.loading_failed();
        assert!(state.busy.is_none());
        assert_eq!(state.message.unwrap().kind, MessageKind::Error);
    }

    #[test]
    fn back_asks_for_confirmation_and_discards_on_confirm() {
        let mut state = editor(&[(10, 10), (10, 10)]);
        state.request_back();
        assert!(state.confirm_back);
        assert_eq!(state.screen, Screen::Editor);
        state.cancel_back();
        assert!(!state.confirm_back);
        state.request_back();
        state.confirm_back();
        assert_eq!(state.screen, Screen::Start);
        assert!(state.collage.is_none());
        assert_eq!(state.images.len(), 0);
        assert!(!state.confirm_back);
    }

    #[test]
    fn back_on_start_screen_is_not_handled_and_back_again_closes_dialog() {
        let mut state = AppState::new();
        assert_eq!(state.request_back(), BackOutcome::NotHandled);
        assert!(!state.confirm_back);
        assert!(state.begin_loading());
        assert_eq!(state.request_back(), BackOutcome::NotHandled);

        let mut state = editor(&[(10, 10)]);
        assert_eq!(state.request_back(), BackOutcome::Handled);
        assert!(state.confirm_back);
        assert_eq!(state.request_back(), BackOutcome::Handled);
        assert!(!state.confirm_back);
    }

    #[test]
    fn going_back_clears_the_editor_message() {
        let mut state = AppState::new();
        state.begin_loading();
        state.finish_loading(vec![Ok(loaded(10, 10)), broken()], false);
        assert!(state.message.is_some());
        state.request_back();
        state.confirm_back();
        assert_eq!(state.message, None);
    }

    #[test]
    fn dragging_a_cell_pans_the_image_after_slop() {
        let mut state = editor(&[(400, 100)]);
        let before = collage(&state).cells()[0].transform;
        state.press(150.0, 150.0);
        state.move_to(153.0, 150.0);
        assert_eq!(collage(&state).cells()[0].transform, before, "inside slop");
        state.move_to(190.0, 150.0);
        let after = collage(&state).cells()[0].transform;
        assert!(
            after.center().x < before.center().x,
            "dragging right reveals the left side"
        );
        assert!(matches!(state.gesture, Gesture::Panning { .. }));
        state.release(190.0, 150.0);
        assert_eq!(state.gesture, Gesture::Idle);
    }

    #[test]
    fn pressing_in_a_gap_does_not_pan() {
        let mut state = editor(&[(400, 100), (400, 100)]);
        state.set_padding(0.1);
        state.press(1.0, 1.0);
        state.move_to(100.0, 100.0);
        assert!(matches!(state.gesture, Gesture::Pressed { cell: None, .. }));
    }

    #[test]
    fn long_press_then_release_over_other_cell_swaps_images() {
        let mut state = editor(&[(100, 100), (200, 100), (100, 200)]);
        let ids_before = image_ids(&state);
        let from = center_of_cell(&state, 0);
        let to = center_of_cell(&state, 2);
        state.press(from.0, from.1);
        state.long_press();
        assert!(matches!(
            state.gesture,
            Gesture::Swapping {
                source: CellId(0),
                target: None
            }
        ));
        state.move_to(to.0, to.1);
        let highlight = state.swap_highlight();
        assert_eq!(highlight.source, Some(CellId(0)));
        assert_eq!(highlight.target, Some(CellId(2)));
        state.release(to.0, to.1);
        let ids_after = image_ids(&state);
        assert_eq!(ids_after, vec![ids_before[2], ids_before[1], ids_before[0]]);
        assert_eq!(state.gesture, Gesture::Idle);
        assert_eq!(state.swap_highlight(), SwapHighlight::default());
    }

    #[test]
    fn releasing_a_swap_on_the_same_cell_or_outside_changes_nothing() {
        let mut state = editor(&[(100, 100), (100, 100)]);
        let before = image_ids(&state);
        let p = center_of_cell(&state, 0);
        state.press(p.0, p.1);
        state.long_press();
        state.release(p.0, p.1);
        assert_eq!(image_ids(&state), before);

        state.press(p.0, p.1);
        state.long_press();
        state.move_to(-50.0, -50.0);
        state.release(-50.0, -50.0);
        assert_eq!(image_ids(&state), before);
    }

    #[test]
    fn releasing_a_swap_outside_every_cell_cancels_even_after_hovering_a_target() {
        let mut state = editor(&[(100, 100), (100, 100)]);
        state.set_padding(0.1);
        let before = image_ids(&state);
        let from = center_of_cell(&state, 0);
        let to = center_of_cell(&state, 1);
        state.press(from.0, from.1);
        state.long_press();
        state.move_to(to.0, to.1);
        assert!(state.swap_highlight().target.is_some());
        state.release(1.0, 1.0); // in the padding
        assert_eq!(image_ids(&state), before);
    }

    #[test]
    fn begin_loading_is_refused_in_the_editor() {
        let mut state = editor(&[(100, 100)]);
        assert!(!state.begin_loading());
        assert_eq!(state.images.len(), 1);
        assert!(state.busy.is_none());
    }

    #[test]
    fn long_press_after_dragging_is_ignored() {
        let mut state = editor(&[(400, 100)]);
        state.press(150.0, 150.0);
        state.move_to(200.0, 150.0);
        state.long_press();
        assert!(matches!(state.gesture, Gesture::Panning { .. }));
    }

    #[test]
    fn long_press_in_a_gap_does_nothing() {
        let mut state = editor(&[(100, 100), (100, 100)]);
        state.set_padding(0.1);
        state.press(1.0, 1.0);
        state.long_press();
        assert!(matches!(state.gesture, Gesture::Pressed { cell: None, .. }));
    }

    #[test]
    fn cancelled_gesture_returns_to_idle_without_swap() {
        let mut state = editor(&[(100, 100), (100, 100)]);
        let before = image_ids(&state);
        let p = center_of_cell(&state, 0);
        state.press(p.0, p.1);
        state.long_press();
        state.cancel_gesture();
        assert_eq!(state.gesture, Gesture::Idle);
        assert_eq!(image_ids(&state), before);
    }

    #[test]
    fn wheel_zooms_the_cell_under_the_pointer() {
        let mut state = editor(&[(100, 100), (100, 100)]);
        let p = center_of_cell(&state, 1);
        state.scroll(60.0, p.0, p.1);
        let cells = collage(&state).cells();
        assert!(cells[1].transform.zoom() > 1.0);
        assert!((cells[0].transform.zoom() - 1.0).abs() < 1e-6);
        state.scroll(-600.0, p.0, p.1);
        assert!((collage(&state).cells()[1].transform.zoom() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn wheel_outside_cells_or_with_unknown_canvas_is_ignored() {
        let mut state = editor(&[(100, 100)]);
        state.scroll(60.0, -10.0, -10.0);
        assert!((collage(&state).cells()[0].transform.zoom() - 1.0).abs() < 1e-6);
        state.canvas = Size::default();
        state.scroll(60.0, 150.0, 150.0);
        assert!((collage(&state).cells()[0].transform.zoom() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn pinch_zooms_and_ignores_single_pointer_moves() {
        let mut state = editor(&[(100, 100)]);
        state.press(150.0, 150.0);
        state.pinch_started();
        assert_eq!(state.gesture, Gesture::Pinching { cell: None });
        state.pinch(2.0, 150.0, 150.0);
        let zoom = collage(&state).cells()[0].transform.zoom();
        assert!((zoom - 2.0).abs() < 1e-3);
        state.move_to(250.0, 250.0);
        assert_eq!(
            state.gesture,
            Gesture::Pinching {
                cell: Some(CellId(0))
            }
        );
        state.pinch(f32::NAN, 150.0, 150.0);
        assert!((collage(&state).cells()[0].transform.zoom() - zoom).abs() < 1e-6);
    }

    #[test]
    fn pinch_keeps_zooming_the_first_cell_when_the_center_drifts() {
        let mut state = editor(&[(100, 100), (100, 100)]);
        let first = center_of_cell(&state, 0);
        let second = center_of_cell(&state, 1);
        state.pinch_started();
        state.pinch(2.0, first.0, first.1);
        state.pinch(1.5, second.0, second.1);
        let cells = collage(&state).cells();
        assert!((cells[0].transform.zoom() - 3.0).abs() < 1e-3);
        assert!((cells[1].transform.zoom() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn pinch_starting_in_a_gap_picks_the_cell_at_a_later_update() {
        let mut state = editor(&[(100, 100), (100, 100)]);
        state.set_padding(0.1);
        let p = center_of_cell(&state, 1);
        state.pinch_started();
        state.pinch(2.0, 1.0, 1.0);
        assert_eq!(state.gesture, Gesture::Pinching { cell: None });
        state.pinch(2.0, p.0, p.1);
        assert!((collage(&state).cells()[1].transform.zoom() - 2.0).abs() < 1e-3);
    }

    #[test]
    fn pinch_without_pinch_start_is_ignored() {
        let mut state = editor(&[(100, 100)]);
        state.pinch(2.0, 150.0, 150.0);
        assert!((collage(&state).cells()[0].transform.zoom() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn pinch_undoes_the_pan_of_its_first_finger() {
        let mut state = editor(&[(400, 100)]);
        let before = collage(&state).cells()[0].transform;
        state.press(150.0, 150.0);
        state.move_to(190.0, 150.0);
        assert_ne!(collage(&state).cells()[0].transform, before);
        state.pinch_started();
        assert_eq!(collage(&state).cells()[0].transform, before);
    }

    /// Center of the first divider's handle in pixels.
    fn handle_center(state: &AppState) -> (f32, f32) {
        let handles = view::divider_handles(collage(state), state.canvas, state.touch_target);
        let c = handles[0].1.center();
        (c.x, c.y)
    }

    #[test]
    fn dragging_a_divider_handle_moves_the_divider() {
        let mut state = editor(&[(100, 100), (100, 100)]);
        let before = collage(&state).placements();
        let (x, y) = handle_center(&state);
        state.press(x, y);
        assert!(matches!(state.gesture, Gesture::DraggingDivider { .. }));
        state.move_to(x + 30.0, y + 30.0);
        state.release(x + 30.0, y + 30.0);
        assert_ne!(collage(&state).placements(), before);
        assert_eq!(state.gesture, Gesture::Idle);
    }

    #[test]
    fn divider_stays_under_the_finger_after_hitting_a_limit() {
        let mut state = editor(&[(100, 100), (100, 100)]);
        let id = collage(&state).dividers()[0].id;
        let (x, y) = handle_center(&state);
        state.press(x + 3.0, y + 3.0);
        // Far beyond the limit: the core stops the divider.
        state.move_to(x + 1000.0, y + 1000.0);
        // Back to a reachable position: the divider keeps the grab offset of 3 px to the finger, not the
        // overshoot.
        state.move_to(x + 30.0, y + 30.0);
        let (cx, cy) = state.divider_center_pixels(id).unwrap();
        match collage(&state).dividers()[0].dir {
            collage_core::SplitDir::LeftRight => assert!((cx - (x + 27.0)).abs() < 0.5),
            collage_core::SplitDir::TopBottom => assert!((cy - (y + 27.0)).abs() < 0.5),
        }
    }

    #[test]
    fn pressing_just_outside_the_handle_pans_instead() {
        let mut state = editor(&[(400, 100), (100, 100)]);
        let (x, y) = handle_center(&state);
        let outside = state.touch_target / 2.0 + 2.0;
        // Step sideways, across the divider, just beyond the square handle.
        let (px, py) = match collage(&state).dividers()[0].dir {
            collage_core::SplitDir::LeftRight => (x - outside, y),
            collage_core::SplitDir::TopBottom => (x, y - outside),
        };
        state.press(px, py);
        assert!(matches!(
            state.gesture,
            Gesture::Pressed { cell: Some(_), .. }
        ));
        state.move_to(px + 30.0, py + 30.0);
        assert!(matches!(state.gesture, Gesture::Panning { .. }));
    }

    #[test]
    fn divider_handle_works_without_spacing() {
        let mut state = editor(&[(100, 100), (100, 100)]);
        state.set_spacing(0.0);
        let before = collage(&state).placements();
        let (x, y) = handle_center(&state);
        state.press(x, y);
        state.move_to(x + 20.0, y + 20.0);
        assert_ne!(collage(&state).placements(), before);
    }

    #[test]
    fn overlapping_handles_at_a_t_junction_pick_the_nearest() {
        let mut state = editor(&[(100, 100), (100, 100), (100, 100)]);
        state.set_spacing(0.0);
        let handles = view::divider_handles(collage(&state), state.canvas, state.touch_target);
        assert!(handles.len() >= 2);
        // Press exactly on the second handle's center; with overlap the nearest center must win.
        let (id, rect) = handles[1];
        let c = rect.center();
        state.press(c.x, c.y);
        assert!(matches!(state.gesture, Gesture::DraggingDivider { id: hit, .. } if hit == id));
    }

    #[test]
    fn divider_drag_with_unknown_canvas_or_invalid_delta_is_ignored() {
        let mut state = editor(&[(100, 100), (100, 100)]);
        let before = collage(&state).placements();
        state.drag_divider(DividerId(0), (f32::NAN, 0.0));
        state.canvas = Size::default();
        state.drag_divider(DividerId(0), (10.0, 10.0));
        assert_eq!(collage(&state).placements(), before);
        assert_eq!(state.message, None);
    }

    #[test]
    fn unknown_divider_id_reports_an_error() {
        let mut state = editor(&[(100, 100), (100, 100)]);
        state.drag_divider(DividerId(99), (10.0, 10.0));
        assert_eq!(state.message.unwrap().kind, MessageKind::Error);
    }

    #[test]
    fn selecting_a_template_changes_layout_and_marks_it_active() {
        let mut state = editor(&[(100, 100), (100, 100), (100, 100)]);
        assert!(state.templates.len() > 1);
        let before = collage(&state).placements();
        state.select_template(1);
        assert_eq!(state.template_index, Some(1));
        assert_ne!(collage(&state).placements(), before);
    }

    #[test]
    fn invalid_template_index_is_ignored() {
        let mut state = editor(&[(100, 100), (100, 100)]);
        state.select_template(-1);
        state.select_template(1000);
        assert_eq!(state.template_index, Some(0));
    }

    #[test]
    fn style_changes_are_clamped_to_core_limits() {
        let mut state = editor(&[(100, 100), (100, 100)]);
        state.set_spacing(10.0);
        state.set_padding(-1.0);
        state.set_corner_radius(f32::NAN);
        let style = collage(&state).style();
        assert!((style.spacing - MAX_SPACING).abs() < 1e-6);
        assert_eq!(style.padding, 0.0);
        assert_eq!(style.corner_radius, 0.0);
        state.set_spacing(0.0);
        assert_eq!(collage(&state).style().spacing, 0.0);
    }

    #[test]
    fn background_and_aspect_selection_validates_index() {
        let mut state = editor(&[(100, 100)]);
        state.select_background(1);
        assert_eq!(collage(&state).style().background, BACKGROUND_PRESETS[1]);
        state.select_background(99);
        state.select_background(-1);
        assert_eq!(collage(&state).style().background, BACKGROUND_PRESETS[1]);
        state.select_aspect(3);
        assert_eq!(collage(&state).aspect(), AspectRatio::Landscape16x9);
        state.select_aspect(4);
        assert_eq!(collage(&state).aspect(), AspectRatio::Landscape16x9);
    }

    #[test]
    fn invalid_canvas_sizes_and_insets_are_sanitized() {
        let mut state = AppState::new();
        state.set_canvas(200.0, 100.0);
        state.set_canvas(f32::NAN, 5.0);
        state.set_canvas(-1.0, 5.0);
        assert_eq!(state.canvas, Size::new(200.0, 100.0));
        state.set_insets(Insets {
            top: -4.0,
            bottom: f32::INFINITY,
            left: 3.0,
            right: f32::NAN,
        });
        assert_eq!(
            state.insets,
            Insets {
                top: 0.0,
                bottom: 0.0,
                left: 3.0,
                right: 0.0
            }
        );
    }

    #[test]
    fn editing_without_collage_is_harmless() {
        let mut state = AppState::new();
        state.set_canvas(100.0, 100.0);
        state.press(10.0, 10.0);
        state.move_to(80.0, 80.0);
        state.long_press();
        state.release(80.0, 80.0);
        state.scroll(60.0, 10.0, 10.0);
        state.select_template(0);
        state.set_spacing(0.01);
        assert_eq!(state.message, None);
    }

    #[test]
    fn edits_while_exporting_change_nothing() {
        let mut state = editor(&[(100, 100), (100, 100), (100, 100)]);
        state.begin_export(ExportAction::Save).unwrap();
        let placements = collage(&state).placements();
        let cells: Vec<_> = collage(&state)
            .cells()
            .iter()
            .map(|c| c.transform)
            .collect();
        let ids = image_ids(&state);
        let style = collage(&state).style();
        let aspect = collage(&state).aspect();
        let (x, y) = center_of_cell(&state, 0);
        let (hx, hy) = handle_center(&state);

        state.press(x, y);
        state.move_to(x + 50.0, y);
        state.long_press();
        state.release(x + 50.0, y);
        state.press(hx, hy);
        state.move_to(hx + 30.0, hy + 30.0);
        state.release(hx + 30.0, hy + 30.0);
        state.scroll(60.0, x, y);
        state.pinch_started();
        state.pinch(2.0, x, y);
        state.select_template(1);
        state.select_aspect(3);
        state.set_spacing(0.02);
        state.set_padding(0.05);
        state.set_corner_radius(0.05);
        state.select_background(1);

        assert_eq!(state.gesture, Gesture::Idle);
        assert_eq!(state.template_index, Some(0));
        assert_eq!(collage(&state).placements(), placements);
        let after: Vec<_> = collage(&state)
            .cells()
            .iter()
            .map(|c| c.transform)
            .collect();
        assert_eq!(after, cells);
        assert_eq!(image_ids(&state), ids);
        assert_eq!(collage(&state).style(), style);
        assert_eq!(collage(&state).aspect(), aspect);
    }

    #[test]
    fn export_starts_in_the_editor_and_blocks_input() {
        let mut state = editor(&[(40, 30), (30, 40)]);
        state.show_notice("old");
        let job = state.begin_export(ExportAction::Share).unwrap();
        assert_eq!(job.action, ExportAction::Share);
        assert_eq!(job.sources.len(), 2);
        assert_eq!(job.collage.cells().len(), 2);
        assert_eq!(state.busy, Some(Busy::Exporting));
        assert_eq!(Busy::Exporting.text(), texts::EXPORTING);
        assert!(state.message.is_none());
    }

    #[test]
    fn export_is_refused_on_the_start_screen_and_while_busy() {
        let mut state = AppState::new();
        assert!(state.begin_export(ExportAction::Save).is_none());
        assert!(state.busy.is_none());

        let mut state = editor(&[(40, 30)]);
        assert!(state.begin_export(ExportAction::Save).is_some());
        assert!(state.begin_export(ExportAction::Save).is_none());
        assert!(state.begin_export(ExportAction::Share).is_none());
    }

    #[test]
    fn export_cannot_start_while_loading() {
        let mut state = editor(&[(40, 30)]);
        state.busy = Some(Busy::Loading);
        assert!(state.begin_export(ExportAction::Save).is_none());
    }

    #[test]
    fn starting_an_export_ends_a_running_gesture() {
        let mut state = editor(&[(40, 30), (30, 40)]);
        let (x, y) = center_of_cell(&state, 0);
        state.press(x, y);
        assert_ne!(state.gesture, Gesture::Idle);
        assert!(state.begin_export(ExportAction::Save).is_some());
        assert_eq!(state.gesture, Gesture::Idle);
    }

    #[test]
    fn finishing_an_export_returns_the_jpeg_and_unblocks() {
        let mut state = editor(&[(40, 30)]);
        state.begin_export(ExportAction::Save).unwrap();
        assert_eq!(state.finish_export(Ok(vec![1, 2, 3])), Some(vec![1, 2, 3]));
        assert!(state.busy.is_none());
        assert!(state.message.is_none());
        assert_eq!(state.screen, Screen::Editor);
    }

    #[test]
    fn failed_export_unblocks_and_shows_a_specific_error() {
        let mut state = editor(&[(40, 30)]);
        state.begin_export(ExportAction::Save).unwrap();
        let error = CoreError::MissingSource(collage_core::ImageId(0));
        assert_eq!(state.finish_export(Err(error)), None);
        assert!(state.busy.is_none());
        let message = state.message.as_ref().unwrap();
        assert_eq!(message.kind, MessageKind::Error);
        assert_eq!(message.text, texts::EXPORT_SOURCE_UNREADABLE);

        state.begin_export(ExportAction::Save).unwrap();
        state.finish_export(Err(CoreError::Encode("x".into())));
        assert_eq!(state.message.as_ref().unwrap().text, texts::GENERIC_ERROR);
    }

    #[test]
    fn export_thread_failure_unblocks() {
        let mut state = editor(&[(40, 30)]);
        state.begin_export(ExportAction::Save).unwrap();
        state.export_failed();
        assert!(state.busy.is_none());
        assert_eq!(state.message.as_ref().unwrap().kind, MessageKind::Error);
    }

    #[test]
    fn back_while_exporting_is_handled_and_changes_nothing() {
        let mut state = editor(&[(40, 30)]);
        state.begin_export(ExportAction::Save).unwrap();
        assert_eq!(state.request_back(), BackOutcome::Handled);
        assert!(!state.confirm_back);
        assert_eq!(state.screen, Screen::Editor);
        assert!(state.collage.is_some());
        // After the export, back works as usual.
        state.finish_export(Ok(vec![0]));
        assert_eq!(state.request_back(), BackOutcome::Handled);
        assert!(state.confirm_back);
    }

    #[test]
    fn export_of_one_and_max_images_collects_all_sources() {
        for count in [1, MAX_IMAGES] {
            let mut state = editor(&vec![(40, 30); count]);
            let job = state.begin_export(ExportAction::Save).unwrap();
            assert_eq!(job.sources.len(), count);
        }
    }
}
