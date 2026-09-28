//! Port of `resources/page/page.go`.
//!
//! Owner: Wave B task T11 (page-api-paths).
//!
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
use nh_common::Result;
use nh_common::herrors::Error;
use nh_common::object::GoResult;
use nh_common::paths::pathparser::Path;
use nh_config::common_config::SitemapConfig;
use nh_helpers::source::file_info::File;
use nh_resource::resourcetypes::{Resource, Resources};

use crate::page_outputformat::OutputFormats;
use crate::related::{IndexConfig, Keyword};
use crate::site::SiteRef;

/// Go type string of the page.Pages named slice.
pub const PAGES_TYPE: &str = "page.Pages";
/// Go type string of the `page.Page` interface.
pub const PAGE_TYPE: &str = "page.Page";

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

    /// Go `CurrentSection()` (TreeProvider): the page itself for branch kinds, else its
    /// section (home for root pages). Needed by the permalink expander (`:sections`).
    /// The default is for skeleton implementors only: every real page type overrides it.
    fn current_section(&self) -> Option<PageRef> {
        unimplemented!("nh_page::page::Page::current_section must be implemented by the page type")
    }
    /// Go `SectionsEntries()` (e.g. `["docs", "functions"]`).
    fn sections_entries(&self) -> Vec<String> {
        unimplemented!("nh_page::page::Page::sections_entries must be implemented by the page type")
    }
    /// Go `SectionsPath()` (e.g. `/docs/functions`).
    fn sections_path(&self) -> String {
        unimplemented!("nh_page::page::Page::sections_path must be implemented by the page type")
    }

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

    // ---- added by T12 (page-collections); defaults for implementors that lack them ----
    /// Go `RelatedDocsHandlerProvider.GetInternalRelatedDocsHandler()` (the site's handler);
    /// `None` when the page is not a provider (`.Related` then fails with `invalid type`).
    fn related_docs_handler(&self) -> Option<Arc<crate::pages_related::RelatedDocsHandler>> {
        None
    }
    /// Go `related.FragmentProvider.Fragments(ctx).Identifiers`; `None` when the page is not a
    /// `FragmentProvider`.
    fn fragments_identifiers(&self, _ctx: HostCtx<'_>) -> Option<Vec<String>> {
        None
    }
    /// Go `FragmentProvider.ApplyFilterToHeadings(ctx, fn)` with a heading ID filter: the page
    /// wrapped as `*hugolib.pageHeadingsFiltered`; `None` when not a `FragmentProvider`.
    fn apply_filter_to_headings(
        &self,
        _ctx: HostCtx<'_>,
        _filter: &dyn Fn(&str) -> bool,
    ) -> Option<PageRef> {
        None
    }
    /// Go `IsAncestor(other)` (navigation's narrow `Page`); by default through the template API.
    fn is_ancestor(&self, other: &Value) -> bool {
        matches!(
            self.tpl_call_method(&(), "IsAncestor", std::slice::from_ref(other)),
            Some(Ok(Value::Bool(true)))
        )
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
    Value::list(
        SliceType::Named(Arc::from(PAGES_TYPE)),
        ps.iter().map(|p| p.to_value()).collect(),
    )
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

/// Go type strings of the concrete types that implement `page.Page`.
pub fn implements_page(type_name: &str) -> bool {
    matches!(
        type_name,
        "page.Page"
            | "*page.nopPage"
            | "*hugolib.pageState"
            | "hugolib.pageWithWeight0"
            | "*hugolib.pageWithOrdinal"
            | "*hugolib.pageForShortcode"
            | "*hugolib.pageForRenderHooks"
    )
}

/// Registers the `page.Page` and `resource.Resource` interfaces with `hreflect` (so
/// `Scratch.Add`/`append` keep `page.Pages`/`resource.Resources` typed as Go does) and installs
/// [`resource_from_value`] as nh-resource's value converter. Idempotent; call it once at
/// startup (nh-hugolib) and in tests.
pub fn init() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        nh_resource::resourcetypes::register();
        nh_common::hreflect::register_interface(PAGE_TYPE, implements_page);
        nh_common::hreflect::register_named_elem(PAGES_TYPE, PAGE_TYPE);
        nh_resource::resourcetypes::register_value_to_resource(resource_from_value);
    });
}

