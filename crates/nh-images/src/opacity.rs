//! Port of `resources/images/opacity.go`.
//!
//! Owner: Wave B task T10 (images).

use go_image::color::{Alpha, Color};
use go_image::{Image, Rectangle, Uniform, ZP, draw, rect};

/// Go: `images.opacityFilter`.
pub struct OpacityFilter {
    pub opacity: f32,
}

impl gift::Filter for OpacityFilter {
    // Go: resources/images/opacity.go:Draw
    fn draw(&self, dst: &mut dyn draw::Image, src: &dyn Image, _options: Option<&gift::Options>) {
        // 0 is fully transparent and 255 is opaque.
        // Go's uint8(float32) converts through int32 (FCVTZS, saturating, NaN -> 0) and
        // truncates.
        let alpha = (self.opacity * 255.0) as i32 as u8;
        let mask = Uniform::new(Color::Alpha(Alpha { a: alpha }));
        let r = dst.bounds();
        draw::draw_mask(dst, r, src, ZP, Some(&mask), ZP, draw::Op::Over);
    }

    // Go: resources/images/opacity.go:Bounds
    fn bounds(&self, src_bounds: Rectangle) -> Rectangle {
        rect(0, 0, src_bounds.dx(), src_bounds.dy())
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/images/opacity.go (39 lines; 0/2 funcs executed)
//   types: opacityFilter
// OK L30-35: (f opacityFilter) Draw(dst draw.Image, src image.Image, options *gift.Options)
// OK L37-39: (f opacityFilter) Bounds(srcBounds image.Rectangle) image.Rectangle
// ---------------------------------------------------------------------------
