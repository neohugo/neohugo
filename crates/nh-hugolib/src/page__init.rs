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
//!
//! Go's lazy provider closure reads `p.pageOutput` when it runs, i.e. the page's CURRENT output
//! at first use; the port's factory does the same through the frozen `HugoSites` (the lazy
//! providers are installed by `preparePagesForRender`, after freezing).

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use nh_common::Result;
use nh_media::output::output_format::Formats;
use nh_page::page_lazy_contentprovider::{LazyContentProvider, OutputFormatContentProvider};

use crate::hugo_sites::HugoSites;
use crate::page::{PageLazy, PageState};
use crate::page__output::{ContentProviderSlot, PageOutput, upgrade_hs};
use crate::page__paths::{PagePaths, new_page_paths};
use crate::page__per_output::PageContentOutput;

impl PageState {
    /// Go: `initPage()` -> `ps.init.Do()` -> the `initLazyProviders` closure. Runs once; the
    /// result (or error) is cached in `self.lazy`.
    // Go: hugolib/page.go:initPage
    pub fn init_page(&self, h: &HugoSites) -> Result<&PageLazy> {
        match self.lazy.get_or_init(|| self.init_lazy_providers(h)) {
            Ok(l) => Ok(l),
            Err(err) => Err(err.clone()),
        }
    }

    /// Go: `(ps *pageState) initLazyProviders()` — the closure `ps.init` runs: page paths, one
    /// page output per render format NAME (shared by every global index with that name), a
    /// content output for the first, then the common providers.
    // Go: hugolib/page__meta.go:initLazyProviders
    fn init_lazy_providers(&self, h: &HugoSites) -> Result<PageLazy> {
        let ps = self;
        let pp = new_page_paths(h, ps)?;

        let output_formats_for_page: Formats;
        let standalone: Formats;
        let render_formats: &Formats = match &ps.meta.standalone_output_format {
            None => {
                output_formats_for_page = ps.meta.output_formats(&h.sites[ps.site_idx].conf);
                &h.render_formats
            }
            Some(f) => {
                // One of the fixed output format pages, e.g. 404.
                standalone = Formats(vec![f.clone()]);
                output_formats_for_page = standalone.clone();
                &standalone
            }
        };

        // Prepare output formats for all sites.
        // We do this even if this page does not get rendered on
        // its own. It may be referenced via one of the site collections etc.
        // it will then need an output format.
        let mut page_outputs: Vec<Arc<PageOutput>> = Vec::with_capacity(render_formats.0.len());
        let mut created: HashMap<String, Arc<PageOutput>> = HashMap::new();
        let should_render_page = !ps.meta.no_render();

        for (i, f) in render_formats.0.iter().enumerate() {
            if let Some(po) = created.get(&f.name) {
                page_outputs.push(po.clone());
                continue;
            }

            let mut render = should_render_page;
            if render {
                render = output_formats_for_page.get_by_name(&f.name).is_some();
            }

            let po = Arc::new(PageOutput::new(h, ps, &pp, f.clone(), render));

            // Create a content provider for the first,
            // we may be able to reuse it.
            if i == 0 {
                let content_provider = PageContentOutput::new(&po, i)?;
                po.set_content_provider(Some(content_provider));
            }

            page_outputs.push(po.clone());
            created.insert(f.name.clone(), po);
        }

        ps.init_common_providers(&pp)?;

        Ok(PageLazy {
            paths: pp,
            outputs: page_outputs,
        })
    }

    /// Go: `initCommonProviders(pp)` — the target path descriptor; the output formats provider
    /// is `PageLazy.paths`; positions (`posNextPrev`), the ref provider and the sites provider
    /// are T23's lazy parts of `PageCommon`.
    // Go: hugolib/page.go:initCommonProviders
    pub fn init_common_providers(&self, pp: &PagePaths) -> Result<()> {
        // A page without output formats has Go's zero `pagePaths{}` (no targets): its target
        // path descriptor stays the zero one (unset).
        if !pp.target_paths.is_empty() {
            let _ = self
                .common
                .target_path_descriptor
                .set(pp.target_path_descriptor.clone());
        }
        Ok(())
    }

