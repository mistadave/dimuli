//! Image loading: decoding, EXIF orientation and preview generation.
//!
//! Everything here runs on a background thread. `slint::Image` is not `Send`, so previews travel as
//! [`SharedPixelBuffer`]s and become images in the UI thread (see [`ImageStore::insert`]).

use std::collections::HashMap;
use std::io;
use std::panic::AssertUnwindSafe;
use std::path::{Path, PathBuf};

use collage_core::decode::decode_unoriented;
use collage_core::{Collage, CoreError, ImageId, ImageMeta, ImageSource};
use image::DynamicImage;
use image::metadata::Orientation;
use slint::{Image, Rgba8Pixel, SharedPixelBuffer};

/// Longest edge of a preview in pixels; limits memory used by the UI.
pub(crate) const PREVIEW_MAX_EDGE: u32 = 2048;

/// A decoded image, ready to be handed to the UI thread.
#[derive(Debug)]
pub(crate) struct LoadedImage {
    pub(crate) path: PathBuf,
    /// Original width in pixels after applying the EXIF orientation.
    pub(crate) width: u32,
    /// Original height in pixels after applying the EXIF orientation.
    pub(crate) height: u32,
    /// Oriented preview, at most [`PREVIEW_MAX_EDGE`] on its longest edge.
    pub(crate) preview: SharedPixelBuffer<Rgba8Pixel>,
}

/// `true` if the orientation transposes the image, i.e. swaps width and height.
fn swaps_axes(orientation: Orientation) -> bool {
    matches!(
        orientation,
        Orientation::Rotate90
            | Orientation::Rotate270
            | Orientation::Rotate90FlipH
            | Orientation::Rotate270FlipH
    )
}

/// Decodes `path` (with the core's limits) and creates the oriented preview.
pub(crate) fn load_image(path: &Path) -> Result<LoadedImage, CoreError> {
    let (decoded, orientation) = decode_unoriented(path)?;
    let (width, height) = (decoded.width(), decoded.height());

    // Orient the small preview instead of the full image to save time and memory.
    let mut preview = downscale(decoded, PREVIEW_MAX_EDGE);
    preview.apply_orientation(orientation);
    let buffer = preview_buffer(&preview)?;
    let (width, height) = if swaps_axes(orientation) {
        (height, width)
    } else {
        (width, height)
    };
    Ok(LoadedImage {
        path: path.to_path_buf(),
        width,
        height,
        preview: buffer,
    })
}

/// Copies `image` once into a Slint pixel buffer. RGB and RGBA sources are written directly; other
/// formats take a detour through `to_rgba8` (rare, so the extra copy is acceptable).
fn preview_buffer(image: &DynamicImage) -> Result<SharedPixelBuffer<Rgba8Pixel>, CoreError> {
    let (width, height) = (image.width(), image.height());
    let invalid = CoreError::InvalidImageSize { width, height };
    if width == 0 || height == 0 {
        return Err(invalid);
    }
    let mut buffer = SharedPixelBuffer::<Rgba8Pixel>::new(width, height);
    let target = buffer.make_mut_slice();
    match image {
        DynamicImage::ImageRgb8(rgb) => {
            let raw = rgb.as_raw();
            if raw.len() / 3 != target.len() {
                return Err(invalid);
            }
            for (&[r, g, b], dst) in raw.as_chunks::<3>().0.iter().zip(target.iter_mut()) {
                *dst = Rgba8Pixel { r, g, b, a: 255 };
            }
        }
        DynamicImage::ImageRgba8(rgba) => fill_rgba(rgba.as_raw(), target, invalid)?,
        other => fill_rgba(other.to_rgba8().as_raw(), target, invalid)?,
    }
    Ok(buffer)
}

/// Writes tightly packed RGBA bytes into `target`; fails if the lengths do not match.
fn fill_rgba(
    raw: &[u8],
    target: &mut [Rgba8Pixel],
    on_mismatch: CoreError,
) -> Result<(), CoreError> {
    if raw.len() / 4 != target.len() {
        return Err(on_mismatch);
    }
    for (&[r, g, b, a], dst) in raw.as_chunks::<4>().0.iter().zip(target.iter_mut()) {
        *dst = Rgba8Pixel { r, g, b, a };
    }
    Ok(())
}

