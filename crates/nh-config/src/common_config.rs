//! Port of `config/commonConfig.go`.
//!
//! Owner: Wave B task T04 (config-base-media).

use std::any::Any;
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::sync::Arc;

use go_value::{HostCtx, Kind, Map, MapType, Object, Value};
use nh_common::glob::gobwas::{self, Matcher};
use nh_common::{Error, Result};

use crate::config_provider::Provider;
use crate::decode::{FieldRef, weak_decode_into};
use crate::decode_struct;
use crate::goregexp::Regexp;

/// Go: `config.BaseConfig`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BaseConfig {
    pub working_dir: String,
    pub cache_dir: String,
    pub themes_dir: String,
    pub publish_dir: String,
}

decode_struct!(BaseConfig, "config.BaseConfig", |s| vec![
    FieldRef::new("WorkingDir", &mut s.working_dir),
    FieldRef::new("CacheDir", &mut s.cache_dir),
    FieldRef::new("ThemesDir", &mut s.themes_dir),
    FieldRef::new("PublishDir", &mut s.publish_dir),
]);

/// Go: `config.CommonDirs`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CommonDirs {
    /// The directory where Hugo will look for themes.
    pub themes_dir: String,
    /// Where to put the generated files.
    pub publish_dir: String,
    /// The directory to put the generated resources files.
    pub resource_dir: String,
    /// The project root directory.
    pub working_dir: String,
    /// The root directory for all cache files.
    pub cache_dir: String,
    /// The content source directory. Deprecated: Use module mounts.
    pub content_dir: String,
    /// The data source directory. Deprecated: Use module mounts.
    pub data_dir: String,
    /// The layout source directory. Deprecated: Use module mounts.
    pub layout_dir: String,
    /// The i18n source directory. Deprecated: Use module mounts.
    pub i18n_dir: String,
    /// The archetypes source directory. Deprecated: Use module mounts.
    pub arche_type_dir: String,
    /// The assets source directory. Deprecated: Use module mounts.
    pub asset_dir: String,
}

decode_struct!(CommonDirs, "config.CommonDirs", |s| vec![
    FieldRef::new("ThemesDir", &mut s.themes_dir),
    FieldRef::new("PublishDir", &mut s.publish_dir),
    FieldRef::new("ResourceDir", &mut s.resource_dir),
    FieldRef::new("WorkingDir", &mut s.working_dir),
    FieldRef::new("CacheDir", &mut s.cache_dir),
    FieldRef::new("ContentDir", &mut s.content_dir),
    FieldRef::new("DataDir", &mut s.data_dir),
    FieldRef::new("LayoutDir", &mut s.layout_dir),
    FieldRef::new("I18nDir", &mut s.i18n_dir),
    FieldRef::new("ArcheTypeDir", &mut s.arche_type_dir),
    FieldRef::new("AssetDir", &mut s.asset_dir),
]);

/// Go: `config.LoadConfigResult`.
pub struct LoadConfigResult {
    pub cfg: std::sync::Arc<dyn Provider>,
    pub config_files: Vec<String>,
    pub base_config: BaseConfig,
}

/// Go: `config.BuildStats` (`writeStats=true` -> `Enable`): configures if and what to write to
/// the hugo_stats.json file.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BuildStats {
    pub enable: bool,
    pub disable_tags: bool,
    pub disable_classes: bool,
    pub disable_ids: bool,
}

decode_struct!(BuildStats, "config.BuildStats", |s| vec![
    FieldRef::new("Enable", &mut s.enable),
    FieldRef::new("DisableTags", &mut s.disable_tags),
    FieldRef::new("DisableClasses", &mut s.disable_classes),
    FieldRef::new("DisableIDs", &mut s.disable_ids),
]);

