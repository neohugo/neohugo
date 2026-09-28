//! Port of `resources/page/page_matcher.go`.
//!
//! Owner: Wave B task T11 (page-api-paths).

use go_value::{Map, MapType, Value};
use nh_common::Result;
use nh_common::herrors::Error;
use nh_common::kinds;
use nh_common::loggers::Logger;
use nh_common::maps::ordered::Ordered;
use nh_config::decode::FieldRef;
use nh_config::namespace::ConfigNamespace;

use crate::page::Page;

/// Go: `page.PageMatcher` (cascade `_target` / `target`). The pattern matching is case
/// insensitive.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct PageMatcher {
    /// A Glob pattern matching the content path below /content.
    pub path: String,
    /// A Glob pattern matching the Page's Kind(s).
    pub kind: String,
    /// A Glob pattern matching the Page's language.
    pub lang: String,
    /// A Glob pattern matching the Page's Environment.
    pub environment: String,
}

nh_config::decode_struct!(PageMatcher, "page.PageMatcher", |s| vec![
    FieldRef::new("Path", &mut s.path),
    FieldRef::new("Kind", &mut s.kind),
    FieldRef::new("Lang", &mut s.lang),
    FieldRef::new("Environment", &mut s.environment),
]);

impl PageMatcher {
    /// Matches returns whether p matches this matcher. A glob that does not compile matches.
    // Go: resources/page/page_matcher.go:Matches
    pub fn matches(&self, p: &dyn Page) -> bool {
        if !self.kind.is_empty()
            && let Ok(g) = nh_common::glob::glob::get_glob(&self.kind)
            && !g.matches(&p.kind())
        {
            return false;
        }

        if !self.lang.is_empty()
            && let Ok(g) = nh_common::glob::glob::get_glob(&self.lang)
            && !g.matches(&p.lang())
        {
            return false;
        }

        if !self.path.is_empty() {
            let g = nh_common::glob::glob::get_glob(&self.path);
            // TODO(bep) Path() vs filepath vs leading slash.
            let mut pth = go_unicode::strings::to_lower_str(go_path::filepath::to_slash(&p.path()))
                .into_owned();
            if !pth.starts_with('/') {
                pth = format!("/{pth}");
            }
            if let Ok(g) = g
                && !g.matches(&pth)
            {
                return false;
            }
        }

        if !self.environment.is_empty()
            && let Ok(g) = nh_common::glob::glob::get_glob(&self.environment)
            && !g.matches(&p.site().0.hugo().environment)
        {
            return false;
        }

        true
    }
}

/// Go: `page.PageMatcherParamsConfig`.
#[derive(Clone, Debug)]
pub struct PageMatcherParamsConfig {
    /// Apply Params to all Pages matching Target.
    pub params: Map,
    /// Fields holds all fields but Params.
    pub fields: Map,
    /// Target is the PageMatcher that this config applies to.
    pub target: PageMatcher,
}

impl PageMatcherParamsConfig {
    // Go: resources/page/page_matcher.go:init
    fn init(&mut self) -> Result<()> {
        nh_common::maps::params::prepare_params(&mut self.params);
        nh_common::maps::params::prepare_params(&mut self.fields);
        Ok(())
    }

    /// The source-structure form of the config (Go marshals the struct; used for printing the
    /// configuration only).
    fn to_value(&self) -> Value {
        let mut t = Map::new(MapType::StringAny);
        t.insert("Path", Value::string(self.target.path.as_str()));
        t.insert("Kind", Value::string(self.target.kind.as_str()));
        t.insert("Lang", Value::string(self.target.lang.as_str()));
        t.insert(
            "Environment",
            Value::string(self.target.environment.as_str()),
        );
        let mut m = Map::new(MapType::StringAny);
        m.insert("Params", Value::map(self.params.clone()));
        m.insert("Fields", Value::map(self.fields.clone()));
        m.insert("Target", Value::map(t));
        Value::map(m)
    }
}

