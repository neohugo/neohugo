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
//! The content helpers of page.go (`getContentConverter`, `wrapError`, `pathOrTitle`,
//! `posFromInput`, `posOffset`, `parseError`, `getPageInfoForError`, `HasShortcode`) are in
//! page__per_output.rs (T22), next to their callers.
//!
//! The methods Go declares on `*pageState` that templates reach are here as functions over a
//! [`PageHandle`] (Go's receiver); `tplapi::page_methods` dispatches the template calls to them.

use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};

use go_value::Value;
use nh_common::Result;
use nh_common::herrors::Error;
use nh_common::kinds;
use nh_page::page::{PageRef, Pages};
use nh_page::page_outputformat::OutputFormats;

use crate::content_map_page::{
    PageMap, PageMapQueryPagesBelowPath, PageMapQueryPagesInSection, page_predicates as pp,
};
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
        let lazy = self
            .lazy
            .get()
            .and_then(|r| r.as_ref().ok())
            .expect("page outputs not initialised (init_page)");
        &lazy.outputs[self.current_output_idx.load(Ordering::Relaxed)]
    }

    /// Go: `outputFormat()` — the format of the current output.
    // Go: hugolib/page.go:outputFormat
    pub fn output_format(&self) -> &nh_media::output::output_format::OutputFormat {
        static ZERO: std::sync::OnceLock<nh_media::output::output_format::OutputFormat> =
            std::sync::OnceLock::new();
        match self.current_output_opt() {
            Some(po) => &po.f,
            // Go: `nopPageOutput` (zero `output.Format`) before the first shift.
            None => ZERO.get_or_init(Default::default),
        }
    }

    /// Whether Go's `initCommonProviders` ran (the lazy page init succeeded): until then the
    /// page's `Positioner`, `InSectionPositioner`, `OutputFormatsProvider` and `RefProvider` are
    /// `page.NopPage` and its `SitesProvider` is a nil interface (e.g. a page bundled in a leaf
    /// bundle, which is never shifted before rendering).
    pub fn common_providers_initialised(&self) -> bool {
        matches!(self.lazy.get(), Some(Ok(_)))
    }

    /// Go: `p.pageOutput` when it is not `nopPageOutput` (the page was shifted to an output,
    /// which initialises it); `None` for a page never shifted (e.g. a page bundled in a leaf
    /// bundle before rendering), whose per-output methods are the `NopPage` ones in Go.
    pub fn current_output_opt(&self) -> Option<&Arc<PageOutput>> {
        let lazy = self.lazy.get()?.as_ref().ok()?;
        lazy.outputs
            .get(self.current_output_idx.load(Ordering::Relaxed))
    }

    /// Go: `renderResources()` — publish every bundle resource once (before rendering the page).
    // Go: hugolib/page.go:renderResources
    pub fn render_resources(&self, h: &Arc<HugoSites>) -> Result<()> {
        let site = &h.sites[self.site_idx];
        let stats = &site.deps.path_spec().processing_stats;
        for r in PageMap::get_or_create_resources_for_page(h, self.id) {
            if r.as_any().downcast_ref::<PageHandle>().is_some() {
                if h.build_counter.load(Ordering::SeqCst) == 0 {
                    // Pages gets rendered with the owning page but we count them here.
                    nh_helpers::processing_stats::ProcessingStats::incr(&stats.pages);
                }
                continue;
            }

            if nh_resources::resource::is_published(&r) {
                continue;
            }

            let Some(res) = r.publish() else {
                return Err(Error::new(format!(
                    "resource {} does not support resource.Source",
                    r.tpl_type_name()
                )));
            };

            match res {
                Err(err) => {
                    if !nh_common::herrors::is_not_exist(&err) {
                        site.deps.log.errorf(format!(
                            "Failed to publish Resource for page {}: {}",
                            go_strconv::quote(self.path_or_title()),
                            err
                        ));
                    }
                }
                Ok(()) => {
                    nh_helpers::processing_stats::ProcessingStats::incr(&stats.files);
                }
            }
        }

        Ok(())
    }

    /// Go: `resolveTemplate(layouts...)` — `TemplateQuery{Path: PathInfo().BaseReTyped(type),
    /// Category: layout, Desc: {Kind, Lang, LayoutFromUser, OutputFormat, MediaType, IsPlainText}}`.
    // Go: hugolib/page.go:resolveTemplate
    pub fn resolve_template(
        &self,
        h: &Arc<HugoSites>,
    ) -> Result<Option<Arc<nh_tplimpl::templatestore::TemplInfo>>> {
        self.resolve_template_layouts(h, &[])
    }

    /// `resolveTemplate(layouts...)` with layouts: the first one is the layout the user asked
    /// for, and it must match.
    // Go: hugolib/page.go:resolveTemplate
    pub fn resolve_template_layouts(
        &self,
        h: &Arc<HugoSites>,
        layouts: &[String],
    ) -> Result<Option<Arc<nh_tplimpl::templatestore::TemplInfo>>> {
        let (dir, mut d) = self
            .current_output()
            .get_internal_template_base_path_and_descriptor(self);

        if !layouts.is_empty() {
            d.layout_from_user = layouts[0].clone();
            d.layout_from_user_must_match = true;
        }

        let q = nh_tplimpl::templatestore::TemplateQuery {
            path: dir,
            name: String::new(),
            category: nh_tplimpl::category::Category::Layout,
            desc: d,
            consider: None,
        };

        Ok(h.sites[self.site_idx]
            .deps
            .get_template_store()
            .lookup_pages_layout(&q))
    }

    /// Go: `skipRender()` — the segment filter excludes this page output.
    // Go: hugolib/page.go:skipRender
    pub fn skip_render(&self, h: &HugoSites) -> bool {
        let conf = &h.sites[self.site_idx].conf;
        conf.compiled().segment_filter.should_exclude_fine(
            &nh_allconfig::segments::SegmentMatcherFields {
                path: self.meta.path(),
                kind: self.meta.kind().to_string(),
                lang: self.meta.lang().to_string(),
                output: self.current_output().f.name.clone(),
            },
        )
    }

    /// Go: `isRenderedAny()`.
    // Go: hugolib/page.go:isRenderedAny
    pub fn is_rendered_any(&self) -> bool {
        match self.lazy.get() {
            Some(Ok(l)) => l.outputs.iter().any(|o| o.is_rendered()),
            _ => false,
        }
    }

    /// Go: `isContentNodeBranch()`.
    // Go: hugolib/page.go:isContentNodeBranch
    pub fn is_content_node_branch(&self) -> bool {
        self.meta.is_node()
    }

    /// Go: `resetBuildState()` (nothing to do).
    // Go: hugolib/page.go:resetBuildState
    pub fn reset_build_state(&self) {}

    /// Go: `Key()` — `"page-" + pid`.
    // Go: hugolib/page.go:Key
    pub fn key(&self) -> String {
        format!("page-{}", self.pid)
    }

    /// Go: `IdentifierBase()` — the path.
    // Go: hugolib/page.go:IdentifierBase
    pub fn identifier_base(&self) -> String {
        self.meta.path()
    }

    /// Go: `String()` — the filename (forward slashes; content adapters add the path), else the
    /// path.
    // Go: hugolib/page.go:String
    pub fn string(&self) -> String {
        match &self.meta.f {
            Some(f) => {
                // The forward slashes even on Windows is motivated by
                // getting stable tests.
                let mut s = f.filename().replace('\\', "/");
                if f.is_content_adapter() {
                    // Also include the path.
                    s.push(':');
                    s.push_str(&self.meta.path());
                }
                s
            }
            None => self.meta.path(),
        }
    }

    /// Go: `TranslationKey()` — the front matter `translationKey`, else the path.
    // Go: hugolib/page.go:TranslationKey
    pub fn translation_key(&self) -> String {
        if !self.meta.page_config.translation_key.is_empty() {
            return self.meta.page_config.translation_key.clone();
        }
        self.meta.path()
    }

    /// Go: `RawContent()` — the source after the front matter (bytes).
    // Go: hugolib/page.go:RawContent
    pub fn raw_content(&self) -> Vec<u8> {
        let Some(c) = &self.content else {
            return Vec::new();
        };
        if c.pi.items_step2.is_empty() {
            // Go: `itemsStep2 == nil`.
            return Vec::new();
        }
        let mut start = c.pi.pos_main_content;
        if start == -1 {
            start = 0;
        }
        let source = c.pi.content_source();
        source[start as usize..].to_vec()
    }

    /// Go: `errorf(err, format, a...)` — `[lang] page "pathOrTitle": <msg>: <err>` unless `err`
    /// is already a file error.
    // Go: hugolib/page.go:errorf
    pub fn errorf(&self, err: Option<Error>, msg: &str) -> Error {
        if let Some(e) = &err
            && e.pos().is_some()
        {
            // More isn't always better.
            return e.clone();
        }
        let prefix = format!(
            "[{}] page {}: {}",
            self.meta.lang(),
            go_strconv::quote(self.path_or_title()),
            msg
        );
        match err {
            Some(e) => Error::new(format!("{prefix}: {e}")),
            // Go formats a nil error with `%w` as `%!w(<nil>)`.
            None => Error::new(format!("{prefix}: %!w(<nil>)")),
        }
    }
}

