//! Conversion from core data (collage units, normalized crops) to what the UI draws (logical pixels).
//!
//! The core never sees pixels: one collage unit is the collage width, so the scale factor between the two
//! worlds is simply the canvas width in pixels (see [`pixels_per_unit`]).

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use collage_core::collage::{MAX_CORNER_RADIUS, MAX_PADDING, MAX_SPACING, style_scale};
use collage_core::{
    CellId, Collage, Divider, DividerId, Rect, Rgba, Size, SplitDir, Template, Vec2,
};
use slint::{ComponentHandle, Model, ModelRc, VecModel};

use crate::images::ImageStore;
use crate::licenses;
use crate::state::{AppState, Busy, Gesture, MessageKind};
use crate::ui::{
    AppWindow, Backend, CellView, DividerView, LicenseEntry, MessageStyle, Screen, TemplateRect,
    TemplateView,
};

/// Gap used for template previews, relative to the preview size.
const TEMPLATE_PREVIEW_SPACING: f32 = 0.06;
/// Exponent per scroll pixel: one mouse-wheel notch (~60 px) zooms by about 10 %.
const WHEEL_ZOOM_PER_PIXEL: f32 = 0.0016;
/// Scroll deltas are clamped so a single huge event cannot jump to the zoom limits unexpectedly.
const WHEEL_MAX_DELTA: f32 = 600.0;

/// Background colors offered in the settings.
pub(crate) const BACKGROUND_PRESETS: [Rgba; 6] = [
    Rgba::new(255, 255, 255, 255),
    Rgba::new(18, 18, 18, 255),
    Rgba::new(224, 224, 224, 255),
    Rgba::new(245, 235, 220, 255),
    Rgba::new(190, 215, 245, 255),
    Rgba::new(245, 190, 200, 255),
];

/// Converts a core color to a Slint color.
pub(crate) fn to_slint_color(color: Rgba) -> slint::Color {
    slint::Color::from_argb_u8(color.a, color.r, color.g, color.b)
}

/// Pixels per collage unit for a canvas of the given size; `0.0` while the size is unknown or invalid.
pub(crate) fn pixels_per_unit(canvas: Size) -> f32 {
    if canvas.width.is_finite() && canvas.width > 0.0 && canvas.height.is_finite() {
        canvas.width
    } else {
        0.0
    }
}

/// The largest canvas with the collage's width/height ratio that fits into `area` (pixels). Empty (`0x0`)
/// for a non-finite or non-positive area or collage size, so layout glitches never reach the cells.
pub(crate) fn fit_canvas(area: Size, collage: Size) -> Size {
    let valid = |v: f32| v.is_finite() && v > 0.0;
    if !(valid(area.width) && valid(area.height) && valid(collage.width) && valid(collage.height)) {
        return Size::default();
    }
    let ratio = collage.width / collage.height;
    let width = area.width.min(area.height * ratio);
    let height = width / ratio;
    if valid(width) && valid(height) {
        Size::new(width, height)
    } else {
        Size::default()
    }
}

/// Converts a rectangle from collage units to pixels.
pub(crate) fn rect_to_pixels(rect: Rect, pixels_per_unit: f32) -> Rect {
    Rect::new(
        rect.x * pixels_per_unit,
        rect.y * pixels_per_unit,
        rect.width * pixels_per_unit,
        rect.height * pixels_per_unit,
    )
}

/// Converts a length in pixels to collage units. `None` if the scale is unknown or the value is not finite.
pub(crate) fn pixels_to_units(pixels: f32, pixels_per_unit: f32) -> Option<f32> {
    (pixels.is_finite() && pixels_per_unit.is_finite() && pixels_per_unit > 0.0)
        .then(|| pixels / pixels_per_unit)
}

/// Converts a point in canvas pixels to collage units.
pub(crate) fn point_to_units(x: f32, y: f32, pixels_per_unit: f32) -> Option<Vec2> {
    Some(Vec2::new(
        pixels_to_units(x, pixels_per_unit)?,
        pixels_to_units(y, pixels_per_unit)?,
    ))
}

/// Position of `point` inside `rect` relative to the rectangle (`0.0..=1.0`, clamped); the center for
/// degenerate rectangles. Both must be in the same unit.
pub(crate) fn focus_in_rect(point: Vec2, rect: Rect) -> Vec2 {
    let relative = |p: f32, start: f32, len: f32| {
        if len.is_finite() && len > 0.0 && p.is_finite() {
            ((p - start) / len).clamp(0.0, 1.0)
        } else {
            0.5
        }
    };
    Vec2::new(
        relative(point.x, rect.x, rect.width),
        relative(point.y, rect.y, rect.height),
    )
}

/// Zoom factor for a mouse-wheel scroll of `delta` pixels (positive = zoom in). Non-finite input gives `1.0`.
pub(crate) fn wheel_zoom_factor(delta: f32) -> f32 {
    if !delta.is_finite() {
        return 1.0;
    }
    (delta.clamp(-WHEEL_MAX_DELTA, WHEEL_MAX_DELTA) * WHEEL_ZOOM_PER_PIXEL).exp()
}

/// The visible part of an image in source pixels, as needed by Slint's `source-clip-*`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PixelClip {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) width: i32,
    pub(crate) height: i32,
}

/// Converts a normalized crop (`0.0..=1.0`) to whole pixels of an image of `size`.
/// The result is at least 1x1 and always inside the image; a non-finite crop selects the whole image.
pub(crate) fn clip_in_pixels(crop: Rect, size: (u32, u32)) -> PixelClip {
    let (image_width, image_height) = (size.0.max(1), size.1.max(1));
    let (x, width) = clip_axis(crop.x, crop.width, image_width);
    let (y, height) = clip_axis(crop.y, crop.height, image_height);
    PixelClip {
        x,
        y,
        width,
        height,
    }
}

