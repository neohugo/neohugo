//! Port of `resources/resource_cache.go`.
//!
//! Owner: Wave B task T14 (resources-core).


//! Go `resources.ResourceCache`: GLOBAL (all languages) get-or-create caches. First writer wins:
//! `resources.Get` key `cleanKey(path)+"__get"`, Concat by target path, transformations by key.

use std::sync::Arc;

use nh_common::dynacache::Partition;
use nh_common::Result;
use nh_resource::resourcetypes::{Resource, Resources};

/// Go: `resources.ResourceCache`.
pub struct ResourceCache {
    pub cache_resource: Partition<String, Arc<dyn Resource>>,
    pub cache_resource_file: Partition<String, Arc<dyn Resource>>,
    pub cache_resource_remote: Partition<String, Arc<dyn Resource>>,
    pub cache_resources: Partition<String, Resources>,
    pub cache_resource_transformation: Partition<String, Arc<crate::transform::ResourceAdapterInner>>,
}

impl ResourceCache {
    // Go: resources/resource_cache.go:newResourceCache
    pub fn new() -> Self {
        todo!()
    }

    /// Go: `cleanKey(k)` = TrimPrefix(path.Clean(ToLower(ToSlash(k))), "/").
    // Go: resources/resource_cache.go:cleanKey
    pub fn clean_key(key: &str) -> String {
        todo!()
    }

    // Go: resources/resource_cache.go:GetOrCreate
    pub fn get_or_create(&self, key: &str, f: impl FnOnce() -> Result<Arc<dyn Resource>>) -> Result<Arc<dyn Resource>> {
        todo!()
    }

    // Go: resources/resource_cache.go:GetOrCreateResources
    pub fn get_or_create_resources(&self, key: &str, f: impl FnOnce() -> Result<Resources>) -> Result<Resources> {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/resource_cache.go (149 lines; 6/9 funcs executed)
//   types: ResourceCache
// EX L31-60: newResourceCache(rs *Spec, memCache *dynacache.Cache) *ResourceCache
// EX L74-76: (c *ResourceCache) cleanKey(key string) string
//    L78-80: (c *ResourceCache) Get(ctx context.Context, key string) (resource.Resource, bool)
// EX L82-86: (c *ResourceCache) GetOrCreate(key string, f func() (resource.Resource, error)) (resource.Resource, error)
// EX L88-92: (c *ResourceCache) GetOrCreateFile(key string, f func() (resource.Resource, error)) (resource.Resource, error)
//    L94-98: (c *ResourceCache) GetOrCreateResources(key string, f func() (resource.Resources, error)) (resource.Resources, error)
// EX L100-105: (c *ResourceCache) getFilenames(key string) (string, string)
//    L107-126: (c *ResourceCache) getFromFile(key string) (filecache.ItemInfo, io.ReadCloser, transformedResourceMetadata, bool)
// EX L129-149: (c *ResourceCache) writeMeta(key string, meta transformedResourceMetadata) (filecache.ItemInfo, io.WriteCloser, error)
// ---------------------------------------------------------------------------
