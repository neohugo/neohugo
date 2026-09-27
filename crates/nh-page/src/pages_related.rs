//! Port of `resources/page/pages_related.go`.
//!
//! Owner: Wave B task T12 (page-collections).


//! Go `pages_related.go`: `.Site.RegularPages.Related .` — an inverted index per page list
//! (cached by list equality), searched with the related config (threshold, indices weights).

use std::sync::{Arc, Mutex};

use go_value::{HostCtx, Value};
use nh_common::Result;

use crate::page::Pages;
use crate::related::{Config, InvertedIndex};

/// Go: `page.RelatedDocsHandler`.
pub struct RelatedDocsHandler {
    pub cfg: Config,
    pub(crate) posting_lists: Mutex<Vec<(Pages, Arc<InvertedIndex>)>>,
}

impl RelatedDocsHandler {
    // Go: resources/page/pages_related.go:NewRelatedDocsHandler
    pub fn new(cfg: Config) -> Arc<RelatedDocsHandler> {
        Arc::new(RelatedDocsHandler { cfg, posting_lists: Mutex::new(Vec::new()) })
    }

    // Go: resources/page/pages_related.go:getOrCreateIndex
    pub fn get_or_create_index(&self, ctx: HostCtx<'_>, p: &Pages) -> Result<Arc<InvertedIndex>> {
        todo!()
    }
}

/// Go: `Pages.Related(ctx, optsv)` — `optsv` is a page (document) or an options map.
// Go: resources/page/pages_related.go:Related
pub fn related(ctx: HostCtx<'_>, p: &Pages, opts: &Value) -> Result<Pages> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/page/pages_related.go (244 lines; 6/9 funcs executed)
//   types: PageGenealogist, cachedPostingList, RelatedDocsHandler
// EX L56-79: (p Pages) Related(ctx context.Context, optsv any) (Pages, error)
//    L84-101: (p Pages) RelatedIndices(ctx context.Context, doc related.Document, indices ...any) (Pages, error)
//    L105-115: (p Pages) RelatedTo(ctx context.Context, args ...types.KeyValues) (Pages, error)
// EX L117-121: (p Pages) search(ctx context.Context, opts related.SearchOpts) (Pages, error)
// EX L123-154: (p Pages) withInvertedIndex(ctx context.Context, search func(idx *related.InvertedIndex) ([]related.Document, error)) (Pages, error)
// EX L171-173: NewRelatedDocsHandler(cfg related.Config) *RelatedDocsHandler
//    L175-177: (s *RelatedDocsHandler) Clone() *RelatedDocsHandler
// EX L180-187: (s *RelatedDocsHandler) getIndex(p Pages) *related.InvertedIndex
// EX L189-244: (s *RelatedDocsHandler) getOrCreateIndex(ctx context.Context, p Pages) (*related.InvertedIndex, error)
// ---------------------------------------------------------------------------
