//! Image loading: decoding, EXIF orientation and preview generation.
//!
//! Everything here runs on a background thread. `slint::Image` is not `Send`, so previews travel as
//! [`SharedPixelBuffer`]s and become images in the UI thread (see [`ImageStore::insert`]).

use std::collections::HashMap;
use std::fmt;
use std::io;
use std::panic::AssertUnwindSafe;
use std::path::{Path, PathBuf};

use collage_core::{ImageId, ImageMeta};
use image::metadata::Orientation;
use image::{DynamicImage, ImageDecoder, ImageReader, Limits};
use slint::{Image, Rgba8Pixel, SharedPixelBuffer};

/// Longest edge of a preview in pixels; limits memory used by the UI.
pub(crate) const PREVIEW_MAX_EDGE: u32 = 2048;
/// Images with more pixels are rejected before decoding (protects against manipulated files).
pub(crate) const DECODE_MAX_PIXELS: u64 = 100_000_000;
/// Longest accepted side in pixels (a strict decoder limit; also catches 1-pixel-wide giants).
const DECODE_MAX_EDGE: u32 = 16_384;
/// Memory a decoder may allocate at once.
const DECODE_MAX_ALLOC: u64 = 512 * 1024 * 1024;

/// Why an image could not be loaded.
#[derive(Debug)]
pub(crate) enum LoadError {
    /// The file could not be opened or read.
    Io(io::Error),
    /// The file is not a supported or intact image.
    Decode(image::ImageError),
    /// The image has more than [`DECODE_MAX_PIXELS`] pixels.
    TooLarge { width: u32, height: u32 },
    /// The image has a zero side.
    Empty,
    /// The decoder panicked (e.g. on a manipulated file); the image is skipped.
    Panicked,
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::Io(e) => write!(f, "cannot read file: {e}"),
            LoadError::Decode(e) => write!(f, "cannot decode image: {e}"),
            LoadError::TooLarge { width, height } => {
                write!(f, "image too large: {width}x{height}")
            }
            LoadError::Empty => write!(f, "image has no pixels"),
            LoadError::Panicked => write!(f, "decoder panicked"),
        }
    }
}

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

/// Decodes `path` and creates the oriented preview.
pub(crate) fn load_image(path: &Path) -> Result<LoadedImage, LoadError> {
    let mut reader = ImageReader::open(path)
        .map_err(LoadError::Io)?
        .with_guessed_format()
        .map_err(LoadError::Io)?;
    let mut limits = Limits::default();
    limits.max_image_width = Some(DECODE_MAX_EDGE);
    limits.max_image_height = Some(DECODE_MAX_EDGE);
    limits.max_alloc = Some(DECODE_MAX_ALLOC);
    reader.limits(limits);
    let mut decoder = reader.into_decoder().map_err(LoadError::Decode)?;

    let (width, height) = decoder.dimensions();
    if width == 0 || height == 0 {
        return Err(LoadError::Empty);
    }
    if u64::from(width) * u64::from(height) > DECODE_MAX_PIXELS {
        return Err(LoadError::TooLarge { width, height });
    }
    // A broken EXIF block must not make the image unusable.
    let orientation = decoder.orientation().unwrap_or(Orientation::NoTransforms);
    let decoded = DynamicImage::from_decoder(decoder).map_err(LoadError::Decode)?;

    // Orient the small preview instead of the full image to save time and memory.
    let mut preview = downscale(decoded);
    preview.apply_orientation(orientation);
    let rgba = preview.into_rgba8();
    let (preview_width, preview_height) = rgba.dimensions();
    if preview_width == 0 || preview_height == 0 {
        return Err(LoadError::Empty);
    }
    let (width, height) = if swaps_axes(orientation) {
        (height, width)
    } else {
        (width, height)
    };
    Ok(LoadedImage {
        path: path.to_path_buf(),
        width,
        height,
        preview: SharedPixelBuffer::clone_from_slice(rgba.as_raw(), preview_width, preview_height),
    })
}

/// Shrinks `image` to at most [`PREVIEW_MAX_EDGE`] on its longest edge, keeping the aspect ratio.
/// Takes ownership so the full-size pixels are freed as soon as the preview exists.
fn downscale(image: DynamicImage) -> DynamicImage {
    if image.width().max(image.height()) <= PREVIEW_MAX_EDGE {
        return image;
    }
    image.thumbnail(PREVIEW_MAX_EDGE, PREVIEW_MAX_EDGE)
}

