//! Port of `hugolib/page__output.go`.
//!
//! Owner: Wave B task T22 (hugolib-content).

//! Go `pageOutput`: one per render format NAME of a page (shared by every global format index
//! with that name, see `crate::page::PageLazy`). It carries the per-output state that
//! `shiftToOutputFormat` (page__init.rs, T21) switches:
//! * `paginator` (only when `render && ps.IsNode()`): T23's type, RESETTABLE (Go replaces the
//!   `pagePaginatorInit` when a built paginator is shifted to on the rendering site);
//! * `pco`: the real content output, once set by `set_content_provider` (Go `po.pco`);
//! * `provider`: the CURRENT content provider (Go's embedded `ContentProvider`,
//!   `MarkupProvider`, `PageRenderProvider`, `TableOfContentsProvider`, `contentRenderer`):
//!   Nop until a provider is set, the `pco` on the rendering site, or a resettable
//!   `LazyContentProvider` while another site renders.
//!
//! Locks here are held only to read or swap an `Arc`; never while rendering (HUGO_LAYER.md §4.8).
//!
//! Go's `po.p` back pointer is `p` (the page id) plus `hs`, the `HugoSites` self-reference cell
//! shared with `HugoSites` (set by `freeze`): page outputs are created during assembly, before
//! the `Arc<HugoSites>` exists, and content rendering (after freezing) reaches the frozen
//! `HugoSites` through it.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, OnceLock, Weak};

use nh_common::Result;
use nh_markup::converter::converter::Converter;
use nh_media::output::output_format::OutputFormat;
use nh_page::page_lazy_contentprovider::LazyContentProvider;
use nh_tplimpl::templatedescriptor::TemplateDescriptor;

use crate::hugo_sites::HugoSites;
use crate::page::{PageId, PageState};
use crate::page__paginator::PagePaginator;
use crate::page__paths::{PagePaths, TargetPathsHolder};
use crate::page__per_output::PageContentOutput;

/// The `HugoSites` self-reference cell (`HugoSites.self_ref`): empty until `freeze`.
pub type HugoSitesRef = Arc<OnceLock<Weak<HugoSites>>>;

/// Upgrades the self-reference. Content rendering runs after `HugoSites::freeze` (an
/// invariant), so a missing reference is a bug.
pub(crate) fn upgrade_hs(hs: &HugoSitesRef) -> Arc<HugoSites> {
    hs.get()
        .and_then(|w| w.upgrade())
        .expect("content rendering before HugoSites::freeze")
}

/// The provider slot of a page output (Go: the embedded provider interfaces of `pageOutput`).
#[derive(Clone, Default)]
pub enum ContentProviderSlot {
    /// `page.NopPage` (initial state).
    #[default]
    Nop,
    /// The real content output (`setContentProvider(cp)`).
    Pco(Arc<PageContentOutput>),
    /// Installed by `shiftToOutputFormat` for pages of other sites; reset on every such shift.
    Lazy(Arc<LazyContentProvider>),
}

impl ContentProviderSlot {
    /// The content output behind the slot: the `pco`, or the lazy provider's (created on first
    /// use, Go `lcp.init.Do`); `None` for the nop provider.
    pub fn resolve(&self) -> Option<Arc<PageContentOutput>> {
        match self {
            ContentProviderSlot::Nop => None,
            ContentProviderSlot::Pco(p) => Some(p.clone()),
            ContentProviderSlot::Lazy(l) => {
                let cp = l.current()?;
                pco_from_provider(cp)
            }
        }
    }
}

/// Downcasts a lazily created provider (always a `PageContentOutput` in nh-hugolib).
fn pco_from_provider(
    cp: Arc<dyn nh_page::page_lazy_contentprovider::OutputFormatContentProvider>,
) -> Option<Arc<PageContentOutput>> {
    let pco = cp.as_any().downcast_ref::<PageContentOutput>()?;
    pco.self_arc()
}

/// Go: `pageOutput`.
pub struct PageOutput {
    pub f: OutputFormat,
    /// Whether this page should be rendered in this format (it has the format configured).
    pub render: bool,
    /// Go `targetPathsProvider`/`linksProvider`: `pp.targetPaths[f.Name]`, else the first
    /// output format's.
    pub target_paths: TargetPathsHolder,
    /// Go `paginator`: `Some` only if `render && ps.IsNode()`, else pagination is an error
    /// ("pagination not supported for this page: ...").
    pub paginator: Option<PagePaginator>,
    /// Go `pco` (may stay `None`).
    pub pco: Mutex<Option<Arc<PageContentOutput>>>,
    /// Go: the current provider (see the module docs).
    pub provider: Mutex<ContentProviderSlot>,
    /// Go `renderState` (`incrRenderState`).
    pub render_state: AtomicU32,
    /// Go `renderOnce`.
    pub render_once: AtomicBool,
    /// Go `po.p`: the page.
    pub p: PageId,
    /// The site of the page (Go `po.p.s`).
    pub site_idx: usize,
    /// The `HugoSites` self-reference (see the module docs).
    pub hs: HugoSitesRef,
    /// Go `pageState.contentConverter` (`contentConverterInit`): the page's markup converter.
    /// Deviation: kept per page OUTPUT (Go: per page); the converter is stateless between
    /// conversions, so every output renders the same bytes.
    pub(crate) content_converter: OnceLock<Arc<dyn Converter>>,
}

