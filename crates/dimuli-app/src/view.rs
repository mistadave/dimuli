//! Conversion from core data (collage units, normalized crops) to what the UI draws (logical pixels).
//!
//! The core never sees pixels: one collage unit is the collage width, so the scale factor between the two
//! worlds is simply the canvas width in pixels (see [`pixels_per_unit`]).

use std::cell::RefCell;
use std::rc::Rc;

use collage_core::collage::{MAX_CORNER_RADIUS, MAX_PADDING, MAX_SPACING};
use collage_core::{
    CellId, Collage, Divider, DividerId, Rect, Rgba, Size, SplitDir, Template, Vec2,
};
use slint::{ComponentHandle, Model, ModelRc, VecModel};

use crate::images::ImageStore;
use crate::state::{AppState, Busy, MessageKind};
use crate::ui::{AppWindow, Backend, CellView, DividerView, Screen, TemplateRect, TemplateView};

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
    template
        .layout()
        .cell_rects(Rect::new(0.0, 0.0, 1.0, 1.0), TEMPLATE_PREVIEW_SPACING)
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
    let scale = pixels_per_unit(canvas);
    let radius = collage.style().corner_radius;
    collage
        .placements()
        .into_iter()
        .filter_map(|placement| {
            let Some(stored) = images.get(placement.image) else {
                log::error!("no preview for image {:?}", placement.image);
                return None;
            };
            let rect = rect_to_pixels(placement.rect, scale);
            let clip = clip_in_pixels(placement.crop, stored.preview_size);
            Some(CellView {
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
            })
        })
        .collect()
}

/// View entries for all dividers.
pub(crate) fn divider_views(collage: &Collage, canvas: Size) -> Vec<DividerView> {
    let scale = pixels_per_unit(canvas);
    collage
        .dividers()
        .into_iter()
        .map(|divider| divider_view(&divider, scale))
        .collect()
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
    message: Option<(String, bool)>,
    confirm_back: bool,
    insets: [f32; 4],
    cells: Vec<CellView>,
    dividers: Vec<DividerView>,
    /// Set only when the template list changed.
    templates: Option<(Vec<String>, Vec<TemplateView>)>,
    active_template: i32,
    canvas_aspect: f32,
    background: slint::Color,
    aspect_index: i32,
    spacing: f32,
    padding: f32,
    corner_radius: f32,
    background_index: i32,
}

/// The Slint models that are updated in place, plus what they currently show.
pub(crate) struct Models {
    cells: Rc<VecModel<CellView>>,
    dividers: Rc<VecModel<DividerView>>,
    templates: Rc<VecModel<TemplateView>>,
    /// Names of the templates currently in `templates`.
    shown_templates: RefCell<Vec<String>>,
}

impl Models {
    pub(crate) fn new() -> Self {
        Self {
            cells: Rc::new(VecModel::default()),
            dividers: Rc::new(VecModel::default()),
            templates: Rc::new(VecModel::default()),
            shown_templates: RefCell::new(Vec::new()),
        }
    }

    /// Hands the models to the window and sets the constant properties.
    pub(crate) fn attach(&self, window: &AppWindow) {
        let backend = window.global::<Backend>();
        backend.set_cells(ModelRc::from(self.cells.clone()));
        backend.set_dividers(ModelRc::from(self.dividers.clone()));
        backend.set_templates(ModelRc::from(self.templates.clone()));
        backend.set_max_spacing(MAX_SPACING);
        backend.set_max_padding(MAX_PADDING);
        backend.set_max_corner_radius(MAX_CORNER_RADIUS);
        let presets: Vec<slint::Color> = BACKGROUND_PRESETS
            .iter()
            .map(|&c| to_slint_color(c))
            .collect();
        backend.set_background_presets(ModelRc::new(VecModel::from(presets)));
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
        let size = state
            .collage
            .as_ref()
            .map_or(Size::new(1.0, 1.0), Collage::size);
        Snapshot {
            screen: match state.screen {
                crate::state::Screen::Start => Screen::Start,
                crate::state::Screen::Editor => Screen::Editor,
            },
            busy: state.busy.is_some(),
            busy_text: state.busy.map_or("", Busy::text),
            message: state
                .message
                .as_ref()
                .map(|m| (m.text.clone(), m.kind == MessageKind::Error)),
            confirm_back: state.confirm_back,
            insets: [
                state.insets.top,
                state.insets.bottom,
                state.insets.left,
                state.insets.right,
            ],
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
            canvas_aspect: if size.is_empty() {
                1.0
            } else {
                size.width / size.height
            },
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
        backend.set_message_is_error(snapshot.message.as_ref().is_some_and(|m| m.1));
        backend.set_confirm_back_visible(snapshot.confirm_back);
        let [top, bottom, left, right] = snapshot.insets;
        backend.set_inset_top(top);
        backend.set_inset_bottom(bottom);
        backend.set_inset_left(left);
        backend.set_inset_right(right);
        backend.set_active_template(snapshot.active_template);
        backend.set_canvas_aspect(snapshot.canvas_aspect);
        backend.set_collage_background(snapshot.background);
        backend.set_aspect_index(snapshot.aspect_index);
        backend.set_spacing(snapshot.spacing);
        backend.set_padding(snapshot.padding);
        backend.set_corner_radius(snapshot.corner_radius);
        backend.set_background_index(snapshot.background_index);

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
    if model.row_count() != items.len() {
        model.set_vec(items);
        return;
    }
    for (row, item) in items.into_iter().enumerate() {
        if model.row_data(row).as_ref() != Some(&item) {
            model.set_row_data(row, item);
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
}