/// A handle of the page `id` (Go `*pageState`) of `h`.
pub(crate) fn handle_of(h: &Arc<HugoSites>, id: PageId) -> PageHandle {
    PageHandle {
        h: h.clone(),
        id,
        wrapper: PageWrapper::None,
    }
}

/// Go: `pageSiteAdapter.GetPage(ref)` — the page-relative lookup; a miss is `page.NilPage`.
// Go: hugolib/page.go:GetPage
pub fn page_site_adapter_get_page(p: &PageHandle, r: &str) -> Result<PageRef> {
    let site_idx = p.state().site_idx;
    let context = handle_of(&p.h, p.id).page_ref();
    let res = crate::pagecollections::new_page_finder(&p.h, site_idx).get_page(Some(&context), r);
    let (found, err) = match res {
        Ok(f) => (f, None),
        Err(e) => (None, Some(e)),
    };
    // The nil struct has meaning in some situations, mostly to avoid breaking
    // existing sites doing $nilpage.IsDescendant($p), which will always return
    // false.
    let found = found.unwrap_or_else(|| PageRef(nh_page::page_nop::nil_page()));
    match err {
        Some(e) => Err(e),
        None => Ok(found),
    }
}

/// Go: `RelatedKeywords(cfg)` — `NamedPageMetaValue` of the index name, converted to keywords.
// Go: hugolib/page.go:RelatedKeywords
pub fn related_keywords(
    p: &PageHandle,
    cfg: &nh_page::related::IndexConfig,
) -> Result<Vec<nh_page::related::Keyword>> {
    let v = nh_page::page::named_page_meta_value(p, &cfg.name)?;
    match v {
        None => Ok(Vec::new()),
        Some(v) => cfg.to_keywords(&v),
    }
}