/// Compiled cascade (Go `*maps.Ordered[PageMatcher, PageMatcherParamsConfig]`).
pub type Cascade = Ordered<PageMatcher, PageMatcherParamsConfig>;

/// These define the structure of the page tree and cannot currently be set in the cascade.
fn disallowed_cascade_key(k: &str) -> bool {
    matches!(k, "kind" | "path" | "lang")
}

/// See issue 11977.
// Go: resources/page/page_matcher.go:isGlobWithExtension
fn is_glob_with_extension(s: &str) -> bool {
    let last = s.rsplit('/').next().unwrap_or("");
    last.contains('.')
}

/// Go: `page.CheckCascadePattern(logger, m)`.
// Go: resources/page/page_matcher.go:CheckCascadePattern
pub fn check_cascade_pattern(logger: Option<&Logger>, m: &PageMatcher) {
    if let Some(logger) = logger
        && is_glob_with_extension(&m.path)
    {
        logger.erroridf(
            "cascade-pattern-with-extension",
            format!(
                "cascade target path {} looks like a path with an extension; since Hugo v0.123.0 this will not match anything, see  https://gohugo.io/methods/page/path/",
                go_strconv::quote(m.path.as_bytes())
            ),
        );
    }
}

/// Go: `page.DecodeCascadeConfig(logger, handleLegacyFormat, in)` without a logger.
// Go: resources/page/page_matcher.go:DecodeCascadeConfig
pub fn decode_cascade_config(
    handle_legacy_format: bool,
    input: &Value,
) -> Result<ConfigNamespace<Vec<PageMatcherParamsConfig>, Cascade>> {
    decode_cascade_config_with_logger(None, handle_legacy_format, input)
}

/// Go: `page.DecodeCascadeConfig(logger, handleLegacyFormat, in)`.
// Go: resources/page/page_matcher.go:DecodeCascadeConfig
pub fn decode_cascade_config_with_logger(
    logger: Option<&Logger>,
    _handle_legacy_format: bool,
    input: &Value,
) -> Result<ConfigNamespace<Vec<PageMatcherParamsConfig>, Cascade>> {
    let build_config = |input: &Value| -> Result<(Cascade, Option<Value>)> {
        let mut cascade: Cascade = Ordered::new();
        if input.is_invalid() {
            return Ok((
                cascade,
                Some(Value::list(go_value::SliceType::MapStringAny, Vec::new())),
            ));
        }
        let ms = nh_common::maps::maps::to_slice_string_map(input)?;

        let mut cfgs: Vec<PageMatcherParamsConfig> = Vec::new();

        for m in &ms {
            let m = nh_common::maps::params::clean_config_string_map(m);
            let c = map_to_page_matcher_params_config(&m)?;
            // Go ranges over the map (random order); the first disallowed key found is
            // reported. Byte order here.
            for k in m.entries.keys() {
                let k = String::from_utf8_lossy(k.as_bytes());
                if disallowed_cascade_key(&k) {
                    return Err(Error::new(format!(
                        "key {} not allowed in cascade config",
                        go_strconv::quote(k.as_bytes())
                    )));
                }
            }
            cfgs.push(c);
        }

        for cfg in &cfgs {
            let m = cfg.target.clone();
            check_cascade_pattern(logger, &m);
            match cascade.get(&m).cloned() {
                Some(mut c) => {
                    // Merge (Go mutates the stored maps in place).
                    for (k, v) in cfg.params.entries.iter() {
                        if !c.params.entries.contains_key(k) {
                            c.params.entries.insert(k.clone(), v.clone());
                        }
                    }
                    for (k, v) in cfg.fields.entries.iter() {
                        if !c.fields.entries.contains_key(k) {
                            c.fields.entries.insert(k.clone(), v.clone());
                        }
                    }
                    cascade.set(m, c);
                }
                None => {
                    cascade.set(m, cfg.clone());
                }
            }
        }

        let src = Value::any_list(cfgs.iter().map(|c| c.to_value()).collect());
        Ok((cascade, Some(src)))
    };

    nh_config::namespace::decode_namespace(input, build_config)
}

