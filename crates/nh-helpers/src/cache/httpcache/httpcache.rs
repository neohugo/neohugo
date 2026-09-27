//! Port of `cache/httpcache/httpcache.go`.
//!
//! Owner: Wave B task T08 (helpers-source-cache).


use nh_common::Result;

/// Go: `httpcache.Config` (default: `cache.for.excludes = ["**"]` -> never revalidate cached responses).
#[derive(Clone, Debug, Default)]
pub struct Config {
    pub cache_for_includes: Vec<String>,
    pub cache_for_excludes: Vec<String>,
    pub polls: Vec<PollConfig>,
}

/// Go: `httpcache.PollConfig`.
#[derive(Clone, Debug, Default)]
pub struct PollConfig {
    pub for_includes: Vec<String>,
    pub for_excludes: Vec<String>,
    pub disable: bool,
    pub low: std::time::Duration,
    pub high: std::time::Duration,
}

/// Go: `httpcache.ConfigCompiled` (`GetConfigSection("httpCacheCompiled")`).
#[derive(Clone)]
pub struct ConfigCompiled {
    /// Go `For func(string) bool` — whether a URL's cached response should be revalidated.
    pub for_: std::sync::Arc<dyn Fn(&str) -> bool + Send + Sync>,
}

impl Config {
    // Go: cache/httpcache/httpcache.go:Compile
    pub fn compile(&self) -> Result<ConfigCompiled> {
        todo!()
    }
}

/// Go: `httpcache.DecodeConfig(bcfg, m)`.
// Go: cache/httpcache/httpcache.go:DecodeConfig
pub fn decode_config(m: &go_value::Map) -> Result<Config> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: cache/httpcache/httpcache.go (229 lines; 4/8 funcs executed)
//   types: Config, Cache, PollConfig, GlobMatcher, ConfigCompiled, PollConfigCompiled
// EX L59-83: (c *Config) Compile() (ConfigCompiled, error)
//    L103-115: (c PollConfig) MarshalJSON() (b []byte, err error)
// EX L125-127: (gm GlobMatcher) IsZero() bool
//    L134-141: (c *ConfigCompiled) PollConfigFor(s string) PollConfigCompiled
//    L143-150: (c *ConfigCompiled) IsPollingDisabled() bool
//    L157-159: (p PollConfigCompiled) IsZero() bool
// EX L161-189: (gm *GlobMatcher) CompilePredicate() (func(string) bool, error)
// EX L191-229: DecodeConfig(_ config.BaseConfig, m map[string]any) (Config, error)
// ---------------------------------------------------------------------------
