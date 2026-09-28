//! Port of `resources/images/mask.go`.
//!
//! Owner: Wave B task T10 (images).

use std::sync::Arc;

use go_image::color::Alpha as AlphaColor;
use go_image::{Alpha, Gray, Image, RGBA, Rectangle, ZP, draw, rect};
use nh_common::{Error, Result};

use crate::image::{GoImage, ImageSource};

/// Go: `images.maskFilter` applies a mask image to a base image.
pub struct MaskFilter {
    pub mask: Arc<dyn ImageSource>,
}

impl MaskFilter {
    /// Go decodes the mask inside `Draw` and panics with `failed to decode image: %s`; the port
    /// decodes it before drawing and returns the error.
    pub(crate) fn prepare(&self) -> Result<MaskDraw> {
        let mask = self
            .mask
            .decode_image()
            .map_err(|e| Error::new(format!("failed to decode image: {}", e.message())))?;
        Ok(MaskDraw { mask })
    }
}

/// A [`MaskFilter`] with its mask decoded.
pub(crate) struct MaskDraw {
    mask: GoImage,
}

impl gift::Filter for MaskDraw {
    /// Draw applies the mask to the base image.
    // Go: resources/images/mask.go:Draw
    fn draw(
        &self,
        dst: &mut dyn draw::Image,
        base_image: &dyn Image,
        _options: Option<&gift::Options>,
    ) {
        let mut mask_image: &dyn Image = self.mask.image();

        // Ensure the mask is the same size as the base image
        let base_bounds = base_image.bounds();
        let mask_bounds = mask_image.bounds();

        // Resize mask to match base image size if necessary
        let resized_mask;
        if mask_bounds.dx() != base_bounds.dx() || mask_bounds.dy() != base_bounds.dy() {
            let g = gift::new(vec![gift::resize(
                base_bounds.dx(),
                base_bounds.dy(),
                &gift::LANCZOS_RESAMPLING,
            )]);
            let mut m = RGBA::new(g.bounds(mask_image.bounds()));
            g.draw(&mut m, mask_image);
            resized_mask = m;
            mask_image = &resized_mask;
        }

        // Use gift to convert the resized mask to grayscale
        let g = gift::new(vec![gift::grayscale()]);
        let mut grayscale_mask = Gray::new(g.bounds(mask_image.bounds()));
        g.draw(&mut grayscale_mask, mask_image);

        // Convert grayscale mask to alpha mask
        let mut alpha_mask = Alpha::new(base_bounds);
        for y in base_bounds.min.y..base_bounds.max.y {
            for x in base_bounds.min.x..base_bounds.max.x {
                let gray_value = grayscale_mask.gray_at(x, y).y;
                alpha_mask.set_alpha(x, y, AlphaColor { a: gray_value });
            }
        }

        // Create an RGBA output image
        let mut output_image = RGBA::new(base_bounds);

        // Apply the mask using draw.DrawMask
        draw::draw_mask(
            &mut output_image,
            base_bounds,
            base_image,
            ZP,
            Some(&alpha_mask),
            ZP,
            draw::Op::Over,
        );

        // Copy the result to the destination
        gift::new(Vec::new()).draw(dst, &output_image);
    }

    /// Bounds returns the bounds of the resulting image.
    // Go: resources/images/mask.go:Bounds
    fn bounds(&self, img_bounds: Rectangle) -> Rectangle {
        rect(0, 0, img_bounds.dx(), img_bounds.dy())
    }
}

impl MaskFilter {
    /// Go: `maskFilter.Bounds`.
    pub fn bounds(&self, img_bounds: Rectangle) -> Rectangle {
        rect(0, 0, img_bounds.dx(), img_bounds.dy())
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/images/mask.go (63 lines; 0/2 funcs executed)
//   types: maskFilter
// OK L18-58: (f maskFilter) Draw(dst draw.Image, baseImage image.Image, options *gift.Options)
// OK L61-63: (f maskFilter) Bounds(imgBounds image.Rectangle) image.Rectangle
// ---------------------------------------------------------------------------
