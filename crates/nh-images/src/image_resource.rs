//! Port of `resources/images/image_resource.go`.
//!
//! Owner: Wave B task T10 (images).


/// Go: `images.ImageResourceOps` method names (implemented by nh-resources `ImageResource`):
/// `Height`, `Width`, `Process`, `Crop`, `Fill`, `Fit`, `Resize`, `Filter`, `Exif`, `Colors`.
pub const IMAGE_RESOURCE_OPS: &[&str] = &["Height", "Width", "Process", "Crop", "Fill", "Fit", "Resize", "Filter", "Exif", "Colors"];

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/images/image_resource.go (70 lines; 0/0 funcs executed)
//   types: ImageResource, ImageResourceOps
// ---------------------------------------------------------------------------