/// Go: `Eq(other)` — whether `other` (unwrapped) is this `*pageState`.
// Go: hugolib/page.go:Eq
pub fn eq(p: &PageHandle, other: &Value) -> bool {
    let pp = match crate::page_unwrap::unwrap_page(other) {
        Ok(pp) => pp,
        Err(_) => return false,
    };
    match pp {
        None => false,
        Some(pp) => {
            pp.0.as_any()
                .downcast_ref::<PageHandle>()
                .is_some_and(|o| Arc::ptr_eq(&o.h, &p.h) && o.id == p.id)
        }
    }
}

/// Go: `GetTerms(taxonomy)` — the page's terms in the taxonomy, ordered as in front matter.
// Go: hugolib/page.go:GetTerms
pub fn get_terms(p: &PageHandle, taxonomy: &str) -> Pages {
    let ps = p.state();
    PageMap::get_terms_for_page_in_taxonomy(&p.h, ps.site_idx, &ps.meta.path(), taxonomy)
}

/// Go: `RegularPagesRecursive()`.
// Go: hugolib/page.go:RegularPagesRecursive
pub fn regular_pages_recursive(p: &PageHandle) -> Pages {
    let ps = p.state();
    match ps.meta.kind() {
        kinds::KIND_SECTION | kinds::KIND_HOME => PageMap::get_pages_in_section(
            &p.h,
            ps.site_idx,
            &PageMapQueryPagesInSection {
                path: ps.meta.path(),
                include: Some(pp::and(pp::should_list_local(), vec![pp::kind_page()])),
                recursive: true,
                ..Default::default()
            },
        ),
        _ => regular_pages(p),
    }
}