impl PageOutput {
    /// Go: `newPageOutput(ps, pp, f, render)`. Takes `h` for the self-reference cell (Go's
    /// `po.p.s.h` pointer chain).
    // Go: hugolib/page__output.go:newPageOutput
    pub fn new(
        h: &HugoSites,
        ps: &PageState,
        pp: &PagePaths,
        f: OutputFormat,
        render: bool,
    ) -> PageOutput {
        let ft = match pp.target_paths.get(&f.name) {
            Some(ft) => ft.clone(),
            // Link to the main output format
            None => pp
                .target_paths
                .get(&pp.first_output_format.format.name)
                .cloned()
                .unwrap_or_else(zero_target_paths),
        };
        PageOutput::new_with_target_paths(h, ps, ft, f, render)
    }

    /// `newPageOutput` with the target paths already resolved (the T22 tests construct page
    /// outputs without T21's `newPagePaths`).
    // Go: hugolib/page__output.go:newPageOutput
    pub fn new_with_target_paths(
        h: &HugoSites,
        ps: &PageState,
        target_paths: TargetPathsHolder,
        f: OutputFormat,
        render: bool,
    ) -> PageOutput {
        let paginator = if render && ps.meta.is_node() {
            Some(PagePaginator::default())
        } else {
            // Go: `page.PaginatorNotSupportedFunc` ("pagination not supported for this page").
            None
        };

        PageOutput {
            f,
            render,
            target_paths,
            paginator,
            pco: Mutex::new(None),
            provider: Mutex::new(ContentProviderSlot::Nop),
            render_state: AtomicU32::new(0),
            render_once: AtomicBool::new(false),
            p: ps.id,
            site_idx: ps.site_idx,
            hs: h.self_ref.clone(),
            content_converter: OnceLock::new(),
        }
    }

    /// Go: `incrRenderState()`.
    // Go: hugolib/page__output.go:incrRenderState
    pub fn incr_render_state(&self) {
        self.render_state.fetch_add(1, Ordering::SeqCst);
        self.render_once.store(true, Ordering::SeqCst);
    }

    /// Go: `isRendered()` — this output format or its content has been rendered.
    // Go: hugolib/page__output.go:isRendered
    pub fn is_rendered(&self) -> bool {
        if self.render_state.load(Ordering::SeqCst) > 0 {
            return true;
        }
        if let Some(pco) = self.pco()
            && pco.content_rendered.load(Ordering::SeqCst)
        {
            return true;
        }
        false
    }

    /// Go: `setContentProvider(cp)` — no-op for `None`; sets `provider` AND `pco`.
    // Go: hugolib/page__output.go:setContentProvider
    pub fn set_content_provider(&self, cp: Option<Arc<PageContentOutput>>) {
        let Some(cp) = cp else {
            return;
        };
        *self.provider.lock().unwrap_or_else(|e| e.into_inner()) =
            ContentProviderSlot::Pco(cp.clone());
        *self.pco.lock().unwrap_or_else(|e| e.into_inner()) = Some(cp);
    }

    /// Go `po.pco` (may be nil).
    pub fn pco(&self) -> Option<Arc<PageContentOutput>> {
        self.pco.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// The current provider slot (a clone; the lock is released before any rendering).
    pub fn provider_slot(&self) -> ContentProviderSlot {
        self.provider
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Go `po.contentRenderer` (and the other embedded providers): the content output behind
    /// the current provider, `None` for the nop provider.
    pub fn content_renderer(&self) -> Option<Arc<PageContentOutput>> {
        self.provider_slot().resolve()
    }

    /// Go: `(po *pageOutput) GetInternalTemplateBasePathAndDescriptor()` (hugolib/page.go:480-492;
    /// a `pageOutput` method, so it lives with the type): the template base path
    /// `PathInfo().BaseReTyped(pageConfig.Type)` and the descriptor {Kind, Lang, LayoutFromUser,
    /// OutputFormat, MediaType, IsPlainText} of this output.
    // Go: hugolib/page.go:GetInternalTemplateBasePathAndDescriptor
    pub fn get_internal_template_base_path_and_descriptor(
        &self,
        p: &PageState,
    ) -> (String, TemplateDescriptor) {
        let f = &self.f;
        let base = p.meta.path_info().base_re_typed(&p.meta.page_config.type_);
        (
            base,
            TemplateDescriptor {
                kind: p.meta.kind().to_string(),
                lang: p.meta.lang().to_string(),
                layout_from_user: p.meta.layout().to_string(),
                output_format: f.name.clone(),
                media_type: f.media_type.typ.clone(),
                is_plain_text: f.is_plain_text,
                ..Default::default()
            },
        )
    }
}

/// Go's zero `targetPathsHolder{}` (a missing map entry).
fn zero_target_paths() -> TargetPathsHolder {
    TargetPathsHolder {
        rel_url: String::new(),
        paths: Default::default(),
        output_format: nh_page::page_outputformat::OutputFormat::new(
            "",
            "",
            false,
            OutputFormat::default(),
        ),
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/page__output.go (149 lines; 3/6 funcs executed)
//   types: pageOutput
// OK L25-78: newPageOutput( ps *pageState, pp pagePaths, f output.Format, render bool, ) *pageOutput
// OK L114-117: (po *pageOutput) incrRenderState()
// OK L120-128: (po *pageOutput) isRendered() bool
//    L130-132: (po *pageOutput) IdentifierBase() string
//    L134-136: (po *pageOutput) GetDependencyManager() identity.Manager
// OK L138-149: (p *pageOutput) setContentProvider(cp *pageContentOutput)
// ---------------------------------------------------------------------------
