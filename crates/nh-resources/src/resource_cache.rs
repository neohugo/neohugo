//! Port of `resources/resource_cache.go`.
//!
//! Owner: Wave B task T14 (resources-core).

//! Go `resources.ResourceCache`: GLOBAL (all languages) get-or-create caches. First writer wins:
//! `resources.Get` key `cleanKey(path)+"__get"`, Concat by target path, transformations by key.
//! Values are computed WITHOUT holding a lock (`dynacache::Partition::get_or_create`,
//! HUGO_LAYER.md §4.8).

use std::sync::Arc;

use nh_common::Result;
use nh_common::dynacache::{
    Cache as MemCache, ClearWhen, OptionsPartition, Partition, get_or_create_partition,
};
use nh_helpers::cache::filecache::filecache::Cache as FileCache;
use nh_resource::resourcetypes::{Resource, Resources};

use crate::transform::ResourceAdapterInner;

/// Go: `resources.ResourceCache`.
pub struct ResourceCache {
    pub cache_resource: Arc<Partition<String, Arc<dyn Resource>>>,
    pub cache_resource_file: Arc<Partition<String, Arc<dyn Resource>>>,
    pub cache_resource_remote: Arc<Partition<String, Arc<dyn Resource>>>,
    pub cache_resources: Arc<Partition<String, Resources>>,
    pub cache_resource_transformation: Arc<Partition<String, Arc<ResourceAdapterInner>>>,
    /// Go `fileCache` (the `assets` file cache, `resources/_gen/assets`). COLD-CACHE RULE: the
    /// port never reads it (and does not write it).
    pub file_cache: Option<Arc<FileCache>>,
}

impl ResourceCache {
    /// Go: `newResourceCache` over a private memory cache (tests).
    // Go: resources/resource_cache.go:newResourceCache
    pub fn new() -> Self {
        Self::new_with(None, &MemCache::new(Default::default()))
    }

    /// Go: `newResourceCache(rs, memCache)` — the partitions come from the build's shared
    /// memory cache (`/res1`, `/res2`, `/resr`, `/ress`, `/res1/tra`).
    // Go: resources/resource_cache.go:newResourceCache
    pub fn new_with(file_cache: Option<Arc<FileCache>>, mem_cache: &MemCache) -> Self {
        let opts = |w| OptionsPartition {
            clear_when: ClearWhen::OnChange,
            weight: w,
        };
        ResourceCache {
            file_cache,
            cache_resource: get_or_create_partition(mem_cache, "/res1", opts(40)),
            cache_resource_file: get_or_create_partition(mem_cache, "/res2", opts(40)),
            cache_resource_remote: get_or_create_partition(mem_cache, "/resr", opts(40)),
            cache_resources: get_or_create_partition(
                mem_cache,
                "/ress",
                OptionsPartition {
                    clear_when: ClearWhen::OnRebuild,
                    weight: 40,
                },
            ),
            cache_resource_transformation: get_or_create_partition(
                mem_cache,
                "/res1/tra",
                opts(40),
            ),
        }
    }

    /// Go: `cleanKey(k)` = TrimPrefix(path.Clean(ToLower(ToSlash(k))), "/").
    // Go: resources/resource_cache.go:cleanKey
    pub fn clean_key(key: &str) -> String {
        let lower =
            go_unicode::strings::to_lower(go_path::filepath::to_slash(key).as_bytes()).into_owned();
        let lower = String::from_utf8_lossy(&lower).into_owned();
        let cleaned = go_path::path::clean(&lower);
        cleaned
            .strip_prefix('/')
            .map(str::to_string)
            .unwrap_or_else(|| cleaned.to_string())
    }

    // Go: resources/resource_cache.go:Get
    pub fn get(&self, key: &str) -> Option<Arc<dyn Resource>> {
        self.cache_resource.get(&key.to_string())
    }

    // Go: resources/resource_cache.go:GetOrCreate
    pub fn get_or_create(
        &self,
        key: &str,
        f: impl FnOnce() -> Result<Arc<dyn Resource>>,
    ) -> Result<Arc<dyn Resource>> {
        self.cache_resource.get_or_create(key.to_string(), |_| f())
    }

    // Go: resources/resource_cache.go:GetOrCreateFile
    pub fn get_or_create_file(
        &self,
        key: &str,
        f: impl FnOnce() -> Result<Arc<dyn Resource>>,
    ) -> Result<Arc<dyn Resource>> {
        self.cache_resource_file
            .get_or_create(key.to_string(), |_| f())
    }

    // Go: resources/resource_cache.go:GetOrCreateResources
    pub fn get_or_create_resources(
        &self,
        key: &str,
        f: impl FnOnce() -> Result<Resources>,
    ) -> Result<Resources> {
        self.cache_resources.get_or_create(key.to_string(), |_| f())
    }

    // Go: resources/resource_cache.go:getFilenames
    pub(crate) fn get_filenames(key: &str) -> (String, String) {
        let filename_meta = format!("{key}.json");
        let filename_content = format!("{key}.content");

        (filename_meta, filename_content)
    }

    /// Go: `getFromFile(key)`. COLD-CACHE RULE: the port never reads `resources/_gen`, so this
    /// never finds anything (Go's behaviour with an empty file cache).
    // Go: resources/resource_cache.go:getFromFile
    pub fn get_from_file(&self, _key: &str) -> Option<()> {
        None
    }

    /// Go: `writeMeta(key, meta)` writes `<key>.json` and returns a writer for `<key>.content`.
    /// The port does not write the file cache (optional, off by default: nothing ever reads it;
    /// HUGO_LAYER.md §4.5); this returns the JSON Go would write.
    // Go: resources/resource_cache.go:writeMeta
    pub(crate) fn write_meta(
        &self,
        key: &str,
        meta: &crate::transform::TransformedResourceMetadata,
    ) -> Result<(String, Vec<u8>)> {
        let (filename_meta, _) = Self::get_filenames(key);
        let raw = meta.marshal_json()?;
        Ok((filename_meta, raw))
    }
}

impl Default for ResourceCache {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/resource_cache.go (149 lines; 6/9 funcs executed)
//   types: ResourceCache
// OK L31-60: newResourceCache(rs *Spec, memCache *dynacache.Cache) *ResourceCache
// OK L74-76: (c *ResourceCache) cleanKey(key string) string
// OK L78-80: (c *ResourceCache) Get(ctx context.Context, key string) (resource.Resource, bool)
// OK L82-86: (c *ResourceCache) GetOrCreate(key string, f func() (resource.Resource, error)) (resource.Resource, error)
// OK L88-92: (c *ResourceCache) GetOrCreateFile(key string, f func() (resource.Resource, error)) (resource.Resource, error)
// OK L94-98: (c *ResourceCache) GetOrCreateResources(key string, f func() (resource.Resources, error)) (resource.Resources, error)
// OK L100-105: (c *ResourceCache) getFilenames(key string) (string, string)
// OK L107-126: (c *ResourceCache) getFromFile(key string) (filecache.ItemInfo, io.ReadCloser, transformedResourceMetadata, bool) (cold: never found)
// OK L129-149: (c *ResourceCache) writeMeta(key string, meta transformedResourceMetadata) (filecache.ItemInfo, io.WriteCloser, error) (not written)
// ---------------------------------------------------------------------------