/// Go: `PagesRecursive()` — always nil.
// Go: hugolib/page.go:PagesRecursive
pub fn pages_recursive(_p: &PageHandle) -> Option<Pages> {
    None
}

/// Go: `RegularPages()` — `None` is Go's nil (regular pages have none).
// Go: hugolib/page.go:RegularPages
pub fn regular_pages_opt(p: &PageHandle) -> Option<Pages> {
    let ps = p.state();
    let si = ps.site_idx;
    let path = ps.meta.path();
    match ps.meta.kind() {
        kinds::KIND_PAGE => None,
        kinds::KIND_SECTION | kinds::KIND_HOME | kinds::KIND_TAXONOMY => {
            Some(PageMap::get_pages_in_section(
                &p.h,
                si,
                &PageMapQueryPagesInSection {
                    path,
                    include: Some(pp::and(pp::should_list_local(), vec![pp::kind_page()])),
                    ..Default::default()
                },
            ))
        }
        kinds::KIND_TERM => Some(PageMap::get_pages_with_term(
            &p.h,
            si,
            &PageMapQueryPagesBelowPath {
                path,
                include: Some(pp::and(pp::should_list_local(), vec![pp::kind_page()])),
                ..Default::default()
            },
        )),
        _ => Some(crate::site::site_regular_pages(&p.h, si)),
    }
}

/// Go: `RegularPages()` (nil as empty).
pub fn regular_pages(p: &PageHandle) -> Pages {
    regular_pages_opt(p).unwrap_or_default()
}

/// Go: `Pages()` — `None` is Go's nil (regular pages have none).
// Go: hugolib/page.go:Pages
pub fn pages_opt(p: &PageHandle) -> Option<Pages> {
    let ps = p.state();
    let si = ps.site_idx;
    let path = ps.meta.path();
    match ps.meta.kind() {
        kinds::KIND_PAGE => None,
        kinds::KIND_SECTION | kinds::KIND_HOME => Some(PageMap::get_pages_in_section(
            &p.h,
            si,
            &PageMapQueryPagesInSection {
                path,
                key_part: "page-section".into(),
                include: Some(pp::and(
                    pp::should_list_local(),
                    vec![pp::or(pp::kind_page(), vec![pp::kind_section()])],
                )),
                ..Default::default()
            },
        )),
        kinds::KIND_TERM => Some(PageMap::get_pages_with_term(
            &p.h,
            si,
            &PageMapQueryPagesBelowPath {
                path,
                ..Default::default()
            },
        )),
        kinds::KIND_TAXONOMY => Some(PageMap::get_pages_in_section(
            &p.h,
            si,
            &PageMapQueryPagesInSection {
                path,
                key_part: "term".into(),
                include: Some(pp::and(pp::should_list_local(), vec![pp::kind_term()])),
                recursive: true,
                ..Default::default()
            },
        )),
        _ => Some(crate::site::site_pages(&p.h, si)),
    }
}