/// One axis of [`clip_in_pixels`]: returns `(start, length)` in pixels.
fn clip_axis(start: f32, length: f32, pixels: u32) -> (i32, i32) {
    let total = i64::from(pixels.min(i32::MAX as u32));
    if !(start.is_finite() && length.is_finite()) {
        return (0, total as i32);
    }
    let scale = total as f64;
    let first = ((f64::from(start) * scale).round() as i64).clamp(0, total - 1);
    let end =
        (((f64::from(start) + f64::from(length)) * scale).round() as i64).clamp(first + 1, total);
    (first as i32, (end - first) as i32)
}

/// Square touch target (side `size`, pixels) for a divider, centered on its gap. Works for gaps of zero
/// thickness. Each divider has exactly one such handle, so long dividers never block gestures on the cells.
pub(crate) fn divider_handle_rect(gap: Rect, size: f32) -> Rect {
    let center = gap.center();
    Rect::new(center.x - size / 2.0, center.y - size / 2.0, size, size)
}

/// Touch targets of all dividers in canvas pixels.
pub(crate) fn divider_handles(
    collage: &Collage,
    canvas: Size,
    size: f32,
) -> Vec<(DividerId, Rect)> {
    let scale = pixels_per_unit(canvas);
    collage
        .dividers()
        .into_iter()
        .map(|d| {
            (
                d.id,
                divider_handle_rect(rect_to_pixels(d.rect, scale), size),
            )
        })
        .collect()
}

/// The handle hit by `point` (pixels). If handles overlap, the one whose center is closest wins.
pub(crate) fn pick_handle(handles: &[(DividerId, Rect)], point: Vec2) -> Option<DividerId> {
    let distance = |rect: &Rect| {
        let c = rect.center();
        (c.x - point.x).hypot(c.y - point.y)
    };
    handles
        .iter()
        .filter(|(_, rect)| {
            point.x >= rect.x
                && point.x <= rect.right()
                && point.y >= rect.y
                && point.y <= rect.bottom()
        })
        .min_by(|a, b| distance(&a.1).total_cmp(&distance(&b.1)))
        .map(|(id, _)| *id)
}

/// Cell rectangles of `template` on the unit square, for the preview thumbnails (`0.0..=1.0`).
pub(crate) fn template_preview_rects(template: &Template) -> Vec<Rect> {
    // Same rule as the collage itself: dense grids get a smaller gap so their thumbnail cells stay visible.
    // The factor is exactly 1.0 for up to four cells per line, so thumbnails of 1-9 images are unchanged.
    let spacing = TEMPLATE_PREVIEW_SPACING * style_scale(template.layout().max_cells_per_line());
    template
        .layout()
        .cell_rects(Rect::new(0.0, 0.0, 1.0, 1.0), spacing)
        .into_iter()
        .map(|(_, rect)| rect)
        .collect()
}

/// Slint model entry for a template thumbnail.
pub(crate) fn template_view(template: &Template) -> TemplateView {
    let rects: Vec<TemplateRect> = template_preview_rects(template)
        .into_iter()
        .map(|r| TemplateRect {
            x: r.x,
            y: r.y,
            width: r.width,
            height: r.height,
        })
        .collect();
    TemplateView {
        rects: slint::ModelRc::new(slint::VecModel::from(rects)),
    }
}

/// Which cells to mark during a swap gesture.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct SwapHighlight {
    pub(crate) source: Option<CellId>,
    pub(crate) target: Option<CellId>,
}

/// Corner radius in pixels, limited to half of the cell's shorter side (as the renderer does).
fn corner_radius_px(radius_units: f32, rect_px: Rect, pixels_per_unit: f32) -> f32 {
    (radius_units * pixels_per_unit)
        .min(rect_px.width.min(rect_px.height) / 2.0)
        .max(0.0)
}

/// View entries for all cells. Cells whose image is missing from the store are skipped (and logged).
pub(crate) fn cell_views(
    collage: &Collage,
    images: &ImageStore,
    canvas: Size,
    highlight: SwapHighlight,
) -> Vec<CellView> {
    let mut out = Vec::new();
    cell_views_into(&mut out, collage, images, canvas, highlight);
    out
}

/// Like [`cell_views`], but clears and refills `out` so a gesture can reuse one buffer.
pub(crate) fn cell_views_into(
    out: &mut Vec<CellView>,
    collage: &Collage,
    images: &ImageStore,
    canvas: Size,
    highlight: SwapHighlight,
) {
    out.clear();
    let scale = pixels_per_unit(canvas);
    // The export draws the effective radius (scaled for dense grids); the preview must match.
    let radius = collage.effective_style().corner_radius;
    for placement in collage.placements() {
        let Some(stored) = images.get(placement.image) else {
            log::error!("no preview for image {:?}", placement.image);
            continue;
        };
        let rect = rect_to_pixels(placement.rect, scale);
        let clip = clip_in_pixels(placement.crop, stored.preview_size);
        out.push(CellView {
            x: rect.x,
            y: rect.y,
            width: rect.width,
            height: rect.height,
            image: stored.preview.clone(),
            clip_x: clip.x,
            clip_y: clip.y,
            clip_width: clip.width,
            clip_height: clip.height,
            corner_radius: corner_radius_px(radius, rect, scale),
            swap_source: highlight.source == Some(placement.cell),
            swap_target: highlight.target == Some(placement.cell),
        });
    }
}

