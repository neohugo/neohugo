//! Port of `hugolib/page__meta.go`.
//!
//! Owner: Wave B task T20 (hugolib-capture).


//! Go `hugolib/page__meta.go`: page metadata, `setMetaPre` (front matter decoded into
//! `PageConfig` at page creation), the getters (`Name()`, `Type()`, `Kind()`, ...), `getParam`,
//! `newContentConverter`, `outputFormats`, `shouldList`/`noRender`/`noLink`.
//!
//! Parts of page__meta.go live elsewhere: `setMetaPost`, `setMetaPostParams`,
//! `applyDefaultValues` (run by the assembly walks: params normalisation, dates, default titles)
//! -> page__meta_post.rs (T21); `initLazyProviders` -> page__init.rs (T21).

use std::sync::Arc;

use go_value::{Map, Time, Value};
use nh_common::paths::pathparser::Path;
use nh_common::Result;
use nh_helpers::source::file_info::File;
use nh_media::output::output_format::OutputFormat;
use nh_page::pagemeta::page_frontmatter::{Dates, PageConfig};

/// Go: `pageMeta`.
#[derive(Clone)]
pub struct PageMeta {
    /// Set for kind == term: the raw term value (LAST value seen in tree-key walk order).
    pub term: String,
    /// Set for kind == term and taxonomy.
    pub singular: String,
    /// "page", "home", "section", "taxonomy", "term", "404", "sitemap", "robotstxt", "sitemapindex".
    pub kind: String,
    /// For standalone pages (404, sitemap, robots...): their output format.
    pub standalone_output_format: Option<OutputFormat>,
    /// Set for bundled pages; path relative to its bundle root.
    pub resource_path: String,
    pub bundled: bool,
    /// Always set: the canonical path of the page.
    pub path_info: Arc<Path>,
    /// Nil for pages without a file (auto sections, taxonomies, terms, home in some languages).
    pub f: Option<Arc<File>>,
    /// The decoded front matter (normalised).
    pub page_config: PageConfig,
    pub dates_original: Dates,
    pub params_original: Option<Map>,
    /// Normalised params (`maps.Params`), frozen into an Arc after assembly.
    pub params: Arc<Map>,
    /// `setMetaPost` run count (terms are delayed to after `assembleTermsAndTranslations`).
    pub set_meta_post_count: i64,
}

impl PageMeta {
    /// Go: `Name()` — terms: `Unnormalized().BaseNameNoIdentifier()` of the FIRST creating value.
    // Go: hugolib/page__meta.go:Name
    pub fn name(&self) -> String {
        todo!()
    }

    /// Go: `Type()` — `type` param, else Section(), else "page".
    // Go: hugolib/page__meta.go:Type
    pub fn page_type(&self) -> String {
        todo!()
    }

    // Go: hugolib/page__meta.go:Title
    pub fn title(&self) -> &str {
        &self.page_config.title
    }

    // Go: hugolib/page__meta.go:LinkTitle
    pub fn link_title(&self) -> &str {
        if self.page_config.link_title.is_empty() { &self.page_config.title } else { &self.page_config.link_title }
    }

    // Go: hugolib/page__meta.go:Date
    pub fn date(&self) -> Time {
        self.page_config.dates.date.clone()
    }

    // Go: hugolib/page__meta.go:IsNode
    pub fn is_node(&self) -> bool {
        self.kind != nh_common::kinds::KIND_PAGE
    }

    // Go: hugolib/page__meta.go:shouldList / shouldListAny / shouldLink / noRender / noLink
    pub fn should_list(&self, global: bool) -> bool {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/page__meta.go (setMetaPost/setMetaPostParams/applyDefaultValues -> page__meta_post.rs,
//          initLazyProviders -> page__init.rs; both T21) (948 lines; 40/46 funcs executed)
//   types: pageMeta, pageMetaParams
//    L72-75: (m *pageMeta) setMetaPostPrepareRebuild()
// EX L89-98: (m *pageMetaParams) init(preserveOriginal bool)
// EX L100-102: (p *pageMeta) Aliases() []string
//    L104-113: (p *pageMeta) BundleType() string
// EX L115-117: (p *pageMeta) Date() time.Time
// EX L119-121: (p *pageMeta) PublishDate() time.Time
// EX L123-125: (p *pageMeta) Lastmod() time.Time
// EX L127-129: (p *pageMeta) ExpiryDate() time.Time
// EX L131-133: (p *pageMeta) Description() string
// EX L135-137: (p *pageMeta) Lang() string
// EX L139-141: (p *pageMeta) Draft() bool
// EX L143-145: (p *pageMeta) File() *source.File
// EX L147-149: (p *pageMeta) IsHome() bool
//    L151-153: (p *pageMeta) Keywords() []string
// EX L155-157: (p *pageMeta) Kind() string
// EX L159-161: (p *pageMeta) Layout() string
// EX L163-169: (p *pageMeta) LinkTitle() string
// EX L171-179: (p *pageMeta) Name() string
// EX L181-183: (p *pageMeta) IsNode() bool
// EX L185-187: (p *pageMeta) IsPage() bool
//    L194-196: (p *pageMeta) Param(key any) (any, error)
// EX L198-200: (p *pageMeta) Params() maps.Params
// EX L202-204: (p *pageMeta) Path() string
// EX L206-208: (p *pageMeta) PathInfo() *paths.Path
// EX L210-212: (p *pageMeta) IsSection() bool
// EX L214-216: (p *pageMeta) Section() string
// EX L218-220: (p *pageMeta) Sitemap() config.SitemapConfig
// EX L222-224: (p *pageMeta) Title() string
// EX L228-238: (p *pageMeta) Type() string
// EX L240-242: (p *pageMeta) Weight() int
// EX L244-291: (p *pageMeta) setMetaPre(pi *contentParseInfo, logger loggers.Logger, conf config.AllProvider) error
// EX L691-706: (p *pageMeta) shouldList(global bool) bool
//    L708-710: (p *pageMeta) shouldListAny() bool
// EX L712-714: (p *pageMeta) isStandalone() bool
//    L716-722: (p *pageMeta) shouldBeCheckedForMenuDefinitions() bool
// EX L724-726: (p *pageMeta) noRender() bool
// EX L728-730: (p *pageMeta) noLink() bool
// EX L788-835: (p *pageMeta) newContentConverter(ps *pageState, markup string) (converter.Converter, error)
// EX L838-843: (m *pageMeta) outputFormats() output.Formats
// EX L845-847: (p *pageMeta) Slug() string
// EX L849-878: getParam(m resource.ResourceParamsProvider, key string, stringToLower bool) any
// EX L880-882: getParamToLower(m resource.ResourceParamsProvider, key string) any
// ---------------------------------------------------------------------------
