//! Port of `cache/filecache/filecache_config.go`.
//!
//! Owner: Wave B task T08 (helpers-source-cache).


use std::collections::BTreeMap;
use std::time::Duration;

use nh_common::Result;

pub const CACHE_KEY_GET_JSON: &str = "getjson";
pub const CACHE_KEY_GET_CSV: &str = "getcsv";
pub const CACHE_KEY_IMAGES: &str = "images";
pub const CACHE_KEY_ASSETS: &str = "assets";
pub const CACHE_KEY_MODULES: &str = "modules";
pub const CACHE_KEY_GET_RESOURCE: &str = "getresource";
pub const CACHE_KEY_MISC: &str = "misc";

/// Go: `filecache.FileCacheConfig`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FileCacheConfig {
    /// `None` = -1 (never expire); `Some(0)` = disabled reads.
    pub max_age: Option<Duration>,
    /// e.g. `:cacheDir/:project` (getresource), `:resourceDir/_gen` (images, assets).
    pub dir: String,
    pub dir_compiled: String,
    pub is_resource_dir: bool,
}

/// Go: `filecache.Configs`.
pub type Configs = BTreeMap<String, FileCacheConfig>;

/// Go: `filecache.DecodeConfig(fs, bcfg, m)` (`:project` = basename(workingDir) — the golden
/// site dir MUST be named `seeksnack` so the getresource cache is found).
// Go: cache/filecache/filecache_config.go:DecodeConfig
pub fn decode_config(base: &nh_config::common_config::BaseConfig, m: &go_value::Map) -> Result<Configs> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: cache/filecache/filecache_config.go (247 lines; 8/10 funcs executed)
//   types: Configs, FileCacheConfig
// EX L55-57: (c Configs) CacheDirModules() string
// EX L104-106: (f Caches) GetJSONCache() *Cache
// EX L109-111: (f Caches) GetCSVCache() *Cache
// EX L114-116: (f Caches) ImageCache() *Cache
//    L119-121: (f Caches) ModulesCache() *Cache
// EX L124-126: (f Caches) AssetsCache() *Cache
//    L129-131: (f Caches) MiscCache() *Cache
// EX L134-136: (f Caches) GetResourceCache() *Cache
// EX L138-233: DecodeConfig(fs afero.Fs, bcfg config.BaseConfig, m map[string]any) (Configs, error)
// EX L236-247: resolveDirPlaceholder(fs afero.Fs, bcfg config.BaseConfig, placeholder string) (cacheDir string, isResource bool, err error)
// ---------------------------------------------------------------------------
