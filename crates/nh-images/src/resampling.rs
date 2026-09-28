//! Port of `resources/images/resampling.go`.
//!
//! The eleven extra resampling kernels (Hermite ... Cosine) and their FMA sites are ported and
//! verified in the Wave A `gift` crate (`gift::hugo_resampling`, see its PORTING.md "FMA
//! sites"); this module re-exports them. `config.go:imageFilters` is
//! [`gift::hugo_resampling::image_filter`].
//!
//! Owner: Wave B task T10 (images).

pub use gift::hugo_resampling::{
    BARTLETT_RESAMPLING, BLACKMAN_RESAMPLING, BSPLINE_RESAMPLING, CATMULL_ROM_RESAMPLING,
    COSINE_RESAMPLING, GAUSSIAN_RESAMPLING, HAMMING_RESAMPLING, HANN_RESAMPLING,
    HERMITE_RESAMPLING, MITCHELL_NETRAVALI_RESAMPLING, WELCH_RESAMPLING, image_filter,
};

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/images/resampling.go (214 lines; 0/6 funcs executed)
//   types: resamp
// OK L177-179: (r resamp) String() string            (gift::Resamp Display)
// OK L181-183: (r resamp) Support() float32          (gift::Resamp)
// OK L185-187: (r resamp) Kernel(x float32) float32  (gift::Resamp)
// OK L189-200: bcspline(x, b, c float32) float32     (gift::bcspline, identical arm64 code)
// OK L202-207: absf32(x float32) float32             (gift)
// OK L209-214: sinc(x float32) float32               (gift::sinc)
// ---------------------------------------------------------------------------
