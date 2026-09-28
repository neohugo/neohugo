//! Port of `resources/images/smartcrop.go`.
//!
//! STUB for the analysis itself: `github.com/muesli/smartcrop` is not ported (seeksnack never
//! smart-crops). `smartCrop`'s early returns are ported; reaching the analyzer returns
//! [`unsupported`].
//!
//! Owner: Wave B task T10 (images).

use go_image::Rectangle;
use nh_common::{Error, Result};

use crate::image::{GoImage, ImageProcessor};

/// Do not change.
pub const SMART_CROP_IDENTIFIER: &str = "smart";
pub const SMART_CROP_ANCHOR: i64 = 1000;
/// This is just a increment, starting on 0. If Smart Crop improves its cropping, we need a way
/// to trigger a re-generation of the crops in the wild, so increment this.
pub const SMART_CROP_VERSION_NUMBER: i64 = 0;

/// The error for the unported smartcrop analysis.
pub fn unsupported() -> Error {
    Error::feature_not_available("neohugo-rs: smart cropping (anchor \"smart\") is not supported")
}

impl ImageProcessor {
    /// Go: `newSmartCropAnalyzer` (muesli/smartcrop) — not ported.
    // Go: resources/images/smartcrop.go:newSmartCropAnalyzer
    fn new_smart_crop_analyzer(&self, _filter: &'static dyn gift::Resampling) -> Result<()> {
        Err(unsupported())
    }

    // Go: resources/images/smartcrop.go:smartCrop
    pub(crate) fn smart_crop(
        &self,
        img: &GoImage,
        width: i64,
        height: i64,
        filter: &'static dyn gift::Resampling,
    ) -> Result<Rectangle> {
        if width <= 0 || height <= 0 {
            return Ok(Rectangle::default());
        }

        let src_bounds = img.bounds();
        let src_w = src_bounds.dx();
        let src_h = src_bounds.dy();

        if src_w <= 0 || src_h <= 0 {
            return Ok(Rectangle::default());
        }

        if src_w == width && src_h == height {
            return Ok(src_bounds);
        }

        self.new_smart_crop_analyzer(filter)?;
        Err(unsupported())
    }
}

/// Calculates scaling factors using old and new image dimensions.
/// Code borrowed from https://github.com/nfnt/resize/blob/83c6a9932646f83e3267f353373d47347b6036b2/resize.go#L593
// Go: resources/images/smartcrop.go:calcFactorsNfnt
pub fn calc_factors_nfnt(width: u64, height: u64, old_width: f64, old_height: f64) -> (f64, f64) {
    let (scale_x, scale_y);
    if width == 0 {
        if height == 0 {
            scale_x = 1.0;
            scale_y = 1.0;
        } else {
            scale_y = old_height / height as f64;
            scale_x = scale_y;
        }
    } else {
        scale_x = old_width / width as f64;
        if height == 0 {
            scale_y = scale_x;
        } else {
            scale_y = old_height / height as f64;
        }
    }
    (scale_x, scale_y)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/images/smartcrop.go (104 lines; 0/4 funcs executed)
//   types: imagingResizer
// STUB L34-36: (p *ImageProcessor) newSmartCropAnalyzer(filter gift.Resampling) smartcrop.Analyzer
// STUB L44-55: (r imagingResizer) Resize(img image.Image, width, height uint) image.Image
// OK L57-82: (p *ImageProcessor) smartCrop(img image.Image, width, height int, filter gift.Resampling) (image.Rectangle, error)
// OK L86-104: calcFactorsNfnt(width, height uint, oldWidth, oldHeight float64) (scaleX, scaleY float64)
// ---------------------------------------------------------------------------
