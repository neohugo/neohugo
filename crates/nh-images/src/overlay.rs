//! Port of `resources/images/overlay.go`.
//!
//! Owner: Wave B task T10 (images).


use std::sync::Arc;

use crate::image::ImageSource;

/// Go: `images.overlayFilter` — `Draw`: copy src into dst, then `DrawAt(dst, overlay, (x,y),
/// OverOperator)` with the overlay decoded from its encoded PNG (see specs/images.md §5.4 FMA).
pub struct OverlayFilter {
    pub src: Arc<dyn ImageSource>,
    pub x: i64,
    pub y: i64,
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/images/overlay.go (43 lines; 2/2 funcs executed)
//   types: overlayFilter
// EX L31-39: (f overlayFilter) Draw(dst draw.Image, src image.Image, options *gift.Options)
// EX L41-43: (f overlayFilter) Bounds(srcBounds image.Rectangle) image.Rectangle
// ---------------------------------------------------------------------------