/// Go: `Pages()` (nil as empty).
pub fn pages(p: &PageHandle) -> Pages {
    pages_opt(p).unwrap_or_default()
}

/// Go: `Resources()`.
// Go: hugolib/page.go:Resources
pub fn resources(p: &PageHandle) -> nh_resource::resourcetypes::Resources {
    PageMap::get_or_create_resources_for_page(&p.h, p.id)
}

/// Go: `Site()` — the page's `page.Site` (`*page.siteWrapper`).
// Go: hugolib/page.go:Site
pub fn site(p: &PageHandle) -> nh_page::site::SiteRef {
    crate::site::SiteHandle {
        h: p.h.clone(),
        idx: p.state().site_idx,
    }
    .site_ref()
}

/// Go: `IsTranslated()`.
// Go: hugolib/page.go:IsTranslated
pub fn is_translated(p: &PageHandle) -> bool {
    translations(p).is_some_and(|t| !t.is_empty())
}

/// Go: `AllTranslations()` — all translations including the page, sorted by language
/// (cached in `cachePages2` under `path + "/translations-all"`).
// Go: hugolib/page.go:AllTranslations
pub fn all_translations(p: &PageHandle) -> Option<Pages> {
    let ps = p.state();
    let h = &p.h;
    let m = &h.sites[ps.site_idx].page_map;
    let key = format!("{}/translations-all", ps.meta.path());
    // This is called from Translations, so we need to use a different partition, cachePages2,
    // to avoid potential deadlocks.
    let res = m.get_or_create_pages_from_cache(Some(&m.cache_pages2), key, |_| {
        if !ps.meta.page_config.translation_key.is_empty() {
            // translationKey set by user.
            let pas = h
                .translation_key_pages
                .get(&ps.meta.page_config.translation_key)
                .cloned()
                .unwrap_or_default();
            let mut pasc: Pages = pas
                .into_iter()
                .map(|id| handle_of(h, id).page_ref())
                .collect();
            nh_page::pages_sort::sort_by_language(&mut pasc);
            return Ok(pasc);
        }
        let mut ids: Vec<PageId> = Vec::new();
        h.page_trees.tree_pages.for_each_in_dimension(
            &ps.meta.path(),
            nh_doctree::dimensions::DIMENSION_LANGUAGE,
            &mut |n| {
                if let Some(id) = n.page_id() {
                    ids.push(id);
                }
                false
            },
        );

        let link = pp::should_link();
        let mut pas: Pages = ids
            .into_iter()
            .filter(|id| link(h.page(*id)))
            .map(|id| handle_of(h, id).page_ref())
            .collect();
        nh_page::pages_sort::sort_by_language(&mut pas);
        Ok(pas)
    });
    match res {
        // Go's `var pas page.Pages` stays nil when the tree has no node for the path in any
        // language (`ShouldLink.Filter` keeps a non-nil empty slice otherwise, and the
        // translationKey branch is a `make`). The cache holds `Pages`, so the nil case is
        // recomputed here.
        Ok(pas)
            if pas.is_empty()
                && ps.meta.page_config.translation_key.is_empty()
                && !has_dimension_nodes(h, &ps.meta.path()) =>
        {
            None
        }
        Ok(pas) => Some(pas),
        Err(err) => panic!("{}", err.message()),
    }
}

fn has_dimension_nodes(h: &HugoSites, path: &str) -> bool {
    let mut found = false;
    h.page_trees.tree_pages.for_each_in_dimension(
        path,
        nh_doctree::dimensions::DIMENSION_LANGUAGE,
        &mut |n| {
            found |= n.page_id().is_some();
            found
        },
    );
    found
}

