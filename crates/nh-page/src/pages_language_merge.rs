//! Port of `resources/page/pages_language_merge.go`.
//!
//! Owner: Wave B task T12 (page-collections).


use crate::page::Pages;

/// Go: `Pages.MergeByLanguage(other)`.
// Go: resources/page/pages_language_merge.go:MergeByLanguage
pub fn merge_by_language(p1: &Pages, p2: &Pages) -> Pages {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/page/pages_language_merge.go (63 lines; 0/2 funcs executed)
//   types: pagesLanguageMerger
//    L30-49: (p1 Pages) MergeByLanguage(p2 Pages) Pages
//    L54-63: (p1 Pages) MergeByLanguageInterface(in any) (any, error)
// ---------------------------------------------------------------------------