fn is_pages_type(ty: &SliceType) -> bool {
    matches!(ty, SliceType::Named(n) if &**n == PAGES_TYPE)
}

fn list_to_pages(l: &List) -> Option<Pages> {
    l.items.iter().map(page_from_value).collect()
}

/// Go: `page.ToPages(seq any) (Pages, error)` — Pages, `[]Page`, `[]any` of pages, WeightedPages,
/// PageGroup ...; nil -> empty.
// Go: resources/page/pages.go:ToPages
pub fn pages_from_value(v: &Value) -> Result<Pages> {
    match v {
        Value::Invalid => return Ok(Vec::new()),
        Value::List(l) => match &l.ty {
            ty if is_pages_type(ty) => {
                if let Some(p) = list_to_pages(l) {
                    return Ok(p);
                }
            }
            SliceType::Named(n) if &**n == crate::weighted::WEIGHTED_PAGES_TYPE => {
                let mut out = Vec::with_capacity(l.items.len());
                for it in &l.items {
                    match it.downcast::<crate::weighted::WeightedPage>() {
                        Some(w) => out.push(w.page.clone()),
                        None => break,
                    }
                }
                if out.len() == l.items.len() {
                    return Ok(out);
                }
            }
            SliceType::Named(n) if &**n == "[]page.Page" => {
                if let Some(p) = list_to_pages(l) {
                    return Ok(p);
                }
            }
            SliceType::Any => {
                if let Some(p) = list_to_pages(l) {
                    return Ok(p);
                }
            }
            _ => {}
        },
        Value::TypedNil(t) if &**t == PAGES_TYPE || &**t == "[]page.Page" => {
            return Ok(Vec::new());
        }
        Value::TypedNil(t) if &**t == crate::weighted::WEIGHTED_PAGES_TYPE => {
            return Ok(Vec::new());
        }
        // page.PageGroup (a struct value): its Pages field.
        Value::Object(o) if o.type_name() == "page.PageGroup" => {
            if let Some(pv) = o.field("Pages") {
                return pages_from_value(&pv);
            }
        }
        _ => {}
    }
    Err(Error::new(format!(
        "cannot convert type {} to Pages",
        go_type_of(v)
    )))
}

fn go_type_of(v: &Value) -> String {
    match v {
        Value::Invalid => "<nil>".to_string(),
        _ => v.go_type_name().into_owned(),
    }
}

/// Go: `page.NamedPageMetaValue(p, nameLower)` — used by `where`/`sort`/related on page fields.
/// `Ok(None)` is Go's `found == false` with a nil error. Go returns the value together with
/// `found == false` when a params lookup fails; the port returns only the error.
// Go: resources/page/page.go:NamedPageMetaValue
pub fn named_page_meta_value(p: &dyn Page, name_lower: &str) -> Result<Option<Value>> {
    let v = match name_lower {
        "kind" => Value::string(p.kind()),
        "bundletype" => Value::string(p.bundle_type()),
        "mediatype" => p.media_type().to_value(),
        "section" => Value::string(p.section()),
        "lang" => Value::string(p.lang()),
        "aliases" => Value::string_list(p.aliases()),
        "name" => Value::string(p.name()),
        "keywords" => Value::string_list(p.keywords()),
        "description" => Value::string(p.description()),
        "title" => Value::string(Page::title(p)),
        "linktitle" => Value::string(p.link_title()),
        "slug" => Value::string(p.slug()),
        "date" => Value::Time(p.date()),
        "publishdate" => Value::Time(p.publish_date()),
        "expirydate" => Value::Time(p.expiry_date()),
        "lastmod" => Value::Time(p.lastmod()),
        "draft" => Value::Bool(p.draft()),
        "type" => Value::string(p.page_type()),
        "layout" => Value::string(p.layout()),
        "weight" => Value::int(p.weight()),
        _ => {
            // Try params.
            let params = Resource::params(p);
            let r = nh_resource::params::param(&params, None, &Value::string(name_lower));
            match r {
                Ok(Value::Invalid) => return Ok(None),
                Ok(v) => v,
                // Go: a nil value wins over the error (`if v == nil { return nil, false, nil }`);
                // Param returns a nil value together with every error.
                Err(_) => return Ok(None),
            }
        }
    };

    Ok(Some(v))
}

