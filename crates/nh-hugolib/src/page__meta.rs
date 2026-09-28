//! Port of `hugolib/page__meta.go`.
//!
//! Owner: Wave B task T20 (hugolib-capture).

//! Go `hugolib/page__meta.go`: page metadata, `setMetaPre` (front matter decoded into
//! `PageConfig` at page creation), the getters (`Name()`, `Type()`, `Kind()`, ...), `getParam`,
//! `outputFormats`, `shouldList`/`noRender`/`noLink`.
//!
//! Parts of page__meta.go live elsewhere: `setMetaPost`, `setMetaPostParams`,
//! `applyDefaultValues` (run by the assembly walks: params normalisation, dates, default titles)
//! -> page__meta_post.rs (T21); `initLazyProviders` -> page__init.rs (T21).
//!
//! Go's `pageMeta.s` (the site) is `PageState::site_idx`; the getters that need the site
//! (`Lang()`) read [`PageMeta::lang`], set at creation.

use std::sync::Arc;

use go_value::{Map, MapType, SliceType, Time, Value};
use nh_allconfig::allconfig::Config;
use nh_common::Result;
use nh_common::herrors::Error;
use nh_common::kinds;
use nh_common::loggers::Logger;
use nh_common::paths::pathparser::{Path, PathType};
use nh_config::common_config::SitemapConfig;
use nh_config::config_provider::AllProvider;
use nh_helpers::source::file_info::File;
use nh_media::output::output_format::{Formats, OutputFormat};
use nh_page::page_matcher::Cascade;
use nh_page::pagemeta::page_frontmatter::{Dates, PageConfig};
use nh_page::pagemeta::pagemeta as pm;

use crate::page__content_parse::ContentParseInfo;

/// Go: `pageMeta` + `pageMetaParams`.
#[derive(Clone)]
pub struct PageMeta {
    /// Set for kind == term: the raw term value (LAST value seen in tree-key walk order).
    pub term: String,
    /// Set for kind == term and taxonomy.
    pub singular: String,
    /// For standalone pages (404, sitemap, robots...): their output format (Go: the zero
    /// `output.Format` otherwise).
    pub standalone_output_format: Option<OutputFormat>,
    /// Set for bundled pages; path relative to its bundle root.
    pub resource_path: String,
    /// Set if this page is bundled inside another.
    pub bundled: bool,
    /// Always set: the canonical path of the page.
    pub path_info: Arc<Path>,
    /// Nil for pages without a file (auto sections, taxonomies, terms, home in some languages).
    pub f: Option<Arc<File>>,
    /// Go `p.s.Lang()`: the language of the page's site.
    pub lang: String,
    /// The decoded front matter (Go `pageMetaParams.pageConfig`). `kind` is Go's `Kind()`.
    pub page_config: PageConfig,
    /// Go `setMetaPostCount`: `setMetaPost` run count (terms are delayed to after
    /// `assembleTermsAndTranslations`).
    pub set_meta_post_count: i64,
    /// Go `setMetaPostCascadeChanged`.
    pub set_meta_post_cascade_changed: bool,
    /// Only set in watch mode (Go `datesOriginal`).
    pub dates_original: Dates,
    /// Only set in watch mode (Go `paramsOriginal`).
    pub params_original: Option<Map>,
    /// Only set in watch mode (Go `cascadeOriginal`).
    pub cascade_original: Option<Cascade>,
    /// Normalised params (`maps.Params`), frozen into an Arc after assembly (T21).
    pub params: Arc<Map>,
}

impl PageMeta {
    /// A meta with Go's zero values (`&pageMeta{}` with a `pageMetaParams{pageConfig:
    /// &PageConfig{}}`) for the given path.
    pub fn new(path_info: Arc<Path>) -> PageMeta {
        PageMeta {
            term: String::new(),
            singular: String::new(),
            standalone_output_format: None,
            resource_path: String::new(),
            bundled: false,
            path_info,
            f: None,
            lang: String::new(),
            page_config: PageConfig::default(),
            set_meta_post_count: 0,
            set_meta_post_cascade_changed: false,
            dates_original: Dates::default(),
            params_original: None,
            cascade_original: None,
            params: Arc::new(Map::new(MapType::Params)),
        }
    }

