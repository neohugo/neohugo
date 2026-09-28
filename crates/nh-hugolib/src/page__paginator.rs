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
use nh_page::pagination::Pager;

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

impl PagePaginator {
    /// Go: `reset()` — a fresh `pagePaginatorInit`.
    // Go: hugolib/page__paginator.go:reset
    pub fn reset(&self) {
        *self.init.lock().unwrap() = PagePaginatorInit::default();
    }

    /// Whether a paginator was built (Go `p.paginator.current != nil`).
    pub fn is_built(&self) -> bool {
        self.init.lock().unwrap().current.is_some()
    }

    /// Go: `Paginate(seq, options...)`.
    // Go: hugolib/page__paginator.go:Paginate
    pub fn paginate(
        &self,
        source: &crate::page::PageHandle,
        seq: &Value,
        options: &[Value],
    ) -> Result<Option<Arc<Pager>>> {
        todo!()
    }

    /// Go: `Paginator(options...)` — home: `s.RegularPages()`; term/taxonomy: `Pages()`;
    /// other nodes: `RegularPages()`.
    // Go: hugolib/page__paginator.go:Paginator
    pub fn paginator(
        &self,
        source: &crate::page::PageHandle,
        options: &[Value],
    ) -> Result<Option<Arc<Pager>>> {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/page__paginator.go (112 lines; 4/4 funcs executed)
//   types: pagePaginator, pagePaginatorInit
// EX L23-28: newPagePaginator(source *pageState) *pagePaginator
// EX L41-43: (p *pagePaginator) reset()
// EX L45-70: (p *pagePaginator) Paginate(seq any, options ...any) (*page.Pager, error)
// EX L72-112: (p *pagePaginator) Paginator(options ...any) (*page.Pager, error)
// ---------------------------------------------------------------------------
