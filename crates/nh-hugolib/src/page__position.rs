//! Port of `hugolib/page__position.go`.
//!
//! Owner: Wave B task T23 (hugolib-site).


// Wave B: port per the checklist below.

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/page__position.go (86 lines; 2/10 funcs executed)
//   types: nextPrev, pagePosition, pagePositionInSection
// EX L24-26: newPagePosition(n *nextPrev) pagePosition
// EX L28-30: newPagePositionInSection(n *nextPrev) pagePositionInSection
//    L38-43: (n *nextPrev) next() page.Page
//    L45-50: (n *nextPrev) prev() page.Page
//    L56-58: (p pagePosition) Next() page.Page
//    L61-64: (p pagePosition) NextPage() page.Page
//    L66-68: (p pagePosition) Prev() page.Page
//    L71-74: (p pagePosition) PrevPage() page.Page
//    L80-82: (p pagePositionInSection) NextInSection() page.Page
//    L84-86: (p pagePositionInSection) PrevInSection() page.Page
// ---------------------------------------------------------------------------