    /// Go: `pageMetaParams.init(preserveOriginal)`.
    // Go: hugolib/page__meta.go:init
    pub fn init(&mut self, preserve_original: bool) {
        if preserve_original {
            if self.page_config.is_from_content_adapter {
                self.params_original = self.page_config.content_adapter_data.clone();
            } else {
                self.params_original = self.page_config.params.clone();
            }
            self.cascade_original = self.page_config.cascade_compiled.clone();
        }
    }

    // Go: hugolib/page__meta.go:Aliases
    pub fn aliases(&self) -> &[String] {
        &self.page_config.aliases
    }

    // Go: hugolib/page__meta.go:BundleType
    pub fn bundle_type(&self) -> &'static str {
        match self.path_info.path_type() {
            PathType::Leaf => "leaf",
            PathType::Branch => "branch",
            _ => "",
        }
    }

    // Go: hugolib/page__meta.go:Date
    pub fn date(&self) -> Time {
        self.page_config.dates.date.clone()
    }

    // Go: hugolib/page__meta.go:PublishDate
    pub fn publish_date(&self) -> Time {
        self.page_config.dates.publish_date.clone()
    }

    // Go: hugolib/page__meta.go:Lastmod
    pub fn lastmod(&self) -> Time {
        self.page_config.dates.lastmod.clone()
    }

    // Go: hugolib/page__meta.go:ExpiryDate
    pub fn expiry_date(&self) -> Time {
        self.page_config.dates.expiry_date.clone()
    }

    // Go: hugolib/page__meta.go:Description
    pub fn description(&self) -> &str {
        &self.page_config.description
    }

    /// Go: `Lang()` = `p.s.Lang()`.
    // Go: hugolib/page__meta.go:Lang
    pub fn lang(&self) -> &str {
        &self.lang
    }

    // Go: hugolib/page__meta.go:Draft
    pub fn draft(&self) -> bool {
        self.page_config.draft
    }

    // Go: hugolib/page__meta.go:File
    pub fn file(&self) -> Option<&Arc<File>> {
        self.f.as_ref()
    }

    // Go: hugolib/page__meta.go:IsHome
    pub fn is_home(&self) -> bool {
        self.kind() == kinds::KIND_HOME
    }

    // Go: hugolib/page__meta.go:Keywords
    pub fn keywords(&self) -> &[String] {
        &self.page_config.keywords
    }

    // Go: hugolib/page__meta.go:Kind
    pub fn kind(&self) -> &str {
        &self.page_config.kind
    }

    // Go: hugolib/page__meta.go:Layout
    pub fn layout(&self) -> &str {
        &self.page_config.layout
    }

    // Go: hugolib/page__meta.go:LinkTitle
    pub fn link_title(&self) -> &str {
        if !self.page_config.link_title.is_empty() {
            return &self.page_config.link_title;
        }
        self.title()
    }

    /// Go: `Name()` — terms: `Unnormalized().BaseNameNoIdentifier()` of the FIRST creating value.
    // Go: hugolib/page__meta.go:Name
    pub fn name(&self) -> String {
        if !self.resource_path.is_empty() {
            return self.resource_path.clone();
        }
        if self.page_config.kind == kinds::KIND_TERM {
            return self
                .path_info
                .unnormalized()
                .base_name_no_identifier()
                .to_string();
        }
        self.title().to_string()
    }

    // Go: hugolib/page__meta.go:IsNode
    pub fn is_node(&self) -> bool {
        !self.is_page()
    }

    // Go: hugolib/page__meta.go:IsPage
    pub fn is_page(&self) -> bool {
        self.kind() == kinds::KIND_PAGE
    }

    /// Go: `Params()` = `pageConfig.Params` (nil until `setMetaPre`).
    // Go: hugolib/page__meta.go:Params
    pub fn params(&self) -> Option<&Map> {
        self.page_config.params.as_ref()
    }

    /// Go: `Path()` = `pathInfo.Base()`.
    // Go: hugolib/page__meta.go:Path
    pub fn path(&self) -> String {
        self.path_info.base()
    }

    // Go: hugolib/page__meta.go:PathInfo
    pub fn path_info(&self) -> &Arc<Path> {
        &self.path_info
    }

    // Go: hugolib/page__meta.go:IsSection
    pub fn is_section(&self) -> bool {
        self.kind() == kinds::KIND_SECTION
    }

    // Go: hugolib/page__meta.go:Section
    pub fn section(&self) -> &str {
        self.path_info.section()
    }

    // Go: hugolib/page__meta.go:Sitemap
    pub fn sitemap(&self) -> &SitemapConfig {
        &self.page_config.sitemap
    }

    // Go: hugolib/page__meta.go:Title
    pub fn title(&self) -> &str {
        &self.page_config.title
    }

    /// Go: `Type()` — `type` param, else Section(), else "page".
    // Go: hugolib/page__meta.go:Type
    pub fn page_type(&self) -> String {
        if !self.page_config.type_.is_empty() {
            return self.page_config.type_.clone();
        }
        let sect = self.section();
        if !sect.is_empty() {
            return sect.to_string();
        }
        DEFAULT_CONTENT_TYPE.to_string()
    }

    // Go: hugolib/page__meta.go:Weight
    pub fn weight(&self) -> i64 {
        self.page_config.weight
    }

    /// Go: `setMetaPre(pi, logger, conf)` — the front matter (prepared: lower-cased keys) becomes
    /// the page config's params; `cascade`, `path`, `lang` and `kind` are needed early.
    // Go: hugolib/page__meta.go:setMetaPre
    pub fn set_meta_pre(
        &mut self,
        pi: &ContentParseInfo,
        logger: &Logger,
        conf: &dyn AllProvider,
    ) -> Result<()> {
        if let Some(frontmatter) = &pi.front_matter {
            let mut frontmatter = frontmatter.clone();
            // Needed for case insensitive fetching of params values
            nh_common::maps::params::prepare_params(&mut frontmatter);
            let pcfg = &mut self.page_config;
            // Check for any cascade define on itself.
            if let Some(cv) = frontmatter.get(b"cascade") {
                let cascade = nh_page::page_matcher::decode_cascade(Some(logger), true, cv)?;
                pcfg.cascade_compiled = Some(cascade);
            }

            // Look for path, lang and kind, all of which values we need early on.
            if let Some(v) = frontmatter.get(b"path") {
                pcfg.path =
                    nh_common::paths::path::to_slash_preserve_leading(&go_str(&to_string(v)));
                frontmatter.insert("path", Value::string(pcfg.path.as_str()));
            }
            if let Some(v) = frontmatter.get(b"lang") {
                let lang = go_unicode::strings::to_lower_str(&go_str(&to_string(v))).into_owned();
                let pp = conf.path_parser();
                if pp
                    .language_index
                    .as_ref()
                    .is_some_and(|li| li.contains_key(&lang))
                {
                    pcfg.lang = lang;
                    frontmatter.insert("lang", Value::string(pcfg.lang.as_str()));
                }
            }
            if let Some(v) = frontmatter.get(b"kind") {
                let s = go_str(&to_string(v));
                if !s.is_empty() {
                    pcfg.kind = kinds::get_kind_main(&s).to_string();
                    if pcfg.kind.is_empty() {
                        return Err(Error::new(format!(
                            "unknown kind {} in front matter",
                            go_strconv::quote(&s)
                        )));
                    }
                    frontmatter.insert("kind", Value::string(pcfg.kind.as_str()));
                }
            }
            pcfg.params = Some(frontmatter);
        } else if self.page_config.params.is_none() {
            self.page_config.params = Some(Map::new(MapType::Params));
        }

        self.init(conf.watching());

        Ok(())
    }

    /// Go: `shouldList(global)` — whether this page should be included in the list of pages;
    /// `global` indicates site.Pages etc.
    // Go: hugolib/page__meta.go:shouldList
    pub fn should_list(&self, global: bool) -> bool {
        if self.is_standalone() {
            // Never list 404, sitemap and similar.
            return false;
        }

        match self.page_config.build.list.as_str() {
            pm::ALWAYS => true,
            pm::NEVER => false,
            pm::LIST_LOCALLY => !global,
            _ => false,
        }
    }

    // Go: hugolib/page__meta.go:shouldListAny
    pub fn should_list_any(&self) -> bool {
        self.should_list(true) || self.should_list(false)
    }

    // Go: hugolib/page__meta.go:isStandalone
    pub fn is_standalone(&self) -> bool {
        self.standalone_output_format.is_some()
    }

    // Go: hugolib/page__meta.go:shouldBeCheckedForMenuDefinitions
    pub fn should_be_checked_for_menu_definitions(&self) -> bool {
        if !self.should_list(false) {
            return false;
        }

        let k = self.page_config.kind.as_str();
        k == kinds::KIND_HOME || k == kinds::KIND_SECTION || k == kinds::KIND_PAGE
    }

    // Go: hugolib/page__meta.go:noRender
    pub fn no_render(&self) -> bool {
        self.page_config.build.render != pm::ALWAYS
    }

    // Go: hugolib/page__meta.go:noLink
    pub fn no_link(&self) -> bool {
        self.page_config.build.render == pm::NEVER
    }

    /// Go: `outputFormats()` — the output formats this page will be rendered to (`conf` is the
    /// page's site config).
    // Go: hugolib/page__meta.go:outputFormats
    pub fn output_formats(&self, conf: &Config) -> Formats {
        if !self.page_config.configured_output_formats.0.is_empty() {
            return self.page_config.configured_output_formats.clone();
        }
        conf.compiled()
            .kind_output_formats
            .get(self.kind())
            .cloned()
            .unwrap_or_default()
    }

    // Go: hugolib/page__meta.go:Slug
    pub fn slug(&self) -> &str {
        &self.page_config.slug
    }
}

