//! Port of `hugolib/hugo_sites.go`.
//!
//! Owner: Wave B task T20 (hugolib-capture), crate lead of nh-hugolib.


//! Go `hugolib/hugo_sites.go` + the construction half of `site.go` (`NewHugoSites`,
//! `newHugoSites`, `newSiteRefLinker`; site.go:143-454, 858-867). T20 owns construction because
//! its acceptance test (tree dumps after `process`) runs the real `NewHugoSites` path.
//! `Data()`/`loadData`/`handleDataFile` (`.Site.Data`) are in hugo_sites_data.rs (T23): capture
//! does not need them.
//!
//! Construction needs the template store (shortcode lookup during capture) and therefore a
//! func map. Production passes `None` for `NewHugoSitesCfg::func_map_factory` and gets
//! `nh_tplfuncs::tplimplinit::create_func_map`; T20's tests pass a names-only func map built from
//! a Go fixture (every registered function name bound to a stub), so capture does not wait for
//! T18/T19.
//!
//! Ownership model (HUGO_LAYER.md §4): `HugoSites` owns everything in arenas — sites
//! (`Vec<Site>`, language order), pages (`Vec<PageState>`, indexed by [`PageId`]), the shared
//! content trees keyed like Go's doctree. It is built and assembled through `&mut HugoSites`
//! (process + assemble), then FROZEN into `Arc<HugoSites>` for rendering; from then on only
//! interior-mutable caches change (OnceLock/Mutex/atomics). Template-visible handles
//! (`PageHandle`, `SiteHandle`) are `{ h: Arc<HugoSites>, index }` — the Go pointers.

use std::sync::atomic::AtomicUsize;
use std::sync::{Arc, OnceLock};

use go_value::Map;
use nh_allconfig::allconfig::Configs;
use nh_common::dynacache::Partition;
use nh_common::Result;
use nh_config::neohugo::neohugo::HugoInfo;
use nh_deps::deps::Deps;
use nh_media::output::output_format::Formats;
use nh_page::page::Pages;

use crate::content_map_trees::PageTrees;
use crate::page::{PageId, PageState};
use crate::site::Site;

/// Go: `hugolib.HugoSites`.
pub struct HugoSites {
    /// Sites in language order: default content language first, then weight, then lang.
    pub sites: Vec<Site>,
    pub configs: Arc<Configs>,
    pub hugo_info: HugoInfo,
    /// Render formats of ALL sites concatenated (page outputs are indexed by this global index).
    pub render_formats: Formats,
    /// Go `h.currentSite` — index of the site being rendered (collator of `Site.Current()`).
    pub current_site: AtomicUsize,
    /// The first site's deps (shared services).
    pub deps: Arc<Deps>,
    /// The page arena.
    pub pages: Vec<PageState>,
    /// The shared content trees (pages, resources, taxonomy entries).
    pub page_trees: PageTrees,
    /// Go `h.init.data` — `.Site.Data` from the data component (lazy).
    pub data: OnceLock<Arc<Map>>,
    /// Go `cachePages` ("/pags/all"; global lists, e.g. `HugoSites.Pages()`). Dynacache
    /// pattern: never compute under the lock, first stored value wins (HUGO_LAYER.md §4.8).
    pub cache_pages: Partition<String, Pages>,
    /// Test seam for every template execution nh-hugolib starts (template_exec.rs); `None` in a
    /// normal build.
    pub template_executor: Option<Arc<dyn crate::template_exec::TemplateExecutor>>,
    /// Set once frozen: a weak self-reference for creating handles.
    pub(crate) self_ref: OnceLock<std::sync::Weak<HugoSites>>,
}

impl HugoSites {
    /// Go: `hugolib.NewHugoSites(cfg deps.DepsCfg)` — creates deps (first site `Init`, others
    /// `Clone`), sites (publisher, frontmatter handler, related docs handler, page map), the template
    /// store (+ `WithSiteOpts` per site) and the i18n translators.
    // Go: hugolib/site.go:NewHugoSites
    pub fn new(cfg: NewHugoSitesCfg) -> Result<HugoSites> {
        todo!()
    }

    /// Freezes the build state for rendering and wires the `OnceLock`s that need handles
    /// (deps.site, template store site options).
    pub fn freeze(self) -> Arc<HugoSites> {
        todo!()
    }

    pub fn page(&self, id: PageId) -> &PageState {
        &self.pages[id.0 as usize]
    }

    pub fn page_mut(&mut self, id: PageId) -> &mut PageState {
        &mut self.pages[id.0 as usize]
    }

    /// Go: `HugoSites.Pages()` — all sites' `Pages()`, then `SortByDefault` (stable).
    // Go: hugolib/hugo_sites.go:Pages
    pub fn all_pages(self: &Arc<Self>) -> Pages {
        todo!()
    }

