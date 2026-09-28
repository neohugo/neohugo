//! Port of `hugolib/page__tree.go`.
//!
//! Owner: Wave B task T21 (hugolib-assemble).

//! Go `hugolib/page__tree.go`: Parent (LongestPrefix of ContainerDir to a branch; home -> nil),
//! CurrentSection, FirstSection, InSection, IsAncestor/IsDescendant, Sections, SectionsEntries.

use nh_page::page::PageRef;

use crate::page::PageHandle;

// Go: hugolib/page__tree.go:Parent
pub fn parent(p: &PageHandle) -> Option<PageRef> {
    todo!()
}

// Go: hugolib/page__tree.go:CurrentSection
pub fn current_section(p: &PageHandle) -> PageRef {
    todo!()
}

// Go: hugolib/page__tree.go:FirstSection
pub fn first_section(p: &PageHandle) -> PageRef {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/page__tree.go (203 lines; 3/11 funcs executed)
//   types: pageTree
//    L33-44: (pt pageTree) IsAncestor(other any) bool
//    L46-57: (pt pageTree) IsDescendant(other any) bool
// EX L59-75: (pt pageTree) CurrentSection() page.Page
//    L77-101: (pt pageTree) FirstSection() page.Page
//    L103-114: (pt pageTree) InSection(other any) bool
// EX L116-137: (pt pageTree) Parent() page.Page
//    L139-147: (pt pageTree) Ancestors() page.Pages
//    L149-183: (pt pageTree) Sections() page.Pages
// EX L185-187: (pt pageTree) Page() page.Page
//    L189-199: (p pageTree) SectionsEntries() []string
//    L201-203: (p pageTree) SectionsPath() string
// ---------------------------------------------------------------------------
