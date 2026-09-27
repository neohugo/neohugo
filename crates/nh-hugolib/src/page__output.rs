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

use std::sync::atomic::AtomicU32;
use std::sync::{Arc, Mutex};

use nh_common::Result;
use nh_media::output::output_format::OutputFormat;
use nh_page::page_lazy_contentprovider::LazyContentProvider;

use crate::page__paginator::PagePaginator;
use crate::page__paths::{PagePaths, TargetPathsHolder};
use crate::page__per_output::PageContentOutput;

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
}

impl PageOutput {
    /// Go: `newPageOutput(ps, pp, f, render)`.
    // Go: hugolib/page__output.go:newPageOutput
    pub fn new(ps: &crate::page::PageState, pp: &PagePaths, f: OutputFormat, render: bool) -> PageOutput {
        todo!()
    }

    /// Go: `setContentProvider(cp)` — no-op for `None`; sets `provider` AND `pco`.
    // Go: hugolib/page__output.go:setContentProvider
    pub fn set_content_provider(&self, cp: Option<Arc<PageContentOutput>>) {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/page__output.go (149 lines; 3/6 funcs executed)
//   types: pageOutput
// EX L25-78: newPageOutput( ps *pageState, pp pagePaths, f output.Format, render bool, ) *pageOutput
// EX L114-117: (po *pageOutput) incrRenderState()
//    L120-128: (po *pageOutput) isRendered() bool
//    L130-132: (po *pageOutput) IdentifierBase() string
//    L134-136: (po *pageOutput) GetDependencyManager() identity.Manager
// EX L138-149: (p *pageOutput) setContentProvider(cp *pageContentOutput)
// ---------------------------------------------------------------------------