/// `config.BuildStats` as a Go value (`m["buildstats"] = BuildStats{Enable: bb}` in
/// `DecodeBuildConfig`, assigned by mapstructure as it is).
impl Object for BuildStats {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("config.BuildStats")
    }
    fn kind(&self) -> Kind {
        Kind::Struct
    }
    fn has_method(&self, name: &str) -> bool {
        name == "Enabled"
    }
    fn call_method(
        &self,
        _ctx: HostCtx<'_>,
        name: &str,
        _args: &[Value],
    ) -> Option<go_value::Result<Value>> {
        match name {
            "Enabled" => Some(Ok(Value::Bool(self.enabled()))),
            _ => None,
        }
    }
    fn field(&self, name: &str) -> Option<Value> {
        self.struct_fields()?
            .into_iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v)
    }
    fn struct_fields(&self) -> Option<Vec<(Cow<'_, str>, Value)>> {
        Some(vec![
            (Cow::Borrowed("Enable"), Value::Bool(self.enable)),
            (Cow::Borrowed("DisableTags"), Value::Bool(self.disable_tags)),
            (
                Cow::Borrowed("DisableClasses"),
                Value::Bool(self.disable_classes),
            ),
            (Cow::Borrowed("DisableIDs"), Value::Bool(self.disable_ids)),
        ])
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl BuildStats {
    // Go: config/commonConfig.go:Enabled
    pub fn enabled(&self) -> bool {
        if !self.enable {
            return false;
        }
        !self.disable_tags || !self.disable_classes || !self.disable_ids
    }
}

/// The compiled source matcher of a [`CacheBuster`] (Go: `compiledSource`).
#[derive(Clone, Debug)]
struct CompiledCacheBuster {
    source_re: Regexp,
}

/// Go: `config.CacheBuster`: configures cache busting for assets.
#[derive(Clone, Debug, Default)]
pub struct CacheBuster {
    /// Trigger for files matching this regexp.
    pub source: String,
    /// Cache bust targets matching this regexp. This regexp can contain group matches (e.g.
    /// $1) from the source regexp.
    pub target: String,
    compiled: Option<Arc<CompiledCacheBuster>>,
}

impl PartialEq for CacheBuster {
    fn eq(&self, other: &Self) -> bool {
        self.source == other.source && self.target == other.target
    }
}

decode_struct!(CacheBuster, "config.CacheBuster", |s| vec![
    FieldRef::new("Source", &mut s.source),
    FieldRef::new("Target", &mut s.target),
]);

impl CacheBuster {
    /// A cache buster with the given source and target (not compiled).
    pub fn new(source: &str, target: &str) -> Self {
        CacheBuster {
            source: source.to_string(),
            target: target.to_string(),
            compiled: None,
        }
    }

    // Go: config/commonConfig.go:(*CacheBuster).CompileConfig
    pub fn compile_config(&mut self) -> Result<()> {
        if self.compiled.is_some() {
            return Ok(());
        }

        let source_re = Regexp::compile(&self.source).map_err(|e| {
            Error::new(format!(
                "failed to compile cache buster source {}: {}",
                go_strconv::quote(&self.source),
                e
            ))
        })?;
        self.compiled = Some(Arc::new(CompiledCacheBuster { source_re }));
        // Go also declares a compileErr that the target compilation assigns later (inside the
        // returned closure), after this function has returned it: it is always nil here.
        Ok(())
    }

    /// Go: `c.compiledSource(s)`: the target matcher for the file `s`, `None` when `s` does not
    /// match the source (or when the target regexp does not compile).
    fn compiled_source(&self, s: &str) -> Option<Regexp> {
        let compiled = self.compiled.as_ref()?;
        let m = compiled.source_re.find_string_submatch(s)?;
        let groups = &m[1..];
        let mut current_target = self.target.clone();
        // Replace $1, $2 etc. in target. (Go replaces in the original target on each
        // iteration, so only the last group's replacement is kept.)
        for (i, g) in groups.iter().enumerate() {
            current_target = self.target.replace(&format!("${}", i + 1), g);
        }
        Regexp::compile(&current_target).ok()
    }
}

/// Go: `config.defaultBuild`.
fn default_build() -> BuildConfig {
    BuildConfig {
        use_resource_cache_when: "fallback".to_string(),
        build_stats: BuildStats::default(),
        no_js_config_in_assets: false,
        cache_busters: vec![CacheBuster::new(
            r"(postcss|tailwind)\.config\.js",
            CSS_TARGET_CACHEBUSTER_RE,
        )],
    }
}

/// Keep this a little coarse grained, some false positives are OK.
const CSS_TARGET_CACHEBUSTER_RE: &str = "(css|styles|scss|sass)";

/// Go: `config.BuildConfig`: holds some build related configuration.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BuildConfig {
    /// When to use the resource file cache: "fallback" (default), "always", "never".
    pub use_resource_cache_when: String,
    /// When enabled, will collect and write a hugo_stats.json with some build related
    /// aggregated data (e.g. CSS class names). Note that this was a bool <= v0.115.0.
    pub build_stats: BuildStats,
    /// Can be used to toggle off writing of the IntelliSense /assets/jsconfig.js file.
    pub no_js_config_in_assets: bool,
    /// Can used to control how the resource cache gets evicted on rebuilds.
    pub cache_busters: Vec<CacheBuster>,
}