    /// Go: `shiftToOutputFormat(isRenderingSite, idx)` — see the module docs. `idx` refers to
    /// the full set of output formats for all sites (`HugoSites.render_formats`).
    // Go: hugolib/page.go:shiftToOutputFormat
    pub fn shift_to_output_format(
        &self,
        h: &HugoSites,
        is_rendering_site: bool,
        idx: usize,
    ) -> Result<()> {
        let lazy = self.init_page(h)?;

        let idx = if lazy.outputs.len() == 1 { 0 } else { idx };

        self.current_output_idx.store(idx, Ordering::SeqCst);
        let Some(po) = lazy.outputs.get(idx) else {
            panic!(
                "runtime error: index out of range [{idx}] with length {}",
                lazy.outputs.len()
            );
        };

        // Reset any built paginator. This will trigger when re-rendering pages in
        // server mode.
        if is_rendering_site
            && let Some(pag) = &po.paginator
            && pag.is_built()
        {
            pag.reset();
        }

        if is_rendering_site {
            let mut cp = po.pco();
            if cp.is_none() && self.can_reuse_page_output_content() {
                // Look for content to reuse.
                for (i, o) in lazy.outputs.iter().enumerate() {
                    if i == idx {
                        continue;
                    }
                    if let Some(c) = o.pco() {
                        cp = Some(c);
                        break;
                    }
                }
            }

            let cp = match cp {
                Some(cp) => cp,
                None => PageContentOutput::new(po, idx)?,
            };
            po.set_content_provider(Some(cp));
        } else {
            // We attempt to assign pageContentOutputs while preparing each site
            // for rendering and before rendering each site. This lets us share
            // content between page outputs to conserve resources. But if a template
            // unexpectedly calls a method of a ContentProvider that is not yet
            // initialized, we assign a LazyContentProvider that performs the
            // initialization just in time.
            if let ContentProviderSlot::Lazy(lcp) = po.provider_slot() {
                lcp.reset();
            } else {
                // Go's closure reads `p.pageOutput` when it runs: the page's CURRENT output
                // at first use (through the frozen HugoSites).
                let hs = po.hs.clone();
                let id = self.id;
                let lcp = LazyContentProvider::new(Box::new(move || {
                    let h = upgrade_hs(&hs);
                    let ps = h.page(id);
                    let idx = ps.current_output_idx.load(Ordering::SeqCst);
                    let po = ps.current_output().clone();
                    let cp = PageContentOutput::new(&po, idx)?;
                    Ok(cp as Arc<dyn OutputFormatContentProvider>)
                }));
                *po.provider.lock().unwrap_or_else(|e| e.into_inner()) =
                    ContentProviderSlot::Lazy(Arc::new(lcp));
            }
        }

        Ok(())
    }

    /// Go: `incrPageOutputTemplateVariation()`.
    // Go: hugolib/page.go:incrPageOutputTemplateVariation
    pub fn incr_page_output_template_variation(&self) {
        self.page_output_template_variations_state
            .fetch_add(1, Ordering::SeqCst);
    }

    /// Go: `canReusePageOutputContent()` — `pageOutputTemplateVariationsState == 1`.
    // Go: hugolib/page.go:canReusePageOutputContent
    pub fn can_reuse_page_output_content(&self) -> bool {
        self.page_output_template_variations_state
            .load(Ordering::SeqCst)
            == 1
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/page__meta.go (initLazyProviders only)
// OK L884-948: (ps *pageState) initLazyProviders() error
// Source: hugolib/page.go (page init + output shifting only)
// OK L117-119: (p *pageState) incrPageOutputTemplateVariation()
// OK L121-123: (p *pageState) canReusePageOutputContent() bool
// OK L463-477: (ps *pageState) initCommonProviders(pp pagePaths) error
// OK L517-522: (p *pageState) initPage() error
// OK L675-747: (p *pageState) shiftToOutputFormat(isRenderingSite bool, idx int) error
// ---------------------------------------------------------------------------
