//! Port of `hugolib/page__paginator.go`.
//!
//! Owner: Wave B task T23 (hugolib-site).

//! Go `hugolib/page__paginator.go`: `.Paginator` and `.Paginate` share ONE `sync.Once` per page
//! output — the first call builds the pager (head.html calls `.Paginator` first, so
//! `.Paginate (sort ...)` in index.html is IGNORED). `current` is advanced by `renderPaginator`.
//!
//! RESETTABLE: Go's `reset()` replaces the whole `pagePaginatorInit` (a fresh `sync.Once`);
//! `shiftToOutputFormat` calls it when a page output with a BUILT paginator is shifted to on the
//! rendering site (page.go:691-694). The page output is shared by format name (en/html and
//! th/html are the same `PageOutput`), so the reset/keep decisions must follow Go exactly.
//!
//! Go quirk to keep: when the `Once` body fails, only that first call returns the error; later
//! calls return `(p.current, nil)` = `(nil, nil)`.
//!
//! Never compute under the lock (HUGO_LAYER.md §4.8): check `init.done` under the lock, release
//! it, build the paginator (which queries page collections), then store it if still not done
//! (first stored value wins).

use std::sync::{Arc, Mutex};

use go_value::Value;
use nh_common::Result;
use nh_common::herrors::Error;
use nh_page::page::pages_to_value;
use nh_page::pagination::Pager;

use crate::page::PageHandle;

/// Go: `pagePaginatorInit` (`init sync.Once` + `current *page.Pager`).
#[derive(Default)]
pub struct PagePaginatorInit {
    /// The `Once` has run.
    pub done: bool,
    /// The current pager (page 1 after init; renderPaginator moves it to 2..N).
    pub current: Option<Arc<Pager>>,
}

/// Go: `pagePaginator`.
#[derive(Default)]
pub struct PagePaginator {
    pub init: Mutex<PagePaginatorInit>,
}

/// Go: `newPagePaginator(source)` (the source page is passed to each call in the port).
// Go: hugolib/page__paginator.go:newPagePaginator
pub fn new_page_paginator() -> PagePaginator {
    PagePaginator::default()
}

impl PagePaginator {
    /// Go: `reset()` — a fresh `pagePaginatorInit`.
    // Go: hugolib/page__paginator.go:reset
    pub fn reset(&self) {
        *self.lock() = PagePaginatorInit::default();
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, PagePaginatorInit> {
        self.init.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Whether a paginator was built (Go `p.paginator.current != nil`).
    pub fn is_built(&self) -> bool {
        self.lock().current.is_some()
    }

    /// The current pager (Go `p.paginator.current`).
    pub fn current(&self) -> Option<Arc<Pager>> {
        self.lock().current.clone()
    }

    /// Sets the current pager (Go `p.paginator.current = pager`, renderPaginator).
    pub fn set_current(&self, pager: Option<Arc<Pager>>) {
        self.lock().current = pager;
    }

    /// Go's `p.init.Do(body)`: runs `body` if the `Once` has not run (outside the lock), stores
    /// its pager unless another call stored first, and returns `(p.current, initErr)`.
    fn do_once(&self, body: impl FnOnce() -> Result<Arc<Pager>>) -> Result<Option<Arc<Pager>>> {
        {
            let g = self.lock();
            if g.done {
                return Ok(g.current.clone());
            }
        }
        let res = body();
        let mut g = self.lock();
        if g.done {
            // Another call completed the Once first (first stored wins).
            return Ok(g.current.clone());
        }
        g.done = true;
        match res {
            Ok(pager) => {
                g.current = Some(pager);
                Ok(g.current.clone())
            }
            Err(err) => Err(err),
        }
    }

    /// Go: `Paginate(seq, options...)`.
    // Go: hugolib/page__paginator.go:Paginate
    pub fn paginate(
        &self,
        source: &PageHandle,
        seq: &Value,
        options: &[Value],
    ) -> Result<Option<Arc<Pager>>> {
        self.do_once(|| {
            let ps = source.state();
            let conf = &source.h.sites[ps.site_idx].deps.conf;
            let pager_size = nh_page::pagination::resolve_pager_size(&**conf, options)?;

            let pd = paginator_target_path_descriptor(source)?;
            let paginator = nh_page::pagination::paginate(&pd, seq, pager_size)?;

            Ok(paginator.pagers()[0].clone())
        })
    }

    /// Go: `Paginator(options...)` — home: `s.RegularPages()`; term/taxonomy: `Pages()`;
    /// other nodes: `RegularPages()`.
    // Go: hugolib/page__paginator.go:Paginator
    pub fn paginator(&self, source: &PageHandle, options: &[Value]) -> Result<Option<Arc<Pager>>> {
        self.do_once(|| {
            let ps = source.state();
            let conf = &source.h.sites[ps.site_idx].deps.conf;
            let pager_size = nh_page::pagination::resolve_pager_size(&**conf, options)?;

            let pd = paginator_target_path_descriptor(source)?;

            let pages = match ps.meta.kind() {
                nh_common::kinds::KIND_HOME => {
                    // From Hugo 0.57 we made home.Pages() work like any other
                    // section. To avoid the default paginators for the home page
                    // changing in the wild, we make this a special case.
                    crate::site::site_regular_pages(&source.h, ps.site_idx)
                }
                nh_common::kinds::KIND_TERM | nh_common::kinds::KIND_TAXONOMY => {
                    crate::page::pages(source)
                }
                _ => crate::page::regular_pages(source),
            };

            let paginator =
                nh_page::pagination::paginate(&pd, &pages_to_value(&pages), pager_size)?;

            Ok(paginator.pagers()[0].clone())
        })
    }
}

/// Go: `pd := p.source.targetPathDescriptor; pd.Type = p.source.outputFormat()`.
fn paginator_target_path_descriptor(
    source: &PageHandle,
) -> Result<nh_page::page_paths::TargetPathDescriptor> {
    let ps = source.state();
    let mut pd = ps
        .common
        .target_path_descriptor
        .get()
        .cloned()
        .ok_or_else(|| {
            // A page without output formats never has a paginator (`render` is false).
            Error::new("neohugo-rs: pagination of a page without a target path descriptor")
        })?;
    pd.type_ = ps.current_output().f.clone();
    Ok(pd)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/page__paginator.go (112 lines; 4/4 funcs executed)
//   types: pagePaginator, pagePaginatorInit
// OK L23-28: newPagePaginator(source *pageState) *pagePaginator
// OK L41-43: (p *pagePaginator) reset()
// OK L45-70: (p *pagePaginator) Paginate(seq any, options ...any) (*page.Pager, error)
// OK L72-112: (p *pagePaginator) Paginator(options ...any) (*page.Pager, error)
// ---------------------------------------------------------------------------
