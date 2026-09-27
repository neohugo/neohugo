//! Port of `resources/image_cache.go`.
//!
//! Owner: Wave B task T14 (resources-core).


use std::sync::Arc;

use nh_common::dynacache::Partition;
use nh_common::Result;
use nh_helpers::cache::filecache::filecache::Cache as FileCache;
use nh_helpers::pathspec::PathSpec;

use crate::image::ImageResource;
use crate::transform::ResourceAdapter;

/// Go: `resources.ImageCache` — memory cache keyed by target path; the file cache
/// (`resources/_gen/images`) is WRITE-ONLY (optional) in the Rust port: reading it would change
/// output names (cold-cache rule). The ENCODED bytes of every processed image are kept in
/// `encoded` (the in-memory stand-in for Go's file cache entry): later steps (`Resize` ->
/// `Filter`, the overlay source) decode them, as Go does (images.md §9.10).
pub struct ImageCache {
    pub path_spec: Arc<PathSpec>,
    pub fcache: Option<Arc<FileCache>>,
    pub mcache: Partition<String, Arc<ResourceAdapter>>,
    /// Encoded bytes per processed image (key = Go's file-cache key of the image).
    pub encoded: Partition<String, Arc<Vec<u8>>>,
}

impl ImageCache {
    // Go: resources/image_cache.go:newImageCache
    pub fn new(fcache: Option<Arc<FileCache>>, ps: Arc<PathSpec>) -> ImageCache {
        todo!()
    }

    /// Go: `getOrCreate(parent, conf, createImage)` — create path only.
    // Go: resources/image_cache.go:getOrCreate
    pub fn get_or_create(
        &self,
        parent: &Arc<ImageResource>,
        conf: &nh_images::config::ImageConfig,
        create: &dyn Fn() -> Result<(Arc<ImageResource>, nh_images::image::GoImage)>,
    ) -> Result<Arc<ResourceAdapter>> {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/image_cache.go (125 lines; 2/2 funcs executed)
//   types: ImageCache
// EX L36-113: (c *ImageCache) getOrCreate( parent *imageResource, conf images.ImageConfig, createImage func() (*imageResource, image.Image, error), ) (*resourceA...
// EX L115-125: newImageCache(fileCache *filecache.Cache, memCache *dynacache.Cache, ps *helpers.PathSpec) *ImageCache
// ---------------------------------------------------------------------------
