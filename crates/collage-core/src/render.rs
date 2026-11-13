//! Export renderer: decodes the images, crops them with [`crate::source_crop`] (via
//! [`Collage::placements`], exactly like the preview), composes the collage and encodes it as JPEG.

use std::path::PathBuf;

use image::codecs::jpeg::JpegEncoder;
use image::imageops::{self, FilterType};
use image::metadata::Orientation;
use image::{DynamicImage, Rgb, RgbImage, RgbaImage};

use crate::collage::{Collage, ImageId, Rgba};
use crate::decode::decode_unoriented;
use crate::error::CoreError;
use crate::geometry::Rect;

/// Default length of the longer edge of the exported image in pixels.
pub const EXPORT_LONG_EDGE: u32 = 3000;
/// Largest long edge accepted by the renderer (bounds memory for the output image).
pub const MAX_EXPORT_LONG_EDGE: u32 = 8192;
/// Default JPEG quality (1–100).
pub const EXPORT_JPEG_QUALITY: u8 = 90;

/// Where the original pixels of an image live.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageSource {
    /// Image this file belongs to.
    pub id: ImageId,
    /// Path of the original file.
    pub path: PathBuf,
}

/// Export settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExportOptions {
    /// Length of the longer output edge in pixels; clamped to `1..=MAX_EXPORT_LONG_EDGE`.
    pub long_edge: u32,
    /// JPEG quality; clamped to `1..=100`.
    pub jpeg_quality: u8,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            long_edge: EXPORT_LONG_EDGE,
            jpeg_quality: EXPORT_JPEG_QUALITY,
        }
    }
}

/// Renders `collage` and encodes it as JPEG.
///
/// `sources` must contain a file for every image used by the collage.
pub fn render(
    collage: &Collage,
    sources: &[ImageSource],
    opts: ExportOptions,
) -> Result<Vec<u8>, CoreError> {
    let canvas = compose(collage, sources, opts)?;
    encode_jpeg(&canvas, opts.jpeg_quality)
}

/// Pixel size of the export: the long edge is `long_edge` (sanitized), the other follows the aspect ratio.
fn output_size(collage: &Collage, long_edge: u32) -> (u32, u32) {
    let long = long_edge.clamp(1, MAX_EXPORT_LONG_EDGE);
    // `Collage::size` derives from a fixed set of aspect ratios, so both sides are finite and positive.
    // Even a degenerate size would only saturate to 1 pixel below (no panic, no NaN reaching `compose`).
    let size = collage.size();
    let long_f = long as f32;
    let scaled = |short: f32, long_side: f32| ((long_f * short / long_side).round() as u32).max(1);
    if size.width >= size.height {
        (long, scaled(size.height, size.width))
    } else {
        (scaled(size.width, size.height), long)
    }
}

/// Composes the collage into an RGB image (no encoding).
pub(crate) fn compose(
    collage: &Collage,
    sources: &[ImageSource],
    opts: ExportOptions,
) -> Result<RgbImage, CoreError> {
    let (width, height) = output_size(collage, opts.long_edge);
    let style = collage.style();
    let mut canvas = RgbImage::from_pixel(width, height, background_rgb(style.background));
    let size = collage.size();
    // Pixels per collage unit; x and y are equal up to rounding of the canvas size.
    let scale_x = width as f32 / size.width;
    let scale_y = height as f32 / size.height;
    let radius_px = style.corner_radius * scale_x;

    for placement in collage.placements() {
        let source = sources
            .iter()
            .find(|s| s.id == placement.image)
            .ok_or(CoreError::MissingSource(placement.image))?;
        let Some((x0, y0, x1, y1)) = pixel_rect(placement.rect, scale_x, scale_y, width, height)
        else {
            continue;
        };
        // One image at a time. The image is decoded unoriented and consumed by `render_cell`, so the full-size
        // pixels are freed before the next image is decoded.
        let (image, orientation) = decode_unoriented(&source.path)?;
        let cell = render_cell(image, orientation, placement.crop, x1 - x0, y1 - y0);
        blit_rounded(&mut canvas, &cell, x0, y0, radius_px);
    }
    Ok(canvas)
}

/// Background color blended over white (JPEG has no alpha).
fn background_rgb(color: Rgba) -> Rgb<u8> {
    let a = u32::from(color.a);
    let mix = |c: u8| ((u32::from(c) * a + 255 * (255 - a) + 127) / 255) as u8;
    Rgb([mix(color.r), mix(color.g), mix(color.b)])
}

