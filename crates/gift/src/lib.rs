//! Byte-exact port of `github.com/disintegration/gift` v1.2.1 (Go Image
//! Filtering Toolkit) over the `go-image` types, plus the extra resampling
//! filters of neohugo `resources/images/resampling.go` ([`hugo_resampling`]).
//!
//! Floating-point results are bit-identical to the Go code as compiled by
//! go1.27.1 for darwin/arm64 (the golden build): every `x*y + z` the Go
//! compiler fuses into an FMA instruction is written with `mul_add`, every
//! other operation is plain IEEE arithmetic, and the Go `math` functions gift
//! calls (`Exp`, `Log`, `Pow`, `Sin`, `Cos`, `Sincos`) are ported with their
//! own fusion ([`gomath`]). See PORTING.md for the list of fused sites.
//!
//! Go → Rust API map (Go `int` is `i64`):
//!
//! * `gift.New(filters...)` → [`GIFT::new`] / [`new`]; methods `bounds`,
//!   `draw`, `draw_at`, `add`, `empty`, `set_parallelization`.
//! * `gift.Filter` → trait [`Filter`] (`draw(dst, src, Option<&Options>)`,
//!   `bounds`); filters are shared as `Arc<dyn Filter>` and can be downcast
//!   (the trait has `Any` as a supertrait) to the concrete filter structs,
//!   whose fields are public.
//! * `gift.Resampling` → trait [`Resampling`]; the built-in filters are
//!   `&'static` [`Resamp`] statics ([`BOX_RESAMPLING`], ...).
//! * Filter constructors keep Go's names in snake_case: [`resize`],
//!   [`resize_to_fit`], [`resize_to_fill`], [`crop`], [`crop_to_size`],
//!   [`rotate`], [`rotate90`], ..., [`gaussian_blur`], [`unsharp_mask`],
//!   [`sigmoid_filter`] (Go `Sigmoid`), [`color_func`], ...

// Lints that fight a faithful line-by-line port of the Go code.
#![allow(
    clippy::too_many_arguments,
    clippy::needless_range_loop,
    clippy::collapsible_else_if,
    clippy::collapsible_if,
    clippy::manual_range_contains,
    clippy::neg_multiply,
    clippy::upper_case_acronyms,
    clippy::new_ret_no_self,
    clippy::type_complexity,
    clippy::excessive_precision,
    // Go's math constants (LOG2E, SQRT2) are ported verbatim.
    clippy::approx_constant,
    clippy::needless_return,
    clippy::int_plus_one,
    clippy::manual_clamp
)]
#![forbid(unsafe_code)]

mod colors;
mod convolution;
mod effects;
mod gift;
pub mod gomath;
pub mod hugo_resampling;
mod pixels;
mod rank;
mod resize;
mod transform;
mod utils;

#[cfg(test)]
mod go_tests;

pub use colors::{
    ColorFilter, ColorchanFilter, brightness, color_balance, color_func, colorize,
    colorspace_linear_to_srgb, colorspace_srgb_to_linear, contrast, gamma, grayscale, hue, invert,
    saturation, sepia, sigmoid_filter, threshold,
};
pub use convolution::{
    ConvolutionFilter, GaussianBlurFilter, HvConvolutionFilter, MeanFilter, UnsharpMaskFilter,
    convolution, gaussian_blur, mean, sobel, unsharp_mask,
};
pub use effects::{PixelateFilter, pixelate};
pub use gift::{
    COPY_OPERATOR, DEFAULT_OPTIONS, Filter, GIFT, OVER_OPERATOR, Operator, Options, new,
};
pub use rank::{RankFilter, RankMode, maximum, median, minimum};
pub use resize::{
    BOX_RESAMPLING, CUBIC_RESAMPLING, LANCZOS_RESAMPLING, LINEAR_RESAMPLING,
    NEAREST_NEIGHBOR_RESAMPLING, Resamp, Resampling, ResizeFilter, ResizeToFillFilter,
    ResizeToFitFilter, bcspline, resize, resize_to_fill, resize_to_fit, sinc,
};
pub use transform::{
    Anchor, BOTTOM_ANCHOR, BOTTOM_LEFT_ANCHOR, BOTTOM_RIGHT_ANCHOR, CENTER_ANCHOR,
    CUBIC_INTERPOLATION, CropFilter, CropToSizeFilter, Interpolation, LEFT_ANCHOR,
    LINEAR_INTERPOLATION, NEAREST_NEIGHBOR_INTERPOLATION, RIGHT_ANCHOR, RotateFilter, TOP_ANCHOR,
    TOP_LEFT_ANCHOR, TOP_RIGHT_ANCHOR, TransformFilter, TransformType, crop, crop_to_size,
    flip_horizontal, flip_vertical, rotate, rotate90, rotate180, rotate270, transpose, transverse,
};
pub use utils::CopyimageFilter;