    /// Go: `HugoSites.RegularPages()`.
    // Go: hugolib/hugo_sites.go:RegularPages
    pub fn all_regular_pages(self: &Arc<Self>) -> Pages {
        todo!()
    }

    /// Go: `(s *Site) preparePagesForRender(isRenderingSite, idx)` — `shiftToOutputFormat` on every
    /// page of site `site_idx` (walk order), called for ALL sites before each render format.
    /// (Thin loop owned here; `shift_to_output_format` is T21's page__init.rs; tested by T24.)
    // Go: hugolib/hugo_sites.go:preparePagesForRender
    pub fn prepare_pages_for_render(self: &Arc<Self>, site_idx: usize, is_rendering_site: bool, idx: usize) -> Result<()> {
        todo!()
    }

    /// Go: `(h *HugoSites) resolveSite(lang)`.
    // Go: hugolib/hugo_sites.go:resolveSite
    pub fn resolve_site(&self, lang: &str) -> Option<usize> {
        self.sites.iter().position(|s| s.language.lang == lang)
    }
}

/// Builds a site's template func map (Go `tplimplinit.CreateFuncMap(s.Deps)`).
pub type FuncMapFactory = Arc<dyn Fn(&Arc<Deps>) -> nh_tplimpl::engine::FuncMap + Send + Sync>;

/// Inputs of `NewHugoSites` (Go: `deps.DepsCfg`).
pub struct NewHugoSitesCfg {
    pub configs: Arc<Configs>,
    pub fs: nh_hugofs::fs::Fs,
    pub log: nh_common::loggers::Logger,
    /// `None` = `nh_tplfuncs::tplimplinit::create_func_map` (production). Tests may pass a
    /// names-only func map (see the module docs).
    pub func_map_factory: Option<FuncMapFactory>,
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/hugo_sites.go (616 lines; 18/30 funcs executed) — Data/loadData/handleDataFile/readData
//          -> hugo_sites_data.rs (T23)
//   types: HugoSites, buildCounters, fatalErrorHandler, hugoSitesInit, BuildCfg
//    L109-113: (h *HugoSites) ShouldSkipFileChangeEvent(ev fsnotify.Event) bool
// EX L115-117: (h *HugoSites) Close() error
// EX L119-121: (h *HugoSites) isRebuild() bool
// EX L123-135: (h *HugoSites) resolveSite(lang string) *Site
// EX L142-147: (c *buildCounters) loggFields() logg.Fields
//    L162-170: (f *fatalErrorHandler) FatalError(err error)
// EX L172-176: (f *fatalErrorHandler) getErr() error
// EX L178-180: (f *fatalErrorHandler) Done() <-chan bool
// EX L199-213: (h *HugoSites) Pages() page.Pages
//    L216-231: (h *HugoSites) RegularPages() page.Pages
// EX L233-243: (h *HugoSites) gitInfoForPage(p page.Page) (*source.GitInfo, error)
// EX L245-255: (h *HugoSites) codeownersForPage(p page.Page) ([]string, error)
// EX L257-289: (h *HugoSites) pickOneAndLogTheRest(errors []error) error
//    L291-293: (h *HugoSites) isMultilingual() bool
//    L296-302: (h *HugoSites) LanguageSet() map[string]int
//    L304-309: (h *HugoSites) NumLogErrors() int
// EX L311-317: (h *HugoSites) PrintProcessingStats(w io.Writer)
//    L321-346: (h *HugoSites) GetContentPage(filename string) page.Page
// EX L348-365: (h *HugoSites) loadGitInfo() error
//    L368-373: (h *HugoSites) reset(config *BuildCfg)
//    L376-381: (h *HugoSites) resetLogs()
//    L383-390: (h *HugoSites) withSite(fn func(s *Site) error) error
//    L392-403: (h *HugoSites) withPage(fn func(s string, p *pageState) bool)
// EX L432-479: (cfg *BuildCfg) shouldRender(infol logg.LevelLogger, p *pageState) bool
// EX L481-496: (s *Site) preparePagesForRender(isRenderingSite bool, idx int) error
// Source: hugolib/site.go (construction only; the rest of site.go is in site.rs and others)
// EX L143-335: NewHugoSites(cfg deps.DepsCfg) (*HugoSites, error)
// EX L337-454: newHugoSites(cfg deps.DepsCfg, d *deps.Deps, pageTrees *pageTrees, sites []*Site) (*HugoSites, error)
// EX L858-867: newSiteRefLinker(s *Site) (siteRefLinker, error)
// ---------------------------------------------------------------------------
