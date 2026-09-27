//! Port of `resources/images/process.go`.
//!
//! Owner: Wave B task T10 (images).


/// Go: `images.processFilter` (`images.Process "resize 600x"`).
pub struct ProcessFilter {
    pub spec: String,
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/images/process.go (43 lines; 0/3 funcs executed)
//   types: ImageProcessSpecProvider, processFilter
//    L33-35: (f processFilter) Draw(dst draw.Image, src image.Image, options *gift.Options)
//    L37-39: (f processFilter) Bounds(srcBounds image.Rectangle) image.Rectangle
//    L41-43: (f processFilter) ImageProcessSpec() string
// ---------------------------------------------------------------------------
