//! Port of `resources/images/dither.go`.
//!
//! STUB: `images.Dither` needs `github.com/makeworld-the-better-one/dither/v2` (~1,300 lines
//! of float code with its own FMA sites: linearisation with `math.Pow`, error diffusion,
//! ordered matrices), which is not ported. The constructor returns [`unsupported`].
//!
//! Owner: Wave B task T10 (images).

use nh_common::Error;

/// The error `images.Dither` returns.
pub fn unsupported() -> Error {
    Error::feature_not_available("neohugo-rs: images.Dither is not supported")
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/images/dither.go (71 lines; 0/2 funcs executed)
//   types: ditherFilter
// STUB L65-67: (f ditherFilter) Draw(dst draw.Image, src image.Image, options *gift.Options)
// STUB L69-71: (f ditherFilter) Bounds(srcBounds image.Rectangle) image.Rectangle
// ---------------------------------------------------------------------------
