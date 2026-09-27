//! Port of `resources/page/pages_prev_next.go`.
//!
//! Owner: Wave B task T12 (page-collections).


use crate::page::{Page, PageRef, Pages};

/// Go: `Pages.Next(cur)` (previous element in the list — Hugo's reversed semantics).
// Go: resources/page/pages_prev_next.go:Next
pub fn next(p: &Pages, cur: &dyn Page) -> Option<PageRef> {
    todo!()
}

// Go: resources/page/pages_prev_next.go:Prev
pub fn prev(p: &Pages, cur: &dyn Page) -> Option<PageRef> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/page/pages_prev_next.go (34 lines; 0/2 funcs executed)
//    L17-23: (p Pages) Next(cur Page) Page
//    L26-34: (p Pages) Prev(cur Page) Page
// ---------------------------------------------------------------------------
