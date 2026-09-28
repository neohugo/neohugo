//! Port of `hugolib/page__common.go`.
//!
//! Owner: Wave B task T23 (hugolib-site).

//! Go `pageCommon` holds the parts of a page that are neither meta, output nor content. Most of
//! its embedded provider interfaces are the page itself in Go; in the port they are the methods
//! of `PageHandle` (tplapi/page_methods.rs). What remains here is state:
//! * `store` (`.Scratch` == `.Store`, shared by all outputs and pagers of the page);
//! * the target path descriptor (set by `initCommonProviders`, T21);
//! * the next/prev positions (Go `posNextPrev`/`posNextPrevSection`, filled by the site's lazy
//!   `prevNext`/`prevNextInSection` inits, site.rs);
//! * the page menus (Go `pageMenus.pm`, page__menus.rs).

use std::sync::{Arc, OnceLock};

use nh_common::maps::scratch::Scratch;
use nh_page::navigation::menu::PageMenus;
use nh_page::page::Pages;
use nh_page::page_paths::TargetPathDescriptor;

/// Go `nextPrev{prevPage, nextPage}` after the site's init: `(next, prev)`.
pub type NextPrev = (
    Option<nh_page::page::PageRef>,
    Option<nh_page::page::PageRef>,
);

/// Go: `pageCommon` (state that is not meta, output or content).
#[derive(Default)]
pub struct PageCommon {
    /// Go `store` — `.Scratch` == `.Store`, shared by all outputs of the page.
    pub store: Arc<Scratch>,
    /// Go `targetPathDescriptor` (created in `newPageFromMeta`/`initPage`).
    pub target_path_descriptor: OnceLock<TargetPathDescriptor>,
    /// Lazy `.AllTranslations` (sorted by language weight...). Unused: the port caches the
    /// translations in the site's `PageMap` partitions under Go's keys (page.rs).
    pub all_translations: OnceLock<Pages>,
    /// Unused (see `all_translations`).
    pub translations: OnceLock<Pages>,
    /// Go `posNextPrev` (`(next, prev)`), set by the site's `prevNext` init (site.rs).
    pub next_prev: OnceLock<NextPrev>,
    /// Go `posNextPrevSection`, set by the site's `prevNextInSection` init.
    pub next_prev_in_section: OnceLock<NextPrev>,
    /// Go `pageMenus.pm` (`pmInit`): the page's own menu entries from front matter (`None` is
    /// Go's nil map).
    pub page_menus: OnceLock<Option<PageMenus>>,
    /// The page's menu entries as the site's `assembleMenus` left them (Go mutates the same
    /// entries: children, page values). Set only for the pages the assembly visits.
    pub page_menus_assembled: OnceLock<PageMenus>,
    /// Go `pageConfig.Params` as one template value (`.Params` returns the same map every
    /// time), created on first use after assembly.
    pub params: OnceLock<Arc<go_value::Map>>,
    /// Go `sWrapped` (`page.WrapSite(m.s)`): the page's own `page.Site` wrapper.
    pub site_wrapped: OnceLock<nh_page::site::SiteRef>,
}

impl PageCommon {
    /// Go: `(p *pageCommon) Store()`.
    // Go: hugolib/page__common.go:Store
    pub fn store(&self) -> Arc<Scratch> {
        self.store.clone()
    }

    /// Go: `(p *pageCommon) Scratch()` — an alias of `Store()`.
    // Go: hugolib/page__common.go:Scratch
    pub fn scratch(&self) -> Arc<Scratch> {
        self.store()
    }

    /// Go: `(p *pageCommon) getNextPrev()` — the positions (`None` before the site's init ran).
    // Go: hugolib/page__common.go:getNextPrev
    pub fn get_next_prev(&self) -> Option<&NextPrev> {
        self.next_prev.get()
    }

    /// Go: `(p *pageCommon) getNextPrevInSection()`.
    // Go: hugolib/page__common.go:getNextPrevInSection
    pub fn get_next_prev_in_section(&self) -> Option<&NextPrev> {
        self.next_prev_in_section.get()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/page__common.go (113 lines; 2/4 funcs executed)
//   types: nextPrevProvider, nextPrevInSectionProvider, pageCommon
// OK L33-35: (p *pageCommon) getNextPrev() *nextPrev
// OK L41-43: (p *pageCommon) getNextPrevInSection() *nextPrev
// OK L106-108: (p *pageCommon) Store() *maps.Scratch
// OK L111-113: (p *pageCommon) Scratch() *maps.Scratch
// ---------------------------------------------------------------------------
