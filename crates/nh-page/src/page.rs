//! Port of `resources/page/page.go`.
//!
//! Owner: Wave B task T11 (page-api-paths).


//! Go `resources/page.Page`: the page interface as seen by every crate below nh-hugolib.
//!
//! * [`Page`] is object-safe and extends `nh_resource::Resource` (Go: `page.Page` embeds
//!   `resource.Resource`), so a page can live in `resource.Resources` and its whole template API
//!   is reachable through `Resource::tpl_call_method` (implemented by nh-hugolib's page handle).
//! * The typed methods below are the ones Rust code in lower crates needs (sorting, grouping,
//!   related, pagination, permalinks, `where`/`sort` fast paths). Everything else a template can
//!   call is only in the dynamic table.
//! * [`PageRef`] is THE template-value wrapper for pages. `Value::Object(Arc<PageRef>)`; use
//!   [`PageRef::to_value`] / [`page_from_value`] / [`pages_from_value`]. Identity (Go pointer
//!   equality, `compare.Eqer`) is [`Page::page_id`] of the unwrapped page.

use std::any::Any;
use std::borrow::Cow;
use std::sync::Arc;

use go_value::{GoString, HostCtx, List, Map, Object, SliceType, Time, Value};
use nh_common::object::GoResult;
use nh_common::paths::pathparser::Path;
use nh_common::Result;
use nh_config::common_config::SitemapConfig;
use nh_helpers::source::file_info::File;
use nh_resource::resourcetypes::{Resource, Resources};

use crate::page_outputformat::OutputFormats;
use crate::related::{IndexConfig, Keyword};
use crate::site::SiteRef;

/// Go type string of the page.Pages named slice.
pub const PAGES_TYPE: &str = "page.Pages";

/// Go: `page.Page` (typed subset + dynamic template API via `Resource`).
pub trait Page: Resource {
    /// Stable unique id of the underlying page (Go: pointer identity / `pid`). Wrappers
    /// (`pageWithWeight0`, `pageWithOrdinal`) return the id of the page they wrap.
    fn page_id(&self) -> u64;
    /// Go: `unwrapPage` — the page without taxonomy wrappers.
    fn unwrap_page(self: Arc<Self>) -> Arc<dyn Page>;
    /// Go: `resource.Weight0Provider` (term `.Pages` entries: `pageWithWeight0`).
    fn weight0(&self) -> Option<i64> {
        None
    }
    /// Go: `collections.Order` (`pageWithOrdinal` from `GetTerms`).
    fn ordinal(&self) -> Option<i64> {
        None
    }

    // ---- PageMetaProvider ----
    fn kind(&self) -> String;
    fn title(&self) -> String;
    fn link_title(&self) -> String;
    fn description(&self) -> String;
    /// Go `int`.
    fn weight(&self) -> i64;
    fn date(&self) -> Time;
    fn lastmod(&self) -> Time;
    fn publish_date(&self) -> Time;
    fn expiry_date(&self) -> Time;
    fn is_home(&self) -> bool;
    fn is_node(&self) -> bool;
    fn is_page(&self) -> bool;
    fn is_section(&self) -> bool;
    fn section(&self) -> String;
    /// Go `Type()`: front-matter `type`, else section, else "page".
    fn page_type(&self) -> String;
    fn layout(&self) -> String;
    fn lang(&self) -> String;
    /// Go `Path()` (the canonical path, e.g. `/biscuit/koalas-march-chocolate`).
    fn path(&self) -> String;
    /// Go `PathInfo()`.
    fn path_info(&self) -> Arc<Path>;
    fn slug(&self) -> String;
    fn draft(&self) -> bool;
    fn aliases(&self) -> Vec<String>;
    fn keywords(&self) -> Vec<String>;
    fn bundle_type(&self) -> String;
    fn sitemap(&self) -> SitemapConfig;
    /// Go `Param(key)` — page params then site params, case-insensitive nested.
    fn param(&self, key: &Value) -> Result<Value>;
    /// Go `Params()` (maps.Params).
    fn page_params(&self) -> Arc<Map>;

    // ---- relations ----
    fn site(&self) -> SiteRef;
    fn file(&self) -> Option<Arc<File>>;
    fn parent(&self) -> Option<PageRef>;
    fn pages(&self) -> Pages;
    fn regular_pages(&self) -> Pages;
    fn resources(&self) -> Resources;
    fn output_formats(&self) -> OutputFormats;
    fn all_translations(&self) -> Pages;
    fn translations(&self) -> Pages;

