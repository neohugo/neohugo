//! Port of `cache/filecache/filecache_config.go`.
//!
//! Owner: Wave B task T08 (helpers-source-cache).

use std::collections::BTreeMap;
use std::sync::Arc;

use go_path::{filepath, path};
use go_time::Duration;
use go_value::{Map, MapType, Value};
use nh_common::Result;
use nh_common::herrors::Error;
use nh_config::common_config::BaseConfig;
use nh_config::decode::{Decoder, DecoderConfig, FieldRef, string_to_time_duration_hook};
use nh_hugofs::afero::{Fs, OsFs};

use super::filecache::{Cache, Caches};

const RESOURCES_GEN_DIR: &str = ":resourceDir/_gen";
const CACHE_DIR_PROJECT: &str = ":cacheDir/:project";

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
    /// Max age of cache entries in this cache. A negative value means forever, 0 means the
    /// cache is disabled (Go `time.Duration`).
    pub max_age: Duration,
    /// The directory where files are stored, e.g. `:cacheDir/:project` (getresource),
    /// `:resourceDir/_gen` (images, assets).
    pub dir: String,
    pub dir_compiled: String,
    /// Will resources/_gen will get its own composite filesystem that also checks any theme.
    pub is_resource_dir: bool,
}

nh_config::decode_struct!(FileCacheConfig, "filecache.FileCacheConfig", |s| vec![
    FieldRef::new("MaxAge", &mut s.max_age),
    FieldRef::new("Dir", &mut s.dir),
    FieldRef::new("DirCompiled", &mut s.dir_compiled),
    FieldRef::new("IsResourceDir", &mut s.is_resource_dir),
]);

// Go: cache/filecache/filecache_config.go:defaultCacheConfig
fn default_cache_config() -> FileCacheConfig {
    FileCacheConfig {
        max_age: Duration(-1), // Never expire
        dir: CACHE_DIR_PROJECT.to_string(),
        ..Default::default()
    }
}

/// Go: `filecache.Configs`.
pub type Configs = BTreeMap<String, FileCacheConfig>;

/// Go: `Configs.CacheDirModules()` (for internal use).
// Go: cache/filecache/filecache_config.go:CacheDirModules
pub fn cache_dir_modules(c: &Configs) -> String {
    c.get(CACHE_KEY_MODULES)
        .map(|v| v.dir_compiled.clone())
        .unwrap_or_default()
}

// Go: cache/filecache/filecache_config.go:defaultCacheConfigs
fn default_cache_configs() -> Configs {
    let mut c = Configs::new();
    c.insert(
        CACHE_KEY_MODULES.to_string(),
        FileCacheConfig {
            max_age: Duration(-1),
            dir: ":cacheDir/modules".to_string(),
            ..Default::default()
        },
    );
    c.insert(CACHE_KEY_GET_JSON.to_string(), default_cache_config());
    c.insert(CACHE_KEY_GET_CSV.to_string(), default_cache_config());
    c.insert(
        CACHE_KEY_IMAGES.to_string(),
        FileCacheConfig {
            max_age: Duration(-1),
            dir: RESOURCES_GEN_DIR.to_string(),
            ..Default::default()
        },
    );
    c.insert(
        CACHE_KEY_ASSETS.to_string(),
        FileCacheConfig {
            max_age: Duration(-1),
            dir: RESOURCES_GEN_DIR.to_string(),
            ..Default::default()
        },
    );
    c.insert(
        CACHE_KEY_GET_RESOURCE.to_string(),
        FileCacheConfig {
            max_age: Duration(-1), // Never expire
            dir: CACHE_DIR_PROJECT.to_string(),
            ..Default::default()
        },
    );
    c.insert(
        CACHE_KEY_MISC.to_string(),
        FileCacheConfig {
            max_age: Duration(-1),
            dir: CACHE_DIR_PROJECT.to_string(),
            ..Default::default()
        },
    );
    c
}

