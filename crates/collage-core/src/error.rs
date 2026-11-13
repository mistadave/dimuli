//! Error type of the collage core.

use std::path::PathBuf;

use crate::MAX_IMAGES;
use crate::collage::ImageId;
use crate::layout::{CellId, DividerId};

/// Errors returned by collage operations.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum CoreError {
    /// `Collage::new` was called without images.
    #[error("a collage needs at least one image")]
    NoImages,
    /// More images than [`MAX_IMAGES`].
    #[error("too many images: {0} (maximum is {MAX_IMAGES})")]
    TooManyImages(usize),
    /// Template and image count differ.
    #[error("template has {template} cells but the collage has {images} images")]
    TemplateMismatch {
        /// Cells in the template.
        template: usize,
        /// Images in the collage.
        images: usize,
    },
    /// An image has a zero side.
    #[error("invalid image size {width}x{height}")]
    InvalidImageSize {
        /// Width in pixels.
        width: u32,
        /// Height in pixels.
        height: u32,
    },
    /// No cell with this id.
    #[error("unknown cell {0:?}")]
    UnknownCell(CellId),
    /// No divider with this id.
    #[error("unknown divider {0:?}")]
    UnknownDivider(DividerId),
    /// The export has no source file for this image.
    #[error("no source file for image {0:?}")]
    MissingSource(ImageId),
    /// An image file could not be read or decoded.
    #[error("cannot decode {}: {reason}", path.display())]
    ImageDecode {
        /// The file that failed.
        path: PathBuf,
        /// Human-readable cause (for logs).
        reason: String,
    },
    /// An image exceeds the decode limits.
    #[error("image too large: {width}x{height}")]
    ImageTooLarge {
        /// Width in pixels.
        width: u32,
        /// Height in pixels.
        height: u32,
    },
    /// The composed collage could not be encoded.
    #[error("cannot encode collage: {0}")]
    Encode(String),
}