/// View entries for all dividers.
pub(crate) fn divider_views(collage: &Collage, canvas: Size) -> Vec<DividerView> {
    let mut out = Vec::new();
    divider_views_into(&mut out, collage, canvas);
    out
}

/// Like [`divider_views`], but clears and refills `out`.
pub(crate) fn divider_views_into(out: &mut Vec<DividerView>, collage: &Collage, canvas: Size) {
    out.clear();
    let scale = pixels_per_unit(canvas);
    out.extend(
        collage
            .dividers()
            .into_iter()
            .map(|divider| divider_view(&divider, scale)),
    );
}

fn divider_view(divider: &Divider, scale: f32) -> DividerView {
    let center = rect_to_pixels(divider.rect, scale).center();
    DividerView {
        id: i32::try_from(divider.id.0).unwrap_or(i32::MAX),
        center_x: center.x,
        center_y: center.y,
        vertical: divider.dir == SplitDir::LeftRight,
    }
}

/// Everything the UI shows, derived from the [`AppState`]. Built while the state is borrowed and applied
/// afterwards, so no borrow is held while Slint properties change.
pub(crate) struct Snapshot {
    screen: Screen,
    busy: bool,
    busy_text: &'static str,
    message: Option<(String, MessageKind)>,
    confirm_back: bool,
    cells: Vec<CellView>,
    dividers: Vec<DividerView>,
    /// Set only when the template list changed.
    templates: Option<(Vec<String>, Vec<TemplateView>)>,
    active_template: i32,
    canvas_width: f32,
    canvas_height: f32,
    background: slint::Color,
    aspect_index: i32,
    spacing: f32,
    padding: f32,
    corner_radius: f32,
    background_index: i32,
    default_aspect_index: i32,
    default_spacing: f32,
    default_padding: f32,
    default_corner_radius: f32,
    default_background_index: i32,
    export_size_index: i32,
    export_quality_index: i32,
}

/// Slint index of an optional list position; `-1` (nothing selected) if absent.
fn index_or_none(index: Option<usize>) -> i32 {
    index.and_then(|i| i32::try_from(i).ok()).unwrap_or(-1)
}

/// The Slint models that are updated in place, plus what they currently show.
pub(crate) struct Models {
    cells: Rc<VecModel<CellView>>,
    dividers: Rc<VecModel<DividerView>>,
    templates: Rc<VecModel<TemplateView>>,
    /// Names of the templates currently in `templates`.
    shown_templates: RefCell<Vec<String>>,
    /// Reused row buffers of the gesture fast path.
    cell_rows: RefCell<Vec<CellView>>,
    divider_rows: RefCell<Vec<DividerView>>,
    /// `divider_rows` holds fresh data that `dividers` does not show yet.
    dividers_dirty: Cell<bool>,
    /// Whether the license list of the info screen has been built.
    licenses_loaded: Cell<bool>,
}

impl Models {
    pub(crate) fn new() -> Self {
        Self {
            cells: Rc::new(VecModel::default()),
            dividers: Rc::new(VecModel::default()),
            templates: Rc::new(VecModel::default()),
            shown_templates: RefCell::new(Vec::new()),
            cell_rows: RefCell::new(Vec::new()),
            divider_rows: RefCell::new(Vec::new()),
            dividers_dirty: Cell::new(false),
            licenses_loaded: Cell::new(false),
        }
    }

    /// Hands the models to the window and sets the constant properties.
    pub(crate) fn attach(&self, window: &AppWindow) {
        let backend = window.global::<Backend>();
        backend.set_cells(ModelRc::from(self.cells.clone()));
        backend.set_dividers(ModelRc::from(self.dividers.clone()));
        backend.set_templates(ModelRc::from(self.templates.clone()));
        backend.set_app_version(env!("CARGO_PKG_VERSION").into());
        backend.set_app_license(env!("CARGO_PKG_LICENSE").into());
        backend.set_max_spacing(MAX_SPACING);
        backend.set_max_padding(MAX_PADDING);
        backend.set_max_corner_radius(MAX_CORNER_RADIUS);
        let presets: Vec<slint::Color> = BACKGROUND_PRESETS
            .iter()
            .map(|&c| to_slint_color(c))
            .collect();
        backend.set_background_presets(ModelRc::new(VecModel::from(presets)));
    }

    /// Fills the license list of the info screen on first use: parsing about a hundred license texts is not
    /// needed at every app start.
    pub(crate) fn ensure_licenses(&self, window: &AppWindow) {
        if self.licenses_loaded.replace(true) {
            return;
        }
        let backend = window.global::<Backend>();
        let licenses: Vec<LicenseEntry> = licenses::all()
            .into_iter()
            .map(|entry| LicenseEntry {
                name: entry.name.into(),
                packages: entry.packages.into(),
                text: entry.text.into(),
            })
            .collect();
        backend.set_licenses(ModelRc::new(VecModel::from(licenses)));
    }

