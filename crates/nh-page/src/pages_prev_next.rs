//! Port of `resources/page/pages_prev_next.go`.
//!
//! Owner: Wave B task T12 (page-collections).

use crate::page::{Page, PageRef, Pages};
use crate::pages_sort_search::search_page;

/// Go: `Pages.Next(cur)` — the page BEFORE `cur` in the list (Hugo's reversed semantics).
// Go: resources/page/pages_prev_next.go:Next
pub fn next(p: &Pages, cur: &dyn Page) -> Option<PageRef> {
    let x = search_page(cur, p);
    if x <= 0 {
        return None;
    }
    Some(p[(x - 1) as usize].clone())
}

/// Go: `Pages.Prev(cur)` — the page AFTER `cur` in the list.
// Go: resources/page/pages_prev_next.go:Prev
pub fn prev(p: &Pages, cur: &dyn Page) -> Option<PageRef> {
    let x = search_page(cur, p);

    if x == -1 || p.len() as i64 - x < 2 {
        return None;
    }

    Some(p[(x + 1) as usize].clone())
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/page/pages_prev_next.go (34 lines; 0/2 funcs executed)
// OK L17-23: (p Pages) Next(cur Page) Page
// OK L26-34: (p Pages) Prev(cur Page) Page
// ---------------------------------------------------------------------------
