//! Which part of an image is visible in a cell.
//!
//! This is the single source of truth for cropping: the UI preview and the export both use
//! [`source_crop`], so the export always looks like the preview.

use crate::geometry::{Rect, Size, Vec2};

/// Maximum zoom factor relative to "cover" (image exactly fills the cell).
pub const MAX_ZOOM: f32 = 5.0;

/// How an image is framed inside its cell.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CellTransform {
    zoom: f32,
    center: Vec2,
}

impl Default for CellTransform {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            center: Vec2::new(0.5, 0.5),
        }
    }
}

impl CellTransform {
    /// `zoom` is clamped to `1.0..=MAX_ZOOM`, `center` (normalized image coordinates) to `0.0..=1.0`.
    /// NaN values fall back to the defaults.
    pub fn new(zoom: f32, center: Vec2) -> Self {
        let zoom = if zoom.is_nan() {
            1.0
        } else {
            zoom.clamp(1.0, MAX_ZOOM)
        };
        let clamp01 = |v: f32| if v.is_nan() { 0.5 } else { v.clamp(0.0, 1.0) };
        Self {
            zoom,
            center: Vec2::new(clamp01(center.x), clamp01(center.y)),
        }
    }

    /// Zoom factor; `1.0` means the image just covers the cell.
    pub fn zoom(&self) -> f32 {
        self.zoom
    }

    /// Requested center of the visible area in normalized image coordinates (`0.0..=1.0`).
    /// The effective center may differ, because [`source_crop`] keeps the crop inside the image.
    pub fn center(&self) -> Vec2 {
        self.center
    }
}

