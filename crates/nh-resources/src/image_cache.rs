//! Port of `resources/image_cache.go`.
//!
//! Owner: Wave B task T14 (resources-core).

use std::sync::Arc;

use nh_common::Result;
use nh_common::dynacache::{
    Cache as MemCache, ClearWhen, OptionsPartition, Partition, get_or_create_partition,
};
use nh_helpers::cache::filecache::filecache::Cache as FileCache;
use nh_helpers::pathspec::PathSpec;

use crate::image::{ImageResource, ImageResourceParts};
use crate::transform::{BaseResource, ResourceAdapter};

/// Go: `resources.ImageCache` — memory cache keyed by target path; the file cache
/// (`resources/_gen/images`) is never read (reading it would change output names: cold-cache
/// rule) and not written (optional, off). The ENCODED bytes of every processed image are kept
/// in `encoded` (the in-memory stand-in for Go's file cache entry, keyed by the file cache id
/// Go uses, `cleanID(relTargetPath)`): later steps (`Resize` -> `Filter`, the overlay source)
/// decode them, as Go does (images.md §9.10). The image cache is shared by all sites.
pub struct ImageCache {
    pub path_spec: Arc<PathSpec>,
    pub fcache: Option<Arc<FileCache>>,
    pub mcache: Arc<Partition<String, Arc<ResourceAdapter>>>,
    /// Encoded bytes per processed image (key = Go's file-cache id of the image).
    pub encoded: Arc<Partition<String, Arc<Vec<u8>>>>,
}

impl ImageCache {
    /// Go: `newImageCache` over a private memory cache.
    // Go: resources/image_cache.go:newImageCache
    pub fn new(fcache: Option<Arc<FileCache>>, ps: Arc<PathSpec>) -> ImageCache {
        Self::new_with(fcache, &MemCache::new(Default::default()), ps)
    }

    /// Go: `newImageCache(fileCache, memCache, ps)` — the `/imgs` partition of the build's
    /// shared memory cache (and `/imgs/enc` for the encoded bytes).
    // Go: resources/image_cache.go:newImageCache
    pub fn new_with(
        fcache: Option<Arc<FileCache>>,
        mem_cache: &MemCache,
        ps: Arc<PathSpec>,
    ) -> ImageCache {
        let opts = OptionsPartition {
            clear_when: ClearWhen::OnChange,
            weight: 70,
        };
        ImageCache {
            fcache,
            mcache: get_or_create_partition(mem_cache, "/imgs", opts),
            encoded: get_or_create_partition(mem_cache, "/imgs/enc", opts),
            path_spec: ps,
        }
    }

    /// The encoded bytes of a processed image by its file-cache id (`cleanID(relTargetPath)`).
    pub fn encoded_bytes(&self, rel_target_path: &str) -> Option<Arc<Vec<u8>>> {
        self.encoded
            .get(&nh_helpers::cache::filecache::filecache::clean_id(
                rel_target_path,
            ))
    }

    /// Go: `getOrCreate(parent, conf, createImage)` — the create path only (cold cache).
    /// First writer wins; nothing is computed while a lock is held.
    // Go: resources/image_cache.go:(*ImageCache).getOrCreate
    pub(crate) fn get_or_create(
        &self,
        parent: &Arc<ImageResource>,
        conf: &nh_images::config::ImageConfig,
        create_image: &dyn Fn() -> Result<(ImageResourceParts, nh_images::image::GoImage)>,
    ) -> Result<Arc<ResourceAdapter>> {
        let rel_target =
            parent.rel_target_path_from_config(conf, &parent.image.proc.cfg.source_hash);
        let rel_target_path = rel_target.target_path();
        let mut mem_key = rel_target_path.clone();

        // For multihost sites, we duplicate language versions of the same resource,
        // so we need to include the language in the key.
        // Note that we don't need to include the language in the file cache key,
        // as the hash will take care of any different content.
        if self.path_spec.cfg.is_multihost() {
            mem_key = format!("{}{mem_key}", self.path_spec.lang());
        }
        let mem_key = nh_common::dynacache::clean_key(&mem_key);

        self.mcache.get_or_create(mem_key, |_| {
            // The definition of this counter is not that we have processed that amount
            // (e.g. resized etc.), it can be fetched from file cache,
            //  but the count of processed image variations for this site.
            nh_helpers::processing_stats::ProcessingStats::incr(
                &self.path_spec.processing_stats.processed_images,
            );

            // Go: `fcache.ReadOrCreate(relTargetPath, read, create)`. The port's "file cache" is
            // `encoded`, which only holds what this build created: `resources/_gen` is never
            // read (cold-cache rule).
            let id = nh_helpers::cache::filecache::filecache::clean_id(&rel_target_path);

            if let Some(bytes) = self.encoded.get(&id) {
                // read clones the parent to its new name and copies the content to the
                // destinations. (Only reached when this build already created the file under
                // another memory key: multihost sites.)
                let mut base = parent.base.clone_resource();
                let mut target_path = base.get_resource_paths();
                target_path.file = rel_target.file.clone();
                base.set_target_path(target_path);
                base.set_open_source(nh_common::hugio::new_open_read_seek_closer_from_bytes(
                    bytes.clone(),
                ));
                base.set_source_filename_is_hash(true);
                base.set_media_type(
                    conf.target_format
                        .map(|t| t.media_type())
                        .unwrap_or_default(),
                );
                let base = Arc::new(base);
                let image = parent.image.with_spec(base.clone());
                image.init_config(&mut std::io::Cursor::new(bytes.as_slice()))?;
                let img = ImageResource::from_parts(image, parent.root(), base);
                return Ok(ResourceAdapter::new(
                    parent.base.spec.clone(),
                    true,
                    BaseResource::Image(Arc::new(img)),
                ));
            }

            // create creates the image and encodes it to the cache (w).
            let (mut parts, conv) = create_image()?;
            let mut target_path = parts.base.get_resource_paths();
            target_path.file = rel_target.file.clone();
            parts.base.set_target_path(target_path);

            let mut w = Vec::new();
            parts.image.encode_to(conf, &conv, &mut w)?;
            let encoded = self.encoded.get_or_create(id, |_| Ok(Arc::new(w)))?;
            parts
                .base
                .set_open_source(nh_common::hugio::new_open_read_seek_closer_from_bytes(
                    encoded,
                ));

            let img = parts.build();

            let img_adapter = ResourceAdapter::new(
                parent.base.spec.clone(),
                true,
                BaseResource::Image(Arc::new(img)),
            );

            Ok(img_adapter)
        })
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/image_cache.go (125 lines; 2/2 funcs executed)
//   types: ImageCache
// OK L36-113: (c *ImageCache) getOrCreate( parent *imageResource, conf images.ImageConfig, createImage func() (*imageResource, image.Image, error), ) (*resourceA...
// OK L115-125: newImageCache(fileCache *filecache.Cache, memCache *dynacache.Cache, ps *helpers.PathSpec) *ImageCache
// ---------------------------------------------------------------------------