const DEFAULT_CONTENT_TYPE: &str = "page";

/// `cast.ToString(v)`.
fn to_string(v: &Value) -> go_value::GoString {
    nh_common::cast::caste::to_string(v)
}

/// A Go string (bytes) as a Rust string (paths, languages and kinds are text).
fn go_str(s: &go_value::GoString) -> String {
    String::from_utf8_lossy(s.as_bytes()).into_owned()
}

/// Go: `getParam(m, key, stringToLower)` over a page's params (`m.Params()`).
// Go: hugolib/page__meta.go:getParam
pub fn get_param(params: Option<&Map>, key: &str, string_to_lower: bool) -> Value {
    let key = go_unicode::strings::to_lower_str(key);
    let Some(v) = params.and_then(|m| m.get(key.as_bytes())) else {
        return Value::Invalid;
    };

    if matches!(v, Value::Invalid) {
        return Value::Invalid;
    }

    match v {
        Value::Bool(_) => v.clone(),
        Value::String(s) => {
            if string_to_lower {
                return Value::string(go_unicode::bytes::to_lower(s.as_bytes()));
            }
            v.clone()
        }
        Value::Int(..) => Value::int(nh_common::cast::caste::to_int(v)),
        Value::Float(..) => Value::float64(nh_common::cast::caste::to_float64(v)),
        Value::Time(_) => v.clone(),
        Value::List(l) if l.ty == SliceType::String => {
            if string_to_lower {
                let items = l
                    .items
                    .iter()
                    .map(|it| match it {
                        Value::String(s) => {
                            Value::string(go_unicode::bytes::to_lower(s.as_bytes()))
                        }
                        other => other.clone(),
                    })
                    .collect();
                return Value::List(Arc::new(go_value::List::new(SliceType::String, items)));
            }
            v.clone()
        }
        _ => v.clone(),
    }
}