/// The visible part of an image inside a cell, in normalized image coordinates (`0.0..=1.0` per axis).
///
/// The image always covers the whole cell (no empty areas): at zoom 1 the crop has the cell's aspect ratio
/// and touches two opposite image edges. The crop is shifted as needed to stay inside the image.
/// For empty image or cell sizes the whole image is returned.
pub fn source_crop(image: Size, cell: Size, transform: CellTransform) -> Rect {
    if image.is_empty() || cell.is_empty() {
        return Rect::new(0.0, 0.0, 1.0, 1.0);
    }
    let image_aspect = image.width / image.height;
    let cell_aspect = cell.width / cell.height;
    let (cover_w, cover_h) = if cell_aspect > image_aspect {
        (1.0, image_aspect / cell_aspect)
    } else {
        (cell_aspect / image_aspect, 1.0)
    };
    let width = cover_w / transform.zoom;
    let height = cover_h / transform.zoom;
    let cx = transform
        .center
        .x
        .clamp(width / 2.0, (1.0 - width / 2.0).max(width / 2.0));
    let cy = transform
        .center
        .y
        .clamp(height / 2.0, (1.0 - height / 2.0).max(height / 2.0));
    Rect::new(cx - width / 2.0, cy - height / 2.0, width, height)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f32 = 1e-5;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < EPS
    }

    fn rect_close(a: Rect, b: Rect) -> bool {
        close(a.x, b.x) && close(a.y, b.y) && close(a.width, b.width) && close(a.height, b.height)
    }

    fn inside_unit(r: &Rect) -> bool {
        r.x >= -EPS && r.y >= -EPS && r.right() <= 1.0 + EPS && r.bottom() <= 1.0 + EPS
    }

    #[test]
    fn same_aspect_shows_whole_image() {
        let crop = source_crop(
            Size::new(400.0, 300.0),
            Size::new(0.4, 0.3),
            CellTransform::default(),
        );
        assert!(rect_close(crop, Rect::new(0.0, 0.0, 1.0, 1.0)), "{crop:?}");
    }

    #[test]
    fn wide_image_in_square_cell_crops_sides() {
        let crop = source_crop(
            Size::new(200.0, 100.0),
            Size::new(0.5, 0.5),
            CellTransform::default(),
        );
        assert!(close(crop.width, 0.5));
        assert!(close(crop.height, 1.0));
        assert!(close(crop.x, 0.25));
    }

    #[test]
    fn tall_image_in_wide_cell_crops_top_and_bottom() {
        let crop = source_crop(
            Size::new(100.0, 400.0),
            Size::new(1.0, 0.5),
            CellTransform::default(),
        );
        assert!(close(crop.width, 1.0));
        assert!(close(crop.height, 0.125));
        assert!(close(crop.center().y, 0.5));
    }

    #[test]
    fn crop_has_cell_aspect_ratio_in_pixels() {
        let image = Size::new(4000.0, 3000.0);
        let cell = Size::new(0.3, 0.7);
        let crop = source_crop(image, cell, CellTransform::new(2.0, Vec2::new(0.2, 0.9)));
        let crop_aspect = (crop.width * image.width) / (crop.height * image.height);
        assert!(close(crop_aspect, cell.width / cell.height));
    }

    #[test]
    fn zoom_shrinks_crop() {
        let image = Size::new(100.0, 100.0);
        let cell = Size::new(1.0, 1.0);
        let crop = source_crop(image, cell, CellTransform::new(2.0, Vec2::new(0.5, 0.5)));
        assert!(
            rect_close(crop, Rect::new(0.25, 0.25, 0.5, 0.5)),
            "{crop:?}"
        );
    }

    #[test]
    fn crop_never_leaves_image() {
        let image = Size::new(300.0, 200.0);
        for center in [
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(0.0, 1.0),
        ] {
            for zoom in [1.0, 1.5, MAX_ZOOM] {
                let crop =
                    source_crop(image, Size::new(0.2, 0.9), CellTransform::new(zoom, center));
                assert!(inside_unit(&crop), "{crop:?} leaves the image");
            }
        }
    }

    #[test]
    fn transform_clamps_values() {
        let t = CellTransform::new(100.0, Vec2::new(-1.0, f32::NAN));
        assert!(close(t.zoom(), MAX_ZOOM));
        assert_eq!(t.center(), Vec2::new(0.0, 0.5));
        assert!(close(CellTransform::new(0.1, Vec2::default()).zoom(), 1.0));
    }

    #[test]
    fn empty_sizes_return_full_image() {
        let full = Rect::new(0.0, 0.0, 1.0, 1.0);
        let t = CellTransform::default();
        assert_eq!(
            source_crop(Size::new(0.0, 10.0), Size::new(1.0, 1.0), t),
            full
        );
        assert_eq!(
            source_crop(Size::new(10.0, 10.0), Size::new(1.0, 0.0), t),
            full
        );
    }

    #[test]
    fn extreme_aspect_ratios_stay_inside_image() {
        let t = CellTransform::new(1.0, Vec2::new(0.9, 0.1));
        for (image, cell) in [
            (Size::new(10_000.0, 100.0), Size::new(0.2, 0.9)),
            (Size::new(100.0, 10_000.0), Size::new(0.9, 0.05)),
            (Size::new(1.0, 1.0), Size::new(0.9, 0.05)),
        ] {
            let crop = source_crop(image, cell, t);
            assert!(inside_unit(&crop), "{crop:?}");
            assert!(crop.width > 0.0 && crop.height > 0.0);
        }
    }

    #[test]
    fn non_finite_sizes_return_full_image() {
        let full = Rect::new(0.0, 0.0, 1.0, 1.0);
        let t = CellTransform::default();
        let inf = f32::INFINITY;
        assert_eq!(
            source_crop(Size::new(100.0, 100.0), Size::new(inf, inf), t),
            full
        );
        assert_eq!(
            source_crop(Size::new(inf, 100.0), Size::new(1.0, 1.0), t),
            full
        );
        assert_eq!(
            source_crop(Size::new(100.0, f32::NAN), Size::new(1.0, 1.0), t),
            full
        );
    }
}