/// Shrinks `image` to at most `max_edge` on its longest edge, keeping the aspect ratio.
/// Takes ownership so the full-size pixels are freed as soon as the preview exists.
fn downscale(image: DynamicImage, max_edge: u32) -> DynamicImage {
    if image.width().max(image.height()) <= max_edge {
        return image;
    }
    image.thumbnail(max_edge, max_edge)
}

/// Loads `paths` one after another on a new thread (one at a time keeps peak memory low) and calls `done`
/// with one result per path, in order. Failures are logged here; `done` runs on the loader thread.
///
/// Fails only if the thread cannot be spawned; `done` is not called in that case.
pub(crate) fn load_in_background<F>(paths: Vec<PathBuf>, done: F) -> io::Result<()>
where
    F: FnOnce(Vec<Result<LoadedImage, CoreError>>) + Send + 'static,
{
    load_in_background_with(paths, load_image, done)
}

/// Like [`load_in_background`] with an injectable `loader`. A panic inside `loader` is caught per image,
/// so `done` always runs and the UI never stays busy forever.
fn load_in_background_with<L, F>(paths: Vec<PathBuf>, loader: L, done: F) -> io::Result<()>
where
    L: Fn(&Path) -> Result<LoadedImage, CoreError> + Send + 'static,
    F: FnOnce(Vec<Result<LoadedImage, CoreError>>) + Send + 'static,
{
    std::thread::Builder::new()
        .name("image-loader".into())
        .spawn(move || {
            let results = paths
                .iter()
                .map(|path| {
                    std::panic::catch_unwind(AssertUnwindSafe(|| loader(path)))
                        .unwrap_or_else(|_| {
                            Err(CoreError::ImageDecode {
                                path: path.clone(),
                                reason: "decoder panicked".to_owned(),
                            })
                        })
                        .inspect_err(|e| log::error!("skipping {}: {e}", path.display()))
                })
                .collect();
            done(results);
        })
        .map(|_| ())
}

/// An image known to the app.
pub(crate) struct StoredImage {
    /// The original file, used by the export.
    pub(crate) path: PathBuf,
    pub(crate) preview: Image,
    /// Size of `preview` in pixels (differs from `meta` when the original was downscaled).
    pub(crate) preview_size: (u32, u32),
}

/// All images of the current session, by id.
#[derive(Default)]
pub(crate) struct ImageStore {
    images: HashMap<ImageId, StoredImage>,
    next_id: u32,
}

impl ImageStore {
    /// Adds a loaded image and returns its metadata for the core.
    /// `None` if the image has an invalid size.
    pub(crate) fn insert(&mut self, loaded: LoadedImage) -> Option<ImageMeta> {
        let id = ImageId(self.next_id);
        let meta = ImageMeta::new(id, loaded.width, loaded.height).ok()?;
        self.next_id = self.next_id.checked_add(1)?;
        let preview_size = (loaded.preview.width(), loaded.preview.height());
        log::debug!("loaded {} as {id:?}", loaded.path.display());
        let path = loaded.path;
        self.images.insert(
            id,
            StoredImage {
                path,
                preview: Image::from_rgba8(loaded.preview),
                preview_size,
            },
        );
        Some(meta)
    }

    pub(crate) fn get(&self, id: ImageId) -> Option<&StoredImage> {
        self.images.get(&id)
    }

    /// The export sources for the images used by `collage`. Images without a stored file are left out;
    /// the renderer reports them as missing.
    pub(crate) fn sources(&self, collage: &Collage) -> Vec<ImageSource> {
        collage
            .cells()
            .iter()
            .filter_map(|cell| {
                let id = cell.image.id();
                self.images.get(&id).map(|stored| ImageSource {
                    id,
                    path: stored.path.clone(),
                })
            })
            .collect()
    }