decode_struct!(BuildConfig, "config.BuildConfig", |s| vec![
    FieldRef::new("UseResourceCacheWhen", &mut s.use_resource_cache_when),
    FieldRef::new("BuildStats", &mut s.build_stats),
    FieldRef::new("NoJSConfigInAssets", &mut s.no_js_config_in_assets),
    FieldRef::new("CacheBusters", &mut s.cache_busters),
]);

impl BuildConfig {
    // Go: config/commonConfig.go:clone
    fn clone_config(&self) -> BuildConfig {
        self.clone()
    }

    /// Go: `UseResourceCache(err)` — whether to read the transformed-resource file cache.
    // Go: config/commonConfig.go:UseResourceCache
    pub fn use_resource_cache(&self, err: Option<&nh_common::Error>) -> bool {
        if self.use_resource_cache_when == "never" {
            return false;
        }

        if self.use_resource_cache_when == "fallback" {
            return err.is_some_and(nh_common::herrors::is_feature_not_available);
        }

        true
    }

    /// MatchCacheBuster returns the cache buster for the given path p, `None` if none.
    // Go: config/commonConfig.go:MatchCacheBuster
    pub fn match_cache_buster(&self, p: &str) -> Option<Box<dyn Fn(&str) -> bool + Send + Sync>> {
        let matchers: Vec<Regexp> = self
            .cache_busters
            .iter()
            .filter_map(|cb| cb.compiled_source(p))
            .collect();
        if matchers.is_empty() {
            return None;
        }
        Some(Box::new(move |cache_key: &str| {
            matchers.iter().any(|m| m.match_string(cache_key))
        }))
    }

    // Go: config/commonConfig.go:(*BuildConfig).CompileConfig
    pub fn compile_config(&mut self) -> Result<()> {
        for cb in self.cache_busters.iter_mut() {
            let source = cb.source.clone();
            cb.compile_config().map_err(|e| {
                Error::new(format!(
                    "failed to compile cache buster {}: {}",
                    go_strconv::quote(&source),
                    e
                ))
            })?;
        }
        Ok(())
    }
}

/// Go: `config.DecodeBuildConfig(cfg)` (legacy bool `writeStats` -> `buildStats.enable`).
// Go: config/commonConfig.go:DecodeBuildConfig
pub fn decode_build_config(cfg: &dyn Provider) -> BuildConfig {
    let mut m = cfg.get_string_map("build");

    let mut b = default_build().clone_config();

    // writeStats was a bool <= v0.115.0.
    if let Some(Value::Bool(bb)) = m.get(b"writestats").cloned() {
        m.insert(
            "buildstats",
            Value::object(BuildStats {
                enable: bb,
                ..Default::default()
            }),
        );
    }

    if weak_decode_into(&Value::map(m), &mut b).is_err() {
        return b;
    }

    b.use_resource_cache_when =
        go_unicode::strings::to_lower_str(&b.use_resource_cache_when).into_owned();
    let when = b.use_resource_cache_when.as_str();
    if when != "never" && when != "always" && when != "fallback" {
        b.use_resource_cache_when = "fallback".to_string();
    }

    b
}

/// Go: `config.SitemapConfig`: configures the sitemap to be generated.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SitemapConfig {
    /// The page change frequency.
    pub change_freq: String,
    /// The priority of the page (-1 = unset).
    pub priority: f64,
    /// The sitemap filename.
    pub filename: String,
    /// Whether to disable page inclusion.
    pub disable: bool,
}