/// Go: `getParamToLower(m, key)`.
// Go: hugolib/page__meta.go:getParamToLower
pub fn get_param_to_lower(params: Option<&Map>, key: &str) -> Value {
    get_param(params, key, true)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/page__meta.go (setMetaPost/setMetaPostParams/applyDefaultValues -> page__meta_post.rs,
//          initLazyProviders -> page__init.rs; both T21) (948 lines; 40/46 funcs executed)
//   types: pageMeta, pageMetaParams
//    L72-75: (m *pageMeta) setMetaPostPrepareRebuild()
// OK L89-98: (m *pageMetaParams) init(preserveOriginal bool)
// OK L100-102: (p *pageMeta) Aliases() []string
// OK L104-113: (p *pageMeta) BundleType() string
// OK L115-117: (p *pageMeta) Date() time.Time
// OK L119-121: (p *pageMeta) PublishDate() time.Time
// OK L123-125: (p *pageMeta) Lastmod() time.Time
// OK L127-129: (p *pageMeta) ExpiryDate() time.Time
// OK L131-133: (p *pageMeta) Description() string
// OK L135-137: (p *pageMeta) Lang() string
// OK L139-141: (p *pageMeta) Draft() bool
// OK L143-145: (p *pageMeta) File() *source.File
// OK L147-149: (p *pageMeta) IsHome() bool
// OK L151-153: (p *pageMeta) Keywords() []string
// OK L155-157: (p *pageMeta) Kind() string
// OK L159-161: (p *pageMeta) Layout() string
// OK L163-169: (p *pageMeta) LinkTitle() string
// OK L171-179: (p *pageMeta) Name() string
// OK L181-183: (p *pageMeta) IsNode() bool
// OK L185-187: (p *pageMeta) IsPage() bool
//    L194-196: (p *pageMeta) Param(key any) (any, error)
// OK L198-200: (p *pageMeta) Params() maps.Params
// OK L202-204: (p *pageMeta) Path() string
// OK L206-208: (p *pageMeta) PathInfo() *paths.Path
// OK L210-212: (p *pageMeta) IsSection() bool
// OK L214-216: (p *pageMeta) Section() string
// OK L218-220: (p *pageMeta) Sitemap() config.SitemapConfig
// OK L222-224: (p *pageMeta) Title() string
// OK L228-238: (p *pageMeta) Type() string
// OK L240-242: (p *pageMeta) Weight() int
// OK L244-291: (p *pageMeta) setMetaPre(pi *contentParseInfo, logger loggers.Logger, conf config.AllProvider) error
// OK L691-706: (p *pageMeta) shouldList(global bool) bool
// OK L708-710: (p *pageMeta) shouldListAny() bool
// OK L712-714: (p *pageMeta) isStandalone() bool
// OK L716-722: (p *pageMeta) shouldBeCheckedForMenuDefinitions() bool
// OK L724-726: (p *pageMeta) noRender() bool
// OK L728-730: (p *pageMeta) noLink() bool
// EX L788-835: (p *pageMeta) newContentConverter(ps *pageState, markup string) (converter.Converter, error)  [T22: needs the render-hook page]
// OK L838-843: (m *pageMeta) outputFormats() output.Formats
// OK L845-847: (p *pageMeta) Slug() string
// OK L849-878: getParam(m resource.ResourceParamsProvider, key string, stringToLower bool) any
// OK L880-882: getParamToLower(m resource.ResourceParamsProvider, key string) any
// ---------------------------------------------------------------------------
