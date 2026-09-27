//! Byte-exact port of the Go 1.27.1 standard library image packages:
//!
//! * `image` (image.go, geom.go, ycbcr.go, names.go, format.go) — at the crate root,
//! * `image/color` and `image/color/palette` — [`color`], [`color::palette`],
//! * `image/draw` — [`draw`],
//! * `image/jpeg` (reader.go, scan.go, huffman.go, dct.go, writer.go) — [`jpeg`],
//! * `image/internal/imageutil` — private `imageutil` module.
//!
//! Go interfaces are modelled as follows (see PORTING.md for the rationale):
//!
//! * `color.Color` is the [`color::Color`] enum over the standard colour types,
//! * `color.Model` is the [`color::Model`] enum,
//! * `image.Image` is the [`Image`] trait (object safe, `Any`-downcastable so that
//!   Go type switches become `downcast_ref::<T>()` chains),
//! * `image.RGBA64Image` / `image.PalettedImage` are [`RGBA64Image`] /
//!   [`PalettedImage`], reached from `&dyn Image` with `as_rgba64_image` /
//!   `as_paletted_image` (Go's interface type assertions),
//! * `draw.Image` / `draw.RGBA64Image` are [`draw::Image`] / [`draw::RGBA64Image`].
//!
//! Go `int` is `i64` throughout (coordinates, strides, offsets).

// Lints that fight a faithful line-by-line port of the Go code.
#![allow(
    clippy::too_many_arguments,
    clippy::needless_range_loop,
    clippy::identity_op,
    clippy::erasing_op,
    clippy::upper_case_acronyms,
    clippy::manual_range_contains,
    clippy::collapsible_else_if,
    clippy::collapsible_if,
    clippy::neg_multiply,
    clippy::precedence,
    clippy::comparison_chain,
    clippy::new_ret_no_self,
    clippy::should_implement_trait,
    clippy::len_zero,
    clippy::manual_memcpy,
    clippy::single_match,
    clippy::match_like_matches_macro,
    clippy::type_complexity,
    clippy::collapsible_match,
    clippy::manual_checked_ops,
    clippy::manual_clamp
)]
#![forbid(unsafe_code)]

pub mod color;
pub mod draw;
mod format;
mod geom;
mod image;
mod imageutil;
pub mod jpeg;
mod names;
mod ycbcr;

pub use format::{
    BoxError, Config, DecodeConfigFn, DecodeFn, ErrFormat, decode, decode_config, register_format,
};
pub use geom::{Point, Rectangle, ZP, ZR, pt, rect};
pub use image::{
    Alpha, Alpha16, CMYK, Gray, Gray16, Image, NRGBA, NRGBA64, Paletted, PalettedImage, RGBA,
    RGBA64, RGBA64Image,
};
pub use names::{BLACK, OPAQUE, TRANSPARENT, Uniform, WHITE};
pub use ycbcr::{NYCbCrA, YCbCr, YCbCrSubsampleRatio};

/// Go-semantics shift helpers. Go defines shifts by any unsigned amount: a
/// left shift or unsigned right shift by >= the bit width yields 0 and a
/// signed right shift by >= the width yields 0 or -1. Rust's `<<`/`>>` panic
/// (debug) or mask (release) instead.
#[inline]
pub(crate) fn go_shl_u32(x: u32, s: u32) -> u32 {
    if s >= 32 { 0 } else { x << s }
}

#[inline]
pub(crate) fn go_shr_u32(x: u32, s: u32) -> u32 {
    if s >= 32 { 0 } else { x >> s }
}

#[inline]
pub(crate) fn go_shr_i32(x: i32, s: u32) -> i32 {
    if s >= 32 { x >> 31 } else { x >> s }
}

#[inline]
pub(crate) fn go_shl_i32(x: i32, s: u32) -> i32 {
    if s >= 32 { 0 } else { x.wrapping_shl(s) }
}