    /// Number of stored images.
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.images.len()
    }

    /// Forgets all images (back to the start screen).
    pub(crate) fn clear(&mut self) {
        self.images.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgb, RgbImage};

    /// A temporary directory that is removed on drop, also when the test fails.
    struct TempDir(PathBuf);

    impl TempDir {
        fn join(&self, name: &str) -> PathBuf {
            self.0.join(name)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn temp_dir(name: &str) -> TempDir {
        let dir = std::env::temp_dir().join(format!("dimuli-test-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        TempDir(dir)
    }

    fn gradient(width: u32, height: u32) -> RgbImage {
        ImageBuffer::from_fn(width, height, |x, y| {
            Rgb([(x % 256) as u8, (y % 256) as u8, 7])
        })
    }

    #[test]
    fn loads_png_with_exact_size() {
        let dir = temp_dir("png");
        let path = dir.join("a.png");
        gradient(40, 20).save(&path).unwrap();
        let loaded = load_image(&path).unwrap();
        assert_eq!((loaded.width, loaded.height), (40, 20));
        assert_eq!((loaded.preview.width(), loaded.preview.height()), (40, 20));
        assert_eq!(loaded.path, path);
    }

    #[test]
    fn loads_png_with_source_colours() {
        let dir = temp_dir("colours");
        let path = dir.join("c.png");
        gradient(40, 20).save(&path).unwrap();
        let loaded = load_image(&path).unwrap();
        // Pixel (5, 3) of the gradient is (5, 3, 7); buffers are row-major.
        let pixel = loaded.preview.as_slice().get(3 * 40 + 5).unwrap();
        assert_eq!((pixel.r, pixel.g, pixel.b, pixel.a), (5, 3, 7, 255));
    }

    #[test]
    fn preview_buffer_adds_opaque_alpha_to_rgb() {
        let img = DynamicImage::ImageRgb8(
            RgbImage::from_raw(2, 1, vec![10, 20, 30, 40, 50, 60]).unwrap(),
        );
        let buf = preview_buffer(&img).unwrap();
        assert_eq!((buf.width(), buf.height()), (2, 1));
        let px: Vec<_> = buf
            .as_slice()
            .iter()
            .map(|p| (p.r, p.g, p.b, p.a))
            .collect();
        assert_eq!(px, vec![(10, 20, 30, 255), (40, 50, 60, 255)]);
    }

    #[test]
    fn preview_buffer_keeps_rgba_alpha() {
        let img =
            DynamicImage::ImageRgba8(image::RgbaImage::from_raw(1, 1, vec![1, 2, 3, 4]).unwrap());
        let buf = preview_buffer(&img).unwrap();
        let p = buf.as_slice().first().unwrap();
        assert_eq!((p.r, p.g, p.b, p.a), (1, 2, 3, 4));
    }

    #[test]
    fn preview_buffer_converts_other_formats() {
        let img = DynamicImage::ImageLuma8(image::GrayImage::from_raw(1, 1, vec![77]).unwrap());
        let buf = preview_buffer(&img).unwrap();
        let p = buf.as_slice().first().unwrap();
        assert_eq!((p.r, p.g, p.b, p.a), (77, 77, 77, 255));
    }

    #[test]
    fn preview_buffer_rejects_empty_images() {
        let img = DynamicImage::ImageRgb8(RgbImage::new(0, 0));
        assert_eq!(
            preview_buffer(&img).unwrap_err(),
            CoreError::InvalidImageSize {
                width: 0,
                height: 0
            }
        );
    }

    #[test]
    fn loads_jpeg() {
        let dir = temp_dir("jpeg");
        let path = dir.join("a.jpg");
        gradient(33, 17).save(&path).unwrap();
        let loaded = load_image(&path).unwrap();
        assert_eq!((loaded.width, loaded.height), (33, 17));
    }

    #[test]
    fn loads_image_with_wrong_extension_by_content() {
        let dir = temp_dir("ext");
        let path = dir.join("a.dat");
        gradient(8, 8)
            .save_with_format(&path, image::ImageFormat::Png)
            .unwrap();
        assert!(load_image(&path).is_ok());
    }

    #[test]
    fn preview_is_limited_to_max_edge_and_keeps_aspect() {
        let wide = DynamicImage::ImageRgb8(gradient(40, 10));
        let small = downscale(wide, 16);
        assert_eq!(small.width(), 16);
        let expected = 10.0 * 16.0 / 40.0;
        assert!((small.height() as f32 - expected).abs() <= 1.0);
        // Images within the limit are returned untouched.
        let fits = downscale(DynamicImage::ImageRgb8(gradient(16, 8)), 16);
        assert_eq!((fits.width(), fits.height()), (16, 8));
    }

    #[test]
    fn extreme_aspect_ratio_keeps_one_pixel_side() {
        let strip = DynamicImage::ImageRgb8(gradient(48, 1));
        let small = downscale(strip, 16);
        assert_eq!((small.width(), small.height()), (16, 1));
        let tall = downscale(DynamicImage::ImageRgb8(gradient(1, 48)), 16);
        assert_eq!((tall.width(), tall.height()), (1, 16));
    }

    #[test]
    fn corrupt_file_is_an_error_not_a_panic() {
        let dir = temp_dir("corrupt");
        let path = dir.join("broken.png");
        std::fs::write(&path, b"\x89PNG\r\n\x1a\nthis is not a png").unwrap();
        assert!(matches!(
            load_image(&path),
            Err(CoreError::ImageDecode { .. })
        ));
    }

    #[test]
    fn truncated_png_is_an_error() {
        let dir = temp_dir("truncated");
        let path = dir.join("a.png");
        gradient(64, 64).save(&path).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        std::fs::write(&path, &bytes[..bytes.len() / 2]).unwrap();
        assert!(load_image(&path).is_err());
    }

    #[test]
    fn missing_file_is_an_io_error() {
        let path = std::env::temp_dir().join("dimuli-test-does-not-exist.png");
        assert!(matches!(
            load_image(&path),
            Err(CoreError::ImageDecode { .. })
        ));
    }

    #[test]
    fn non_image_file_is_an_error() {
        let dir = temp_dir("text");
        let path = dir.join("a.png");
        std::fs::write(&path, b"hello").unwrap();
        assert!(load_image(&path).is_err());
    }

    /// Inserts an EXIF block with the given orientation right after the JPEG start marker.
    fn jpeg_with_orientation(image: &RgbImage, orientation: u8) -> Vec<u8> {
        let mut plain = Vec::new();
        image
            .write_to(&mut io::Cursor::new(&mut plain), image::ImageFormat::Jpeg)
            .unwrap();
        let mut exif = b"Exif\0\0".to_vec();
        exif.extend_from_slice(&[b'I', b'I', 0x2A, 0, 8, 0, 0, 0]); // TIFF header, IFD at offset 8
        exif.extend_from_slice(&[1, 0]); // one entry
        exif.extend_from_slice(&[0x12, 0x01, 3, 0, 1, 0, 0, 0, orientation, 0, 0, 0]);
        exif.extend_from_slice(&[0, 0, 0, 0]); // no next IFD
        let length = (exif.len() + 2) as u16;
        let mut out = plain.get(..2).unwrap().to_vec();
        out.extend_from_slice(&[0xFF, 0xE1]);
        out.extend_from_slice(&length.to_be_bytes());
        out.extend_from_slice(&exif);
        out.extend_from_slice(plain.get(2..).unwrap());
        out
    }

    #[test]
    fn exif_rotation_swaps_reported_size_and_preview() {
        let dir = temp_dir("exif");
        let path = dir.join("rotated.jpg");
        std::fs::write(&path, jpeg_with_orientation(&gradient(40, 20), 6)).unwrap();
        let loaded = load_image(&path).unwrap();
        assert_eq!((loaded.width, loaded.height), (20, 40));
        assert_eq!((loaded.preview.width(), loaded.preview.height()), (20, 40));
    }

    #[test]
    fn exif_orientation_one_keeps_size() {
        let dir = temp_dir("exif1");
        let path = dir.join("upright.jpg");
        std::fs::write(&path, jpeg_with_orientation(&gradient(40, 20), 1)).unwrap();
        let loaded = load_image(&path).unwrap();
        assert_eq!((loaded.width, loaded.height), (40, 20));
    }

    fn png_chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let mut body = kind.to_vec();
        body.extend_from_slice(data);
        let mut chunk = (data.len() as u32).to_be_bytes().to_vec();
        chunk.extend_from_slice(&body);
        chunk.extend_from_slice(&crc32(&body).to_be_bytes());
        chunk
    }

    /// A PNG consisting of signature, an IHDR announcing the given size and a stub IDAT; no pixel data.
    /// The decoder needs the IDAT header to finish reading the image info.
    fn png_header_only(width: u32, height: u32) -> Vec<u8> {
        let mut ihdr = Vec::new();
        ihdr.extend_from_slice(&width.to_be_bytes());
        ihdr.extend_from_slice(&height.to_be_bytes());
        ihdr.extend_from_slice(&[8, 2, 0, 0, 0]); // 8 bit RGB, no interlace
        let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
        out.extend_from_slice(&png_chunk(b"IHDR", &ihdr));
        out.extend_from_slice(&png_chunk(b"IDAT", &[0]));
        out
    }

    fn crc32(data: &[u8]) -> u32 {
        let mut crc = 0xFFFF_FFFFu32;
        for &byte in data {
            crc ^= u32::from(byte);
            for _ in 0..8 {
                crc = if crc & 1 != 0 {
                    (crc >> 1) ^ 0xEDB8_8320
                } else {
                    crc >> 1
                };
            }
        }
        !crc
    }

    #[test]
    fn oversized_image_is_rejected_before_decoding() {
        let dir = temp_dir("huge");
        let path = dir.join("huge.png");
        std::fs::write(&path, png_header_only(20_000, 20_000)).unwrap();
        let r = load_image(&path);
        assert!(matches!(r, Err(CoreError::ImageTooLarge { .. })), "{r:?}");
        // Within the edge limit but above the pixel budget.
        std::fs::write(&path, png_header_only(10_100, 10_000)).unwrap();
        let r = load_image(&path);
        assert!(matches!(r, Err(CoreError::ImageTooLarge { .. })), "{r:?}");
    }

    #[test]
    fn panicking_loader_is_skipped_and_done_still_runs() {
        let (tx, rx) = std::sync::mpsc::channel();
        let loader = |path: &Path| -> Result<LoadedImage, CoreError> {
            if path.to_str() == Some("boom") {
                std::panic::panic_any("decoder bug");
            }
            Err(CoreError::InvalidImageSize {
                width: 0,
                height: 0,
            })
        };
        load_in_background_with(
            vec!["ok".into(), "boom".into(), "ok".into()],
            loader,
            move |results| {
                let _ = tx.send(results);
            },
        )
        .unwrap();
        let results = rx.recv_timeout(std::time::Duration::from_secs(10)).unwrap();
        assert_eq!(results.len(), 3);
        assert!(matches!(
            results[0],
            Err(CoreError::InvalidImageSize {
                width: 0,
                height: 0
            })
        ));
        assert!(matches!(
            &results[1],
            Err(CoreError::ImageDecode { reason, .. }) if reason.contains("panicked")
        ));
        assert!(matches!(
            results[2],
            Err(CoreError::InvalidImageSize {
                width: 0,
                height: 0
            })
        ));
    }

    #[test]
    fn transposing_orientations_swap_axes() {
        assert!(swaps_axes(Orientation::Rotate90));
        assert!(swaps_axes(Orientation::Rotate270FlipH));
        assert!(!swaps_axes(Orientation::NoTransforms));
        assert!(!swaps_axes(Orientation::Rotate180));
        assert!(!swaps_axes(Orientation::FlipHorizontal));
    }

    #[test]
    fn background_loading_reports_results_in_order_and_isolates_failures() {
        let dir = temp_dir("bg");
        let good = dir.join("good.png");
        gradient(10, 5).save(&good).unwrap();
        let bad = dir.join("bad.png");
        std::fs::write(&bad, b"nope").unwrap();

        let (tx, rx) = std::sync::mpsc::channel();
        load_in_background(vec![bad, good], move |results| {
            let _ = tx.send(results);
        })
        .unwrap();
        let results = rx.recv_timeout(std::time::Duration::from_secs(10)).unwrap();
        assert_eq!(results.len(), 2);
        assert!(results[0].is_err());
        assert_eq!(results[1].as_ref().unwrap().width, 10);
    }

    #[test]
    fn store_builds_sources_with_original_paths_for_the_collage() {
        let dir = temp_dir("sources");
        let path = dir.join("a.png");
        gradient(12, 6).save(&path).unwrap();
        let mut store = ImageStore::default();
        let meta = store.insert(load_image(&path).unwrap()).unwrap();
        let template = collage_core::templates_for(1).remove(0);
        let collage = Collage::new(&[meta], &template).unwrap();
        let sources = store.sources(&collage);
        assert_eq!(
            sources,
            vec![ImageSource {
                id: meta.id(),
                path
            }]
        );
        // After clearing, nothing is left to export.
        store.clear();
        assert!(store.sources(&collage).is_empty());
    }

    #[test]
    fn store_assigns_increasing_ids_and_keeps_preview_size() {
        let dir = temp_dir("store");
        let path = dir.join("a.png");
        gradient(12, 6).save(&path).unwrap();
        let mut store = ImageStore::default();
        let a = store.insert(load_image(&path).unwrap()).unwrap();
        let b = store.insert(load_image(&path).unwrap()).unwrap();
        assert_ne!(a.id(), b.id());
        assert_eq!(store.len(), 2);
        assert_eq!(store.get(a.id()).unwrap().preview_size, (12, 6));
        store.clear();
        assert_eq!(store.len(), 0);
        assert!(store.get(a.id()).is_none());
    }
}