/// DecodeCascade decodes in which could be either a map or a slice of maps.
// Go: resources/page/page_matcher.go:DecodeCascade
pub fn decode_cascade(
    logger: Option<&Logger>,
    handle_legacy_format: bool,
    input: &Value,
) -> Result<Cascade> {
    let conf = decode_cascade_config_with_logger(logger, handle_legacy_format, input)?;
    Ok(conf.config)
}

// Go: resources/page/page_matcher.go:mapToPageMatcherParamsConfig
fn map_to_page_matcher_params_config(m: &Map) -> Result<PageMatcherParamsConfig> {
    let mut pcfg = PageMatcherParamsConfig {
        params: Map::new(MapType::Params),
        fields: Map::new(MapType::Params),
        target: PageMatcher::default(),
    };
    // Go ranges over the map in random order: with both `_target` and `target` the one seen
    // last wins, and `params` and `Params` merge first-wins. Byte order here.
    for (k, v) in m.entries.iter() {
        let kl = go_unicode::strings::to_lower(k.as_bytes());
        match &kl[..] {
            b"_target" | b"target" => {
                let mut target = PageMatcher::default();
                decode_page_matcher(v, &mut target)?;
                pcfg.target = target;
            }
            b"params" => {
                let params = nh_common::maps::maps::to_string_map(v);
                for (k, v) in params.entries.iter() {
                    if !pcfg.params.entries.contains_key(k) {
                        pcfg.params.entries.insert(k.clone(), v.clone());
                    }
                }
            }
            _ => {
                pcfg.fields.entries.insert(k.clone(), v.clone());
            }
        }
    }
    pcfg.init()?;
    Ok(pcfg)
}

/// decodePageMatcher decodes m into v.
// Go: resources/page/page_matcher.go:decodePageMatcher
fn decode_page_matcher(m: &Value, v: &mut PageMatcher) -> Result<()> {
    nh_config::decode::weak_decode_into(m, v)?;

    v.kind = go_unicode::strings::to_lower_str(&v.kind).into_owned();
    if !v.kind.is_empty() {
        let found = match nh_common::glob::glob::get_glob(&v.kind) {
            Ok(g) => kinds::ALL_KINDS_IN_PAGES.iter().any(|k| g.matches(k)),
            // Go ignores the error and calls Match on a nil glob, which panics; the port
            // returns Go's panic message as an error.
            Err(_) => {
                return Err(Error::new(
                    "runtime error: invalid memory address or nil pointer dereference",
                ));
            }
        };
        if !found {
            return Err(Error::new(format!(
                "{} did not match a valid Page Kind",
                go_strconv::quote(v.kind.as_bytes())
            )));
        }
    }

    v.path = go_path::filepath::to_slash(&go_unicode::strings::to_lower_str(&v.path)).to_string();

    Ok(())
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/page/page_matcher.go (238 lines; 1/8 funcs executed)
//   types: PageMatcher, PageMatcherParamsConfig
// OK L50-85: (m PageMatcher) Matches(p Page) bool
// OK L96-100: isGlobWithExtension(s string) bool
// OK L102-106: CheckCascadePattern(logger loggers.Logger, m PageMatcher)
// OK L108-164: DecodeCascadeConfig(logger loggers.Logger, handleLegacyFormat bool, in any) (*config.ConfigNamespace[[]PageMatcherParamsConfig, *maps.Ordered[PageM...
// OK L167-173: DecodeCascade(logger loggers.Logger, handleLegacyFormat bool, in any) (*maps.Ordered[PageMatcher, PageMatcherParamsConfig], error)
// OK L175-203: mapToPageMatcherParamsConfig(m map[string]any) (PageMatcherParamsConfig, error)
// OK L206-223: decodePageMatcher(m any, v *PageMatcher) error
// OK L234-238: (p *PageMatcherParamsConfig) init() error
// ---------------------------------------------------------------------------
