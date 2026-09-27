//! Port of `hugolib/pagecollections.go`.
//!
//! Owner: Wave B task T21 (hugolib-assemble).


//! Go `hugolib/pagecollections.go`: `pageFinder` — GetPage ref normalisation (trailing `/`
//! trimmed, `.md` appended, `/_index.md` for `/`, PathParser base), tree lookup in the site's
//! language, reverse index by base name for bare refs.

use std::sync::Arc;

use nh_common::Result;
use nh_page::page::PageRef;

use crate::hugo_sites::HugoSites;

// Go: hugolib/pagecollections.go:getPageNew
pub fn get_page(h: &Arc<HugoSites>, site_idx: usize, context: Option<&PageRef>, ref_: &str) -> Result<Option<PageRef>> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/pagecollections.go (256 lines; 7/9 funcs executed)
//   types: pageFinder
// EX L36-42: newPageFinder(m *pageMap) *pageFinder
// EX L46-56: (c *pageFinder) getPageRef(context page.Page, ref string) (page.Page, error)
// EX L58-67: (c *pageFinder) getPage(context page.Page, ref string) (page.Page, error)
//    L70-74: (c *pageFinder) getPageOldVersion(kind string, sections ...string) page.Page
// EX L79-114: (c *pageFinder) getPageForRefs(ref ...string) (page.Page, error)
// EX L118-149: (c *pageFinder) getContentNode(context page.Page, isReflink bool, ref string) (contentNodeI, error)
// EX L151-222: (c *pageFinder) getContentNodeForRef(context page.Page, isReflink, hadExtension bool, inRef, ref string) (contentNodeI, error)
//    L224-247: (c *pageFinder) getContentNodeFromRefReverseLookup(ref string, fi hugofs.FileMetaInfo) (contentNodeI, error)
// EX L249-256: (c *pageFinder) getContentNodeFromPath(s string, ref string) (contentNodeI, error)
// ---------------------------------------------------------------------------
