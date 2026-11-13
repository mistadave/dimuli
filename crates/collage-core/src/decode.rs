//! Image decoding with safety limits and EXIF orientation.
//!
//! Together with [`crate::render`] this is the only place where the core touches pixel data.

use std::path::Path;

use image::metadata::Orientation;
use image::{DynamicImage, ImageDecoder, ImageReader, Limits};

use crate::error::CoreError;

/// Images with more pixels are rejected before decoding (protects against manipulated files).
pub const DECODE_MAX_PIXELS: u64 = 100_000_000;
/// Longest accepted side in pixels (a strict decoder limit; also catches 1-pixel-wide giants).
pub const DECODE_MAX_EDGE: u32 = 16_384;
/// Memory a decoder may allocate at once.
pub const DECODE_MAX_ALLOC: u64 = 512 * 1024 * 1024;

/// Decodes the image at `path` and applies its EXIF orientation, so the result is upright.
///
/// Rejects zero-sized images, images beyond the decode limits and unreadable or broken files.
/// A broken EXIF block is ignored (no transformation) instead of failing.
pub fn decode_image(path: &Path) -> Result<DynamicImage, CoreError> {
    let (mut image, orientation) = decode_unoriented(path)?;
    image.apply_orientation(orientation);
    Ok(image)
}

