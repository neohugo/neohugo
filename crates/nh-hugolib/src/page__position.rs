//! Port of `hugolib/page__position.go`.
//!
//! Owner: Wave B task T23 (hugolib-site).

//! Go `pagePosition`/`pagePositionInSection` over a `*nextPrev` whose `init` is the site's lazy
//! `prevNext`/`prevNextInSection` branch (site.go `prepareInits`, `crate::site`). Only regular
//! pages have positions (`initCommonProviders`); every other page answers with `page.NopPage`'s
//! nil `page.Page` (`None` here).
//!
//! The site init fills the positions of every page in its list at once; the port keeps them
//! in each page's `PageCommon` (`OnceLock`s) and runs the site init when a page's position is
//! still unset (a page the init does not reach keeps Go's nil positions).

use nh_page::page::PageRef;

use crate::page::PageHandle;
use crate::page__common::NextPrev;

/// Go: `newPagePosition(n)` — the positions of a regular page (`None` for other kinds, whose
/// `Positioner` is `page.NopPage`).
// Go: hugolib/page__position.go:newPagePosition
pub fn new_page_position(p: &PageHandle) -> Option<&NextPrev> {
    let ps = p.state();
    // Go: `page.NopPage` until `initCommonProviders` (the lazy page init) set the positioner.
    if !ps.meta.is_page() || !ps.common_providers_initialised() {
        return None;
    }
    Some(next_prev(p, false))
}

/// Go: `newPagePositionInSection(n)`.
// Go: hugolib/page__position.go:newPagePositionInSection
pub fn new_page_position_in_section(p: &PageHandle) -> Option<&NextPrev> {
    let ps = p.state();
    // Go: `page.NopPage` until `initCommonProviders` (the lazy page init) set the positioner.
    if !ps.meta.is_page() || !ps.common_providers_initialised() {
        return None;
    }
    Some(next_prev(p, true))
}

/// The page's `nextPrev` after `n.init.Do()`.
fn next_prev(p: &PageHandle, in_section: bool) -> &NextPrev {
    let ps = p.state();
    let cell = if in_section {
        &ps.common.next_prev_in_section
    } else {
        &ps.common.next_prev
    };
    if cell.get().is_none() {
        if in_section {
            crate::site::init_prev_next_in_section(&p.h, ps.site_idx);
        } else {
            crate::site::init_prev_next(&p.h, ps.site_idx);
        }
    }
    // A page the site's init did not reach keeps nil positions.
    cell.get_or_init(|| (None, None))
}

/// Go: `(n *nextPrev) next()`.
// Go: hugolib/page__position.go:next
pub fn next(n: Option<&NextPrev>) -> Option<PageRef> {
    n.and_then(|n| n.0.clone())
}

/// Go: `(n *nextPrev) prev()`.
// Go: hugolib/page__position.go:prev
pub fn prev(n: Option<&NextPrev>) -> Option<PageRef> {
    n.and_then(|n| n.1.clone())
}

/// Go: `pagePosition.Next()`.
// Go: hugolib/page__position.go:Next
pub fn page_next(p: &PageHandle) -> Option<PageRef> {
    next(new_page_position(p))
}

/// Go: `pagePosition.NextPage()` (deprecated alias of `Next`; Go logs a deprecation once).
// Go: hugolib/page__position.go:NextPage
pub fn page_next_page(p: &PageHandle) -> Option<PageRef> {
    if new_page_position(p).is_some() {
        nh_config::neohugo::neohugo::deprecate(
            ".Page.NextPage",
            "Use .Page.Next instead.",
            "v0.123.0",
        );
    }
    page_next(p)
}

/// Go: `pagePosition.Prev()`.
// Go: hugolib/page__position.go:Prev
pub fn page_prev(p: &PageHandle) -> Option<PageRef> {
    prev(new_page_position(p))
}

/// Go: `pagePosition.PrevPage()` (deprecated alias of `Prev`).
// Go: hugolib/page__position.go:PrevPage
pub fn page_prev_page(p: &PageHandle) -> Option<PageRef> {
    if new_page_position(p).is_some() {
        nh_config::neohugo::neohugo::deprecate(
            ".Page.PrevPage",
            "Use .Page.Prev instead.",
            "v0.123.0",
        );
    }
    page_prev(p)
}

/// Go: `pagePositionInSection.NextInSection()`.
// Go: hugolib/page__position.go:NextInSection
pub fn next_in_section(p: &PageHandle) -> Option<PageRef> {
    next(new_page_position_in_section(p))
}

/// Go: `pagePositionInSection.PrevInSection()`.
// Go: hugolib/page__position.go:PrevInSection
pub fn prev_in_section(p: &PageHandle) -> Option<PageRef> {
    prev(new_page_position_in_section(p))
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/page__position.go (86 lines; 2/10 funcs executed)
//   types: nextPrev, pagePosition, pagePositionInSection
// OK L24-26: newPagePosition(n *nextPrev) pagePosition
// OK L28-30: newPagePositionInSection(n *nextPrev) pagePositionInSection
// OK L38-43: (n *nextPrev) next() page.Page
// OK L45-50: (n *nextPrev) prev() page.Page
// OK L56-58: (p pagePosition) Next() page.Page
// OK L61-64: (p pagePosition) NextPage() page.Page
// OK L66-68: (p pagePosition) Prev() page.Page
// OK L71-74: (p pagePosition) PrevPage() page.Page
// OK L80-82: (p pagePositionInSection) NextInSection() page.Page
// OK L84-86: (p pagePositionInSection) PrevInSection() page.Page
// ---------------------------------------------------------------------------