impl Caches {
    /// Go: `GetJSONCache()` — the file cache for getJSON.
    // Go: cache/filecache/filecache_config.go:GetJSONCache
    pub fn get_json_cache(&self) -> Option<Arc<Cache>> {
        self.0.get(CACHE_KEY_GET_JSON).cloned()
    }

    /// Go: `GetCSVCache()` — the file cache for getCSV.
    // Go: cache/filecache/filecache_config.go:GetCSVCache
    pub fn get_csv_cache(&self) -> Option<Arc<Cache>> {
        self.0.get(CACHE_KEY_GET_CSV).cloned()
    }

    /// Go: `ImageCache()` — the file cache for processed images.
    // Go: cache/filecache/filecache_config.go:ImageCache
    pub fn image_cache(&self) -> Option<Arc<Cache>> {
        self.0.get(CACHE_KEY_IMAGES).cloned()
    }

    /// Go: `ModulesCache()` — the file cache for Hugo Modules.
    // Go: cache/filecache/filecache_config.go:ModulesCache
    pub fn modules_cache(&self) -> Option<Arc<Cache>> {
        self.0.get(CACHE_KEY_MODULES).cloned()
    }

    /// Go: `AssetsCache()` — the file cache for assets (processed resources, SCSS etc.).
    // Go: cache/filecache/filecache_config.go:AssetsCache
    pub fn assets_cache(&self) -> Option<Arc<Cache>> {
        self.0.get(CACHE_KEY_ASSETS).cloned()
    }

    /// Go: `MiscCache()` — the file cache for miscellaneous stuff.
    // Go: cache/filecache/filecache_config.go:MiscCache
    pub fn misc_cache(&self) -> Option<Arc<Cache>> {
        self.0.get(CACHE_KEY_MISC).cloned()
    }

    /// Go: `GetResourceCache()` — the file cache for remote resources.
    // Go: cache/filecache/filecache_config.go:GetResourceCache
    pub fn get_resource_cache(&self) -> Option<Arc<Cache>> {
        self.0.get(CACHE_KEY_GET_RESOURCE).cloned()
    }
}