    /// Builds the snapshot of `state`.
    pub(crate) fn snapshot(&self, state: &AppState) -> Snapshot {
        let (cells, dividers) = match &state.collage {
            Some(collage) => (
                cell_views(collage, &state.images, state.canvas, state.swap_highlight()),
                divider_views(collage, state.canvas),
            ),
            None => (Vec::new(), Vec::new()),
        };
        let templates_changed = !self
            .shown_templates
            .borrow()
            .iter()
            .map(String::as_str)
            .eq(state.templates.iter().map(Template::name));
        let style = state.collage.as_ref().map(Collage::style);
        let background = style.map_or(Rgba::WHITE, |s| s.background);
        let aspect = state.collage.as_ref().map(Collage::aspect);
        Snapshot {
            screen: match state.screen {
                crate::state::Screen::Start => Screen::Start,
                crate::state::Screen::Editor => Screen::Editor,
                crate::state::Screen::Settings => Screen::Settings,
                crate::state::Screen::About => Screen::About,
            },
            busy: state.busy.is_some_and(Busy::is_visible),
            busy_text: state.busy.filter(|b| b.is_visible()).map_or("", Busy::text),
            message: state.message.as_ref().map(|m| (m.text.clone(), m.kind)),
            confirm_back: state.confirm_back,
            cells,
            dividers,
            templates: templates_changed.then(|| {
                (
                    state
                        .templates
                        .iter()
                        .map(|t| t.name().to_owned())
                        .collect(),
                    state.templates.iter().map(template_view).collect(),
                )
            }),
            active_template: state
                .template_index
                .and_then(|i| i32::try_from(i).ok())
                .unwrap_or(-1),
            canvas_width: state.canvas.width,
            canvas_height: state.canvas.height,
            background: to_slint_color(background),
            aspect_index: aspect
                .and_then(|a| collage_core::AspectRatio::ALL.iter().position(|&x| x == a))
                .and_then(|i| i32::try_from(i).ok())
                .unwrap_or(-1),
            spacing: style.map_or(0.0, |s| s.spacing),
            padding: style.map_or(0.0, |s| s.padding),
            corner_radius: style.map_or(0.0, |s| s.corner_radius),
            background_index: BACKGROUND_PRESETS
                .iter()
                .position(|&c| c == background)
                .and_then(|i| i32::try_from(i).ok())
                .unwrap_or(-1),
            default_aspect_index: index_or_none(state.settings.aspect_index()),
            default_spacing: state.settings.spacing,
            default_padding: state.settings.padding,
            default_corner_radius: state.settings.corner_radius,
            default_background_index: index_or_none(Some(state.settings.background_index())),
            export_size_index: index_or_none(state.settings.export_size_index()),
            export_quality_index: index_or_none(state.settings.jpeg_quality_index()),
        }
    }

    /// Gesture fast path, step 1: refills the reused row buffers from `state`. Divider rows are only
    /// refreshed while a divider is dragged; nothing else (messages, templates, properties) is touched.
    pub(crate) fn fill_gesture_rows(&self, state: &AppState) {
        let mut cells = self.cell_rows.borrow_mut();
        let mut dividers = self.divider_rows.borrow_mut();
        let Some(collage) = &state.collage else {
            cells.clear();
            dividers.clear();
            self.dividers_dirty.set(true);
            return;
        };
        cell_views_into(
            &mut cells,
            collage,
            &state.images,
            state.canvas,
            state.swap_highlight(),
        );
        if matches!(state.gesture, Gesture::DraggingDivider { .. }) {
            divider_views_into(&mut dividers, collage, state.canvas);
            self.dividers_dirty.set(true);
        }
    }

    /// Gesture fast path, step 2: writes only the rows that changed into the Slint models.
    pub(crate) fn apply_gesture_rows(&self) {
        sync_rows(&self.cells, &self.cell_rows.borrow());
        if self.dividers_dirty.replace(false) {
            sync_rows(&self.dividers, &self.divider_rows.borrow());
        }
    }

    /// Writes the snapshot into the window's properties and models.
    pub(crate) fn apply(&self, window: &AppWindow, snapshot: Snapshot) {
        let backend = window.global::<Backend>();
        backend.set_screen(snapshot.screen);
        backend.set_busy(snapshot.busy);
        backend.set_busy_text(snapshot.busy_text.into());
        backend.set_message_text(
            snapshot
                .message
                .as_ref()
                .map_or("", |m| m.0.as_str())
                .into(),
        );
        backend.set_message_style(match snapshot.message.as_ref().map(|m| m.1) {
            Some(MessageKind::Error) => MessageStyle::Error,
            Some(MessageKind::Success) => MessageStyle::Success,
            Some(MessageKind::Notice) | None => MessageStyle::Notice,
        });
        backend.set_confirm_back_visible(snapshot.confirm_back);
        backend.set_active_template(snapshot.active_template);
        backend.set_collage_background(snapshot.background);
        backend.set_aspect_index(snapshot.aspect_index);
        backend.set_spacing(snapshot.spacing);
        backend.set_padding(snapshot.padding);
        backend.set_corner_radius(snapshot.corner_radius);
        backend.set_background_index(snapshot.background_index);
        backend.set_default_aspect_index(snapshot.default_aspect_index);
        backend.set_default_spacing(snapshot.default_spacing);
        backend.set_default_padding(snapshot.default_padding);
        backend.set_default_corner_radius(snapshot.default_corner_radius);
        backend.set_default_background_index(snapshot.default_background_index);
        backend.set_export_size_index(snapshot.export_size_index);
        backend.set_export_quality_index(snapshot.export_quality_index);

        // Canvas size and rows change in the same apply, so no frame mixes old cells with a new canvas.
        backend.set_canvas_width(snapshot.canvas_width);
        backend.set_canvas_height(snapshot.canvas_height);
        sync_model(&self.cells, snapshot.cells);
        sync_model(&self.dividers, snapshot.dividers);
        if let Some((names, views)) = snapshot.templates {
            self.templates.set_vec(views);
            *self.shown_templates.borrow_mut() = names;
        }
    }
}