/// Go: `page.Clear()` (clears the page sort cache). The cache and its `clear` are T12's
/// (pages_cache.rs); nh-hugolib calls this at the start of every `Site.render`.
// Go: resources/page/page.go:Clear
pub fn clear() {
    crate::pages_cache::clear()
}

/// Go: `page.PageWithContext` — a page bound to a context, so the context-taking content
/// methods can be called without one. The methods call the page's template API.
pub struct PageWithContext<'a> {
    pub page: Arc<dyn Page>,
    pub ctx: HostCtx<'a>,
}

// `Len` is Go's method name (the rendered content length); there is no `IsEmpty`.
#[allow(clippy::len_without_is_empty)]
impl PageWithContext<'_> {
    fn call(&self, name: &str) -> Result<Value> {
        match self.page.tpl_call_method(self.ctx, name, &[]) {
            Some(r) => r.map_err(Error::from),
            None => Err(Error::new(format!("{name}: no such method"))),
        }
    }

    // Go: resources/page/page.go:Content
    pub fn content(&self) -> Result<Value> {
        self.call("Content")
    }

    // Go: resources/page/page.go:Plain
    pub fn plain(&self) -> Result<GoString> {
        self.page.plain(self.ctx)
    }

    // Go: resources/page/page.go:PlainWords
    pub fn plain_words(&self) -> Result<Value> {
        self.call("PlainWords")
    }

    // Go: resources/page/page.go:Summary
    pub fn summary(&self) -> Result<Value> {
        self.call("Summary")
    }

    // Go: resources/page/page.go:Truncated
    pub fn truncated(&self) -> Result<Value> {
        self.call("Truncated")
    }

    // Go: resources/page/page.go:FuzzyWordCount
    pub fn fuzzy_word_count(&self) -> Result<Value> {
        self.call("FuzzyWordCount")
    }

    // Go: resources/page/page.go:WordCount
    pub fn word_count(&self) -> Result<Value> {
        self.call("WordCount")
    }

    // Go: resources/page/page.go:ReadingTime
    pub fn reading_time(&self) -> Result<Value> {
        self.call("ReadingTime")
    }

    // Go: resources/page/page.go:Len
    pub fn len(&self) -> Result<i64> {
        self.page.content_len(self.ctx)
    }
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
// OK L39-42: Clear() error (T12: delegates to pages_cache::clear)
// OK L263-319: NamedPageMetaValue(p PageMetaResource, nameLower string) (any, bool, error)
// OK L540-542: (p PageWithContext) Content() (any, error)
// OK L544-546: (p PageWithContext) Plain() string
// OK L548-550: (p PageWithContext) PlainWords() []string
// OK L552-554: (p PageWithContext) Summary() template.HTML
// OK L556-558: (p PageWithContext) Truncated() bool
// OK L560-562: (p PageWithContext) FuzzyWordCount() int
// OK L564-566: (p PageWithContext) WordCount() int
// OK L568-570: (p PageWithContext) ReadingTime() int
// OK L572-574: (p PageWithContext) Len() int
// ---------------------------------------------------------------------------