/// Go: `filecache.DecodeConfig(fs, bcfg, m)` (`:project` = basename(workingDir) — the golden
/// site dir MUST be named `seeksnack` so the getresource cache is found).
///
/// Go ranges over the input map and the config map in random order: with several invalid
/// entries the error reported is random; the port uses byte order.
// Go: cache/filecache/filecache_config.go:DecodeConfig
pub fn decode_config(fs: &dyn Fs, bcfg: &BaseConfig, m: &Map) -> Result<Configs> {
    let mut c = Configs::new();
    let mut valid: BTreeMap<String, bool> = BTreeMap::new();
    // Add defaults
    for (k, v) in default_cache_configs() {
        valid.insert(k.clone(), true);
        c.insert(k, v);
    }

    let is_os_fs = fs.as_any().is::<OsFs>();

    for (k, v) in &m.entries {
        let is_params = matches!(v, Value::Map(mm) if mm.ty == MapType::Params);
        if !is_params {
            continue;
        }
        let mut cc = default_cache_config();

        let hook = string_to_time_duration_hook;
        let decoder = Decoder::new(DecoderConfig {
            decode_hook: Some(&hook),
            weakly_typed_input: true,
            ..Default::default()
        });

        if let Err(e) = decoder.decode_input(v, &mut cc) {
            let e = Error::from(e);
            return Err(Error::new(format!(
                "failed to decode filecache config: {}",
                e.message()
            )));
        }

        if cc.dir.is_empty() {
            return Err(Error::new("must provide cache Dir"));
        }

        let name =
            go_unicode::strings::to_lower_str(&String::from_utf8_lossy(k.as_bytes())).into_owned();
        if !valid.get(&name).copied().unwrap_or(false) {
            return Err(Error::new(format!(
                "{} is not a valid cache name",
                go_strconv::quote(name.as_bytes())
            )));
        }

        c.insert(name, cc);
    }

    let keys: Vec<String> = c.keys().cloned().collect();
    for k in keys {
        let mut v = c[&k].clone();
        let dir = filepath::to_slash(&filepath::clean(&v.dir)).to_string();
        let had_slash = dir.starts_with('/');
        let mut parts: Vec<String> = dir.split('/').map(str::to_string).collect();

        for part in parts.iter_mut() {
            if part.starts_with(':') {
                let (resolved, is_resource) = resolve_dir_placeholder(fs, bcfg, part)?;
                if is_resource {
                    v.is_resource_dir = true;
                }
                *part = resolved;
            }
        }

        let mut dir = path::join(&parts);
        if had_slash {
            dir.insert(0, '/');
        }
        v.dir_compiled = filepath::clean(filepath::from_slash(&dir));

        if !v.is_resource_dir {
            if is_os_fs && !filepath::is_abs(&v.dir_compiled) {
                return Err(Error::new(format!(
                    "{} must resolve to an absolute directory",
                    go_strconv::quote(v.dir_compiled.as_bytes())
                )));
            }

            // Avoid cache in root, e.g. / (Unix) or c:\ (Windows)
            // (filepath.VolumeName is "" on unix.)
            if v.dir_compiled.len() == 1 {
                return Err(Error::new(format!(
                    "{} is a root folder and not allowed as cache dir",
                    go_strconv::quote(v.dir_compiled.as_bytes())
                )));
            }
        }

        if !v.dir_compiled.starts_with("_gen") {
            // We do cache eviction (file removes) and since the user can set
            // his/hers own cache directory, we really want to make sure
            // we do not delete any files that do not belong to this cache.
            // We do add the cache name as the root, but this is an extra safe
            // guard. We skip the files inside /resources/_gen/ because
            // that would be breaking.
            v.dir_compiled = filepath::join(&[
                v.dir_compiled.as_str(),
                super::filecache::FILECACHE_ROOT_DIRNAME,
                k.as_str(),
            ]);
        } else {
            v.dir_compiled = filepath::join(&[v.dir_compiled.as_str(), k.as_str()]);
        }

        c.insert(k, v);
    }

    Ok(c)
}

/// Resolves `:resourceDir` => /myproject/resources etc., `:cacheDir` => ...
// Go: cache/filecache/filecache_config.go:resolveDirPlaceholder
fn resolve_dir_placeholder(
    _fs: &dyn Fs,
    bcfg: &BaseConfig,
    placeholder: &str,
) -> Result<(String, bool)> {
    match go_unicode::strings::to_lower_str(placeholder).as_ref() {
        ":resourcedir" => return Ok((String::new(), true)),
        ":cachedir" => return Ok((bcfg.cache_dir.clone(), false)),
        ":project" => return Ok((filepath::base(&bcfg.working_dir).to_string(), false)),
        _ => {}
    }

    Err(Error::new(format!(
        "{} is not a valid placeholder (valid values are :cacheDir or :resourceDir)",
        go_strconv::quote(placeholder.as_bytes())
    )))
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: cache/filecache/filecache_config.go (247 lines; 8/10 funcs executed)
//   types: Configs, FileCacheConfig
// OK L55-57: (c Configs) CacheDirModules() string
// OK L104-106: (f Caches) GetJSONCache() *Cache
// OK L109-111: (f Caches) GetCSVCache() *Cache
// OK L114-116: (f Caches) ImageCache() *Cache
// OK L119-121: (f Caches) ModulesCache() *Cache
// OK L124-126: (f Caches) AssetsCache() *Cache
// OK L129-131: (f Caches) MiscCache() *Cache
// OK L134-136: (f Caches) GetResourceCache() *Cache
// OK L138-233: DecodeConfig(fs afero.Fs, bcfg config.BaseConfig, m map[string]any) (Configs, error)
// OK L236-247: resolveDirPlaceholder(fs afero.Fs, bcfg config.BaseConfig, placeholder string) (cacheDir string, isResource bool, err error)
// ---------------------------------------------------------------------------
