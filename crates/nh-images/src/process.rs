//! Port of `resources/images/process.go`.
//!
//! Owner: Wave B task T10 (images).

/// Go: `images.ImageProcessSpecProvider`: a filter that is an image process spec (the resource
/// layer turns it into the spec's filters before drawing).
pub trait ImageProcessSpecProvider {
    fn image_process_spec(&self) -> &str;
}

/// Go: `images.processFilter` (`images.Process "resize 600x"`). Go's `Draw` and `Bounds` panic
/// with `not supported`; the port's `GiftFilter::to_gift` returns that error.
pub struct ProcessFilter {
    pub spec: String,
}

impl ImageProcessSpecProvider for ProcessFilter {
    // Go: resources/images/process.go:ImageProcessSpec
    fn image_process_spec(&self) -> &str {
        &self.spec
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/images/process.go (43 lines; 0/3 funcs executed)
//   types: ImageProcessSpecProvider, processFilter
// OK L33-35: (f processFilter) Draw(dst draw.Image, src image.Image, options *gift.Options)
// OK L37-39: (f processFilter) Bounds(srcBounds image.Rectangle) image.Rectangle
// OK L41-43: (f processFilter) ImageProcessSpec() string
// ---------------------------------------------------------------------------
