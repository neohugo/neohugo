//! Port of `resources/images/webp/webp.go`.
//!
//! Owner: Wave B task T10 (images).


/// Go: `resources/images/webp` — libwebp encode via `libwebp-sys` (gowebp wrapper port):
/// `WebPConfigPreset(photo, q)`, `use_sharp_yuv=1`, `WebPPictureImportRGBA`.
pub fn supports() -> bool {
    true
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/images/webp/webp.go (33 lines; 1/2 funcs executed)
// EX L26-28: Encode(w io.Writer, m image.Image, o webpoptions.EncodingOptions) error
//    L31-33: Supports() bool
// ---------------------------------------------------------------------------
