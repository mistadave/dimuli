//! The collage model and all editing operations.

use crate::MAX_IMAGES;
use crate::crop::{CellTransform, source_crop};
use crate::error::CoreError;
use crate::geometry::{Rect, Size, Vec2};
use crate::layout::{CellId, Divider, DividerId, LayoutNode};
use crate::templates::Template;

/// Upper limits for [`Style`] values, in collage units.
pub const MAX_SPACING: f32 = 0.05;
/// Maximum border around the collage.
pub const MAX_PADDING: f32 = 0.1;
/// Maximum corner radius of a cell.
pub const MAX_CORNER_RADIUS: f32 = 0.1;

/// Identifies an image managed by the app.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ImageId(pub u32);

/// What the core needs to know about an image: its id and pixel size (after EXIF orientation).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageMeta {
    id: ImageId,
    width: u32,
    height: u32,
}

impl ImageMeta {
    /// Fails if a side is zero.
    pub fn new(id: ImageId, width: u32, height: u32) -> Result<Self, CoreError> {
        if width == 0 || height == 0 {
            return Err(CoreError::InvalidImageSize { width, height });
        }
        Ok(Self { id, width, height })
    }

    /// Id of the image.
    pub fn id(&self) -> ImageId {
        self.id
    }

    /// Width in pixels.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Height in pixels.
    pub fn height(&self) -> u32 {
        self.height
    }

    fn size(&self) -> Size {
        Size::new(self.width as f32, self.height as f32)
    }
}

/// Output format of the collage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AspectRatio {
    #[default]
    /// 1:1
    Square,
    /// 4:5 (portrait)
    Portrait4x5,
    /// 9:16 (story, portrait)
    Story9x16,
    /// 16:9 (landscape)
    Landscape16x9,
}

impl AspectRatio {
    /// All formats in display order.
    pub const ALL: [AspectRatio; 4] = [
        AspectRatio::Square,
        AspectRatio::Portrait4x5,
        AspectRatio::Story9x16,
        AspectRatio::Landscape16x9,
    ];

    /// Size of the collage in collage units (width is always `1.0`).
    pub fn size(self) -> Size {
        let (w, h) = match self {
            AspectRatio::Square => (1.0, 1.0),
            AspectRatio::Portrait4x5 => (4.0, 5.0),
            AspectRatio::Story9x16 => (9.0, 16.0),
            AspectRatio::Landscape16x9 => (16.0, 9.0),
        };
        Size::new(1.0, h / w)
    }
}

/// An sRGB color with alpha.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgba {
    /// Red.
    pub r: u8,
    /// Green.
    pub g: u8,
    /// Blue.
    pub b: u8,
    /// Alpha (255 = opaque).
    pub a: u8,
}

impl Rgba {
    /// Opaque white.
    pub const WHITE: Rgba = Rgba::new(255, 255, 255, 255);

    /// Creates a color from its components.
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
}

/// Visual settings. Lengths are in collage units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Style {
    /// Gap between cells.
    pub spacing: f32,
    /// Border around the whole collage.
    pub padding: f32,
    /// Corner radius of every cell (the renderer limits it to half the cell's shorter side).
    pub corner_radius: f32,
    /// Background color (visible in gaps and padding).
    pub background: Rgba,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            spacing: 0.01,
            padding: 0.01,
            corner_radius: 0.0,
            background: Rgba::WHITE,
        }
    }
}

impl Style {
    /// Clamps all lengths to their valid range; NaN becomes `0.0`.
    fn sanitized(self) -> Self {
        let clamp = |v: f32, max: f32| if v.is_nan() { 0.0 } else { v.clamp(0.0, max) };
        Self {
            spacing: clamp(self.spacing, MAX_SPACING),
            padding: clamp(self.padding, MAX_PADDING),
            corner_radius: clamp(self.corner_radius, MAX_CORNER_RADIUS),
            background: self.background,
        }
    }
}

/// Content of one cell.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cell {
    /// The image shown in this cell.
    pub image: ImageMeta,
    /// How the image is framed.
    pub transform: CellTransform,
}

