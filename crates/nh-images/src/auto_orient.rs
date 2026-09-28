//! Port of `resources/images/auto_orient.go`.
//!
//! Owner: Wave B task T10 (images).

use std::sync::Arc;

use crate::exif::ExifInfo;

/// Go: `transformationFilters` (EXIF orientation -> gift transform).
// Go: resources/images/auto_orient.go:transformationFilters
fn transformation_filter(orientation: i64) -> Option<Arc<dyn gift::Filter>> {
    Some(match orientation {
        2 => gift::flip_horizontal(),
        3 => gift::rotate180(),
        4 => gift::flip_vertical(),
        5 => gift::transpose(),
        6 => gift::rotate270(),
        7 => gift::transverse(),
        8 => gift::rotate90(),
        _ => return None,
    })
}

/// Go: `images.autoOrientFilter`. Its `Draw` and `Bounds` panic with `not supported` in Go (the
/// resource layer replaces it by [`AutoOrientFilter::auto_orient`]'s filter); the port's
/// `GiftFilter::to_gift` returns that error.
pub struct AutoOrientFilter;

/// Go: `images.ImageFilterFromOrientationProvider`.
pub trait ImageFilterFromOrientationProvider {
    fn auto_orient(&self, exif_info: Option<&ExifInfo>) -> Option<Arc<dyn gift::Filter>>;
}

impl ImageFilterFromOrientationProvider for AutoOrientFilter {
    // Go: resources/images/auto_orient.go:AutoOrient
    fn auto_orient(&self, exif_info: Option<&ExifInfo>) -> Option<Arc<dyn gift::Filter>> {
        if let Some(exif_info) = exif_info
            && let Some(v) = exif_info.tags.get("Orientation")
        {
            let orientation = nh_common::cast::caste::to_int(v);
            if let Some(filter) = transformation_filter(orientation) {
                return Some(filter);
            }
        }

        None
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/images/auto_orient.go (62 lines; 0/3 funcs executed)
//   types: autoOrientFilter, ImageFilterFromOrientationProvider
// OK L43-45: (f autoOrientFilter) Draw(dst draw.Image, src image.Image, options *gift.Options)
// OK L47-49: (f autoOrientFilter) Bounds(srcBounds image.Rectangle) image.Rectangle
// OK L51-62: (f autoOrientFilter) AutoOrient(exifInfo *exif.ExifInfo) gift.Filter
// ---------------------------------------------------------------------------