/// Loads `paths` one after another on a new thread (one at a time keeps peak memory low) and calls `done`
/// with one result per path, in order. Failures are logged here; `done` runs on the loader thread.
///
/// Fails only if the thread cannot be spawned; `done` is not called in that case.
pub(crate) fn load_in_background<F>(paths: Vec<PathBuf>, done: F) -> io::Result<()>
where
    F: FnOnce(Vec<Result<LoadedImage, LoadError>>) + Send + 'static,
{
    load_in_background_with(paths, load_image, done)
}

/// Like [`load_in_background`] with an injectable `loader`. A panic inside `loader` is caught per image,
/// so `done` always runs and the UI never stays busy forever.
fn load_in_background_with<L, F>(paths: Vec<PathBuf>, loader: L, done: F) -> io::Result<()>
where
    L: Fn(&Path) -> Result<LoadedImage, LoadError> + Send + 'static,
    F: FnOnce(Vec<Result<LoadedImage, LoadError>>) + Send + 'static,
{
    std::thread::Builder::new()
        .name("image-loader".into())
        .spawn(move || {
            let results = paths
                .iter()
                .map(|path| {
                    std::panic::catch_unwind(AssertUnwindSafe(|| loader(path)))
                        .unwrap_or(Err(LoadError::Panicked))
                        .inspect_err(|e| log::error!("skipping {}: {e}", path.display()))
                })
                .collect();
            done(results);
        })
        .map(|_| ())
}

/// An image known to the app.
pub(crate) struct StoredImage {
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
        self.images.insert(
            id,
            StoredImage {
                preview: Image::from_rgba8(loaded.preview),
                preview_size,
            },
        );
        Some(meta)
    }

    pub(crate) fn get(&self, id: ImageId) -> Option<&StoredImage> {
        self.images.get(&id)
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
        let dir = temp_dir("big");
        let path = dir.join("wide.png");
        gradient(PREVIEW_MAX_EDGE * 2 + 8, 100).save(&path).unwrap();
        let loaded = load_image(&path).unwrap();
        assert_eq!(loaded.width, PREVIEW_MAX_EDGE * 2 + 8);
        assert_eq!(loaded.preview.width(), PREVIEW_MAX_EDGE);
        let expected = 100.0 * PREVIEW_MAX_EDGE as f32 / loaded.width as f32;
        assert!((loaded.preview.height() as f32 - expected).abs() <= 1.0);
        assert!(loaded.preview.height() >= 1);
    }

    #[test]
    fn extreme_aspect_ratio_keeps_one_pixel_side() {
        let dir = temp_dir("strip");
        let path = dir.join("strip.png");
        gradient(PREVIEW_MAX_EDGE * 3, 1).save(&path).unwrap();
        let loaded = load_image(&path).unwrap();
        assert_eq!(loaded.preview.width(), PREVIEW_MAX_EDGE);
        assert_eq!(loaded.preview.height(), 1);
    }

    #[test]
    fn corrupt_file_is_an_error_not_a_panic() {
        let dir = temp_dir("corrupt");
        let path = dir.join("broken.png");
        std::fs::write(&path, b"\x89PNG\r\n\x1a\nthis is not a png").unwrap();
        assert!(matches!(load_image(&path), Err(LoadError::Decode(_))));
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
        assert!(matches!(load_image(&path), Err(LoadError::Io(_))));
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
        assert!(matches!(load_image(&path), Err(LoadError::Decode(_))));
        // Within the edge limit but above the pixel budget.
        std::fs::write(&path, png_header_only(10_100, 10_000)).unwrap();
        let r = load_image(&path);
        assert!(matches!(r, Err(LoadError::TooLarge { .. })), "{r:?}");
    }

    #[test]
    fn panicking_loader_is_skipped_and_done_still_runs() {
        let (tx, rx) = std::sync::mpsc::channel();
        let loader = |path: &Path| -> Result<LoadedImage, LoadError> {
            if path.to_str() == Some("boom") {
                std::panic::panic_any("decoder bug");
            }
            Err(LoadError::Empty)
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
        assert!(matches!(results[0], Err(LoadError::Empty)));
        assert!(matches!(results[1], Err(LoadError::Panicked)));
        assert!(matches!(results[2], Err(LoadError::Empty)));
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