/// Like [`decode_image`], but returns the pixels as stored together with the EXIF orientation that still has
/// to be applied. Lets callers shrink the image first and orient the small result (saves time and memory).
///
/// All limit checks of [`decode_image`] apply.
pub fn decode_unoriented(path: &Path) -> Result<(DynamicImage, Orientation), CoreError> {
    let fail = |reason: String| CoreError::ImageDecode {
        path: path.to_path_buf(),
        reason,
    };
    let mut reader = ImageReader::open(path)
        .map_err(|e| fail(e.to_string()))?
        .with_guessed_format()
        .map_err(|e| fail(e.to_string()))?;
    let mut limits = Limits::default();
    limits.max_image_width = Some(DECODE_MAX_EDGE);
    limits.max_image_height = Some(DECODE_MAX_EDGE);
    limits.max_alloc = Some(DECODE_MAX_ALLOC);
    reader.limits(limits);
    let mut decoder = reader.into_decoder().map_err(|e| fail(e.to_string()))?;

    let (width, height) = decoder.dimensions();
    if width == 0 || height == 0 {
        return Err(CoreError::InvalidImageSize { width, height });
    }
    if u64::from(width) * u64::from(height) > DECODE_MAX_PIXELS {
        return Err(CoreError::ImageTooLarge { width, height });
    }
    // A broken EXIF block must not make the image unusable.
    let orientation = decoder.orientation().unwrap_or(Orientation::NoTransforms);
    let image = DynamicImage::from_decoder(decoder).map_err(|e| fail(e.to_string()))?;
    Ok((image, orientation))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use image::{ImageFormat, Rgb, RgbImage};

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("dimuli-core-{name}-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    /// JPEG with an EXIF APP1 segment that sets `orientation`.
    pub(crate) fn jpeg_with_orientation(img: &RgbImage, orientation: u16) -> Vec<u8> {
        let mut jpeg = Vec::new();
        img.write_to(&mut std::io::Cursor::new(&mut jpeg), ImageFormat::Jpeg)
            .unwrap();
        let mut tiff: Vec<u8> = b"II*\0".to_vec();
        tiff.extend_from_slice(&8u32.to_le_bytes());
        tiff.extend_from_slice(&1u16.to_le_bytes()); // one IFD entry
        tiff.extend_from_slice(&0x0112u16.to_le_bytes()); // Orientation
        tiff.extend_from_slice(&3u16.to_le_bytes()); // SHORT
        tiff.extend_from_slice(&1u32.to_le_bytes());
        tiff.extend_from_slice(&orientation.to_le_bytes());
        tiff.extend_from_slice(&[0, 0]);
        tiff.extend_from_slice(&0u32.to_le_bytes()); // no next IFD
        let mut payload = b"Exif\0\0".to_vec();
        payload.extend_from_slice(&tiff);
        let len = (payload.len() + 2) as u16;
        let mut out = vec![0xFF, 0xD8, 0xFF, 0xE1];
        out.extend_from_slice(&len.to_be_bytes());
        out.extend_from_slice(&payload);
        out.extend_from_slice(jpeg.get(2..).unwrap());
        out
    }

    #[test]
    fn decodes_png() {
        let dir = temp_dir("decode-png");
        let path = dir.join("a.png");
        RgbImage::from_pixel(7, 5, Rgb([1, 2, 3]))
            .save(&path)
            .unwrap();
        let img = decode_image(&path).unwrap();
        assert_eq!((img.width(), img.height()), (7, 5));
    }

    #[test]
    fn unoriented_decode_returns_stored_pixels_and_orientation() {
        let dir = temp_dir("decode-unoriented");
        let path = dir.join("rot.jpg");
        std::fs::write(
            &path,
            jpeg_with_orientation(&RgbImage::from_pixel(8, 4, Rgb([200, 10, 10])), 6),
        )
        .unwrap();
        let (img, orientation) = decode_unoriented(&path).unwrap();
        assert_eq!((img.width(), img.height()), (8, 4));
        assert_eq!(orientation, Orientation::Rotate90);
    }

    #[test]
    fn missing_file_is_an_error() {
        let err = decode_image(Path::new("/definitely/not/here.jpg")).unwrap_err();
        assert!(matches!(err, CoreError::ImageDecode { .. }), "{err:?}");
    }

    #[test]
    fn garbage_file_is_an_error() {
        let dir = temp_dir("decode-garbage");
        let path = dir.join("bad.jpg");
        std::fs::write(&path, b"not an image at all").unwrap();
        assert!(decode_image(&path).is_err());
    }

    #[test]
    fn exif_orientation_rotates_image() {
        let dir = temp_dir("decode-exif");
        let path = dir.join("rot.jpg");
        // 8x4 landscape, orientation 6 = rotate 90 CW -> 4x8 portrait.
        std::fs::write(
            &path,
            jpeg_with_orientation(&RgbImage::from_pixel(8, 4, Rgb([200, 10, 10])), 6),
        )
        .unwrap();
        let img = decode_image(&path).unwrap();
        assert_eq!((img.width(), img.height()), (4, 8));
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

    fn png_chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let mut body = kind.to_vec();
        body.extend_from_slice(data);
        let mut chunk = (data.len() as u32).to_be_bytes().to_vec();
        chunk.extend_from_slice(&body);
        chunk.extend_from_slice(&crc32(&body).to_be_bytes());
        chunk
    }

    /// A PNG with signature, an IHDR announcing the given size and a stub IDAT; no pixel data.
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

    fn decode_header_only(name: &str, width: u32, height: u32) -> Result<DynamicImage, CoreError> {
        let path = temp_dir(name).join("h.png");
        std::fs::write(&path, png_header_only(width, height)).unwrap();
        decode_image(&path)
    }

    #[test]
    fn edge_beyond_limit_is_rejected_as_decode_error() {
        // The decoder's width/height limit fires before our own checks.
        let r = decode_header_only("limit-edge", DECODE_MAX_EDGE + 1, 1);
        assert!(matches!(r, Err(CoreError::ImageDecode { .. })), "{r:?}");
        let r = decode_header_only("limit-edge-h", 1, DECODE_MAX_EDGE + 1);
        assert!(matches!(r, Err(CoreError::ImageDecode { .. })), "{r:?}");
    }

    #[test]
    fn pixel_budget_exceeded_is_image_too_large() {
        let r = decode_header_only("limit-pixels", 10_100, 10_000);
        assert_eq!(
            r.unwrap_err(),
            CoreError::ImageTooLarge {
                width: 10_100,
                height: 10_000
            }
        );
    }

    #[test]
    fn zero_sized_image_is_an_error() {
        // The PNG decoder already refuses zero sides; `InvalidImageSize` is a second line of defense.
        let r = decode_header_only("limit-zero", 0, 10);
        assert!(
            matches!(
                r,
                Err(CoreError::ImageDecode { .. } | CoreError::InvalidImageSize { .. })
            ),
            "{r:?}"
        );
    }
}