/// Integer pixel rect `(x0, y0, x1, y1)` of a collage-unit rect. Edges are rounded (not origin and size), so
/// cells that touch in collage units touch in pixels as well. `None` if the rect has no pixels.
fn pixel_rect(
    rect: Rect,
    scale_x: f32,
    scale_y: f32,
    width: u32,
    height: u32,
) -> Option<(u32, u32, u32, u32)> {
    let edge = |v: f32, scale: f32, max: u32| -> u32 {
        let p = (v * scale).round();
        if p.is_nan() {
            0
        } else {
            p.clamp(0.0, max as f32) as u32
        }
    };
    let x0 = edge(rect.x, scale_x, width);
    let y0 = edge(rect.y, scale_y, height);
    let x1 = edge(rect.right(), scale_x, width);
    let y1 = edge(rect.bottom(), scale_y, height);
    (x1 > x0 && y1 > y0).then_some((x0, y0, x1, y1))
}

/// Cuts the normalized `crop` (relative to the *oriented* image) out of `image` and resizes it to
/// `width` x `height`.
///
/// Memory: the crop is mapped back into the stored (unoriented) pixels, read through a view without copying
/// and resized straight to the small cell size; the orientation is applied to that small result only. Peak
/// memory is therefore the decoded image plus the cell (8-bit RGB, RGBA and gray sources). Other formats
/// (16-bit, float, gray+alpha) need one 8-bit RGBA copy of the visible region on top of the decoded image.
fn render_cell(
    image: DynamicImage,
    orientation: Orientation,
    crop: Rect,
    width: u32,
    height: u32,
) -> RgbaImage {
    let (iw, ih) = (image.width(), image.height());
    let (ow, oh) = if swaps_axes(orientation) {
        (ih, iw)
    } else {
        (iw, ih)
    };
    let edge = |v: f32, max: u32| -> u32 {
        let p = (v * max as f32).round();
        if p.is_nan() {
            0
        } else {
            p.clamp(0.0, max as f32) as u32
        }
    };
    let x0 = edge(crop.x, ow).min(ow.saturating_sub(1));
    let y0 = edge(crop.y, oh).min(oh.saturating_sub(1));
    let x1 = edge(crop.right(), ow).clamp(x0 + 1, ow);
    let y1 = edge(crop.bottom(), oh).clamp(y0 + 1, oh);
    let region = stored_region(orientation, iw, ih, (x0, y0, x1, y1));
    // The cell in stored orientation has swapped sides when the orientation swaps axes.
    let (tw, th) = if swaps_axes(orientation) {
        (height, width)
    } else {
        (width, height)
    };
    let cell = resize_region(image, region, tw, th);
    let mut oriented = DynamicImage::ImageRgba8(cell);
    oriented.apply_orientation(orientation);
    oriented.into_rgba8()
}

/// Whether the orientation transposes width and height.
fn swaps_axes(orientation: Orientation) -> bool {
    matches!(
        orientation,
        Orientation::Rotate90
            | Orientation::Rotate270
            | Orientation::Rotate90FlipH
            | Orientation::Rotate270FlipH
    )
}

/// Maps the pixel `(x, y)` of the oriented image to the pixel of the stored `iw` x `ih` image.
fn stored_pixel(orientation: Orientation, iw: u32, ih: u32, x: u32, y: u32) -> (u32, u32) {
    let flip_x = |v: u32| iw.saturating_sub(1).saturating_sub(v);
    let flip_y = |v: u32| ih.saturating_sub(1).saturating_sub(v);
    match orientation {
        Orientation::NoTransforms => (x, y),
        Orientation::Rotate180 => (flip_x(x), flip_y(y)),
        Orientation::FlipHorizontal => (flip_x(x), y),
        Orientation::FlipVertical => (x, flip_y(y)),
        Orientation::Rotate90 => (y, flip_y(x)),
        Orientation::Rotate270 => (flip_x(y), x),
        Orientation::Rotate90FlipH => (y, x),
        Orientation::Rotate270FlipH => (flip_x(y), flip_y(x)),
    }
}