/// Updates `model` in place: unchanged rows are left alone so Slint only redraws what changed.
fn sync_model<T: Clone + PartialEq + 'static>(model: &VecModel<T>, items: Vec<T>) {
    sync_rows(model, &items);
}

/// Slice version of [`sync_model`]: replaces the model only if the row count differs, otherwise sets just
/// the rows that changed.
fn sync_rows<T: Clone + PartialEq + 'static>(model: &VecModel<T>, items: &[T]) {
    if model.row_count() != items.len() {
        model.set_vec(items.to_vec());
        return;
    }
    for (row, item) in items.iter().enumerate() {
        if model.row_data(row).as_ref() != Some(item) {
            model.set_row_data(row, item.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::images::LoadedImage;
    use collage_core::{ImageMeta, templates_for};
    use slint::{Rgba8Pixel, SharedPixelBuffer};

    const EPS: f32 = 1e-3;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < EPS
    }

    fn store_with(count: usize, preview: (u32, u32)) -> (ImageStore, Vec<ImageMeta>) {
        let mut store = ImageStore::default();
        let metas = (0..count)
            .map(|i| {
                store
                    .insert(LoadedImage {
                        path: format!("{i}.png").into(),
                        width: preview.0 * 2,
                        height: preview.1 * 2,
                        preview: SharedPixelBuffer::<Rgba8Pixel>::new(preview.0, preview.1),
                    })
                    .unwrap()
            })
            .collect();
        (store, metas)
    }

    #[test]
    fn scale_is_canvas_width_and_zero_for_invalid_canvas() {
        assert!(close(pixels_per_unit(Size::new(300.0, 500.0)), 300.0));
        assert_eq!(pixels_per_unit(Size::new(0.0, 100.0)), 0.0);
        assert_eq!(pixels_per_unit(Size::new(-5.0, 100.0)), 0.0);
        assert_eq!(pixels_per_unit(Size::new(f32::NAN, 100.0)), 0.0);
        assert_eq!(pixels_per_unit(Size::new(100.0, f32::INFINITY)), 0.0);
    }

    #[test]
    fn rect_conversion_scales_all_components() {
        let r = rect_to_pixels(Rect::new(0.1, 0.2, 0.5, 0.25), 200.0);
        assert!(
            close(r.x, 20.0) && close(r.y, 40.0) && close(r.width, 100.0) && close(r.height, 50.0)
        );
    }

    #[test]
    fn pixel_to_unit_round_trip() {
        let scale = 360.0;
        let units = pixels_to_units(90.0, scale).unwrap();
        assert!(close(units, 0.25));
        assert!(close(units * scale, 90.0));
    }

    #[test]
    fn pixel_to_unit_rejects_invalid_input() {
        assert_eq!(pixels_to_units(10.0, 0.0), None);
        assert_eq!(pixels_to_units(f32::NAN, 100.0), None);
        assert_eq!(pixels_to_units(f32::INFINITY, 100.0), None);
        assert_eq!(point_to_units(1.0, f32::NAN, 100.0), None);
        assert_eq!(
            point_to_units(50.0, 100.0, 100.0),
            Some(Vec2::new(0.5, 1.0))
        );
    }

    #[test]
    fn focus_is_relative_and_clamped() {
        let rect = Rect::new(0.2, 0.2, 0.4, 0.2);
        let f = focus_in_rect(Vec2::new(0.4, 0.3), rect);
        assert!(close(f.x, 0.5) && close(f.y, 0.5));
        let outside = focus_in_rect(Vec2::new(5.0, -5.0), rect);
        assert_eq!(outside, Vec2::new(1.0, 0.0));
        assert_eq!(
            focus_in_rect(Vec2::new(f32::NAN, 0.0), Rect::new(0.0, 0.0, 0.0, 1.0)),
            Vec2::new(0.5, 0.0)
        );
    }

    #[test]
    fn wheel_zoom_is_symmetric_and_safe() {
        assert!(close(wheel_zoom_factor(0.0), 1.0));
        assert!(wheel_zoom_factor(60.0) > 1.05 && wheel_zoom_factor(60.0) < 1.2);
        assert!(close(
            wheel_zoom_factor(60.0) * wheel_zoom_factor(-60.0),
            1.0
        ));
        assert_eq!(wheel_zoom_factor(f32::NAN), 1.0);
        assert!(wheel_zoom_factor(f32::INFINITY) == 1.0);
        assert!(wheel_zoom_factor(1e9).is_finite());
    }

    #[test]
    fn clip_rounds_to_whole_pixels_inside_the_image() {
        let clip = clip_in_pixels(Rect::new(0.25, 0.5, 0.5, 0.25), (200, 100));
        assert_eq!(
            clip,
            PixelClip {
                x: 50,
                y: 50,
                width: 100,
                height: 25
            }
        );
    }

    #[test]
    fn clip_of_full_crop_is_whole_image() {
        let clip = clip_in_pixels(Rect::new(0.0, 0.0, 1.0, 1.0), (640, 480));
        assert_eq!(
            clip,
            PixelClip {
                x: 0,
                y: 0,
                width: 640,
                height: 480
            }
        );
    }

    #[test]
    fn clip_never_leaves_the_image_or_collapses() {
        let tiny = clip_in_pixels(Rect::new(0.999, 0.999, 0.0001, 0.0), (10, 10));
        assert!(tiny.width >= 1 && tiny.height >= 1);
        assert!(tiny.x + tiny.width <= 10 && tiny.y + tiny.height <= 10);
        let outside = clip_in_pixels(Rect::new(-1.0, 2.0, 5.0, 5.0), (10, 20));
        assert!(outside.x >= 0 && outside.y >= 0);
        assert!(outside.x + outside.width <= 10 && outside.y + outside.height <= 20);
    }

    #[test]
    fn clip_handles_non_finite_crop_and_extreme_images() {
        let nan = clip_in_pixels(Rect::new(f32::NAN, 0.0, 1.0, 1.0), (30, 20));
        assert_eq!(
            nan,
            PixelClip {
                x: 0,
                y: 0,
                width: 30,
                height: 20
            }
        );
        let strip = clip_in_pixels(Rect::new(0.0, 0.0, 0.5, 1.0), (2048, 1));
        assert_eq!(
            strip,
            PixelClip {
                x: 0,
                y: 0,
                width: 1024,
                height: 1
            }
        );
        let empty = clip_in_pixels(Rect::new(0.0, 0.0, 1.0, 1.0), (0, 0));
        assert_eq!((empty.width, empty.height), (1, 1));
    }

    #[test]
    fn handle_is_a_square_centered_on_the_gap_even_without_spacing() {
        let rect = divider_handle_rect(Rect::new(100.0, 10.0, 0.0, 200.0), 48.0);
        assert!(close(rect.width, 48.0) && close(rect.height, 48.0));
        assert!(close(rect.center().x, 100.0) && close(rect.center().y, 110.0));
    }

    #[test]
    fn pick_handle_hits_inside_and_misses_outside() {
        let handles = [(DividerId(0), Rect::new(0.0, 0.0, 48.0, 48.0))];
        assert_eq!(
            pick_handle(&handles, Vec2::new(24.0, 24.0)),
            Some(DividerId(0))
        );
        assert_eq!(
            pick_handle(&handles, Vec2::new(47.0, 1.0)),
            Some(DividerId(0))
        );
        assert_eq!(pick_handle(&handles, Vec2::new(49.0, 24.0)), None);
        assert_eq!(pick_handle(&[], Vec2::new(0.0, 0.0)), None);
    }

    #[test]
    fn overlapping_handles_pick_the_closest_center() {
        let handles = [
            (DividerId(0), Rect::new(0.0, 0.0, 48.0, 48.0)),
            (DividerId(1), Rect::new(30.0, 0.0, 48.0, 48.0)),
        ];
        assert_eq!(
            pick_handle(&handles, Vec2::new(40.0, 24.0)),
            Some(DividerId(1))
        );
        assert_eq!(
            pick_handle(&handles, Vec2::new(35.0, 24.0)),
            Some(DividerId(0))
        );
        assert_eq!(
            pick_handle(&handles, Vec2::new(10.0, 24.0)),
            Some(DividerId(0))
        );
    }

    #[test]
    fn template_previews_cover_the_unit_square_without_overlap() {
        for count in 1..=collage_core::MAX_IMAGES {
            for template in templates_for(count) {
                let rects = template_preview_rects(&template);
                assert_eq!(rects.len(), count);
                for (i, a) in rects.iter().enumerate() {
                    assert!(
                        a.x >= -EPS
                            && a.y >= -EPS
                            && a.right() <= 1.0 + EPS
                            && a.bottom() <= 1.0 + EPS
                    );
                    assert!(a.width > 0.0 && a.height > 0.0);
                    for b in rects.iter().skip(i + 1) {
                        let overlap_x = a.right().min(b.right()) - a.x.max(b.x);
                        let overlap_y = a.bottom().min(b.bottom()) - a.y.max(b.y);
                        assert!(overlap_x <= EPS || overlap_y <= EPS);
                    }
                }
            }
        }
    }

    #[test]
    fn template_previews_of_one_to_nine_are_unchanged() {
        let unit = Rect::new(0.0, 0.0, 1.0, 1.0);
        for count in 1..=9 {
            for template in templates_for(count) {
                let expected: Vec<Rect> = template
                    .layout()
                    .cell_rects(unit, TEMPLATE_PREVIEW_SPACING)
                    .into_iter()
                    .map(|(_, r)| r)
                    .collect();
                // Bit identity: the scale factor is exactly 1.0 up to four cells per line.
                assert_eq!(template_preview_rects(&template), expected);
            }
        }
    }

    #[test]
    fn template_previews_of_dense_grids_use_scaled_spacing() {
        let template = &templates_for(36)[2];
        let rects = template_preview_rects(template);
        let gap = rects[1].x - rects[0].right();
        assert!((gap - 0.03).abs() < 1e-6, "gap {gap}");
        for count in 10..=collage_core::MAX_IMAGES {
            for template in templates_for(count) {
                for r in template_preview_rects(&template) {
                    assert!(r.width >= 0.05 && r.height >= 0.05, "{count}: {r:?}");
                }
            }
        }
    }

    #[test]
    fn cell_views_use_the_effective_corner_radius() {
        let canvas = Size::new(400.0, 400.0);
        let max = collage_core::Style {
            corner_radius: MAX_CORNER_RADIUS,
            ..collage_core::Style::default()
        };
        let (store, metas) = store_with(36, (64, 48));
        let mut dense = Collage::new(&metas, &templates_for(36)[0]).unwrap();
        dense.set_style(max);
        let effective = dense.effective_style().corner_radius;
        assert!(effective < MAX_CORNER_RADIUS);
        let views = cell_views(&dense, &store, canvas, SwapHighlight::default());
        assert_eq!(views.len(), 36);
        for (view, placement) in views.iter().zip(dense.placements()) {
            let rect = rect_to_pixels(placement.rect, 400.0);
            let want = corner_radius_px(effective, rect, 400.0);
            let stored = corner_radius_px(MAX_CORNER_RADIUS, rect, 400.0);
            assert!((view.corner_radius - want).abs() < 1e-4);
            assert!(view.corner_radius < stored);
        }

        let (store, metas) = store_with(9, (64, 48));
        let mut classic = Collage::new(&metas, &templates_for(9)[0]).unwrap();
        classic.set_style(max);
        let views = cell_views(&classic, &store, canvas, SwapHighlight::default());
        for (view, placement) in views.iter().zip(classic.placements()) {
            let rect = rect_to_pixels(placement.rect, 400.0);
            let want = corner_radius_px(MAX_CORNER_RADIUS, rect, 400.0);
            assert!((view.corner_radius - want).abs() < 1e-4);
        }
    }

    fn dense_canvas(aspect: collage_core::AspectRatio) -> Size {
        fit_canvas(Size::new(411.0, 640.0), aspect.size())
    }

    fn dense_collage(
        count: usize,
        template: &Template,
        aspect: collage_core::AspectRatio,
    ) -> Collage {
        let (_, metas) = store_with(count, (64, 48));
        let mut collage = Collage::new(&metas, template).unwrap();
        collage.set_aspect(aspect);
        collage
    }

    #[test]
    fn divider_handles_stay_grabbable_with_many_images() {
        for count in [25, collage_core::MAX_IMAGES] {
            for template in templates_for(count) {
                for aspect in collage_core::AspectRatio::ALL {
                    let collage = dense_collage(count, &template, aspect);
                    let canvas = dense_canvas(aspect);
                    let handles = divider_handles(&collage, canvas, 48.0);
                    assert!(!handles.is_empty());
                    for (id, rect) in &handles {
                        assert_eq!(
                            pick_handle(&handles, rect.center()),
                            Some(*id),
                            "{count} images, {aspect:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn cells_keep_free_area_beside_the_handles_at_the_cap() {
        // 25 images in 16:9 leave three cells fully under handles; the owner accepted that (D-07), so only
        // the cap is asserted here.
        let count = collage_core::MAX_IMAGES;
        let template = &templates_for(count)[0];
        for aspect in collage_core::AspectRatio::ALL {
            let collage = dense_collage(count, template, aspect);
            let canvas = dense_canvas(aspect);
            let scale = pixels_per_unit(canvas);
            let handles = divider_handles(&collage, canvas, 48.0);
            let mut worst = 1.0_f32;
            for placement in collage.placements() {
                let rect = rect_to_pixels(placement.rect, scale);
                let (mut total, mut free) = (0_u32, 0_u32);
                let (x0, x1) = (rect.x.round() as i32, rect.right().round() as i32);
                let (y0, y1) = (rect.y.round() as i32, rect.bottom().round() as i32);
                for y in y0..y1 {
                    for x in x0..x1 {
                        total += 1;
                        let centre = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                        if pick_handle(&handles, centre).is_none() {
                            free += 1;
                        }
                    }
                }
                assert!(total > 0);
                worst = worst.min(free as f32 / total as f32);
            }
            println!("free area at {count} images, {aspect:?}: worst cell {worst:.3}");
            assert!(worst >= 0.25, "{aspect:?}: {worst}");
        }
    }

    #[test]
    fn cell_views_match_placements_in_pixels() {
        let (store, metas) = store_with(2, (200, 100));
        let collage = Collage::new(&metas, &templates_for(2)[0]).unwrap();
        let canvas = Size::new(400.0, 400.0);
        let views = cell_views(&collage, &store, canvas, SwapHighlight::default());
        assert_eq!(views.len(), 2);
        let placements = collage.placements();
        for (view, placement) in views.iter().zip(&placements) {
            assert!(close(view.x, placement.rect.x * 400.0));
            assert!(close(view.width, placement.rect.width * 400.0));
            assert!(view.clip_width >= 1 && view.clip_width <= 200);
            assert!(view.clip_height >= 1 && view.clip_height <= 100);
            assert!(!view.swap_source && !view.swap_target);
        }
    }

    #[test]
    fn cell_views_mark_swap_source_and_target() {
        let (store, metas) = store_with(3, (50, 50));
        let collage = Collage::new(&metas, &templates_for(3)[0]).unwrap();
        let highlight = SwapHighlight {
            source: Some(CellId(0)),
            target: Some(CellId(2)),
        };
        let views = cell_views(&collage, &store, Size::new(100.0, 100.0), highlight);
        let flags: Vec<(bool, bool)> = views
            .iter()
            .map(|v| (v.swap_source, v.swap_target))
            .collect();
        assert_eq!(flags.len(), 3);
        assert_eq!(flags.iter().filter(|f| f.0).count(), 1);
        assert_eq!(flags.iter().filter(|f| f.1).count(), 1);
    }

    #[test]
    fn cell_views_with_unknown_canvas_have_zero_size_but_valid_clips() {
        let (store, metas) = store_with(1, (10, 10));
        let collage = Collage::new(&metas, &templates_for(1)[0]).unwrap();
        let views = cell_views(&collage, &store, Size::default(), SwapHighlight::default());
        assert_eq!(views.len(), 1);
        assert_eq!(views[0].width, 0.0);
        assert!(views[0].clip_width >= 1);
    }

    #[test]
    fn cell_views_skip_cells_without_image() {
        let (_, metas) = store_with(2, (10, 10));
        let collage = Collage::new(&metas, &templates_for(2)[0]).unwrap();
        let empty = ImageStore::default();
        assert!(
            cell_views(
                &collage,
                &empty,
                Size::new(100.0, 100.0),
                SwapHighlight::default()
            )
            .is_empty()
        );
    }

    #[test]
    fn corner_radius_is_limited_to_half_the_shorter_side() {
        let rect = Rect::new(0.0, 0.0, 100.0, 20.0);
        assert!(close(corner_radius_px(0.1, rect, 100.0), 10.0));
        assert!(close(corner_radius_px(0.01, rect, 100.0), 1.0));
        assert_eq!(corner_radius_px(-1.0, rect, 100.0), 0.0);
    }

    #[test]
    fn divider_views_are_centered_on_the_gap_with_zero_spacing() {
        let (_, metas) = store_with(2, (10, 10));
        let mut collage = Collage::new(&metas, &templates_for(2)[0]).unwrap();
        let mut style = collage.style();
        style.spacing = 0.0;
        collage.set_style(style);
        let views = divider_views(&collage, Size::new(300.0, 300.0));
        assert_eq!(views.len(), 1);
        assert_eq!(views[0].id, 0);
        let handles = divider_handles(&collage, Size::new(300.0, 300.0), 48.0);
        let c = handles[0].1.center();
        assert!(close(c.x, views[0].center_x) && close(c.y, views[0].center_y));
    }

    #[test]
    fn fit_canvas_keeps_the_collage_ratio_inside_the_area() {
        let area = Size::new(300.0, 300.0);
        let square = fit_canvas(area, Size::new(1.0, 1.0));
        assert!(close(square.width, 300.0) && close(square.height, 300.0));
        let wide = fit_canvas(area, Size::new(1.0, 9.0 / 16.0));
        assert!(close(wide.width, 300.0) && close(wide.height, 168.75));
        let tall = fit_canvas(area, Size::new(1.0, 16.0 / 9.0));
        assert!(close(tall.width, 168.75) && close(tall.height, 300.0));
    }

    #[test]
    fn fit_canvas_is_empty_for_invalid_input() {
        let ok = Size::new(1.0, 1.0);
        let area = Size::new(100.0, 100.0);
        assert_eq!(fit_canvas(Size::default(), ok), Size::default());
        assert_eq!(fit_canvas(Size::new(f32::NAN, 5.0), ok), Size::default());
        assert_eq!(fit_canvas(Size::new(-1.0, 5.0), ok), Size::default());
        assert_eq!(fit_canvas(area, Size::new(1.0, 0.0)), Size::default());
        assert_eq!(fit_canvas(area, Size::new(1.0, f32::NAN)), Size::default());
        assert_eq!(
            fit_canvas(area, Size::new(1.0, f32::INFINITY)),
            Size::default()
        );
    }

    fn three_image_editor() -> AppState {
        let mut state = AppState::new();
        assert!(state.begin_loading());
        let results = (0..3)
            .map(|i| {
                Ok(LoadedImage {
                    path: format!("{i}.png").into(),
                    width: 128,
                    height: 64,
                    preview: SharedPixelBuffer::<Rgba8Pixel>::new(64, 32),
                })
            })
            .collect();
        state.finish_loading(results, false, 0);
        state.set_canvas_area(300.0, 300.0);
        state.touch_target = 48.0;
        state
    }

    #[test]
    fn gesture_rows_change_only_the_panned_cell() {
        let mut state = three_image_editor();
        let center = state.collage.as_ref().unwrap().placements()[0]
            .rect
            .center();
        let (x, y) = (center.x * 300.0, center.y * 300.0);
        // Zoom into cell 0 first: a fully visible image has no room to pan.
        state.scroll(300.0, x, y);
        let models = Models::new();
        models.fill_gesture_rows(&state);
        let before = models.cell_rows.borrow().clone();
        assert_eq!(before.len(), 3);
        state.press(x, y);
        state.move_to(x + 30.0, y + 30.0);
        assert!(matches!(state.gesture, Gesture::Panning { .. }));
        models.fill_gesture_rows(&state);
        let after = models.cell_rows.borrow().clone();
        assert_eq!(after.len(), 3);
        assert_ne!(before[0], after[0]);
        assert_eq!(before[1], after[1]);
        assert_eq!(before[2], after[2]);
    }

    #[test]
    fn gesture_rows_reuse_their_buffer() {
        let state = three_image_editor();
        let models = Models::new();
        models.fill_gesture_rows(&state);
        let capacity = models.cell_rows.borrow().capacity();
        let ptr = models.cell_rows.borrow().as_ptr();
        models.fill_gesture_rows(&state);
        assert_eq!(models.cell_rows.borrow().capacity(), capacity);
        assert_eq!(models.cell_rows.borrow().as_ptr(), ptr);
    }

    #[test]
    fn gesture_rows_include_dividers_only_while_dragging_a_divider() {
        let mut state = three_image_editor();
        let models = Models::new();
        models.fill_gesture_rows(&state);
        assert!(models.divider_rows.borrow().is_empty());
        assert!(!models.dividers_dirty.get());

        let divider = state.collage.as_ref().unwrap().dividers()[0];
        let c = rect_to_pixels(divider.rect, 300.0).center();
        state.press(c.x, c.y);
        state.move_to(c.x + 12.0, c.y + 12.0);
        assert!(matches!(state.gesture, Gesture::DraggingDivider { .. }));
        models.fill_gesture_rows(&state);
        assert!(!models.divider_rows.borrow().is_empty());
        assert!(models.dividers_dirty.get());
    }
}
