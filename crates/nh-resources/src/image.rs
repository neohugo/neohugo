//! Port of `resources/image.go`.
//!
//! Owner: Wave B task T14 (resources-core).


//! Go `resources/image.go`: `imageResource` — Resize/Filter/... with the `_hu_` naming formula
//! `p1 + "_hu_" + HashStringHex(incomingID, root hash, conf.Key, imagingConfig.SourceHash) + ext`
//! and the Filter key `HashString(gfilters)` (specs/images.md §3-4).
//!
//! DECODE FROM ENCODED BYTES (images.md §3.1, §3.5, §9.10): every `Resize`/`Filter` decodes the
//! ENCODED bytes of its parent (Go `DecodeImage()` reads the file cache entry of an intermediate:
//! a JPEG/PNG round trip), and `images.Overlay` decodes the watermark's encoded PNG. The Rust
//! port never reads `resources/_gen`, so `ImageCache` keeps the encoded bytes of every processed
//! image in memory (keyed like Go's file cache) and `decode_image` decodes those bytes. Never
//! pass decoded pixels from one step to the next: the results differ.

use std::sync::Arc;

use go_value::Value;
use nh_common::Result;
use nh_images::config::ImageConfig;
use nh_images::image::{GoImage, Image};
use nh_resource::internal::resourcepaths::ResourcePaths;

use crate::resource::GenericResource;
use crate::transform::ResourceAdapter;

/// Go: `resources.imageResource`.
pub struct ImageResource {
    pub image: Image,
    /// The root image this was derived from (for exif / dominant colors).
    pub(crate) root: Option<Arc<ImageResource>>,
    pub base: Arc<GenericResource>,
}

impl ImageResource {
    // Go: resources/image.go:Resize
    pub fn resize(self: &Arc<Self>, spec: &str) -> Result<Arc<ResourceAdapter>> {
        todo!()
    }

    // Go: resources/image.go:Filter
    pub fn filter(self: &Arc<Self>, filters: &[Value]) -> Result<Arc<ResourceAdapter>> {
        todo!()
    }

    // Go: resources/image.go:processOptions
    pub(crate) fn process_options(self: &Arc<Self>, options: &[String]) -> Result<Arc<ResourceAdapter>> {
        todo!()
    }

    /// Go: `doWithImageConfig(conf, f)` via the ImageCache (keyed by target path; cold path only).
    // Go: resources/image.go:doWithImageConfig
    pub(crate) fn do_with_image_config(self: &Arc<Self>, conf: ImageConfig, f: &dyn Fn(&GoImage) -> Result<GoImage>) -> Result<Arc<ResourceAdapter>> {
        todo!()
    }

    /// Go: `DecodeImage()` — decode the ENCODED bytes of this (possibly processed) image.
    // Go: resources/image.go:DecodeImage
    pub fn decode_image(&self) -> Result<GoImage> {
        todo!()
    }

    // Go: resources/image.go:relTargetPathFromConfig
    pub(crate) fn rel_target_path_from_config(&self, conf: &ImageConfig, imaging_config_source_hash: &str) -> ResourcePaths {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/image.go (480 lines; 8/21 funcs executed)
//   types: imageResource, imageMeta, giphy
//    L78-80: (i *imageResource) Exif() *exif.ExifInfo
//    L82-143: (i *imageResource) getExif() *exif.ExifInfo
//    L147-161: (i *imageResource) Colors() ([]images.Color, error)
//    L163-165: (i *imageResource) targetPath() string
//    L168-175: (i *imageResource) Clone() resource.Resource
//    L177-184: (i *imageResource) cloneTo(targetPath string) resource.Resource
//    L186-205: (i *imageResource) cloneWithUpdates(u *transformationUpdate) (baseResource, error)
//    L211-213: (i *imageResource) Process(spec string) (images.ImageResource, error)
// EX L218-220: (i *imageResource) Resize(spec string) (images.ImageResource, error)
//    L224-226: (i *imageResource) Crop(spec string) (images.ImageResource, error)
//    L230-232: (i *imageResource) Fit(spec string) (images.ImageResource, error)
//    L237-239: (i *imageResource) Fill(spec string) (images.ImageResource, error)
// EX L241-293: (i *imageResource) Filter(filters ...any) (images.ImageResource, error)
// EX L295-298: (i *imageResource) processActionSpec(action, spec string) (images.ImageResource, error)
// EX L300-326: (i *imageResource) processOptions(options []string) (images.ImageResource, error)
// EX L337-397: (i *imageResource) doWithImageConfig(conf images.ImageConfig, f func(src image.Image) (image.Image, error)) (images.ImageResource, error)
//    L404-406: (g *giphy) GIF() *gif.GIF
// EX L410-426: (i *imageResource) DecodeImage() (image.Image, error)
// EX L428-443: (i *imageResource) clone(img image.Image) *imageResource
//    L445-457: (i *imageResource) getImageMetaCacheTargetPath() string
// EX L459-480: (i *imageResource) relTargetPathFromConfig(conf images.ImageConfig, imagingConfigSourceHash string) internal.ResourcePaths
// ---------------------------------------------------------------------------
