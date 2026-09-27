//! Port of `hugolib/page__new.go`.
//!
//! Owner: Wave B task T20 (hugolib-capture).


//! Go `hugolib/page__new.go`: page creation from a content file or a synthetic meta (kind
//! detection: home / taxonomy (pluralTreeKey) / term (taxonomy prefix, character-level) /
//! section / page; site from the file language).

use nh_common::Result;

use crate::hugo_sites::HugoSites;
use crate::page::PageId;

/// Go: `pageMetaSource` / `newPageFromMeta` inputs.
pub struct NewPageMeta {
    pub path_info: std::sync::Arc<nh_common::paths::pathparser::Path>,
    pub kind: String,
    pub term: String,
    pub singular: String,
    pub file: Option<nh_hugofs::fileinfo::FileMetaInfo>,
    pub lang_index: usize,
}

/// Go: `h.newPage(m)` / `doNewPage`.
// Go: hugolib/page__new.go:newPage
pub fn new_page(h: &mut HugoSites, m: NewPageMeta) -> Result<PageId> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/page__new.go (265 lines; 2/2 funcs executed)
// EX L37-52: (h *HugoSites) newPage(m *pageMeta) (*pageState, *paths.Path, error)
// EX L54-265: (h *HugoSites) doNewPage(m *pageMeta) (*pageState, *paths.Path, error)
// ---------------------------------------------------------------------------
