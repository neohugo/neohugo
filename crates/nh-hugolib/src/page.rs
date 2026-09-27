//! Port of `hugolib/page.go`.
//!
//! Owner: Wave B task T23 (hugolib-site).


//! Go `hugolib/page.go` (`pageState`) — split into sub-structs owned by the module that ports the
//! corresponding Go file (so parallel Wave B tasks never edit the same struct):
//! * `meta: PageMeta` — page__meta.rs (T20)
//! * `common: PageCommon` — page__common.rs (T23)
//! * `lazy: OnceLock<Result<PageLazy>>` — Go `ps.init` (lazy.Init running `initLazyProviders`):
//!   `PagePaths` (page__paths.rs, T21) + the page outputs (page__output.rs, T22), created by
//!   `init_page()` in page__init.rs (T21) on the first `shift_to_output_format` (which already
//!   happens in `assembleResources`, during assembly)
//! * `content: Option<Arc<CachedContent>>` — page__content_parse.rs (T20) / page__content.rs (T22)
//!
//! `shift_to_output_format`, `init_page` and `init_common_providers` are in page__init.rs (T21).

use std::any::Any;
use std::borrow::Cow;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};

use go_value::{HostCtx, Map, Time, Value};
use nh_common::object::GoResult;
use nh_common::Result;

use crate::hugo_sites::HugoSites;
use crate::page__common::PageCommon;
use crate::page__content::CachedContent;
use crate::page__meta::PageMeta;
use crate::page__output::PageOutput;
use crate::page__paths::PagePaths;

/// Index of a page in the `HugoSites.pages` arena.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PageId(pub u32);

/// Go: `pageState`.
pub struct PageState {
    pub id: PageId,
    /// Go `pid` (unique id; used for hugocontext and identity).
    pub pid: u64,
    /// Index of the owning site (Go `p.s`).
    pub site_idx: usize,
    pub meta: PageMeta,
    pub common: PageCommon,
    /// Go `ps.init` + `pageOutputs`: created once by `init_page()` (page__init.rs).
    pub lazy: OnceLock<Result<PageLazy>>,
    /// Go `pageOutputIdx` + embedded `*pageOutput`: switched by `shiftToOutputFormat` during the
    /// render loop (other pages' `.Permalink`, `.Content`, `.Plain` follow the current format!).
    pub current_output_idx: AtomicUsize,
    /// Go `pageOutputTemplateVariationsState` (`canReusePageOutputContent` == 1).
    pub page_output_template_variations_state: AtomicU32,
    pub content: Option<Arc<CachedContent>>,
}

/// Go: the state `initLazyProviders` creates (page__meta.go:884-947).
pub struct PageLazy {
    pub paths: PagePaths,
    /// Go `pageOutputs`: one slot per GLOBAL render format (`HugoSites.render_formats`), but
    /// slots with the same format NAME share one `Arc<PageOutput>` (Go's `created` map,
    /// page__meta.go:907-916): e.g. en/html (index 0) and th/html (index 7) are the SAME output,
    /// so paginator, content provider and target paths are shared. Standalone pages (404,
    /// sitemap, robots) have exactly one slot.
    pub outputs: Vec<Arc<PageOutput>>,
}

impl PageState {
    /// Go: `p.pageOutput` (the current output). Panics if called before `init_page()` (an
    /// invariant: every caller runs after assembly, which initialises every page).
    pub fn current_output(&self) -> &Arc<PageOutput> {
        let lazy = self.lazy.get().and_then(|r| r.as_ref().ok()).expect("page outputs not initialised (init_page)");
        &lazy.outputs[self.current_output_idx.load(Ordering::Relaxed)]
    }

    /// Go: `renderResources()` — publish every bundle resource once (before rendering the page).
    // Go: hugolib/page.go:renderResources
    pub fn render_resources(&self, h: &Arc<HugoSites>) -> Result<()> {
        todo!()
    }

    /// Go: `resolveTemplate(layouts...)` — `TemplateQuery{Path: PathInfo().BaseReTyped(type),
    /// Category: layout, Desc: {Kind, Lang, LayoutFromUser, OutputFormat, MediaType, IsPlainText}}`.
    // Go: hugolib/page.go:resolveTemplate
    pub fn resolve_template(&self, h: &Arc<HugoSites>) -> Result<Option<Arc<nh_tplimpl::templatestore::TemplInfo>>> {
        todo!()
    }
}

/// Go wrapper types around `*pageState` that templates can observe (method set, `%T`,
/// identity via `Unwrapv`/`page_id`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PageWrapper {
    /// `*hugolib.pageState` itself.
    #[default]
    None,
    /// `hugolib.pageWithWeight0` (weighted pages of taxonomies; `Weight0` = the taxonomy weight).
    Weight0(i64),
    /// `*hugolib.pageWithOrdinal` (pages from `.Pages` of a menu/section with ordinal).
    Ordinal(i64),
    /// `*hugolib.pageForShortcode` (`.Page` inside shortcodes). Method set = `PageWithoutContent`
    /// + `TableOfContents` (returns the TOC placeholder) + NopPage `Markup`/`Content` providers +
    /// `Unwrapv`/`String` — NOT the full `*pageState` set (tplapi/page_methods.rs).
    ForShortcode,
    /// `*hugolib.pageForRenderHooks` (`.Page` inside link/image/heading/table render hooks).
    /// Method set = `PageWithoutContent` + the page's `TableOfContents` + NopPage `Markup`/`Content`
    /// providers + `Unwrapv`/`String`.
    ForRenderHooks,
}

