//! Port of `hugolib/site_sections.go`.
//!
//! Owner: Wave B task T21 (hugolib-assemble).


//! Go `hugolib/site_sections.go`: `.Site.Sections` / `.Site.Home`.

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/site_sections.go (30 lines; 1/2 funcs executed)
//    L21-24: (s *Site) Sections() page.Pages
// EX L27-30: (s *Site) Home() page.Page
// ---------------------------------------------------------------------------
