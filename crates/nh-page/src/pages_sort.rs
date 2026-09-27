//! Port of `resources/page/pages_sort.go`.
//!
//! Owner: Wave B task T12 (page-collections).


//! Go `resources/page/pages_sort.go`. All page sorts are `sort.Stable` (Rust stable sort is
//! equivalent for strict weak orders). Tie-breaks use the x/text collator of the site returned by
//! `p.Site().Current()` — i.e. the site being rendered when a list is first computed.

use crate::page::{Page, PageRef, Pages};

/// Go: `page.DefaultPageSort` — ordinal, weight0, weight (0 last), `Date().Unix()` desc,
/// collated LinkTitle, `compare.LessStrings(PathInfo().Path())`.
// Go: resources/page/pages_sort.go:DefaultPageSort
pub fn default_page_sort(p1: &dyn Page, p2: &dyn Page) -> bool {
    todo!()
}

/// Go: `lessPageLanguage` (language weight, date desc, `compare.Strings(LinkTitle)`, filename).
// Go: resources/page/pages_sort.go:lessPageLanguage
pub fn less_page_language(p1: &dyn Page, p2: &dyn Page) -> bool {
    todo!()
}

/// Go: `page.SortByDefault(pages)` (in place, stable).
// Go: resources/page/pages_sort.go:SortByDefault
pub fn sort_by_default(pages: &mut Pages) {
    todo!()
}

/// Go: `page.SortByLanguage(pages)`.
// Go: resources/page/pages_sort.go:SortByLanguage
pub fn sort_by_language(pages: &mut Pages) {
    todo!()
}

// Go: resources/page/pages_sort.go:Reverse (copy)
pub fn reverse(p: &Pages) -> Pages {
    p.iter().rev().cloned().collect()
}

// Go: resources/page/pages_sort.go:ByWeight
pub fn by_weight(p: &Pages) -> Pages { todo!() }
// Go: resources/page/pages_sort.go:ByTitle (collator)
pub fn by_title(p: &Pages) -> Pages { todo!() }
// Go: resources/page/pages_sort.go:ByLinkTitle (collator)
pub fn by_link_title(p: &Pages) -> Pages { todo!() }
// Go: resources/page/pages_sort.go:ByDate
pub fn by_date(p: &Pages) -> Pages { todo!() }
// Go: resources/page/pages_sort.go:ByPublishDate
pub fn by_publish_date(p: &Pages) -> Pages { todo!() }
// Go: resources/page/pages_sort.go:ByLastmod
pub fn by_lastmod(p: &Pages) -> Pages { todo!() }
// Go: resources/page/pages_sort.go:ByParam
pub fn by_param(p: &Pages, key: &go_value::Value) -> Pages { todo!() }
// Go: resources/page/pages_sort.go:Limit
pub fn limit(p: &Pages, n: usize) -> Pages { p.iter().take(n).cloned().collect() }

/// Go: `collatorStringCompare(getString, p1, p2)` — collator of `p1.Site().Current().Language()`.
pub fn collator_string_compare(p1: &dyn Page, a: &str, b: &str) -> i32 {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/page/pages_sort.go (431 lines; 9/20 funcs executed)
//   types: pageSorter, pageBy
// EX L44-55: getOrdinals(p1, p2 Page) (int, int)
// EX L57-68: getWeight0s(p1, p2 Page) (int, int)
// EX L71-77: (by pageBy) Sort(pages Pages)
// EX L160-160: (ps *pageSorter) Len() int
// EX L161-161: (ps *pageSorter) Swap(i, j int)
// EX L164-164: (ps *pageSorter) Less(i, j int) bool
//    L167-172: (p Pages) Limit(n int) Pages
//    L220-224: (p Pages) ByWeight() Pages
// EX L227-229: SortByDefault(pages Pages)
//    L236-242: (p Pages) ByTitle() Pages
//    L249-255: (p Pages) ByLinkTitle() Pages
//    L262-268: (p Pages) ByDate() Pages
//    L275-281: (p Pages) ByPublishDate() Pages
//    L288-298: (p Pages) ByExpiryDate() Pages
//    L305-315: (p Pages) ByLastmod() Pages
//    L322-343: (p Pages) ByLength(ctx context.Context) Pages
//    L350-356: (p Pages) ByLanguage() Pages
// EX L359-361: SortByLanguage(pages Pages)
// EX L368-380: (p Pages) Reverse() Pages
//    L387-431: (p Pages) ByParam(paramsKey any) Pages
// ---------------------------------------------------------------------------