decode_struct!(SitemapConfig, "config.SitemapConfig", |s| vec![
    FieldRef::new("ChangeFreq", &mut s.change_freq),
    FieldRef::new("Priority", &mut s.priority),
    FieldRef::new("Filename", &mut s.filename),
    FieldRef::new("Disable", &mut s.disable),
]);

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
    fn call_method(
        &self,
        _ctx: go_value::HostCtx<'_>,
        _name: &str,
        _args: &[go_value::Value],
    ) -> Option<go_value::Result<go_value::Value>> {
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
            (
                std::borrow::Cow::Borrowed("ChangeFreq"),
                go_value::Value::string(self.change_freq.as_str()),
            ),
            (
                std::borrow::Cow::Borrowed("Priority"),
                go_value::Value::float64(self.priority),
            ),
            (
                std::borrow::Cow::Borrowed("Filename"),
                go_value::Value::string(self.filename.as_str()),
            ),
            (
                std::borrow::Cow::Borrowed("Disable"),
                go_value::Value::Bool(self.disable),
            ),
        ])
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Go: `config.DecodeSitemap(prototype, input)` — merge front matter/site `sitemap` maps.
// Go: config/commonConfig.go:DecodeSitemap
pub fn decode_sitemap(prototype: SitemapConfig, input: &Map) -> Result<SitemapConfig> {
    let mut prototype = prototype;
    let r = weak_decode_into(&Value::map(input.clone()), &mut prototype);
    // Go returns the (partially) decoded prototype together with the error.
    r.map(|_| prototype)
}

/// [`decode_sitemap`] returning the partially decoded value along with the error, like Go.
pub fn decode_sitemap_partial(
    prototype: SitemapConfig,
    input: &Value,
) -> (SitemapConfig, Option<Error>) {
    let mut prototype = prototype;
    let r = weak_decode_into(input, &mut prototype);
    (prototype, r.err())
}

/// Go: `config.Headers`.
#[derive(Clone, Debug, PartialEq)]
pub struct Headers {
    pub for_: String,
    pub values: Map,
}

impl Default for Headers {
    fn default() -> Self {
        Headers {
            for_: String::new(),
            values: Map::new(MapType::StringAny),
        }
    }
}

decode_struct!(Headers, "config.Headers", |s| vec![
    FieldRef::new("For", &mut s.for_),
    FieldRef::new("Values", &mut s.values),
]);

/// Go: `config.Redirect`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Redirect {
    /// From is the Glob pattern to match. One of From or FromRe must be set.
    pub from: String,
    /// FromRe is the regexp to match. This regexp can contain group matches (e.g. $1) that can
    /// be used in the To field. One of From or FromRe must be set.
    pub from_re: String,
    /// To is the target URL.
    pub to: String,
    /// Headers to match for the redirect. This maps the HTTP header name to a Glob pattern
    /// with values to match. If the map is empty, the redirect will always be triggered.
    pub from_headers: BTreeMap<String, String>,
    /// HTTP status code to use for the redirect. A status code of 200 will trigger a URL
    /// rewrite.
    pub status: i64,
    /// Forcode redirect, even if original request path exists.
    pub force: bool,
}

decode_struct!(Redirect, "config.Redirect", |s| vec![
    FieldRef::new("From", &mut s.from),
    FieldRef::new("FromRe", &mut s.from_re),
    FieldRef::new("To", &mut s.to),
    FieldRef::new("FromHeaders", &mut s.from_headers),
    FieldRef::new("Status", &mut s.status),
    FieldRef::new("Force", &mut s.force),
]);

impl Redirect {
    // Go: config/commonConfig.go:(Redirect).IsZero
    pub fn is_zero(&self) -> bool {
        self.from.is_empty() && self.from_re.is_empty()
    }
}

/// Go: `config.redirect` (the compiled form).
#[derive(Clone, Debug)]
struct CompiledRedirect {
    from: Option<Matcher>,
    from_re: Option<Regexp>,
    headers: BTreeMap<String, Matcher>,
}

impl CompiledRedirect {
    // Go: config/commonConfig.go:(redirect).matchHeader
    fn match_header(&self, header: &dyn Fn(&str) -> String) -> bool {
        for (k, v) in &self.headers {
            if !v.is_match(header(k).as_bytes()) {
                return false;
            }
        }
        true
    }
}

/// Go: `config.Server` — config for the dev server (decoded for `hugo config` parity; the
/// compiled matchers serve `hugo server` only).
#[derive(Clone, Debug, Default)]
pub struct Server {
    pub headers: Vec<Headers>,
    pub redirects: Vec<Redirect>,
    compiled_headers: Option<Vec<Matcher>>,
    compiled_redirects: Vec<CompiledRedirect>,
}

