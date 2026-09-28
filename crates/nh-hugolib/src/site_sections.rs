//! Port of `hugolib/site_sections.go`.
//!
//! Owner: Wave B task T21 (hugolib-assemble).

//! Go `hugolib/site_sections.go`: `.Site.Sections` / `.Site.Home`.

use nh_page::page::{PageRef, Pages};

use crate::page::{PageHandle, PageWrapper};
use crate::site::SiteHandle;

/// Go: `(s *Site) Home()` — a shortcut to the home page, equivalent to `.Site.GetPage "home"`
/// (`None` is Go's nil: a site without a home page).
// Go: hugolib/site_sections.go:Home
pub fn home(s: &SiteHandle) -> Option<PageRef> {
    s.site().home.map(|id| {
        PageHandle {
            h: s.h.clone(),
            id,
            wrapper: PageWrapper::None,
        }
        .page_ref()
    })
}

/// Go: `(s *Site) Sections()` — the top level sections (`Home().Sections()`).
// Go: hugolib/site_sections.go:Sections
pub fn sections(s: &SiteHandle) -> Pages {
    let Some(id) = s.site().home else {
        // Go dereferences the nil home page.
        panic!("invalid memory address or nil pointer dereference");
    };
    crate::page__tree::sections(&PageHandle {
        h: s.h.clone(),
        id,
        wrapper: PageWrapper::None,
    })
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/site_sections.go (30 lines; 1/2 funcs executed)
// OK L21-24: (s *Site) Sections() page.Pages
// OK L27-30: (s *Site) Home() page.Page
// ---------------------------------------------------------------------------
