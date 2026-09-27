//! Port of `hugolib/page__meta.go` (`initLazyProviders`) + `hugolib/page.go` (`initPage`,
//! `initCommonProviders`, `shiftToOutputFormat`, output template variations).
//!
//! Owner: Wave B task T21 (hugolib-assemble).


//! Split out so that the page-output lifecycle has one owner that runs before rendering:
//! `assembleResources` (T21) already calls `shiftToOutputFormat(true, 0)` on every page, so the
//! assembly task owns the lazy page init and the shifting. It uses T22's constructors
//! (`PageOutput::new`, `PageContentOutput::new`) and T21's `new_page_paths`.
//!
//! `init_page` (Go `initLazyProviders` closure, page__meta.go:884-947, run by `initPage`):
//! 1. `pp = newPagePaths(ps)`;
//! 2. `renderFormats` = `h.render_formats` (all sites) or, for standalone pages (404, sitemap,
//!    robots...), just `[standaloneOutputFormat]`;
//! 3. one `PageOutput` per render format NAME (Go `created := map[string]*pageOutput`): slots
//!    with the same name share the same `Arc<PageOutput>`; `render` = `!noRender()` and the page
//!    has the format; the FIRST slot also gets `newPageContentOutput` as its content provider;
//! 4. `initCommonProviders(pp)` (target path descriptor, `OutputFormatsProvider`, positions).
//!
//! `shift_to_output_format(is_rendering_site, idx)` (page.go:675-747), port exactly:
//! * `init_page()`; a page with ONE output always uses index 0;
//! * set `current_output_idx`;
//! * if `is_rendering_site` and the current output has a BUILT paginator: `paginator.reset()`;
//! * rendering site: `cp = po.pco`; if none and `canReusePageOutputContent()` take the first other
//!   output's `pco`; if still none `newPageContentOutput(po)`; `po.set_content_provider(cp)`;
//! * other sites: if the current output's provider is a `LazyContentProvider`, `reset()` it; else
//!   install a new `LazyContentProvider` (creating `newPageContentOutput(po)` on first use) as the
//!   output's provider — `pco` itself is NOT touched. Because outputs are shared by name, this
//!   replaces the provider of the same `PageOutput` the page's own site used; the rendering-site
//!   branch installs a real `pco` again the next time the page's site renders.

use std::sync::Arc;

use nh_common::Result;

use crate::hugo_sites::HugoSites;
use crate::page::{PageLazy, PageState};

impl PageState {
    /// Go: `initPage()` -> `ps.init.Do()` -> the `initLazyProviders` closure. Runs once; the
    /// result (or error) is cached in `self.lazy`.
    // Go: hugolib/page.go:initPage
    pub fn init_page(&self, h: &HugoSites) -> Result<&PageLazy> {
        todo!()
    }

    /// Go: `shiftToOutputFormat(isRenderingSite, idx)` — see the module docs.
    // Go: hugolib/page.go:shiftToOutputFormat
    pub fn shift_to_output_format(&self, h: &HugoSites, is_rendering_site: bool, idx: usize) -> Result<()> {
        todo!()
    }

    /// Go: `incrPageOutputTemplateVariation()`.
    // Go: hugolib/page.go:incrPageOutputTemplateVariation
    pub fn incr_page_output_template_variation(&self) {
        self.page_output_template_variations_state.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    /// Go: `canReusePageOutputContent()` — `pageOutputTemplateVariationsState == 1`.
    // Go: hugolib/page.go:canReusePageOutputContent
    pub fn can_reuse_page_output_content(&self) -> bool {
        self.page_output_template_variations_state.load(std::sync::atomic::Ordering::SeqCst) == 1
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/page__meta.go (initLazyProviders only)
// EX L884-948: (ps *pageState) initLazyProviders() error
// Source: hugolib/page.go (page init + output shifting only)
// EX L117-119: (p *pageState) incrPageOutputTemplateVariation()
// EX L121-123: (p *pageState) canReusePageOutputContent() bool
// EX L463-477: (ps *pageState) initCommonProviders(pp pagePaths) error
// EX L517-522: (p *pageState) initPage() error
// EX L675-747: (p *pageState) shiftToOutputFormat(isRenderingSite bool, idx int) error
// ---------------------------------------------------------------------------
