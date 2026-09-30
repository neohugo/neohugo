//! Image processing: [`ImageSpec`], operations and [`ImageFilter`]s, codecs, the deferred
//! [`ImageQueue`] and its cache (`[caches.images]`), and EXIF metadata.
//!
//! A template asks for an operation (`resize "600x400 webp"`, `images.Filter`); the resource
//! layer calls [`ImageQueue::enqueue`], which plans it from the source's metadata and returns
//! the final file name and size at once. Build phase E6 calls [`ImageQueue::process`] for the
//! operations whose URLs were published; it decodes, runs the planned steps with rayon and
//! encodes (JPEG, PNG, GIF, TIFF and BMP with `image`, WebP with libwebp). See the crate
//! README for the accepted differences from Hugo.

#![forbid(unsafe_code)]

mod codec;
mod color;
mod error;
pub mod exif;
mod filter;
mod format;
mod pixels;
mod plan;
mod queue;
mod settings;
mod spec;

pub use color::Color;
pub use error::ImageError;
pub use exif::Exif;
pub use filter::{ImageFilter, ImageInput, MAX_PADDING, PaddingSpec};
pub use format::ImageFormat;
pub use plan::{Size, cover_size, crop_rect, fit_size, resize_size, rotated_size, smart_region};
pub use queue::{Enqueued, ImageCache, ImageQueue};
pub use settings::{DEFAULT_EXIF_EXCLUDE, ExifPart, ExifSettings, Imaging};
pub use spec::{Action, Anchor, Hint, ImageSpec, Resample, ResolvedSpec};

/// The size and format of an encoded image, from its header (`.Width` and `.Height` of an
/// image that is not processed).
///
/// # Errors
/// Bytes that are not an image in a supported format.
pub fn probe(bytes: &[u8], what: &str) -> Result<(Size, ImageFormat), ImageError> {
    codec::probe(bytes, what)
}

/// [`probe`] of a file; reads its header only.
///
/// # Errors
/// An unreadable file, or one that is not an image in a supported format.
pub fn probe_file(path: &std::path::Path) -> Result<(Size, ImageFormat), ImageError> {
    codec::probe_file(path)
}