/// Where a cell is drawn and which part of its image is visible. Used by both UI and export.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CellPlacement {
    /// The cell.
    pub cell: CellId,
    /// The image shown in the cell.
    pub image: ImageId,
    /// Position in collage units.
    pub rect: Rect,
    /// Visible part of the image in normalized image coordinates.
    pub crop: Rect,
}

/// A collage: layout, style, format and one image per cell.
#[derive(Debug, Clone, PartialEq)]
pub struct Collage {
    aspect: AspectRatio,
    style: Style,
    layout: LayoutNode,
    /// Indexed by `CellId`.
    cells: Vec<Cell>,
}

impl Collage {
    /// Creates a collage. Image `i` goes into cell `i` of the template.
    pub fn new(images: &[ImageMeta], template: &Template) -> Result<Self, CoreError> {
        check_image_count(images.len())?;
        check_template(template, images.len())?;
        Ok(Self {
            aspect: AspectRatio::default(),
            style: Style::default(),
            layout: template.layout().clone(),
            cells: images
                .iter()
                .map(|&image| Cell {
                    image,
                    transform: CellTransform::default(),
                })
                .collect(),
        })
    }

    /// Output format.
    pub fn aspect(&self) -> AspectRatio {
        self.aspect
    }

    /// Visual settings.
    pub fn style(&self) -> Style {
        self.style
    }

    /// The layout tree.
    pub fn layout(&self) -> &LayoutNode {
        &self.layout
    }