/// Go: `Translations()` — the translations excluding the page (`None` is Go's nil: none).
// Go: hugolib/page.go:Translations
pub fn translations(p: &PageHandle) -> Option<Pages> {
    let ps = p.state();
    let m = &p.h.sites[ps.site_idx].page_map;
    let key = format!("{}/translations", ps.meta.path());
    let res = m.get_or_create_pages_from_cache(None, key, |_| {
        let mut pas: Pages = Vec::new();
        for pp in all_translations(p).unwrap_or_default() {
            if !page_eq(&pp, p) {
                pas.push(pp);
            }
        }
        Ok(pas)
    });
    match res {
        Ok(pas) if pas.is_empty() => None,
        Ok(pas) => Some(pas),
        Err(err) => panic!("{}", err.message()),
    }
}

/// `pp.Eq(p)` for a page of the collections (a `*pageState` or the nop page).
fn page_eq(pp: &PageRef, p: &PageHandle) -> bool {
    pp.0.as_any()
        .downcast_ref::<PageHandle>()
        .is_some_and(|o| Arc::ptr_eq(&o.h, &p.h) && o.id == p.id)
}

/// Go: `AlternativeOutputFormats()` — the page's output formats except the current one and the
/// `notAlternative` ones (`None` is Go's nil).
// Go: hugolib/page.go:AlternativeOutputFormats
pub fn alternative_output_formats(p: &PageHandle) -> Option<OutputFormats> {
    let ps = p.state();
    let f = ps.output_format();
    let mut o: Option<OutputFormats> = None;
    for of in output_formats(p) {
        if of.format.not_alternative || of.format.name == f.name {
            continue;
        }
        o.get_or_insert_with(Vec::new).push(of);
    }
    o
}