impl PartialEq for Server {
    fn eq(&self, other: &Self) -> bool {
        self.headers == other.headers && self.redirects == other.redirects
    }
}

decode_struct!(Server, "config.Server", |s| vec![
    FieldRef::new("Headers", &mut s.headers),
    FieldRef::new("Redirects", &mut s.redirects),
]);

impl Server {
    // Go: config/commonConfig.go:(*Server).CompileConfig
    pub fn compile_config(&mut self) -> Result<()> {
        if self.compiled_headers.is_some() {
            return Ok(());
        }
        let mut compiled_headers = Vec::new();
        for h in &self.headers {
            let g = gobwas::compile(h.for_.as_bytes(), &[]).map_err(|e| {
                Error::new(format!(
                    "failed to compile Headers glob {}: {}",
                    go_strconv::quote(&h.for_),
                    e
                ))
            })?;
            compiled_headers.push(g);
        }
        if !compiled_headers.is_empty() {
            self.compiled_headers = Some(compiled_headers);
        }
        for r in &self.redirects {
            if r.from.is_empty() && r.from_re.is_empty() {
                return Err(Error::new("redirects must have either From or FromRe set"));
            }
            let mut rd = CompiledRedirect {
                from: None,
                from_re: None,
                headers: BTreeMap::new(),
            };
            if !r.from.is_empty() {
                let g = gobwas::compile(r.from.as_bytes(), &[]).map_err(|e| {
                    Error::new(format!(
                        "failed to compile Redirect glob {}: {}",
                        go_strconv::quote(&r.from),
                        e
                    ))
                })?;
                rd.from = Some(g);
            }
            if !r.from_re.is_empty() {
                let re = Regexp::compile(&r.from_re).map_err(|e| {
                    Error::new(format!(
                        "failed to compile Redirect regexp {}: {}",
                        go_strconv::quote(&r.from_re),
                        e
                    ))
                })?;
                rd.from_re = Some(re);
            }
            for (k, v) in &r.from_headers {
                let g = gobwas::compile(v.as_bytes(), &[]).map_err(|e| {
                    Error::new(format!(
                        "failed to compile Redirect header glob {}: {}",
                        go_strconv::quote(v),
                        e
                    ))
                })?;
                rd.headers.insert(k.clone(), g);
            }
            self.compiled_redirects.push(rd);
        }

        Ok(())
    }

    /// Go: `MatchHeaders(pattern)`: the header key/values of every matching `Headers` entry,
    /// sorted by key (`sort.Slice`, unstable for equal keys).
    // Go: config/commonConfig.go:(*Server).MatchHeaders
    pub fn match_headers(&self, pattern: &str) -> Vec<(String, String)> {
        let Some(compiled) = &self.compiled_headers else {
            return Vec::new();
        };

        let mut matches: Vec<(String, String)> = Vec::new();

        for (i, g) in compiled.iter().enumerate() {
            if g.is_match(pattern.as_bytes()) {
                let h = &self.headers[i];
                for (k, v) in &h.values.entries {
                    matches.push((
                        String::from_utf8_lossy(k).into_owned(),
                        String::from_utf8_lossy(&nh_common::cast::caste::to_string(v)).into_owned(),
                    ));
                }
            }
        }

        go_sort::sort_by(&mut matches, |a, b| a.0 < b.0);

        matches
    }

    /// Go: `MatchRedirect(pattern, header)`; `header` is `http.Header.Get` (`None` = a nil
    /// header).
    // Go: config/commonConfig.go:(*Server).MatchRedirect
    pub fn match_redirect(
        &self,
        pattern: &str,
        header: Option<&dyn Fn(&str) -> String>,
    ) -> Redirect {
        if self.compiled_redirects.is_empty() {
            return Redirect::default();
        }

        let pattern = pattern.strip_suffix("index.html").unwrap_or(pattern);

        for (i, r) in self.compiled_redirects.iter().enumerate() {
            let mut redir = self.redirects[i].clone();

            let mut found = false;

            if let Some(from) = &r.from
                && from.is_match(pattern.as_bytes())
            {
                found = header.is_none_or(|h| r.match_header(h));
                // We need to do regexp group replacements if needed.
            }

            if let Some(from_re) = &r.from_re
                && let Some(m) = from_re.find_string_submatch(pattern)
            {
                if !found {
                    found = header.is_none_or(|h| r.match_header(h));
                }

                if found {
                    // Replace $1, $2 etc. in To.
                    for (i, g) in m[1..].iter().enumerate() {
                        redir.to = redir.to.replace(&format!("${}", i + 1), g);
                    }
                }
            }

            if found {
                return redir;
            }
        }

        Redirect::default()
    }
}

