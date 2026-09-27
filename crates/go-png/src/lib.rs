//! Byte-exact port of the Go 1.27.1 `image/png` package (reader.go,
//! writer.go, paeth.go) on top of [`go_image`] (image types, colour models)
//! and [`go_flate`] (compress/zlib).
//!
//! * [`decode`] / [`decode_config`] return exactly the image type, bounds,
//!   strides, pixels and palette Go's decoder returns (see the type mapping
//!   in PORTING.md), and the same error strings.
//! * [`encode`] / [`Encoder::encode`] write exactly Go's bytes: colour type
//!   selection, PLTE/tRNS, the filter heuristic, the zlib stream (go-flate)
//!   and the IDAT chunk boundaries produced by Go's `bufio.Writer` of 32 KiB.
//! * [`register`] registers the format with [`go_image::decode`].
//!
//! See `PORTING.md` for the Go file → module map and deviations.

// Lints that fight a faithful line-by-line port of the Go code.
#![allow(
    clippy::needless_range_loop,
    clippy::manual_range_contains,
    clippy::collapsible_else_if,
    clippy::collapsible_if,
    clippy::precedence,
    clippy::identity_op,
    clippy::comparison_chain,
    clippy::new_without_default,
    clippy::manual_memcpy,
    clippy::too_many_arguments,
    clippy::needless_return,
    clippy::unnecessary_cast,
    // Go type names (`PassImage::RGBA`) and Go conditions kept verbatim
    // (`len(p) < 1`, `length%3 != 0`).
    clippy::upper_case_acronyms,
    clippy::len_zero,
    clippy::manual_is_multiple_of
)]
#![forbid(unsafe_code)]

mod bufio;
mod error;
mod paeth;
mod reader;
mod writer;

pub use error::Error;
pub use reader::{PNG_HEADER, decode, decode_config, register};
pub use writer::{
    BEST_COMPRESSION, BEST_SPEED, CompressionLevel, DEFAULT_COMPRESSION, Encoder, EncoderBuffer,
    EncoderBufferPool, NO_COMPRESSION, encode,
};
