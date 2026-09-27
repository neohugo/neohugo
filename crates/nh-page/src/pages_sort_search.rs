//! Port of `resources/page/pages_sort_search.go`.
//!
//! Owner: Wave B task T12 (page-collections).


use crate::page::{Page, Pages};

/// Go: `searchPage(p, pages)` — index of `p` (used by Next/Prev), -1 if absent.
// Go: resources/page/pages_sort_search.go:searchPage
pub fn search_page(p: &dyn Page, pages: &Pages) -> i64 {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/page/pages_sort_search.go (125 lines; 0/4 funcs executed)
//    L27-44: searchPage(p Page, pages Pages) int
//    L46-54: searchPageLinear(p Page, pages Pages, start int) int
//    L56-72: searchPageBinary(p Page, pages Pages, less func(p1, p2 Page) bool) int
//    L75-125: isPagesProbablySorted(pages Pages, lessFuncs ...func(p1, p2 Page) bool) func(p1, p2 Page) bool
// ---------------------------------------------------------------------------
