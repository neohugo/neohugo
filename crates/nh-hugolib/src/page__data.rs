//! Port of `hugolib/page__data.go`.
//!
//! Owner: Wave B task T21 (hugolib-assemble).

//! Go `hugolib/page__data.go`: `.Data` (page.Data): taxonomy -> Singular/Plural/Terms/Pages;
//! term -> Singular/Plural/Term/<singular>/Pages; section/home -> Pages; sitemap -> Pages = site Pages.

use go_value::Value;

use crate::page::PageHandle;

// Go: hugolib/page__data.go:Data
pub fn data(p: &PageHandle) -> Value {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/page__data.go (63 lines; 1/1 funcs executed)
//   types: pageData
// EX L31-63: (p *pageData) Data() any
// ---------------------------------------------------------------------------
