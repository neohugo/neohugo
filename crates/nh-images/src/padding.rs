//! Port of `resources/images/padding.go`.
//!
//! Owner: Wave B task T10 (images).

use go_image::color::Color;
use go_image::{Image, RGBA, Rectangle, Uniform, ZP, draw, pt, rect};
use nh_common::{Error, Result};

/// Go: `images.paddingFilter`.
pub struct PaddingFilter {
    pub top: i64,
    pub right: i64,
    pub bottom: i64,
    pub left: i64,
    /// canvas color
    pub ccolor: Color,
}

impl PaddingFilter {
    /// The size checks at the start of Go's `Draw`, which panics; the port checks them before
    /// drawing (from the bounds of the image the filter receives) and returns the message.
    pub(crate) fn check(&self, src_bounds: Rectangle) -> Result<()> {
        let w = src_bounds
            .dx()
            .wrapping_add(self.left)
            .wrapping_add(self.right);
        let h = src_bounds
            .dy()
            .wrapping_add(self.top)
            .wrapping_add(self.bottom);
        if w < 1 {
            return Err(Error::new(
                "final image width will be less than 1 pixel: check padding values",
            ));
        }
        if h < 1 {
            return Err(Error::new(
                "final image height will be less than 1 pixel: check padding values",
            ));
        }
        Ok(())
    }
}

impl gift::Filter for PaddingFilter {
    // Go: resources/images/padding.go:Draw
    fn draw(&self, dst: &mut dyn draw::Image, src: &dyn Image, _options: Option<&gift::Options>) {
        let w = src
            .bounds()
            .dx()
            .wrapping_add(self.left)
            .wrapping_add(self.right);
        let h = src
            .bounds()
            .dy()
            .wrapping_add(self.top)
            .wrapping_add(self.bottom);

        // (w < 1 / h < 1 panic in Go: checked by `check` before drawing.)

        let mut i = RGBA::new(rect(0, 0, w, h));
        let ib = i.bounds();
        draw::draw(&mut i, ib, &Uniform::new(self.ccolor), ZP, draw::Op::Src);
        gift::new(Vec::new()).draw(dst, &i);
        gift::new(Vec::new()).draw_at(dst, src, pt(self.left, self.top), gift::OVER_OPERATOR);
    }

    // Go: resources/images/padding.go:Bounds
    fn bounds(&self, src_bounds: Rectangle) -> Rectangle {
        rect(
            0,
            0,
            src_bounds
                .dx()
                .wrapping_add(self.left)
                .wrapping_add(self.right),
            src_bounds
                .dy()
                .wrapping_add(self.top)
                .wrapping_add(self.bottom),
        )
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/images/padding.go (50 lines; 0/2 funcs executed)
//   types: paddingFilter
// OK L31-46: (f paddingFilter) Draw(dst draw.Image, src image.Image, options *gift.Options)
// OK L48-50: (f paddingFilter) Bounds(srcBounds image.Rectangle) image.Rectangle
// ---------------------------------------------------------------------------
