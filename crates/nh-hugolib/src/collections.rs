//! Port of `hugolib/collections.go`.
//!
//! Owner: Wave B task T21 (hugolib-assemble).

//! Go `hugolib/collections.go`: `pageState.Slice(items)` (collections.Slicer -> page.Pages),
//! `Group(key, pages)`.

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/collections.go (46 lines; 1/2 funcs executed)
// EX L31-33: (p *pageState) Slice(items any) (any, error)
//    L40-46: (p *pageState) Group(key any, in any) (any, error)
// ---------------------------------------------------------------------------
