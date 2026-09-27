//! Port of `hugolib/page__common.go`.
//!
//! Owner: Wave B task T23 (hugolib-site).


use std::sync::{Arc, OnceLock};

use nh_common::maps::scratch::Scratch;
use nh_page::page::Pages;
use nh_page::page_paths::TargetPathDescriptor;

/// Go: `pageCommon` (state that is not meta, output or content).
#[derive(Default)]
pub struct PageCommon {
    /// Go `store` — `.Scratch` == `.Store`, shared by all outputs of the page.
    pub store: Arc<Scratch>,
    /// Go `targetPathDescriptor` (created in `newPageFromMeta`/`initPage`).
    pub target_path_descriptor: OnceLock<TargetPathDescriptor>,
    /// Lazy `.AllTranslations` (sorted by language weight...).
    pub all_translations: OnceLock<Pages>,
    pub translations: OnceLock<Pages>,
    /// Go `posNextPrev` / `posNextPrevSection` (lazy).
    pub next_prev: OnceLock<(Option<nh_page::page::PageRef>, Option<nh_page::page::PageRef>)>,
    pub next_prev_in_section: OnceLock<(Option<nh_page::page::PageRef>, Option<nh_page::page::PageRef>)>,
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/page__common.go (113 lines; 2/4 funcs executed)
//   types: nextPrevProvider, nextPrevInSectionProvider, pageCommon
//    L33-35: (p *pageCommon) getNextPrev() *nextPrev
//    L41-43: (p *pageCommon) getNextPrevInSection() *nextPrev
// EX L106-108: (p *pageCommon) Store() *maps.Scratch
// EX L111-113: (p *pageCommon) Scratch() *maps.Scratch
// ---------------------------------------------------------------------------