/// Maps the half-open pixel rect `(x0, y0, x1, y1)` of the oriented image to `(x, y, width, height)` in the
/// stored `iw` x `ih` image. All orientations are axis-aligned, so two opposite corner pixels suffice.
fn stored_region(
    orientation: Orientation,
    iw: u32,
    ih: u32,
    (x0, y0, x1, y1): (u32, u32, u32, u32),
) -> (u32, u32, u32, u32) {
    let (ax, ay) = stored_pixel(orientation, iw, ih, x0, y0);
    let (bx, by) = stored_pixel(
        orientation,
        iw,
        ih,
        x1.saturating_sub(1).max(x0),
        y1.saturating_sub(1).max(y0),
    );
    let (left, right) = (ax.min(bx), ax.max(bx));
    let (top, bottom) = (ay.min(by), ay.max(by));
    (left, top, right - left + 1, bottom - top + 1)
}

/// Resizes `region` `(x, y, width, height)` of `image` to `tw` x `th` pixels as RGBA.
///
/// Consumes the image so it is freed as soon as the small result exists. Sources with alpha are resized
/// premultiplied (and un-premultiplied afterwards) so colors of transparent pixels do not bleed into edges.
fn resize_region(
    image: DynamicImage,
    (x, y, w, h): (u32, u32, u32, u32),
    tw: u32,
    th: u32,
) -> RgbaImage {
    const FILTER: FilterType = FilterType::CatmullRom;
    match image {
        DynamicImage::ImageRgb8(buf) => {
            let view = imageops::crop_imm(&buf, x, y, w, h);
            DynamicImage::ImageRgb8(imageops::resize(&*view, tw, th, FILTER)).into_rgba8()
        }
        DynamicImage::ImageLuma8(buf) => {
            let view = imageops::crop_imm(&buf, x, y, w, h);
            DynamicImage::ImageLuma8(imageops::resize(&*view, tw, th, FILTER)).into_rgba8()
        }
        DynamicImage::ImageRgba8(mut buf) => {
            // In place, so no second full-size buffer is needed.
            premultiply(&mut buf);
            let view = imageops::crop_imm(&buf, x, y, w, h);
            let mut cell = imageops::resize(&*view, tw, th, FILTER);
            unpremultiply(&mut cell);
            cell
        }
        other => {
            // 16-bit, float and gray+alpha: one 8-bit RGBA copy of the visible region (the decoded image is
            // still alive for the duration of the copy), then the same path as RGBA.
            let mut region = imageops::crop_imm(&other, x, y, w, h).to_image();
            drop(other);
            premultiply(&mut region);
            let mut cell = imageops::resize(&region, tw, th, FILTER);
            unpremultiply(&mut cell);
            cell
        }
    }
}

/// Multiplies the color channels with alpha (opaque pixels are untouched).
fn premultiply(img: &mut RgbaImage) {
    for image::Rgba([r, g, b, a]) in img.pixels_mut() {
        if *a < 255 {
            for c in [r, g, b] {
                *c = ((u32::from(*c) * u32::from(*a) + 127) / 255) as u8;
            }
        }
    }
}

/// Inverse of [`premultiply`].
fn unpremultiply(img: &mut RgbaImage) {
    for image::Rgba([r, g, b, a]) in img.pixels_mut() {
        match *a {
            255 => {}
            0 => (*r, *g, *b) = (0, 0, 0),
            alpha => {
                let alpha = u32::from(alpha);
                for c in [r, g, b] {
                    *c = ((u32::from(*c) * 255 + alpha / 2) / alpha).min(255) as u8;
                }
            }
        }
    }
}

/// Coverage (0..=1) of the pixel at `(px, py)` in a `width` x `height` rounded rectangle with `radius`.
/// Uses the signed distance of the pixel center to the shape, which gives a one-pixel antialiased edge.
fn coverage(px: u32, py: u32, width: u32, height: u32, radius: f32) -> f32 {
    if radius <= 0.0 {
        return 1.0;
    }
    let (w, h) = (width as f32, height as f32);
    let (half_w, half_h) = (w / 2.0, h / 2.0);
    let dx = (px as f32 + 0.5 - half_w).abs() - (half_w - radius);
    let dy = (py as f32 + 0.5 - half_h).abs() - (half_h - radius);
    let outside = (dx.max(0.0).powi(2) + dy.max(0.0).powi(2)).sqrt();
    let distance = outside + dx.max(dy).min(0.0) - radius;
    (0.5 - distance).clamp(0.0, 1.0)
}

