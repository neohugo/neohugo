//! Port of `cache/httpcache/httpcache.go`.
//!
//! Owner: Wave B task T08 (helpers-source-cache).

use std::sync::Arc;

use go_time::Duration;
use go_value::{Map, Value};
use nh_common::Result;
use nh_common::herrors::Error;
use nh_common::predicate::{self, P};
use nh_config::decode::{Decoder, DecoderConfig, FieldRef, string_to_time_duration_hook};

/// Go: `httpcache.Config` (default: `Cache.For.Excludes = ["**"]` -> cached responses are never
/// revalidated).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Config {
    /// Configures the HTTP cache behavior (RFC 9111). When this is not enabled for a resource,
    /// Hugo will go straight to the file cache.
    pub cache: Cache,
    /// Polling configurations for remote resources (watch mode).
    pub polls: Vec<PollConfig>,
}

nh_config::decode_struct!(Config, "httpcache.Config", |s| vec![
    FieldRef::new("Cache", &mut s.cache),
    FieldRef::new("Polls", &mut s.polls),
]);

/// Go: `httpcache.Cache`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Cache {
    /// Enable HTTP cache behavior (RFC 9111) for these resources.
    pub for_: GlobMatcher,
}

nh_config::decode_struct!(Cache, "httpcache.Cache", |s| vec![FieldRef::new(
    "For",
    &mut s.for_
)]);

/// Go: `httpcache.PollConfig`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PollConfig {
    /// What remote resources to apply this configuration to.
    pub for_: GlobMatcher,
    /// Disable polling for this configuration.
    pub disable: bool,
    /// The lower bound for the polling interval.
    pub low: Duration,
    /// The upper bound for the polling interval.
    pub high: Duration,
}

nh_config::decode_struct!(PollConfig, "httpcache.PollConfig", |s| vec![
    FieldRef::new("For", &mut s.for_),
    FieldRef::new("Disable", &mut s.disable),
    FieldRef::new("Low", &mut s.low),
    FieldRef::new("High", &mut s.high),
]);

impl PollConfig {
    /// Go: `PollConfig.MarshalJSON()` — the durations as strings, then the other fields
    /// (`{"Low":"0s","High":"0s","For":{...},"Disable":false}`).
    // Go: cache/httpcache/httpcache.go:MarshalJSON
    pub fn marshal_json(&self) -> String {
        let strs = |v: &[String]| -> String {
            if v.is_empty() {
                return "null".to_string();
            }
            let items: Vec<String> = v.iter().map(|s| json_string(s)).collect();
            format!("[{}]", items.join(","))
        };
        format!(
            "{{\"Low\":{},\"High\":{},\"For\":{{\"Excludes\":{},\"Includes\":{}}},\"Disable\":{}}}",
            json_string(&self.low.string()),
            json_string(&self.high.string()),
            strs(&self.for_.excludes),
            strs(&self.for_.includes),
            self.disable
        )
    }
}

/// A JSON string with Go's `encoding/json` escaping (HTML characters escaped).
fn json_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '<' | '>' | '&' | '\u{2028}' | '\u{2029}' => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Go: `httpcache.GlobMatcher`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GlobMatcher {
    /// Glob patterns that will be excluded.
    pub excludes: Vec<String>,
    /// Glob patterns that will be included.
    pub includes: Vec<String>,
}

nh_config::decode_struct!(GlobMatcher, "httpcache.GlobMatcher", |s| vec![
    FieldRef::new("Excludes", &mut s.excludes),
    FieldRef::new("Includes", &mut s.includes),
]);

impl GlobMatcher {
    // Go: cache/httpcache/httpcache.go:IsZero
    pub fn is_zero(&self) -> bool {
        self.includes.is_empty() && self.excludes.is_empty()
    }

    /// Go: `CompilePredicate()` — includes are OR-ed, excludes AND-ed (negated). Panics like Go
    /// when there is neither.
    // Go: cache/httpcache/httpcache.go:CompilePredicate
    pub fn compile_predicate(&self) -> Result<P<String>> {
        if self.is_zero() {
            panic!("no includes or excludes");
        }
        let mut p: Option<P<String>> = None;
        for include in &self.includes {
            let g = nh_common::glob::gobwas::compile(include.as_bytes(), &['/' as i32])
                .map_err(Error::new)?;
            let g = Arc::new(g);
            let f: P<String> = Arc::new(move |s: &String| g.is_match(s.as_bytes()));
            p = Some(predicate::or(p, vec![f]));
        }

        for exclude in &self.excludes {
            let g = nh_common::glob::gobwas::compile(exclude.as_bytes(), &['/' as i32])
                .map_err(Error::new)?;
            let g = Arc::new(g);
            let f: P<String> = Arc::new(move |s: &String| !g.is_match(s.as_bytes()));
            p = Some(predicate::and(p, vec![f]));
        }

        Ok(p.expect("includes or excludes"))
    }
}

