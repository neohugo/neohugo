//! Port of `hugolib/page_unwrap.go`.
//!
//! Owner: Wave B task T23 (hugolib-site).

// Wave B: port per the checklist below.

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/page_unwrap.go (53 lines; 1/2 funcs executed)
//   types: pageWrapper
// EX L29-44: unwrapPage(in any) (page.Page, error)
//    L46-53: mustUnwrapPage(in any) page.Page
// ---------------------------------------------------------------------------
