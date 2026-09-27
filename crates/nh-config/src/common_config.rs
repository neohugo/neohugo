//! Port of `config/commonConfig.go`.
//!
//! Owner: Wave B task T04 (config-base-media).


use go_value::Map;
use nh_common::Result;

use crate::config_provider::Provider;

/// Go: `config.BaseConfig`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BaseConfig {
    pub working_dir: String,
    pub cache_dir: String,
    pub themes_dir: String,
    pub publish_dir: String,
}

/// Go: `config.CommonDirs`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CommonDirs {
    pub themes_dir: String,
    pub publish_dir: String,
    pub resource_dir: String,
    pub working_dir: String,
    pub cache_dir: String,
    pub content_dir: String,
    pub data_dir: String,
    pub layout_dir: String,
    pub i18n_dir: String,
    pub arche_type_dir: String,
    pub asset_dir: String,
}

/// Go: `config.LoadConfigResult`.
pub struct LoadConfigResult {
    pub cfg: std::sync::Arc<dyn Provider>,
    pub config_files: Vec<String>,
    pub base_config: BaseConfig,
}

/// Go: `config.BuildStats` (`writeStats=true` -> `Enable`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BuildStats {
    pub enable: bool,
    pub disable_tags: bool,
    pub disable_classes: bool,
    pub disable_ids: bool,
}

impl BuildStats {
    // Go: config/commonConfig.go:Enabled
    pub fn enabled(&self) -> bool {
        self.enable && !(self.disable_tags && self.disable_classes && self.disable_ids)
    }
}

/// Go: `config.CacheBuster`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CacheBuster {
    pub source: String,
    pub target: String,
}

/// Go: `config.BuildConfig`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BuildConfig {
    /// "fallback" (default), "always", "never".
    pub use_resource_cache_when: String,
    pub build_stats: BuildStats,
    pub no_js_config_in_assets: bool,
    pub cache_busters: Vec<CacheBuster>,
}

impl BuildConfig {
    /// Go: `UseResourceCache(err)` — whether to read the transformed-resource file cache.
    // Go: config/commonConfig.go:UseResourceCache
    pub fn use_resource_cache(&self, err: Option<&nh_common::Error>) -> bool {
        todo!()
    }
}

/// Go: `config.DecodeBuildConfig(cfg)` (legacy bool `writeStats` -> `buildStats.enable`).
// Go: config/commonConfig.go:DecodeBuildConfig
pub fn decode_build_config(cfg: &dyn Provider) -> BuildConfig {
    todo!()
}

/// Go: `config.SitemapConfig`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SitemapConfig {
    pub change_freq: String,
    /// -1 = unset.
    pub priority: f64,
    pub filename: String,
    pub disable: bool,
}

/// `config.SitemapConfig` as a template value (`.Sitemap.ChangeFreq`, `.Sitemap.Priority`: 1,730
/// calls in sitemap.xml). Go returns the struct BY VALUE (`Sitemap() config.SitemapConfig`), so
/// the value is a `Kind::Struct` with exported fields and no methods. nh-hugolib's page method
/// table wraps `Page::sitemap()` with `Value::object(sitemap_config)`.
impl go_value::Object for SitemapConfig {
    fn type_name(&self) -> std::borrow::Cow<'_, str> {
        std::borrow::Cow::Borrowed("config.SitemapConfig")
    }
    fn kind(&self) -> go_value::Kind {
        go_value::Kind::Struct
    }
    fn has_method(&self, _name: &str) -> bool {
        false
    }
    fn call_method(&self, _ctx: go_value::HostCtx<'_>, _name: &str, _args: &[go_value::Value]) -> Option<go_value::Result<go_value::Value>> {
        None
    }
    /// Exported fields (exact case): ChangeFreq string, Priority float64, Filename string,
    /// Disable bool.
    fn field(&self, name: &str) -> Option<go_value::Value> {
        match name {
            "ChangeFreq" => Some(go_value::Value::string(self.change_freq.as_str())),
            "Priority" => Some(go_value::Value::float64(self.priority)),
            "Filename" => Some(go_value::Value::string(self.filename.as_str())),
            "Disable" => Some(go_value::Value::Bool(self.disable)),
            _ => None,
        }
    }
    /// For `%v`/`%+v` printing (declaration order of config/commonConfig.go).
    fn struct_fields(&self) -> Option<Vec<(std::borrow::Cow<'_, str>, go_value::Value)>> {
        Some(vec![
            (std::borrow::Cow::Borrowed("ChangeFreq"), go_value::Value::string(self.change_freq.as_str())),
            (std::borrow::Cow::Borrowed("Priority"), go_value::Value::float64(self.priority)),
            (std::borrow::Cow::Borrowed("Filename"), go_value::Value::string(self.filename.as_str())),
            (std::borrow::Cow::Borrowed("Disable"), go_value::Value::Bool(self.disable)),
        ])
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Go: `config.DecodeSitemap(prototype, input)` — merge front matter/site `sitemap` maps.
// Go: config/commonConfig.go:DecodeSitemap
pub fn decode_sitemap(prototype: SitemapConfig, input: &Map) -> Result<SitemapConfig> {
    todo!()
}

/// Go: `config.Pagination`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Pagination {
    pub pager_size: i64,
    pub path: String,
    pub disable_aliases: bool,
}

/// Go: `config.PageConfig` (next/prev sort orders).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PageConfig {
    pub next_prev_sort_order: String,
    pub next_prev_in_section_sort_order: String,
}

/// Go: `config.Server` (server-only; decoded for `hugo config` parity only).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Server {
    pub headers: Vec<Map>,
    pub redirects: Vec<Map>,
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: config/commonConfig.go (512 lines; 10/15 funcs executed)
//   types: BaseConfig, CommonDirs, LoadConfigResult, BuildConfig, BuildStats, SitemapConfig, Server, redirect,
//          Headers, Redirect, CacheBuster, Pagination, PageConfig
// EX L125-130: (w BuildStats) Enabled() bool
// EX L132-135: (b BuildConfig) clone() BuildConfig
// EX L137-147: (b BuildConfig) UseResourceCache(err error) bool
//    L150-168: (s BuildConfig) MatchCacheBuster(logger loggers.Logger, p string) (func(string) bool, error)
// EX L170-178: (b *BuildConfig) CompileConfig(logger loggers.Logger) error
// EX L180-207: DecodeBuildConfig(cfg Provider) BuildConfig
// EX L221-224: DecodeSitemap(prototype SitemapConfig, input map[string]any) (SitemapConfig, error)
//    L241-248: (r redirect) matchHeader(header http.Header) bool
// EX L250-293: (s *Server) CompileConfig(logger loggers.Logger) error
//    L295-316: (s *Server) MatchHeaders(pattern string) []types.KeyValueStr
//    L318-359: (s *Server) MatchRedirect(pattern string, header http.Header) Redirect
// EX L404-452: (c *CacheBuster) CompileConfig(logger loggers.Logger) error
//    L454-456: (r Redirect) IsZero() bool
// EX L463-485: DecodeServer(cfg Provider) (Server, error)
// EX L508-512: (c *PageConfig) CompileConfig(loggers.Logger) error
// ---------------------------------------------------------------------------
