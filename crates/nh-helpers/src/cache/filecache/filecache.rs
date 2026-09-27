//! Port of `cache/filecache/filecache.go`.
//!
//! Owner: Wave B task T08 (helpers-source-cache).


//! Go `cache/filecache`. Cold-cache rule (HUGO_LAYER.md §9): the `images` and `assets` caches are
//! never READ in the Rust port (the golden is a cold build and the Go read path changes image
//! names); `getresource` IS read (YouTube responses live there).

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use nh_common::Result;
use nh_hugofs::afero::Fs;

/// Go: `filecache.ItemInfo`.
#[derive(Clone, Debug, Default)]
pub struct ItemInfo {
    /// The name of the item in the cache.
    pub name: String,
}

/// Go: `filecache.Cache`.
pub struct Cache {
    pub fs: Arc<dyn Fs>,
    /// `None` = never expires (Go -1); `Some(0)` = never read (Go 0, `--ignoreCache`).
    pub max_age: Option<Duration>,
    pub prune_all_root_dir: String,
}

impl Cache {
    // Go: cache/filecache/filecache.go:NewCache
    pub fn new(fs: Arc<dyn Fs>, max_age: Option<Duration>, prune_all_root_dir: &str) -> Arc<Cache> {
        todo!()
    }

    // Go: cache/filecache/filecache.go:GetOrCreateBytes
    pub fn get_or_create_bytes(&self, id: &str, create: impl FnOnce() -> Result<Vec<u8>>) -> Result<(ItemInfo, Vec<u8>)> {
        todo!()
    }

    /// Go: `GetBytes(id)` — `None` if missing/expired.
    // Go: cache/filecache/filecache.go:GetBytes
    pub fn get_bytes(&self, id: &str) -> Result<(ItemInfo, Option<Vec<u8>>)> {
        todo!()
    }

    // Go: cache/filecache/filecache.go:WriteCloser
    pub fn write(&self, id: &str, data: &[u8]) -> Result<ItemInfo> {
        todo!()
    }

    /// Go: `AsHTTPCache()` (gohugoio/httpcache.Cache: Get/Set/Delete of raw response dumps).
    // Go: cache/filecache/filecache.go:AsHTTPCache
    pub fn as_http_cache(self: &Arc<Self>) -> Arc<dyn crate::cache::httpcache::transport::HttpCache> {
        todo!()
    }
}

/// Go: `filecache.Caches` (by name: getjson, getcsv, images, assets, modules, getresource, misc).
pub type Caches = BTreeMap<String, Arc<Cache>>;

/// Go: `filecache.NewCaches(p)`.
// Go: cache/filecache/filecache.go:NewCaches
pub fn new_caches(p: &crate::pathspec::PathSpec) -> Result<Caches> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: cache/filecache/filecache.go (499 lines; 14/24 funcs executed)
//   types: Cache, lockTracker, ItemInfo, lockedFile, Caches, httpCache
// EX L70-82: (l *lockTracker) Lock(id string)
// EX L91-98: NewCache(fs afero.Fs, maxAge time.Duration, pruneAllRootDir string) *Cache
// EX L106-109: (l *lockedFile) Close() error
// EX L111-119: (c *Cache) init() error
// EX L123-143: (c *Cache) WriteCloser(id string) (ItemInfo, io.WriteCloser, error)
// EX L149-184: (c *Cache) ReadOrCreate(id string, read func(info ItemInfo, r io.ReadSeeker) error, create func(info ItemInfo, w io.WriteCloser) error, ) (info Ite...
// EX L187-193: (c *Cache) NamedLock(id string) func()
//    L198-232: (c *Cache) GetOrCreate(id string, create func() (io.ReadCloser, error)) (ItemInfo, io.ReadCloser, error)
//    L234-248: (c *Cache) writeReader(id string, r io.Reader) error
//    L251-288: (c *Cache) GetOrCreateBytes(id string, create func() ([]byte, error)) (ItemInfo, []byte, error)
//    L291-310: (c *Cache) GetBytes(id string) (ItemInfo, []byte, error)
//    L313-328: (c *Cache) Get(id string) (ItemInfo, io.ReadCloser, error)
// EX L332-348: (c *Cache) getOrRemove(id string) (hugio.ReadSeekCloser, error)
// EX L350-373: (c *Cache) getBytesAndRemoveIfExpired(id string) ([]byte, bool)
// EX L375-391: (c *Cache) removeIfExpired(id string) (bool, error)
//    L393-401: (c *Cache) isExpired(modTime time.Time) bool
//    L404-418: (c *Cache) GetString(id string) string
//    L424-426: (f Caches) Get(name string) *Cache
// EX L430-461: NewCaches(p *helpers.PathSpec) (Caches, error)
// EX L463-465: cleanID(name string) string
// EX L470-472: (c *Cache) AsHTTPCache() httpcache.Cache
// EX L478-483: (h *httpCache) Get(id string) (resp []byte, ok bool)
//    L485-495: (h *httpCache) Set(id string, resp []byte)
//    L497-499: (h *httpCache) Delete(key string)
// ---------------------------------------------------------------------------