/// Go: `httpcache.ConfigCompiled` (`GetConfigSection("httpCacheCompiled")`).
#[derive(Clone)]
pub struct ConfigCompiled {
    /// Go `For predicate.P[string]` — whether a URL's cached response should be revalidated
    /// (RFC 9111 behaviour); `AlwaysUseCachedResponse` is its negation.
    pub for_: P<String>,
    pub poll_configs: Vec<PollConfigCompiled>,
}

/// Go: `httpcache.PollConfigCompiled`.
#[derive(Clone, Default)]
pub struct PollConfigCompiled {
    /// `None` = Go's nil predicate (the zero value).
    pub for_: Option<P<String>>,
    pub config: PollConfig,
}

impl PollConfigCompiled {
    // Go: cache/httpcache/httpcache.go:(PollConfigCompiled).IsZero
    pub fn is_zero(&self) -> bool {
        self.for_.is_none()
    }
}

impl ConfigCompiled {
    /// Go: `PollConfigFor(s)` — the first poll config matching `s`, else the zero value.
    // Go: cache/httpcache/httpcache.go:PollConfigFor
    pub fn poll_config_for(&self, s: &str) -> PollConfigCompiled {
        let s = s.to_string();
        for pc in &self.poll_configs {
            if let Some(f) = &pc.for_
                && f(&s)
            {
                return pc.clone();
            }
        }
        PollConfigCompiled::default()
    }

    // Go: cache/httpcache/httpcache.go:IsPollingDisabled
    pub fn is_polling_disabled(&self) -> bool {
        for pc in &self.poll_configs {
            if !pc.config.disable {
                return false;
            }
        }
        true
    }
}

impl Config {
    // Go: cache/httpcache/httpcache.go:Compile
    pub fn compile(&self) -> Result<ConfigCompiled> {
        let p = self.cache.for_.compile_predicate()?;

        let mut poll_configs = Vec::new();
        for pc in &self.polls {
            let p = pc.for_.compile_predicate()?;

            poll_configs.push(PollConfigCompiled {
                for_: Some(p),
                config: pc.clone(),
            });
        }

        Ok(ConfigCompiled {
            for_: p,
            poll_configs,
        })
    }
}

/// Go: `httpcache.DefaultConfig`.
// Go: cache/httpcache/httpcache.go:DefaultConfig
pub fn default_config() -> Config {
    Config {
        cache: Cache {
            for_: GlobMatcher {
                excludes: vec!["**".to_string()],
                includes: Vec::new(),
            },
        },
        polls: vec![PollConfig {
            for_: GlobMatcher {
                includes: vec!["**".to_string()],
                excludes: Vec::new(),
            },
            disable: true,
            ..Default::default()
        }],
    }
}

/// Go: `httpcache.DecodeConfig(bcfg, m)`.
// Go: cache/httpcache/httpcache.go:DecodeConfig
pub fn decode_config(m: &Map) -> Result<Config> {
    if m.entries.is_empty() {
        return Ok(default_config());
    }

    let mut c = Config::default();

    let hook = string_to_time_duration_hook;
    let decoder = Decoder::new(DecoderConfig {
        decode_hook: Some(&hook),
        weakly_typed_input: true,
        ..Default::default()
    });

    decoder
        .decode_input(&Value::Map(Arc::new(m.clone())), &mut c)
        .map_err(Error::from)?;

    let dc = default_config();
    if c.cache.for_.is_zero() {
        c.cache.for_ = dc.cache.for_.clone();
    }

    for pc in c.polls.iter_mut() {
        if pc.for_.is_zero() {
            pc.for_ = dc.cache.for_.clone();
            pc.disable = true;
        }
    }

    if c.polls.is_empty() {
        c.polls = dc.polls;
    }

    Ok(c)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: cache/httpcache/httpcache.go (229 lines; 4/8 funcs executed)
//   types: Config, Cache, PollConfig, GlobMatcher, ConfigCompiled, PollConfigCompiled
// OK L59-83: (c *Config) Compile() (ConfigCompiled, error)
// OK L103-115: (c PollConfig) MarshalJSON() (b []byte, err error)
// OK L125-127: (gm GlobMatcher) IsZero() bool
// OK L134-141: (c *ConfigCompiled) PollConfigFor(s string) PollConfigCompiled
// OK L143-150: (c *ConfigCompiled) IsPollingDisabled() bool
// OK L157-159: (p PollConfigCompiled) IsZero() bool
// OK L161-189: (gm *GlobMatcher) CompilePredicate() (func(string) bool, error)
// OK L191-229: DecodeConfig(_ config.BaseConfig, m map[string]any) (Config, error)
// ---------------------------------------------------------------------------
