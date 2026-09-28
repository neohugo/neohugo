//! Port of `resources/page/pages_sort_search.go`.
//!
//! Owner: Wave B task T12 (page-collections).

use crate::page::{Page, Pages};
use crate::pages_sort::{
    default_page_sort, less_page_date, less_page_link_title, less_page_pub_date, less_page_title,
};

/// A page less function (Go `func(p1, p2 Page) bool`).
pub type LessFn = fn(&dyn Page, &dyn Page) -> bool;

/// Go: `pageLessFunctions` — used in page binary search, the most common in front.
const PAGE_LESS_FUNCTIONS: [LessFn; 5] = [
    default_page_sort,
    less_page_date,
    less_page_pub_date,
    less_page_title,
    less_page_link_title,
];

/// Go `c.Eq(p)` on pages: the same (unwrapped) page.
fn page_eq(c: &dyn Page, p: &dyn Page) -> bool {
    c.page_id() == p.page_id()
}

/// Go: `searchPage(p, pages)` — index of `p` (used by Next/Prev), -1 if absent.
// Go: resources/page/pages_sort_search.go:searchPage
pub fn search_page(p: &dyn Page, pages: &Pages) -> i64 {
    if pages.len() < 1000 {
        // For smaller data sets, doing a linear search is faster.
        return search_page_linear(p, pages, 0);
    }

    let Some(less) = is_pages_probably_sorted(pages, &PAGE_LESS_FUNCTIONS) else {
        return search_page_linear(p, pages, 0);
    };

    let i = search_page_binary(p, pages, &*less);
    if i != -1 {
        return i;
    }

    search_page_linear(p, pages, 0)
}

// Go: resources/page/pages_sort_search.go:searchPageLinear
fn search_page_linear(p: &dyn Page, pages: &Pages, start: usize) -> i64 {
    for (i, c) in pages.iter().enumerate().skip(start) {
        if page_eq(&*c.0, p) {
            return i as i64;
        }
    }
    -1
}

// Go: resources/page/pages_sort_search.go:searchPageBinary
pub fn search_page_binary(
    p: &dyn Page,
    pages: &Pages,
    less: &dyn Fn(&dyn Page, &dyn Page) -> bool,
) -> i64 {
    let n = pages.len();

    let f = |i: usize| {
        let c = &*pages[i].0;
        let is_less = less(c, p);
        !is_less || page_eq(c, p)
    };

    let i = go_sort::sort::search(n, f);

    if i == n {
        return -1;
    }

    search_page_linear(p, pages, i)
}

pub type BoxedLess = Box<dyn Fn(&dyn Page, &dyn Page) -> bool>;

/// Go: `isPagesProbablySorted(pages, lessFuncs...)` — the first less function the pages are
/// probably sorted by (forward, or reversed), sampling every 50th page above 500.
// Go: resources/page/pages_sort_search.go:isPagesProbablySorted
pub fn is_pages_probably_sorted(pages: &Pages, less_funcs: &[LessFn]) -> Option<BoxedLess> {
    let n = pages.len() as i64;
    let mut step = 1i64;
    if n > 500 {
        step = 50;
    }

    let is = |less: LessFn| -> bool {
        let mut samples = 0;

        let mut i = n - 1;
        while i > 0 {
            if less(&*pages[i as usize].0, &*pages[(i - 1) as usize].0) {
                return false;
            }
            samples += 1;
            if samples >= 15 {
                return true;
            }
            i -= step;
        }
        samples > 0
    };

    let is_reverse = |less: LessFn| -> bool {
        let mut samples = 0;

        let mut i = 0i64;
        while i < n - 1 {
            if less(&*pages[i as usize].0, &*pages[(i + 1) as usize].0) {
                return false;
            }
            samples += 1;

            if samples > 15 {
                return true;
            }
            i += step;
        }
        samples > 0
    };

    for &less in less_funcs {
        if is(less) {
            return Some(Box::new(less));
        }
        if is_reverse(less) {
            return Some(Box::new(move |p1: &dyn Page, p2: &dyn Page| less(p2, p1)));
        }
    }

    None
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/page/pages_sort_search.go (125 lines; 0/4 funcs executed)
// OK L27-44: searchPage(p Page, pages Pages) int
// OK L46-54: searchPageLinear(p Page, pages Pages, start int) int
// OK L56-72: searchPageBinary(p Page, pages Pages, less func(p1, p2 Page) bool) int
// OK L75-125: isPagesProbablySorted(pages Pages, lessFuncs ...func(p1, p2 Page) bool) func(p1, p2 Page) bool
// ---------------------------------------------------------------------------