/// The template/Rust handle of a page (Go `*pageState` pointer). Implements `nh_page::Page` and
/// `nh_resource::Resource`; its template method table lives in `tplapi::page_methods`.
#[derive(Clone)]
pub struct PageHandle {
    pub h: Arc<HugoSites>,
    pub id: PageId,
    pub wrapper: PageWrapper,
}

impl PageHandle {
    pub fn state(&self) -> &PageState {
        self.h.page(self.id)
    }

    /// The `page.Page` template value.
    pub fn page_ref(&self) -> nh_page::page::PageRef {
        nh_page::page::PageRef(Arc::new(self.clone()))
    }
}

// `impl nh_resource::resourcetypes::Resource for PageHandle` and
// `impl nh_page::page::Page for PageHandle` are written in Wave B (T23, tplapi/page_methods.rs):
// every typed method delegates to the owning module (meta, tree, content, paths...).

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/page.go (785 lines; 25/59 funcs executed) — initCommonProviders, initPage,
//          shiftToOutputFormat, (in)crPageOutputTemplateVariation/canReusePageOutputContent -> page__init.rs (T21)
//   types: pageContext, pageSiteAdapter, pageState, pageHeadingsFiltered, renderStringOpts, pageWithOrdinal,
//          pageWithWeight0
// EX L82-92: (pa pageSiteAdapter) GetPage(ref string) (page.Page, error)
//    L125-127: (p *pageState) IdentifierBase() string
//    L129-131: (p *pageState) GetIdentity() identity.Identity
//    L133-135: (p *pageState) ForEeachIdentity(f func(identity.Identity) bool) bool
//    L137-139: (p *pageState) GetDependencyManager() identity.Manager
//    L141-150: (p *pageState) GetDependencyManagerForScope(scope int) identity.Manager
//    L152-154: (p *pageState) GetDependencyManagerForScopesAll() []identity.Manager
//    L156-158: (p *pageState) Key() string
// EX L161-172: (p *pageState) RelatedKeywords(cfg related.IndexConfig) ([]related.Keyword, error)
//    L174-176: (p *pageState) resetBuildState()
// EX L178-189: (p *pageState) skipRender() bool
//    L191-198: (po *pageState) isRenderedAny() bool
// EX L200-202: (p *pageState) isContentNodeBranch() bool
// EX L206-213: (p *pageState) Eq(other any) bool
//    L215-217: (p *pageState) HeadingsFiltered(context.Context) tableofcontents.Headings
//    L224-226: (p *pageHeadingsFiltered) HeadingsFiltered(context.Context) tableofcontents.Headings
//    L228-230: (p *pageHeadingsFiltered) page() page.Page
//    L233-240: (p *pageState) ApplyFilterToHeadings(ctx context.Context, fn func(*tableofcontents.Heading) bool) related.Document
//    L242-244: (p *pageState) GitInfo() *source.GitInfo
//    L246-248: (p *pageState) CodeOwners() []string
//    L252-254: (p *pageState) GetTerms(taxonomy string) page.Pages
//    L256-258: (p *pageState) MarshalJSON() ([]byte, error)
//    L260-275: (p *pageState) RegularPagesRecursive() page.Pages
//    L277-279: (p *pageState) PagesRecursive() page.Pages
// EX L281-304: (p *pageState) RegularPages() page.Pages
// EX L306-342: (p *pageState) Pages() page.Pages
//    L346-359: (p *pageState) RawContent() string
// EX L361-363: (p *pageState) Resources() resource.Resources
//    L365-371: (p *pageState) HasShortcode(name string) bool
// EX L373-375: (p *pageState) Site() page.Site
//    L377-394: (p *pageState) String() string
// EX L398-400: (p *pageState) IsTranslated() bool
//    L403-408: (p *pageState) TranslationKey() string
// EX L411-443: (p *pageState) AllTranslations() page.Pages
// EX L446-461: (p *pageState) Translations() page.Pages
// EX L480-492: (po *pageOutput) GetInternalTemplateBasePathAndDescriptor() (string, tplimpl.TemplateDescriptor)
// EX L494-514: (p *pageState) resolveTemplate(layouts ...string) (*tplimpl.TemplInfo, bool, error)
// EX L524-554: (p *pageState) renderResources() error
// EX L556-567: (p *pageState) AlternativeOutputFormats() page.OutputFormats
//    L579-590: (p *pageMeta) wrapError(err error, sourceFs afero.Fs) error
//    L593-595: (p *pageState) wrapError(err error) error
//    L597-603: (p *pageState) getPageInfoForError() string
// EX L605-624: (p *pageState) getContentConverter() converter.Converter
//    L626-638: (p *pageState) errorf(err error, format string, a ...any) error
// EX L640-645: (p *pageState) outputFormat() (f output.Format)
//    L647-650: (p *pageState) parseError(err error, input []byte, offset int) error
// EX L652-662: (p *pageState) pathOrTitle() string
//    L664-666: (p *pageState) posFromInput(input []byte, offset int) text.Position
//    L668-670: (p *pageState) posOffset(offset int) text.Position
//    L760-762: (p pageWithOrdinal) Ordinal() int
//    L764-766: (p pageWithOrdinal) page() page.Page
// EX L773-775: (p pageWithWeight0) Weight0() int
//    L777-779: (p pageWithWeight0) page() page.Page
//    L783-785: (p pageWithWeight0) Unwrapv() any
// ---------------------------------------------------------------------------