    /// All cells, indexed by `CellId`.
    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }

    /// Size of the whole collage in collage units.
    pub fn size(&self) -> Size {
        self.aspect.size()
    }

    /// Changes the output format; cells keep their framing.
    pub fn set_aspect(&mut self, aspect: AspectRatio) {
        self.aspect = aspect;
    }

    /// Sets the style; out-of-range values are clamped.
    pub fn set_style(&mut self, style: Style) {
        self.style = style.sanitized();
    }

    /// Replaces the layout. Cells keep their images and framing.
    pub fn apply_template(&mut self, template: &Template) -> Result<(), CoreError> {
        check_template(template, self.cells.len())?;
        self.layout = template.layout().clone();
        Ok(())
    }

    /// Area inside the padding.
    fn content_bounds(&self) -> Rect {
        Rect::from_size(self.size()).inset(self.style.padding)
    }

    /// Placement of every cell in pre-order.
    pub fn placements(&self) -> Vec<CellPlacement> {
        self.layout
            .cell_rects(self.content_bounds(), self.style.spacing)
            .into_iter()
            .filter_map(|(id, rect)| {
                let cell = self.cells.get(id.0 as usize)?;
                Some(CellPlacement {
                    cell: id,
                    image: cell.image.id(),
                    rect,
                    crop: source_crop(cell.image.size(), rect.size(), cell.transform),
                })
            })
            .collect()
    }

    /// All draggable dividers.
    pub fn dividers(&self) -> Vec<Divider> {
        self.layout
            .dividers(self.content_bounds(), self.style.spacing)
    }

    /// Moves a divider by `delta` collage units (see [`LayoutNode::drag_divider`]).
    pub fn drag_divider(&mut self, id: DividerId, delta: Vec2) -> Result<(), CoreError> {
        let bounds = self.content_bounds();
        self.layout
            .drag_divider(bounds, self.style.spacing, id, delta)
    }

    /// The cell at `point` (collage units), if any. Points in gaps or padding hit no cell.
    pub fn cell_at(&self, point: Vec2) -> Option<CellId> {
        self.placements()
            .into_iter()
            .find(|p| p.rect.contains(point))
            .map(|p| p.cell)
    }

    /// Moves the image inside a cell by `delta` collage units, as if dragged with a finger:
    /// dragging right reveals more of the image's left side. Non-finite deltas are ignored.
    pub fn pan(&mut self, id: CellId, delta: Vec2) -> Result<(), CoreError> {
        let (rect, crop) = self.cell_rect_and_crop(id)?;
        if !(delta.x.is_finite() && delta.y.is_finite()) || rect.size().is_empty() {
            return Ok(());
        }
        let center = crop.center();
        let new_center = Vec2::new(
            center.x - delta.x / rect.width * crop.width,
            center.y - delta.y / rect.height * crop.height,
        );
        self.set_transform_clamped(
            id,
            rect,
            CellTransform::new(self.cell(id)?.transform.zoom(), new_center),
        )
    }

    /// Sets the framing of a cell, clamped like [`Collage::pan`] so the image still covers the cell.
    /// Used to restore an earlier framing, e.g. to undo a pan.
    pub fn set_transform(&mut self, id: CellId, transform: CellTransform) -> Result<(), CoreError> {
        let (rect, _) = self.cell_rect_and_crop(id)?;
        self.set_transform_clamped(id, rect, transform)
    }

    /// Zooms the image inside a cell by `factor` (e.g. pinch scale), keeping the image point under `focus`
    /// fixed. `focus` is relative to the cell (`0.0..=1.0`, `(0.5, 0.5)` = center).
    /// Non-finite or non-positive factors and non-finite focus points are ignored.
    pub fn zoom(&mut self, id: CellId, factor: f32, focus: Vec2) -> Result<(), CoreError> {
        let (rect, crop) = self.cell_rect_and_crop(id)?;
        if !(factor.is_finite() && factor > 0.0 && focus.x.is_finite() && focus.y.is_finite()) {
            return Ok(());
        }
        let old_zoom = self.cell(id)?.transform.zoom();
        let probe = CellTransform::new(old_zoom * factor, Vec2::default());
        let new_zoom = probe.zoom();
        let focus = Vec2::new(focus.x.clamp(0.0, 1.0), focus.y.clamp(0.0, 1.0));
        let anchor = Vec2::new(
            crop.x + focus.x * crop.width,
            crop.y + focus.y * crop.height,
        );
        let scale = old_zoom / new_zoom;
        let (width, height) = (crop.width * scale, crop.height * scale);
        let new_center = Vec2::new(
            anchor.x - focus.x * width + width / 2.0,
            anchor.y - focus.y * height + height / 2.0,
        );
        self.set_transform_clamped(id, rect, CellTransform::new(new_zoom, new_center))
    }

    /// Swaps the images (including their framing) of two cells.
    pub fn swap(&mut self, a: CellId, b: CellId) -> Result<(), CoreError> {
        self.cell(a)?;
        self.cell(b)?;
        self.cells.swap(a.0 as usize, b.0 as usize);
        Ok(())
    }

    fn cell(&self, id: CellId) -> Result<&Cell, CoreError> {
        self.cells
            .get(id.0 as usize)
            .ok_or(CoreError::UnknownCell(id))
    }

    fn cell_rect_and_crop(&self, id: CellId) -> Result<(Rect, Rect), CoreError> {
        self.placements()
            .into_iter()
            .find(|p| p.cell == id)
            .map(|p| (p.rect, p.crop))
            .ok_or(CoreError::UnknownCell(id))
    }

    /// Stores `transform` with its center replaced by the effective (clamped) crop center,
    /// so later pans start from what the user actually sees.
    fn set_transform_clamped(
        &mut self,
        id: CellId,
        rect: Rect,
        transform: CellTransform,
    ) -> Result<(), CoreError> {
        let cell = self
            .cells
            .get_mut(id.0 as usize)
            .ok_or(CoreError::UnknownCell(id))?;
        let crop = source_crop(cell.image.size(), rect.size(), transform);
        cell.transform = CellTransform::new(transform.zoom(), crop.center());
        Ok(())
    }
}

fn check_image_count(count: usize) -> Result<(), CoreError> {
    match count {
        0 => Err(CoreError::NoImages),
        n if n > MAX_IMAGES => Err(CoreError::TooManyImages(n)),
        _ => Ok(()),
    }
}