/// Go: `config.DecodeServer(cfg)`.
// Go: config/commonConfig.go:DecodeServer
pub fn decode_server(cfg: &dyn Provider) -> Result<Server> {
    let mut s = Server::default();

    let _ = weak_decode_into(&Value::map(cfg.get_string_map("server")), &mut s);

    for redir in s.redirects.iter_mut() {
        if let Some(t) = redir.to.strip_suffix("index.html") {
            redir.to = t.to_string();
        }
    }

    if s.redirects.is_empty() {
        // Set up a default redirect for 404s.
        s.redirects = vec![Redirect {
            from: "/**".to_string(),
            to: "/404.html".to_string(),
            status: 404,
            ..Default::default()
        }];
    }

    Ok(s)
}

/// Go: `config.Pagination`: configures the pagination behavior.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Pagination {
    /// Default number of elements per pager in pagination.
    pub pager_size: i64,
    /// The path element used during pagination.
    pub path: String,
    /// Whether to disable generation of alias for the first pagination page.
    pub disable_aliases: bool,
}

decode_struct!(Pagination, "config.Pagination", |s| vec![
    FieldRef::new("PagerSize", &mut s.pager_size),
    FieldRef::new("Path", &mut s.path),
    FieldRef::new("DisableAliases", &mut s.disable_aliases),
]);

/// Go: `config.PageConfig` (next/prev sort orders).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PageConfig {
    /// Sort order for Page.Next and Page.Prev. Default "desc".
    pub next_prev_sort_order: String,
    /// Sort order for Page.NextInSection and Page.PrevInSection. Default "desc".
    pub next_prev_in_section_sort_order: String,
}

decode_struct!(PageConfig, "config.PageConfig", |s| vec![
    FieldRef::new("NextPrevSortOrder", &mut s.next_prev_sort_order),
    FieldRef::new(
        "NextPrevInSectionSortOrder",
        &mut s.next_prev_in_section_sort_order
    ),
]);

impl PageConfig {
    // Go: config/commonConfig.go:(*PageConfig).CompileConfig
    pub fn compile_config(&mut self) -> Result<()> {
        self.next_prev_in_section_sort_order =
            go_unicode::strings::to_lower_str(&self.next_prev_in_section_sort_order).into_owned();
        self.next_prev_sort_order =
            go_unicode::strings::to_lower_str(&self.next_prev_sort_order).into_owned();
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: config/commonConfig.go (512 lines; 10/15 funcs executed)
//   types: BaseConfig, CommonDirs, LoadConfigResult, BuildConfig, BuildStats, SitemapConfig, Server, redirect,
//          Headers, Redirect, CacheBuster, Pagination, PageConfig
// OK L125-130: (w BuildStats) Enabled() bool
// OK L132-135: (b BuildConfig) clone() BuildConfig
// OK L137-147: (b BuildConfig) UseResourceCache(err error) bool
// OK L150-168: (s BuildConfig) MatchCacheBuster(logger loggers.Logger, p string) (func(string) bool, error)
// OK L170-178: (b *BuildConfig) CompileConfig(logger loggers.Logger) error
// OK L180-207: DecodeBuildConfig(cfg Provider) BuildConfig
// OK L221-224: DecodeSitemap(prototype SitemapConfig, input map[string]any) (SitemapConfig, error)
// OK L241-248: (r redirect) matchHeader(header http.Header) bool
// OK L250-293: (s *Server) CompileConfig(logger loggers.Logger) error
// OK L295-316: (s *Server) MatchHeaders(pattern string) []types.KeyValueStr
// OK L318-359: (s *Server) MatchRedirect(pattern string, header http.Header) Redirect
// OK L404-452: (c *CacheBuster) CompileConfig(logger loggers.Logger) error
// OK L454-456: (r Redirect) IsZero() bool
// OK L463-485: DecodeServer(cfg Provider) (Server, error)
// OK L508-512: (c *PageConfig) CompileConfig(loggers.Logger) error
// ---------------------------------------------------------------------------