/// Draws `cell` onto `canvas` at `(x0, y0)`, blending with the rounded-corner coverage and the cell's alpha.
fn blit_rounded(canvas: &mut RgbImage, cell: &RgbaImage, x0: u32, y0: u32, radius: f32) {
    let (w, h) = cell.dimensions();
    let radius = radius.clamp(0.0, w.min(h) as f32 / 2.0);
    for (px, py, src) in cell.enumerate_pixels() {
        let alpha = coverage(px, py, w, h, radius) * f32::from(src.0[3]) / 255.0;
        if alpha <= 0.0 {
            continue;
        }
        let Some(dst) = canvas.get_pixel_mut_checked(x0 + px, y0 + py) else {
            continue;
        };
        for (d, s) in dst.0.iter_mut().zip(src.0.iter()) {
            let mixed = f32::from(*s) * alpha + f32::from(*d) * (1.0 - alpha);
            *d = mixed.round().clamp(0.0, 255.0) as u8;
        }
    }
}

fn encode_jpeg(canvas: &RgbImage, quality: u8) -> Result<Vec<u8>, CoreError> {
    let mut bytes = Vec::new();
    JpegEncoder::new_with_quality(&mut bytes, quality.clamp(1, 100))
        .encode_image(canvas)
        .map_err(|e| CoreError::Encode(e.to_string()))?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collage::{AspectRatio, ImageMeta, Style};
    use crate::crop::CellTransform;
    use crate::geometry::Vec2;
    use crate::layout::CellId;
    use crate::templates::templates_for;
    use std::path::Path;

    const RED: [u8; 3] = [220, 30, 30];
    const BLUE: [u8; 3] = [30, 30, 220];
    const BG: Rgba = Rgba::new(10, 200, 10, 255);
    const BG_PX: [u8; 3] = [10, 200, 10];

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("dimuli-render-{name}-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    fn write_png(dir: &Path, name: &str, img: &RgbImage) -> PathBuf {
        let path = dir.join(name);
        img.save(&path).unwrap();
        path
    }

    fn solid(w: u32, h: u32, c: [u8; 3]) -> RgbImage {
        RgbImage::from_pixel(w, h, Rgb(c))
    }

    /// Collage with one image per entry, first template for that count, plus the matching sources.
    fn setup(dir: &Path, images: &[RgbImage]) -> (Collage, Vec<ImageSource>) {
        let mut metas = Vec::new();
        let mut sources = Vec::new();
        for (i, img) in images.iter().enumerate() {
            let id = ImageId(i as u32);
            metas.push(ImageMeta::new(id, img.width(), img.height()).unwrap());
            sources.push(ImageSource {
                id,
                path: write_png(dir, &format!("{i}.png"), img),
            });
        }
        let collage = Collage::new(&metas, &templates_for(images.len())[0]).unwrap();
        (collage, sources)
    }

    fn opts(long_edge: u32) -> ExportOptions {
        ExportOptions {
            long_edge,
            jpeg_quality: 90,
        }
    }

    fn style(spacing: f32, padding: f32, corner_radius: f32) -> Style {
        Style {
            spacing,
            padding,
            corner_radius,
            background: BG,
        }
    }

    fn px(img: &RgbImage, x: u32, y: u32) -> [u8; 3] {
        img.get_pixel(x, y).0
    }

    /// Pixel at the center of a collage-unit point.
    fn at(img: &RgbImage, p: Vec2) -> [u8; 3] {
        let size = img.width() as f32;
        px(img, (p.x * size) as u32, (p.y * size) as u32)
    }

    fn assert_color(actual: [u8; 3], expected: [u8; 3]) {
        for (a, e) in actual.iter().zip(expected.iter()) {
            assert!(a.abs_diff(*e) <= 2, "{actual:?} != {expected:?}");
        }
    }

    #[test]
    fn two_solid_images_fill_their_cells() {
        let dir = temp_dir("solid");
        let (mut collage, sources) = setup(&dir, &[solid(40, 30, RED), solid(30, 40, BLUE)]);
        collage.set_style(style(0.05, 0.05, 0.0));
        let canvas = compose(&collage, &sources, opts(200)).unwrap();
        let placements = collage.placements();
        assert_eq!(placements.len(), 2);
        let colors = [RED, BLUE];
        for (p, color) in placements.iter().zip(colors) {
            assert_color(at(&canvas, p.rect.center()), color);
        }
        // Padding corner and the gap between the cells are background.
        assert_color(px(&canvas, 1, 1), BG_PX);
        let (a, b) = (placements[0].rect, placements[1].rect);
        let gap = Vec2::new((a.right() + b.x) / 2.0, (a.bottom() + b.y) / 2.0);
        // The gap lies between the cells along the split axis; sample the cell-center line of the other axis.
        let probe = if a.right() < b.x {
            Vec2::new(gap.x, a.center().y)
        } else {
            Vec2::new(a.center().x, gap.y)
        };
        assert_color(at(&canvas, probe), BG_PX);
    }

    #[test]
    fn zero_spacing_leaves_no_gap_between_cells() {
        let dir = temp_dir("nogap");
        let (mut collage, sources) = setup(&dir, &[solid(40, 30, RED), solid(30, 40, BLUE)]);
        collage.set_style(style(0.0, 0.0, 0.0));
        let canvas = compose(&collage, &sources, opts(101)).unwrap();
        for p in canvas.pixels() {
            assert!(p.0 != BG_PX, "background pixel found");
        }
    }

    #[test]
    fn crop_shows_only_the_selected_half() {
        let dir = temp_dir("crop");
        // Left half red, right half blue; square cell with a wide image at zoom 2 on the left side.
        let mut img = solid(60, 30, RED);
        for y in 0..30 {
            for x in 30..60 {
                img.put_pixel(x, y, Rgb(BLUE));
            }
        }
        let (mut collage, sources) = setup(&dir, &[img]);
        collage.set_style(style(0.0, 0.0, 0.0));
        collage
            .set_transform(CellId(0), CellTransform::new(2.0, Vec2::new(0.0, 0.5)))
            .unwrap();
        let canvas = compose(&collage, &sources, opts(64)).unwrap();
        for (x, y) in [(1, 1), (32, 32), (62, 62), (2, 60)] {
            assert_color(px(&canvas, x, y), RED);
        }
        collage
            .set_transform(CellId(0), CellTransform::new(2.0, Vec2::new(1.0, 0.5)))
            .unwrap();
        let canvas = compose(&collage, &sources, opts(64)).unwrap();
        for (x, y) in [(1, 1), (32, 32), (62, 62), (2, 60)] {
            assert_color(px(&canvas, x, y), BLUE);
        }
    }

    #[test]
    fn corner_radius_rounds_cell_corners() {
        let dir = temp_dir("radius");
        let (mut collage, sources) = setup(&dir, &[solid(30, 30, RED)]);
        collage.set_style(style(0.0, 0.0, 0.1));
        let canvas = compose(&collage, &sources, opts(200)).unwrap();
        assert_color(px(&canvas, 0, 0), BG_PX);
        assert_color(px(&canvas, 199, 199), BG_PX);
        assert_color(px(&canvas, 100, 100), RED);
        // Straight edges away from the corners stay filled.
        assert_color(px(&canvas, 100, 0), RED);
        // Antialiasing: a pixel on the arc is a mix, neither pure background nor pure image.
        let mixed = (0..20)
            .flat_map(|y| (0..20).map(move |x| (x, y)))
            .map(|(x, y)| px(&canvas, x, y))
            .any(|p| p != BG_PX && p != RED);
        assert!(mixed, "no antialiased edge pixel in the corner");
    }

    #[test]
    fn huge_radius_is_clamped_for_small_cells() {
        let mut canvas = RgbImage::from_pixel(20, 20, Rgb(BG_PX));
        let cell = solid(10, 4, RED);
        let cell = DynamicImage::ImageRgb8(cell).into_rgba8();
        blit_rounded(&mut canvas, &cell, 5, 5, 1000.0);
        assert_color(px(&canvas, 10, 7), RED);
    }

    #[test]
    fn output_size_follows_aspect_and_long_edge() {
        let dir = temp_dir("size");
        let (mut collage, _) = setup(&dir, &[solid(10, 10, RED)]);
        let expected = [
            (AspectRatio::Square, (1000, 1000)),
            (AspectRatio::Portrait4x5, (800, 1000)),
            (AspectRatio::Story9x16, (563, 1000)),
            (AspectRatio::Landscape16x9, (1000, 563)),
        ];
        for (aspect, size) in expected {
            collage.set_aspect(aspect);
            assert_eq!(output_size(&collage, 1000), size, "{aspect:?}");
        }
    }

    #[test]
    fn invalid_options_are_sanitized() {
        let dir = temp_dir("sanitize");
        let (mut collage, sources) = setup(&dir, &[solid(10, 10, RED)]);
        collage.set_aspect(AspectRatio::Story9x16);
        assert_eq!(output_size(&collage, 0), (1, 1));
        assert_eq!(
            output_size(&collage, u32::MAX),
            (4608, MAX_EXPORT_LONG_EDGE)
        );
        let bytes = render(
            &collage,
            &sources,
            ExportOptions {
                long_edge: 0,
                jpeg_quality: 0,
            },
        )
        .unwrap();
        let jpeg = image::load_from_memory(&bytes).unwrap();
        assert_eq!((jpeg.width(), jpeg.height()), (1, 1));
        let bytes = render(
            &collage,
            &sources,
            ExportOptions {
                long_edge: 16,
                jpeg_quality: 255,
            },
        )
        .unwrap();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn render_outputs_decodable_jpeg_of_expected_size() {
        let dir = temp_dir("jpeg");
        let (mut collage, sources) = setup(&dir, &[solid(40, 30, RED), solid(30, 40, BLUE)]);
        collage.set_aspect(AspectRatio::Portrait4x5);
        let bytes = render(&collage, &sources, opts(100)).unwrap();
        assert_eq!(bytes.get(..2), Some(&[0xFF, 0xD8][..]));
        let jpeg = image::load_from_memory(&bytes).unwrap();
        assert_eq!((jpeg.width(), jpeg.height()), (80, 100));
    }

    #[test]
    fn default_options_use_documented_values() {
        let o = ExportOptions::default();
        assert_eq!((o.long_edge, o.jpeg_quality), (3000, 90));
    }

    #[test]
    fn max_images_collage_renders() {
        let dir = temp_dir("max");
        let imgs: Vec<RgbImage> = (0..crate::MAX_IMAGES as u8)
            .map(|i| solid(16, 12, [i * 20, 100, 50]))
            .collect();
        let (collage, sources) = setup(&dir, &imgs);
        let canvas = compose(&collage, &sources, opts(90)).unwrap();
        assert_eq!(canvas.width(), 90);
    }

    #[test]
    fn extreme_aspect_ratio_image_renders() {
        let dir = temp_dir("extreme");
        let (collage, sources) = setup(&dir, &[solid(64, 1, RED), solid(1, 64, BLUE)]);
        assert!(compose(&collage, &sources, opts(64)).is_ok());
    }

    #[test]
    fn missing_source_is_an_error() {
        let dir = temp_dir("missing");
        let (collage, mut sources) = setup(&dir, &[solid(10, 10, RED), solid(10, 10, BLUE)]);
        sources.pop();
        let err = render(&collage, &sources, opts(50)).unwrap_err();
        assert_eq!(err, CoreError::MissingSource(ImageId(1)));
    }

    #[test]
    fn unreadable_file_is_an_error() {
        let dir = temp_dir("unreadable");
        let (collage, mut sources) = setup(&dir, &[solid(10, 10, RED)]);
        sources[0].path = dir.join("does-not-exist.png");
        let err = render(&collage, &sources, opts(50)).unwrap_err();
        assert!(matches!(err, CoreError::ImageDecode { .. }), "{err:?}");
    }

    #[test]
    fn exif_rotated_source_matches_oriented_meta() {
        let dir = temp_dir("exif");
        // Stored 40x20: left half red, right half blue. Orientation 6 (90 deg CW) makes it 20x40 with
        // red on top and blue at the bottom.
        let mut raw = solid(40, 20, RED);
        for y in 0..20 {
            for x in 20..40 {
                raw.put_pixel(x, y, Rgb(BLUE));
            }
        }
        let path = dir.join("rot.jpg");
        std::fs::write(&path, crate::decode::tests::jpeg_with_orientation(&raw, 6)).unwrap();
        let meta = ImageMeta::new(ImageId(0), 20, 40).unwrap();
        let mut collage = Collage::new(&[meta], &templates_for(1)[0]).unwrap();
        collage.set_style(style(0.0, 0.0, 0.0));
        let sources = [ImageSource {
            id: ImageId(0),
            path,
        }];
        let canvas = compose(&collage, &sources, opts(64)).unwrap();
        let top = px(&canvas, 32, 2);
        let bottom = px(&canvas, 32, 61);
        assert!(top[0] > 150 && top[2] < 100, "top {top:?} should be red");
        assert!(
            bottom[2] > 150 && bottom[0] < 100,
            "bottom {bottom:?} should be blue"
        );
    }

    fn save_dynamic(dir: &Path, name: &str, img: DynamicImage) -> PathBuf {
        let path = dir.join(name);
        img.save(&path).unwrap();
        path
    }

    /// One-image collage (no spacing, padding or radius) for a file of the given size.
    fn single(path: PathBuf, w: u32, h: u32) -> (Collage, Vec<ImageSource>) {
        let meta = ImageMeta::new(ImageId(0), w, h).unwrap();
        let mut collage = Collage::new(&[meta], &templates_for(1)[0]).unwrap();
        collage.set_style(style(0.0, 0.0, 0.0));
        let sources = vec![ImageSource {
            id: ImageId(0),
            path,
        }];
        (collage, sources)
    }

    #[test]
    fn sixteen_bit_png_renders() {
        let dir = temp_dir("png16");
        let img = image::ImageBuffer::<Rgb<u16>, Vec<u16>>::from_pixel(
            20,
            20,
            Rgb([220 * 257, 30 * 257, 30 * 257]),
        );
        let path = save_dynamic(&dir, "a.png", DynamicImage::ImageRgb16(img));
        let (collage, sources) = single(path, 20, 20);
        let canvas = compose(&collage, &sources, opts(32)).unwrap();
        assert_color(px(&canvas, 16, 16), RED);
    }

    #[test]
    fn grayscale_png_renders() {
        let dir = temp_dir("gray");
        let img = image::GrayImage::from_pixel(10, 10, image::Luma([100]));
        let path = save_dynamic(&dir, "g.png", DynamicImage::ImageLuma8(img));
        let (collage, sources) = single(path, 10, 10);
        let canvas = compose(&collage, &sources, opts(32)).unwrap();
        assert_color(px(&canvas, 16, 16), [100, 100, 100]);
    }

    #[test]
    fn gray_alpha_png_renders() {
        let dir = temp_dir("graya");
        let img = image::GrayAlphaImage::from_pixel(10, 10, image::LumaA([100, 255]));
        let path = save_dynamic(&dir, "ga.png", DynamicImage::ImageLumaA8(img));
        let (collage, sources) = single(path, 10, 10);
        let canvas = compose(&collage, &sources, opts(32)).unwrap();
        assert_color(px(&canvas, 16, 16), [100, 100, 100]);
    }

    /// Left half transparent red, right half opaque blue.
    fn half_transparent() -> RgbaImage {
        RgbaImage::from_fn(30, 30, |x, _| {
            if x < 15 {
                image::Rgba([220, 30, 30, 0])
            } else {
                image::Rgba([30, 30, 220, 255])
            }
        })
    }

    #[test]
    fn transparent_png_shows_background_and_does_not_bleed() {
        let dir = temp_dir("rgba");
        let path = save_dynamic(&dir, "t.png", DynamicImage::ImageRgba8(half_transparent()));
        let (collage, sources) = single(path, 30, 30);
        let canvas = compose(&collage, &sources, opts(64)).unwrap();
        assert_color(px(&canvas, 4, 32), BG_PX);
        assert_color(px(&canvas, 60, 32), BLUE);
        // The red of the transparent pixels must not tint the blend at the edge.
        for p in canvas.pixels() {
            assert!(p.0[0] < 60, "red bleeds into {:?}", p.0);
        }
    }

    #[test]
    fn translucent_background_is_mixed_over_white() {
        let bg = |a| Rgba::new(0, 0, 0, a);
        assert_eq!(background_rgb(bg(0)), Rgb([255, 255, 255]));
        assert_eq!(background_rgb(bg(255)), Rgb([0, 0, 0]));
        let half = background_rgb(bg(128)).0;
        assert!(half.iter().all(|c| (126..=128).contains(c)), "{half:?}");

        let dir = temp_dir("bg-alpha");
        let (mut collage, sources) = setup(&dir, &[solid(10, 10, RED)]);
        collage.set_style(Style {
            spacing: 0.0,
            padding: 0.2,
            corner_radius: 0.0,
            background: bg(0),
        });
        let canvas = compose(&collage, &sources, opts(50)).unwrap();
        assert_eq!(px(&canvas, 1, 1), [255, 255, 255]);
    }

    const ALL_EXIF: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];

    /// Stored image with four distinct quadrant colors (asymmetric under every orientation).
    fn quadrants() -> RgbImage {
        RgbImage::from_fn(16, 12, |x, y| {
            Rgb(match (x < 8, y < 6) {
                (true, true) => [220, 30, 30],
                (false, true) => [30, 220, 30],
                (true, false) => [30, 30, 220],
                (false, false) => [220, 220, 30],
            })
        })
    }

    fn oriented(img: &RgbImage, exif: u8) -> RgbImage {
        let mut d = DynamicImage::ImageRgb8(img.clone());
        d.apply_orientation(Orientation::from_exif(exif).unwrap());
        d.into_rgb8()
    }

    #[test]
    fn stored_region_matches_apply_orientation_for_all_orientations() {
        // Every pixel gets a unique value, so a wrong mapping shows up in the compared crop.
        let (iw, ih) = (5u32, 3u32);
        let stored = RgbImage::from_fn(iw, ih, |x, y| Rgb([x as u8, y as u8, 0]));
        for exif in ALL_EXIF {
            let o = Orientation::from_exif(exif).unwrap();
            let upright = oriented(&stored, exif);
            let (ow, oh) = upright.dimensions();
            for y0 in 0..oh {
                for x0 in 0..ow {
                    for y1 in y0 + 1..=oh {
                        for x1 in x0 + 1..=ow {
                            let (x, y, w, h) = stored_region(o, iw, ih, (x0, y0, x1, y1));
                            let sub = DynamicImage::ImageRgb8(stored.clone())
                                .crop_imm(x, y, w, h)
                                .to_rgb8();
                            let mut sub = DynamicImage::ImageRgb8(sub);
                            sub.apply_orientation(o);
                            let expected = DynamicImage::ImageRgb8(upright.clone())
                                .crop_imm(x0, y0, x1 - x0, y1 - y0)
                                .to_rgb8();
                            assert_eq!(
                                sub.into_rgb8(),
                                expected,
                                "exif {exif} rect {x0},{y0},{x1},{y1}"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn exif_tagged_export_equals_prerotated_export_for_all_orientations() {
        let dir = temp_dir("exif-all");
        let stored = quadrants();
        for exif in ALL_EXIF {
            let upright = oriented(&stored, exif);
            let (ow, oh) = upright.dimensions();
            let tagged = dir.join(format!("tagged{exif}.jpg"));
            std::fs::write(
                &tagged,
                crate::decode::tests::jpeg_with_orientation(&stored, u16::from(exif)),
            )
            .unwrap();
            let plain = dir.join(format!("plain{exif}.jpg"));
            std::fs::write(
                &plain,
                crate::decode::tests::jpeg_with_orientation(&upright, 1),
            )
            .unwrap();
            for transform in [
                CellTransform::default(),
                CellTransform::new(2.0, Vec2::new(0.0, 0.0)),
                CellTransform::new(2.0, Vec2::new(1.0, 1.0)),
                CellTransform::new(2.0, Vec2::new(1.0, 0.0)),
            ] {
                let render_one = |path: &PathBuf| {
                    let (mut collage, sources) = single(path.clone(), ow, oh);
                    collage.set_transform(CellId(0), transform).unwrap();
                    compose(&collage, &sources, opts(64)).unwrap()
                };
                let (a, b) = (render_one(&tagged), render_one(&plain));
                for (x, y) in [(8, 8), (56, 8), (8, 56), (56, 56), (32, 32)] {
                    let (pa, pb) = (px(&a, x, y), px(&b, x, y));
                    for (ca, cb) in pa.iter().zip(pb.iter()) {
                        assert!(
                            ca.abs_diff(*cb) <= 40,
                            "exif {exif} {transform:?} at {x},{y}: {pa:?} vs {pb:?}"
                        );
                    }
                }
            }
            // Sanity: the four quadrants stay distinct in the full view.
            let (collage, sources) = single(tagged.clone(), ow, oh);
            let full = compose(&collage, &sources, opts(64)).unwrap();
            let corners = [(16, 16), (48, 16), (16, 48), (48, 48)].map(|(x, y)| px(&full, x, y));
            for (i, a) in corners.iter().enumerate() {
                for b in corners.iter().skip(i + 1) {
                    assert_ne!(a, b, "exif {exif}: quadrants not distinct");
                }
            }
        }
    }
}