fn check_template(template: &Template, images: usize) -> Result<(), CoreError> {
    if template.cell_count() != images {
        return Err(CoreError::TemplateMismatch {
            template: template.cell_count(),
            images,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::MIN_CELL_SIZE;
    use crate::templates::templates_for;

    const EPS: f32 = 1e-4;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < EPS
    }

    fn images(sizes: &[(u32, u32)]) -> Vec<ImageMeta> {
        sizes
            .iter()
            .enumerate()
            .map(|(i, &(w, h))| ImageMeta::new(ImageId(i as u32), w, h).unwrap())
            .collect()
    }

    fn collage(sizes: &[(u32, u32)]) -> Collage {
        let imgs = images(sizes);
        Collage::new(&imgs, &templates_for(imgs.len())[0]).unwrap()
    }

    #[test]
    fn new_rejects_invalid_image_counts() {
        let t1 = &templates_for(1)[0];
        assert_eq!(Collage::new(&[], t1), Err(CoreError::NoImages));
        let many = images(&[(10, 10); MAX_IMAGES + 1]);
        assert_eq!(
            Collage::new(&many, t1),
            Err(CoreError::TooManyImages(MAX_IMAGES + 1))
        );
    }

    #[test]
    fn new_rejects_mismatching_template() {
        let imgs = images(&[(10, 10), (10, 10)]);
        let result = Collage::new(&imgs, &templates_for(3)[0]);
        assert_eq!(
            result,
            Err(CoreError::TemplateMismatch {
                template: 3,
                images: 2
            })
        );
    }

    #[test]
    fn image_meta_rejects_zero_size() {
        assert_eq!(
            ImageMeta::new(ImageId(0), 0, 10),
            Err(CoreError::InvalidImageSize {
                width: 0,
                height: 10
            })
        );
    }

    #[test]
    fn placements_cover_content_area_and_images_in_order() {
        let c = collage(&[(100, 100), (200, 100), (100, 200), (50, 50)]);
        let placements = c.placements();
        assert_eq!(placements.len(), 4);
        for (i, p) in placements.iter().enumerate() {
            assert_eq!(p.image, ImageId(i as u32));
            assert!(p.rect.x >= c.style().padding - EPS);
            assert!(p.rect.right() <= 1.0 - c.style().padding + EPS);
        }
    }

    #[test]
    fn aspect_ratio_changes_collage_height() {
        let mut c = collage(&[(100, 100)]);
        c.set_aspect(AspectRatio::Story9x16);
        assert!(close(c.size().height, 16.0 / 9.0));
        let rect = c.placements()[0].rect;
        assert!(close(rect.bottom(), 16.0 / 9.0 - c.style().padding));
    }

    #[test]
    fn style_is_clamped() {
        let mut c = collage(&[(100, 100)]);
        c.set_style(Style {
            spacing: 1.0,
            padding: -1.0,
            corner_radius: f32::NAN,
            background: Rgba::new(0, 0, 0, 255),
        });
        assert!(close(c.style().spacing, MAX_SPACING));
        assert!(close(c.style().padding, 0.0));
        assert!(close(c.style().corner_radius, 0.0));
    }

    #[test]
    fn apply_template_keeps_images() {
        let mut c = collage(&[(100, 100), (100, 100), (100, 100)]);
        let other = templates_for(3).pop().unwrap();
        c.apply_template(&other).unwrap();
        assert_eq!(c.layout(), other.layout());
        let ids: Vec<ImageId> = c.placements().iter().map(|p| p.image).collect();
        assert_eq!(ids, vec![ImageId(0), ImageId(1), ImageId(2)]);
        assert!(c.apply_template(&templates_for(2)[0]).is_err());
    }

    #[test]
    fn swap_exchanges_images_and_framing() {
        let mut c = collage(&[(100, 100), (300, 100)]);
        c.zoom(CellId(0), 2.0, Vec2::new(0.5, 0.5)).unwrap();
        let before = c.cells().to_vec();
        c.swap(CellId(0), CellId(1)).unwrap();
        assert_eq!(c.cells()[0], before[1]);
        assert_eq!(c.cells()[1], before[0]);
        assert_eq!(
            c.swap(CellId(0), CellId(9)),
            Err(CoreError::UnknownCell(CellId(9)))
        );
    }

    #[test]
    fn cell_at_finds_cells_and_misses_gaps() {
        let mut c = collage(&[(100, 100), (100, 100)]);
        c.set_style(Style {
            spacing: 0.04,
            ..Style::default()
        });
        assert_eq!(c.cell_at(Vec2::new(0.25, 0.5)), Some(CellId(0)));
        assert_eq!(c.cell_at(Vec2::new(0.75, 0.5)), Some(CellId(1)));
        assert_eq!(c.cell_at(Vec2::new(0.5, 0.5)), None);
        assert_eq!(c.cell_at(Vec2::new(0.001, 0.5)), None);
    }

    #[test]
    fn pan_moves_crop_opposite_to_finger() {
        // Wide image in a square cell: horizontal room to pan.
        let mut c = collage(&[(400, 100)]);
        let before = c.placements()[0].crop;
        c.pan(CellId(0), Vec2::new(0.1, 0.0)).unwrap();
        let after = c.placements()[0].crop;
        assert!(after.x < before.x);
        assert!(close(after.y, before.y));
    }

    #[test]
    fn pan_is_clamped_and_reversible_from_edge() {
        let mut c = collage(&[(400, 100)]);
        c.pan(CellId(0), Vec2::new(100.0, 0.0)).unwrap();
        assert!(close(c.placements()[0].crop.x, 0.0));
        // Panning back immediately moves away from the edge (no hidden overshoot).
        c.pan(CellId(0), Vec2::new(-0.05, 0.0)).unwrap();
        assert!(c.placements()[0].crop.x > 0.0);
    }

    #[test]
    fn zoom_keeps_focus_point_fixed() {
        let mut c = collage(&[(100, 100)]);
        let focus = Vec2::new(0.25, 0.75);
        let crop = c.placements()[0].crop;
        let anchor = (
            crop.x + focus.x * crop.width,
            crop.y + focus.y * crop.height,
        );
        c.zoom(CellId(0), 2.0, focus).unwrap();
        let crop = c.placements()[0].crop;
        let after = (
            crop.x + focus.x * crop.width,
            crop.y + focus.y * crop.height,
        );
        assert!(close(anchor.0, after.0) && close(anchor.1, after.1));
        assert!(close(c.cells()[0].transform.zoom(), 2.0));
    }

    #[test]
    fn zoom_is_limited() {
        let mut c = collage(&[(100, 100)]);
        c.zoom(CellId(0), 0.1, Vec2::new(0.5, 0.5)).unwrap();
        assert!(close(c.cells()[0].transform.zoom(), 1.0));
        c.zoom(CellId(0), 1000.0, Vec2::new(0.5, 0.5)).unwrap();
        assert!(close(c.cells()[0].transform.zoom(), crate::MAX_ZOOM));
    }

    #[test]
    fn dragging_divider_through_collage_resizes_cells() {
        let mut c = collage(&[(100, 100), (100, 100)]);
        let before = c.placements()[0].rect.width;
        c.drag_divider(c.dividers()[0].id, Vec2::new(0.1, 0.0))
            .unwrap();
        assert!(close(c.placements()[0].rect.width, before + 0.1));
    }

    #[test]
    fn pan_and_zoom_reject_unknown_cell() {
        let mut c = collage(&[(100, 100)]);
        let unknown = CellId(5);
        assert_eq!(
            c.pan(unknown, Vec2::new(0.1, 0.1)),
            Err(CoreError::UnknownCell(unknown))
        );
        assert_eq!(
            c.zoom(unknown, 2.0, Vec2::new(0.5, 0.5)),
            Err(CoreError::UnknownCell(unknown))
        );
    }

    #[test]
    fn pan_vertically_when_zoomed() {
        let mut c = collage(&[(100, 100)]);
        c.zoom(CellId(0), 2.0, Vec2::new(0.5, 0.5)).unwrap();
        let before = c.placements()[0].crop;
        c.pan(CellId(0), Vec2::new(0.0, -0.1)).unwrap();
        let after = c.placements()[0].crop;
        assert!(after.y > before.y, "dragging up reveals the lower part");
        assert!(close(after.x, before.x));
    }

    #[test]
    fn set_transform_restores_an_earlier_framing() {
        let mut c = collage(&[(400, 100)]);
        let before = c.cells()[0].transform;
        c.pan(CellId(0), Vec2::new(0.2, 0.0)).unwrap();
        assert_ne!(c.cells()[0].transform, before);
        c.set_transform(CellId(0), before).unwrap();
        assert_eq!(c.cells()[0].transform, before);
        assert_eq!(
            c.set_transform(CellId(5), before),
            Err(CoreError::UnknownCell(CellId(5)))
        );
    }

    #[test]
    fn invalid_pan_and_zoom_input_is_ignored() {
        let mut c = collage(&[(400, 100)]);
        c.zoom(CellId(0), 2.0, Vec2::new(0.3, 0.3)).unwrap();
        let before = c.cells()[0].transform;
        c.pan(CellId(0), Vec2::new(f32::NAN, 0.0)).unwrap();
        c.pan(CellId(0), Vec2::new(f32::INFINITY, 0.0)).unwrap();
        c.zoom(CellId(0), f32::NAN, Vec2::new(0.5, 0.5)).unwrap();
        c.zoom(CellId(0), -2.0, Vec2::new(0.5, 0.5)).unwrap();
        c.zoom(CellId(0), 0.0, Vec2::new(0.5, 0.5)).unwrap();
        c.zoom(CellId(0), 2.0, Vec2::new(f32::INFINITY, 0.5))
            .unwrap();
        assert_eq!(c.cells()[0].transform, before);
    }

    #[test]
    fn zoom_with_focus_at_corner_keeps_corner_fixed() {
        // 3:2 image in a square cell: at zoom 1 the visible area spans x = 1/6..5/6 and y = 0..1.
        let mut c = collage(&[(300, 200)]);
        c.zoom(CellId(0), 3.0, Vec2::new(1.0, 1.0)).unwrap();
        let crop = c.placements()[0].crop;
        assert!(close(crop.right(), 5.0 / 6.0), "{crop:?}");
        assert!(close(crop.bottom(), 1.0), "{crop:?}");
        assert!(
            close(crop.width, 2.0 / 9.0) && close(crop.height, 1.0 / 3.0),
            "{crop:?}"
        );
    }

    #[test]
    fn swap_with_itself_changes_nothing() {
        let mut c = collage(&[(100, 100), (200, 100)]);
        let before = c.clone();
        c.swap(CellId(1), CellId(1)).unwrap();
        assert_eq!(c, before);
    }

    #[test]
    fn drag_after_template_change_uses_new_layout() {
        let mut c = collage(&[(100, 100), (100, 100), (100, 100)]);
        let other = templates_for(3).pop().unwrap();
        c.apply_template(&other).unwrap();
        let dividers = c.dividers();
        assert_eq!(dividers.len(), 2);
        let before = c.placements();
        c.drag_divider(dividers[0].id, Vec2::new(0.05, 0.05))
            .unwrap();
        assert_ne!(c.placements(), before);
    }

    /// All templates, formats and extreme styles: cells stay inside the content area, never overlap
    /// and are large enough that dividers can still be dragged.
    #[test]
    fn all_templates_produce_valid_cells_in_every_format() {
        let styles = [
            Style::default(),
            Style {
                spacing: MAX_SPACING,
                padding: MAX_PADDING,
                corner_radius: MAX_CORNER_RADIUS,
                background: Rgba::WHITE,
            },
        ];
        for count in 1..=MAX_IMAGES {
            let imgs = images(&vec![(300, 200); count]);
            for template in templates_for(count) {
                for aspect in AspectRatio::ALL {
                    for style in styles {
                        let mut c = Collage::new(&imgs, &template).unwrap();
                        c.set_aspect(aspect);
                        c.set_style(style);
                        let name = format!("{} {aspect:?} {style:?}", template.name());
                        let content = Rect::from_size(c.size()).inset(c.style().padding);
                        let placements = c.placements();
                        assert_eq!(placements.len(), count, "{name}");
                        for (i, a) in placements.iter().enumerate() {
                            let r = a.rect;
                            assert!(r.x >= content.x - EPS && r.y >= content.y - EPS, "{name}");
                            assert!(r.right() <= content.right() + EPS, "{name}");
                            assert!(r.bottom() <= content.bottom() + EPS, "{name}");
                            assert!(
                                r.width >= MIN_CELL_SIZE && r.height >= MIN_CELL_SIZE,
                                "{name}: {r:?}"
                            );
                            for b in placements.iter().skip(i + 1) {
                                let s = b.rect;
                                let overlap_x = r.x < s.right() - EPS && s.x < r.right() - EPS;
                                let overlap_y = r.y < s.bottom() - EPS && s.y < r.bottom() - EPS;
                                assert!(!(overlap_x && overlap_y), "{name}: overlap");
                            }
                        }
                    }
                }
            }
        }
    }
}
