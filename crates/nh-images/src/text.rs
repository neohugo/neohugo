//! Port of `resources/images/text.go`.
//!
//! STUB: drawing text needs `golang.org/x/image/font/opentype` (sfnt parsing, the vector
//! rasterizer, fixed-point metrics) and the embedded Go Regular font, none of which is ported.
//! `images.Text` returns [`unsupported`].
//!
//! Owner: Wave B task T10 (images).

use nh_common::Error;

/// The error `images.Text` returns.
pub fn unsupported() -> Error {
    Error::feature_not_available("neohugo-rs: images.Text is not supported")
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/images/text.go (150 lines; 0/2 funcs executed)
//   types: textFilter
// STUB L45-146: (f textFilter) Draw(dst draw.Image, src image.Image, options *gift.Options)
// STUB L148-150: (f textFilter) Bounds(srcBounds image.Rectangle) image.Rectangle
// ---------------------------------------------------------------------------
