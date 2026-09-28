//! Port of `resources/images/webp/webp.go`.
//!
//! Owner: Wave B task T10 (images).

//! Go `resources/images/webp` — libwebp encode via `libwebp-sys` (gowebp wrapper port):
//! `WebPConfigPreset(hint, q)`, `use_sharp_yuv=1`, `WebPPictureImportRGBA`.

use std::io::Write;

use go_image::{Gray, NRGBA, Point, RGBA, draw};
use libwebp_sys::{EncodingOptions, PixView};
use nh_common::{Error, Result};

use crate::image::GoImage;

fn view(pix: &[u8], stride: i64, r: go_image::Rectangle) -> PixView<'_> {
    PixView {
        pix,
        stride,
        rect: libwebp_sys::Rectangle {
            min: libwebp_sys::Point {
                x: r.min.x,
                y: r.min.y,
            },
            max: libwebp_sys::Point {
                x: r.max.x,
                y: r.max.y,
            },
        },
    }
}

/// Encode writes the Image m to w in Webp format with the given options.
///
/// gowebp encodes `*image.RGBA`, `*image.NRGBA` and `*image.Gray` directly and converts every
/// other image with `ConvertToNRGBA` (`draw.Draw(NewNRGBA(b), b, src, b.Min, draw.Src)`), which
/// the libwebp-sys port leaves to the caller (its deviation 1).
// Go: resources/images/webp/webp.go:Encode
pub fn encode(w: &mut dyn Write, m: &GoImage, o: EncodingOptions) -> Result<()> {
    let img = m.image();
    let res = if let Some(v) = img.as_any().downcast_ref::<RGBA>() {
        libwebp_sys::encode(
            w,
            &libwebp_sys::Image::Rgba(view(&v.pix, v.stride, v.rect)),
            o,
        )
    } else if let Some(v) = img.as_any().downcast_ref::<NRGBA>() {
        libwebp_sys::encode(
            w,
            &libwebp_sys::Image::Nrgba(view(&v.pix, v.stride, v.rect)),
            o,
        )
    } else if let Some(v) = img.as_any().downcast_ref::<Gray>() {
        libwebp_sys::encode(
            w,
            &libwebp_sys::Image::Gray(view(&v.pix, v.stride, v.rect)),
            o,
        )
    } else {
        // Go: libwebp.ConvertToNRGBA.
        let b = img.bounds();
        let mut dst = NRGBA::new(b);
        let r = dst.bounds();
        draw::draw(
            &mut dst,
            r,
            img,
            Point {
                x: b.min.x,
                y: b.min.y,
            },
            draw::Op::Src,
        );
        libwebp_sys::encode(
            w,
            &libwebp_sys::Image::Nrgba(view(&dst.pix, dst.stride, dst.rect)),
            o,
        )
    };
    res.map_err(|e| Error::new(e.to_string()))
}

/// Supports returns whether webp encoding is supported in this build.
// Go: resources/images/webp/webp.go:Supports
pub fn supports() -> bool {
    true
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/images/webp/webp.go (33 lines; 1/2 funcs executed)
// OK L26-28: Encode(w io.Writer, m image.Image, o webpoptions.EncodingOptions) error
// OK L31-33: Supports() bool
// ---------------------------------------------------------------------------
