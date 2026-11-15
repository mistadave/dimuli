//! Platform-independent collage logic for dimuli.
//!
//! All lengths are in **collage units**: `1.0` equals the width of the collage. The height follows from the
//! [`AspectRatio`]. Conversion to pixels happens only in the UI and in the export renderer.

#![warn(missing_docs)]

pub mod collage;
pub mod crop;
pub mod decode;
pub mod error;
pub mod geometry;
pub mod layout;
pub mod render;
pub mod templates;

pub use collage::{AspectRatio, Cell, CellPlacement, Collage, ImageId, ImageMeta, Rgba, Style};
pub use crop::{CellTransform, MAX_ZOOM, source_crop};
pub use decode::{DECODE_MAX_PIXELS, decode_image, decode_unoriented};
pub use error::CoreError;
pub use geometry::{Rect, Size, Vec2};
pub use layout::{CellId, Divider, DividerId, LayoutNode, MIN_CELL_SIZE, SplitDir};
pub use render::{EXPORT_LONG_EDGE, ExportOptions, ImageSource, render};
pub use templates::{Template, templates_for};

/// Maximum number of images in one collage, a 6x6 grid.
///
/// Above it the 48 px divider handles cover too much of the cells on a phone for pan and swap
/// (see `docs/ARCHITECTURE.md`). The core keeps rejecting more ([`CoreError::TooManyImages`], empty
/// `templates_for`); this constant is the single source of the cap.
pub const MAX_IMAGES: usize = 36;