    // ---- content (ctx = the template context as HostCtx) ----
    fn plain(&self, ctx: HostCtx<'_>) -> Result<GoString>;
    /// Go `Len(ctx)` — length of the rendered content.
    fn content_len(&self, ctx: HostCtx<'_>) -> Result<i64>;
    /// Go `RenderString(ctx, args...)` (markdownify uses the home page's).
    fn render_string(&self, ctx: HostCtx<'_>, args: &[Value]) -> Result<Value>;

    // ---- misc ----
    /// Go `RelatedKeywords(cfg)` -> `NamedPageMetaValue` / params lookup.
    fn related_keywords(&self, cfg: &IndexConfig) -> Result<Vec<Keyword>>;
    /// Go `Ref(argsm)` / `RelRef(argsm)` (shortcode `ref`).
    fn ref_(&self, args: &Map) -> Result<String>;
    fn rel_ref(&self, args: &Map) -> Result<String>;
    /// Go `String()` — e.g. `Page(/biscuit/x)`.
    fn page_string(&self) -> String {
        format!("Page({})", self.path())
    }
}

/// The template value of a page.
#[derive(Clone)]
pub struct PageRef(pub Arc<dyn Page>);

impl PageRef {
    pub fn to_value(&self) -> Value {
        Value::Object(Arc::new(self.clone()))
    }

    /// Go pointer equality of the unwrapped pages.
    pub fn same_page(&self, other: &PageRef) -> bool {
        self.0.page_id() == other.0.page_id()
    }
}

impl Object for PageRef {
    fn type_name(&self) -> Cow<'_, str> {
        self.0.tpl_type_name()
    }
    fn has_method(&self, name: &str) -> bool {
        self.0.tpl_has_method(name)
    }
    fn call_method(&self, ctx: HostCtx<'_>, name: &str, args: &[Value]) -> Option<GoResult<Value>> {
        self.0.tpl_call_method(ctx, name, args)
    }
    fn is_zero(&self) -> Option<bool> {
        self.0.tpl_is_zero()
    }
    fn go_string(&self) -> Option<GoString> {
        Some(GoString::from(self.0.page_string()))
    }
    fn identity(&self) -> usize {
        self.0.page_id() as usize
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Go: `page.Pages` (Rust list form).
pub type Pages = Vec<PageRef>;

/// `page.Pages` as a template value.
pub fn pages_to_value(ps: &[PageRef]) -> Value {
    Value::list(SliceType::Named(Arc::from(PAGES_TYPE)), ps.iter().map(|p| p.to_value()).collect())
}

/// A page from a template value.
pub fn page_from_value(v: &Value) -> Option<PageRef> {
    v.downcast::<PageRef>().cloned()
}

/// Any resource (page or not) from a template value.
pub fn resource_from_value(v: &Value) -> Option<Arc<dyn Resource>> {
    if let Some(p) = page_from_value(v) {
        let r: Arc<dyn Resource> = p.0;
        return Some(r);
    }
    nh_resource::resourcetypes::resource_ref_from_value(v)
}

/// Go: `page.ToPages(seq any) (Pages, error)` — Pages, `[]Page`, `[]any` of pages, WeightedPages,
/// PageGroup ...; nil -> empty.
// Go: resources/page/pages.go:ToPages
pub fn pages_from_value(v: &Value) -> Result<Pages> {
    todo!()
}

/// Go: `page.NamedPageMetaValue(p, nameLower)` — used by `where`/`sort`/related on page fields.
// Go: resources/page/page.go:NamedPageMetaValue
pub fn named_page_meta_value(p: &dyn Page, name_lower: &str) -> Result<Option<Value>> {
    todo!()
}

/// Go: `page.Clear()` (clears the page sort cache). The cache and its `clear` are T12's
/// (pages_cache.rs); nh-hugolib calls this at the start of every `Site.render`.
// Go: resources/page/page.go:Clear
pub fn clear() {
    crate::pages_cache::clear()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/page/page.go (574 lines; 2/11 funcs executed)
//   types: AlternativeOutputFormatsProvider, ChildCareProvider, MarkupProvider, ContentProvider, ContentRenderer,
//          FileProvider, GetPageProvider, GitInfoProvider, InSectionPositioner, RelatedDocsHandlerProvider,
//          OutputFormatsProvider, PageProvider, Page, PageFragment, PageMetaResource, PageMetaProvider,
//          PageMetaInternalProvider, PageRenderProvider, PageWithoutContent, Positioner, RawContentProvider,
//          RenderShortcodesProvider, RefProvider, RelatedKeywordsProvider, ShortcodeInfoProvider, SitesProvider,
//          TableOfContentsProvider, TranslationsProvider, TreeProvider, PageWithContext
// EX L39-42: Clear() error
// EX L263-319: NamedPageMetaValue(p PageMetaResource, nameLower string) (any, bool, error)
//    L540-542: (p PageWithContext) Content() (any, error)
//    L544-546: (p PageWithContext) Plain() string
//    L548-550: (p PageWithContext) PlainWords() []string
//    L552-554: (p PageWithContext) Summary() template.HTML
//    L556-558: (p PageWithContext) Truncated() bool
//    L560-562: (p PageWithContext) FuzzyWordCount() int
//    L564-566: (p PageWithContext) WordCount() int
//    L568-570: (p PageWithContext) ReadingTime() int
//    L572-574: (p PageWithContext) Len() int
// ---------------------------------------------------------------------------