/// Go: `OutputFormats()` (the `OutputFormatsProvider` set by `initCommonProviders`: the page
/// paths' output formats).
// Go: hugolib/page__paths.go:OutputFormats
pub fn output_formats(p: &PageHandle) -> OutputFormats {
    match p.state().lazy.get() {
        Some(Ok(l)) => l.paths.output_formats().clone(),
        _ => Vec::new(),
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
    /// `*hugolib.pageForShortcode` (`.Page` inside shortcodes). Method set = `PageWithoutContent`,
    /// `TableOfContents` (returns the TOC placeholder), NopPage `Markup`/`Content` providers and
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
// `impl nh_page::page::Page for PageHandle` are in tplapi/page_methods.rs.

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/page.go (785 lines; 25/59 funcs executed) — initCommonProviders, initPage,
//          shiftToOutputFormat, (in)crPageOutputTemplateVariation/canReusePageOutputContent -> page__init.rs (T21)
//   types: pageContext, pageSiteAdapter, pageState, pageHeadingsFiltered, renderStringOpts, pageWithOrdinal,
//          pageWithWeight0
// OK L82-92: (pa pageSiteAdapter) GetPage(ref string) (page.Page, error)
// OK L125-127: (p *pageState) IdentifierBase() string
//    L129-131: (p *pageState) GetIdentity() identity.Identity  [unsupported: no identity tracking]
//    L133-135: (p *pageState) ForEeachIdentity(f func(identity.Identity) bool) bool  [unsupported]
//    L137-139: (p *pageState) GetDependencyManager() identity.Manager  [unsupported]
//    L141-150: (p *pageState) GetDependencyManagerForScope(scope int) identity.Manager  [unsupported]
//    L152-154: (p *pageState) GetDependencyManagerForScopesAll() []identity.Manager  [unsupported]
// OK L156-158: (p *pageState) Key() string
// OK L161-172: (p *pageState) RelatedKeywords(cfg related.IndexConfig) ([]related.Keyword, error)
// OK L174-176: (p *pageState) resetBuildState()
// OK L178-189: (p *pageState) skipRender() bool
// OK L191-198: (po *pageState) isRenderedAny() bool
// OK L200-202: (p *pageState) isContentNodeBranch() bool
// OK L206-213: (p *pageState) Eq(other any) bool
// OK L215-217: (p *pageState) HeadingsFiltered(context.Context) tableofcontents.Headings  [tplapi/page_methods.rs]
// OK L224-226: (p *pageHeadingsFiltered) HeadingsFiltered(context.Context) tableofcontents.Headings  [tplapi/headings_filtered.rs]
// OK L228-230: (p *pageHeadingsFiltered) page() page.Page  [tplapi/headings_filtered.rs]
// OK L233-240: (p *pageState) ApplyFilterToHeadings(ctx context.Context, fn func(*tableofcontents.Heading) bool) related.Document  [tplapi/page_methods.rs]
// OK L242-244: (p *pageState) GitInfo() *source.GitInfo  [nil: enableGitInfo is an explicit error]
// OK L246-248: (p *pageState) CodeOwners() []string  [nil, see gitinfo.rs]
// OK L252-254: (p *pageState) GetTerms(taxonomy string) page.Pages
//    L256-258: (p *pageState) MarshalJSON() ([]byte, error)  [unsupported: page.MarshalPageToJSON]
// OK L260-275: (p *pageState) RegularPagesRecursive() page.Pages
// OK L277-279: (p *pageState) PagesRecursive() page.Pages
// OK L281-304: (p *pageState) RegularPages() page.Pages
// OK L306-342: (p *pageState) Pages() page.Pages
// OK L346-359: (p *pageState) RawContent() string
// OK L361-363: (p *pageState) Resources() resource.Resources
// OK L365-371: (p *pageState) HasShortcode(name string) bool  [T22: PageState::has_shortcode, page__per_output.rs]
// OK L373-375: (p *pageState) Site() page.Site
// OK L377-394: (p *pageState) String() string
// OK L398-400: (p *pageState) IsTranslated() bool
// OK L403-408: (p *pageState) TranslationKey() string
// OK L411-443: (p *pageState) AllTranslations() page.Pages
// OK L446-461: (p *pageState) Translations() page.Pages
// OK L480-492: (po *pageOutput) GetInternalTemplateBasePathAndDescriptor() (string, tplimpl.TemplateDescriptor)  [T22: PageOutput, page__output.rs]
// OK L494-514: (p *pageState) resolveTemplate(layouts ...string) (*tplimpl.TemplInfo, bool, error)
// OK L524-554: (p *pageState) renderResources() error
// OK L556-567: (p *pageState) AlternativeOutputFormats() page.OutputFormats
// OK L579-590: (p *pageMeta) wrapError(err error, sourceFs afero.Fs) error  [T22: page__per_output.rs]
// OK L593-595: (p *pageState) wrapError(err error) error  [T22: page__per_output.rs]
// OK L597-603: (p *pageState) getPageInfoForError() string  [T22: page__per_output.rs]
// OK L605-624: (p *pageState) getContentConverter() converter.Converter  [T22: page__per_output.rs]
// OK L626-638: (p *pageState) errorf(err error, format string, a ...any) error
// OK L640-645: (p *pageState) outputFormat() (f output.Format)
// OK L647-650: (p *pageState) parseError(err error, input []byte, offset int) error  [T22: page__per_output.rs]
// OK L652-662: (p *pageState) pathOrTitle() string  [T22: page__per_output.rs]
// OK L664-666: (p *pageState) posFromInput(input []byte, offset int) text.Position  [T22: page__per_output.rs]
// OK L668-670: (p *pageState) posOffset(offset int) text.Position  [T22: page__per_output.rs]
// OK L760-762: (p pageWithOrdinal) Ordinal() int  [tplapi/page_methods.rs]
// OK L764-766: (p pageWithOrdinal) page() page.Page  [Page::unwrap_page]
// OK L773-775: (p pageWithWeight0) Weight0() int  [tplapi/page_methods.rs]
// OK L777-779: (p pageWithWeight0) page() page.Page  [Page::unwrap_page]
// OK L783-785: (p pageWithWeight0) Unwrapv() any  [tplapi/page_methods.rs]
// ---------------------------------------------------------------------------
