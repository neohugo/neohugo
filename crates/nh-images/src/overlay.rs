//! Port of `resources/images/overlay.go`.
//!
//! Owner: Wave B task T10 (images).

use std::sync::Arc;

use go_image::{Image, Rectangle, draw, pt, rect};
use nh_common::{Error, Result};

use crate::image::{GoImage, ImageSource};

/// Go: `images.overlayFilter` — `Draw`: copy src into dst, then `DrawAt(dst, overlay, (x,y),
/// OverOperator)` with the overlay decoded from its encoded PNG (see specs/images.md §5.4 FMA).
pub struct OverlayFilter {
    pub src: Arc<dyn ImageSource>,
    pub x: i64,
    pub y: i64,
}

impl OverlayFilter {
    /// Go decodes the overlay inside `Draw` and panics with `failed to decode image: %s` on an
    /// error; the port decodes it before drawing and returns that error.
    pub(crate) fn prepare(&self) -> Result<OverlayDraw> {
        let overlay_src = self
            .src
            .decode_image()
            .map_err(|e| Error::new(format!("failed to decode image: {}", e.message())))?;
        Ok(OverlayDraw {
            overlay: overlay_src,
            x: self.x,
            y: self.y,
        })
    }

    // Go: resources/images/overlay.go:Bounds
    pub fn bounds(&self, src_bounds: Rectangle) -> Rectangle {
        rect(0, 0, src_bounds.dx(), src_bounds.dy())
    }
}

/// An [`OverlayFilter`] with its source decoded: the `gift.Filter` that is drawn.
pub(crate) struct OverlayDraw {
    overlay: GoImage,
    x: i64,
    y: i64,
}

impl gift::Filter for OverlayDraw {
    // Go: resources/images/overlay.go:Draw
    fn draw(&self, dst: &mut dyn draw::Image, src: &dyn Image, _options: Option<&gift::Options>) {
        gift::new(Vec::new()).draw(dst, src);
        gift::new(Vec::new()).draw_at(
            dst,
            self.overlay.image(),
            pt(self.x, self.y),
            gift::OVER_OPERATOR,
        );
    }

    // Go: resources/images/overlay.go:Bounds
    fn bounds(&self, src_bounds: Rectangle) -> Rectangle {
        rect(0, 0, src_bounds.dx(), src_bounds.dy())
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/images/overlay.go (43 lines; 2/2 funcs executed)
//   types: overlayFilter
// OK L31-39: (f overlayFilter) Draw(dst draw.Image, src image.Image, options *gift.Options)
// OK L41-43: (f overlayFilter) Bounds(srcBounds image.Rectangle) image.Rectangle
// ---------------------------------------------------------------------------
