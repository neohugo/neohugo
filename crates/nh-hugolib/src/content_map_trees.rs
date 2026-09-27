//! Port of `hugolib/content_map_page.go` (first part: the trees, the language shifter and the
//! `pageMap` struct).
//!
//! Owner: Wave B task T20 (hugolib-capture).


//! Split from `content_map_page.go` so that capture (T20: inserting pages and resources into the
//! trees) does not depend on the assembly task (T21). This module holds `content_map_page.go`
//! lines 88-259 (`pageMap` struct, `Reset`, `pageTrees`, `Shape`, `createMutableTrees`) and
//! 636-994 (`weightedContentNode`, `contentNodeI`/`contentNodeIs`, `contentNodeShifter`,
//! `newPageMap`, `contentTreeReverseIndex`). The queries (`getPagesInSection`, ...) and the
//! assembly steps stay in `content_map_page.rs` (T21), as `impl PageMap` / `SitePagesAssembler`
//! blocks over the types defined here.
//!
//! Caches follow the dynacache pattern (HUGO_LAYER.md §4.8): never compute while holding a lock;
//! the first stored value wins.

use std::collections::BTreeMap;
use std::sync::Arc;

use nh_common::dynacache::Partition;
use nh_common::Result;
use nh_doctree::dimensions::{Dimension, DimensionFlag};
use nh_doctree::nodeshifttree::{NodeShiftTree, Shifter};
use nh_doctree::treeshifttree::TreeShiftTree;
use nh_page::page::Pages;
use nh_resource::resourcetypes::Resources;

use crate::content_map::ResourceSource;
use crate::page::PageId;

/// A tree node (Go `contentNodeI`): one node, or one per language (Go `contentNodeIs`,
/// `resourceSources`).
#[derive(Clone)]
pub enum ContentNode {
    Page(PageId),
    Resource(Arc<ResourceSource>),
    /// Indexed by language index.
    Pages(Vec<Option<PageId>>),
    Resources(Vec<Option<Arc<ResourceSource>>>),
}

/// Go: `contentNodeShifter{numLanguages}`.
pub struct ContentNodeShifter {
    pub num_languages: usize,
}

impl Shifter<ContentNode> for ContentNodeShifter {
    // Go: hugolib/content_map_page.go:ForEeachInDimension
    fn for_each_in_dimension(&self, n: &ContentNode, d: usize, f: &mut dyn FnMut(&ContentNode) -> bool) {
        todo!()
    }
    // Go: hugolib/content_map_page.go:Insert
    fn insert(&self, old: ContentNode, new: ContentNode) -> (ContentNode, Option<ContentNode>, bool) {
        todo!()
    }
    // Go: hugolib/content_map_page.go:InsertInto
    fn insert_into(&self, old: ContentNode, new: ContentNode, dimension: Dimension) -> (ContentNode, Option<ContentNode>, bool) {
        todo!()
    }
    // Go: hugolib/content_map_page.go:Delete
    fn delete(&self, v: ContentNode, dimension: Dimension) -> (Option<ContentNode>, bool, bool) {
        todo!()
    }
    /// Pages: exact language only. Resources, non-exact: first non-nil language, flag 0.
    // Go: hugolib/content_map_page.go:Shift
    fn shift(&self, v: &ContentNode, dimension: Dimension, exact: bool) -> (Option<ContentNode>, bool, DimensionFlag) {
        todo!()
    }
}

/// Go: `weightedContentNode` (taxonomy entries: key = term key + page key).
#[derive(Clone)]
pub struct WeightedContentNode {
    pub n: PageId,
    pub weight: i64,
    /// The term page with its ordinal (Go `*pageWithOrdinal`).
    pub term: Option<(PageId, i64)>,
}

/// Go: `pageTrees`.
pub struct PageTrees {
    /// Pages keyed by `Path.Base()` (home = "").
    pub tree_pages: NodeShiftTree<ContentNode>,
    /// Resources keyed by `Path.Base()` with extension.
    pub tree_resources: NodeShiftTree<ContentNode>,
    /// Per language: `termKey + pageKey` -> entry.
    pub tree_taxonomy_entries: TreeShiftTree<WeightedContentNode>,
}

impl PageTrees {
    /// Go: the `pageTrees` literal in `NewHugoSites` (site.go:226-244) + `createMutableTrees`.
    // Go: hugolib/content_map_page.go:createMutableTrees
    pub fn new(num_languages: usize) -> PageTrees {
        todo!()
    }
}

/// Go: `pageMap` (per site): the site's view of the shared trees + query caches.
///
/// The caches are dynacache partitions (Go `dynacache.GetOrCreatePartition`): `get_or_create`
/// runs the create function WITHOUT holding the partition lock, and the first stored value wins.
/// The queries themselves are `impl PageMap` blocks in `content_map_page.rs` (T21).
pub struct PageMap {
    pub site_idx: usize,
    /// Go `cachePages1` (pages below path) — NOTE shared key for term Pages/RegularPages.
    pub cache_pages1: Partition<String, Pages>,
    /// Go `cachePages2` (pages in section).
    pub cache_pages2: Partition<String, Pages>,
    /// Go `cacheResources`.
    pub cache_resources: Partition<String, Resources>,
    /// Go `cacheGetTerms`.
    pub cache_get_terms: Partition<String, BTreeMap<String, Pages>>,
    /// Go `cacheContentRendered`: key `sourceKey + "/" + markupScope + outputFormat.Name`
    /// (page__content.go:522-530). Content caches live on the page's SITE pageMap, as in Go.
    pub cache_content_rendered: Partition<String, Arc<Result<crate::page__content::ContentSummary>>>,
    /// Go `cacheContentPlain` (same key scheme).
    pub cache_content_plain: Partition<String, Arc<Result<crate::page__content::ContentPlainPlainWords>>>,
    /// Go `contentTableOfContents` (same key scheme).
    pub cache_content_toc: Partition<String, Arc<Result<crate::page__content::ContentTableOfContents>>>,
}

/// Go: `newPageMap(i, s, mcache, pageTrees)` (partition names as in Go, `i` = site index).
// Go: hugolib/content_map_page.go:newPageMap
pub fn new_page_map(site_idx: usize) -> PageMap {
    PageMap {
        site_idx,
        cache_pages1: Partition::new(format!("/pag1/{site_idx}")),
        cache_pages2: Partition::new(format!("/pag2/{site_idx}")),
        cache_resources: Partition::new(format!("/ress/{site_idx}")),
        cache_get_terms: Partition::new(format!("/gett/{site_idx}")),
        cache_content_rendered: Partition::new(format!("/cont/ren/{site_idx}")),
        cache_content_plain: Partition::new(format!("/cont/pla/{site_idx}")),
        cache_content_toc: Partition::new(format!("/cont/toc/{site_idx}")),
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/content_map_page.go lines 88-259 and 636-994 (the rest is in content_map_page.rs)
//   types: pageMap, pageTrees, weightedContentNode, buildStateReseter, contentNodeI, contentNodeIs,
//          contentNodeShifter, contentTreeReverseIndex
//    L112-114: (m *pageMap) Reset()
//    L147-191: (t *pageTrees) collectAndMarkStaleIdentities(p *paths.Path) []identity.Identity
//    L194-198: (t *pageTrees) collectIdentitiesSurrounding(key string, maxSamplesPerTree int) []identity.Identity
//    L200-221: (t *pageTrees) collectIdentitiesSurroundingIn(key string, maxSamples int, tree *doctree.NodeShiftTree[contentNodeI]) []identity.Identity
//    L223-232: (t *pageTrees) DeletePageAndResourcesBelow(ss ...string)
// EX L235-243: (t pageTrees) Shape(d, v int) *pageTrees
// EX L245-254: (t *pageTrees) createMutableTrees()
//    L659-661: (n contentNodeIs) Path() string
//    L663-665: (n contentNodeIs) isContentNodeBranch() bool
//    L667-669: (n contentNodeIs) GetIdentity() identity.Identity
//    L671-680: (n contentNodeIs) ForEeachIdentity(f func(identity.Identity) bool) bool
//    L682-688: (n contentNodeIs) resetBuildState()
//    L690-694: (n contentNodeIs) MarkStale()
//    L700-744: (s *contentNodeShifter) Delete(n contentNodeI, dimension doctree.Dimension) (contentNodeI, bool, bool)
// EX L746-792: (s *contentNodeShifter) Shift(n contentNodeI, dimension doctree.Dimension, exact bool) (contentNodeI, bool, doctree.DimensionFlag)
// EX L794-811: (s *contentNodeShifter) ForEeachInDimension(n contentNodeI, d int, f func(contentNodeI) bool)
//    L813-852: (s *contentNodeShifter) InsertInto(old, new contentNodeI, dimension doctree.Dimension) (contentNodeI, contentNodeI, bool)
// EX L854-911: (s *contentNodeShifter) Insert(old, new contentNodeI) (contentNodeI, contentNodeI, bool)
// EX L913-969: newPageMap(i int, s *Site, mcache *dynacache.Cache, pageTrees *pageTrees) *pageMap
// EX L971-976: newContentTreeTreverseIndex(init func(get func(key any) (contentNodeI, bool), set func(key any, val contentNodeI))) *contentTreeReverseIndex
//    L983-985: (c *contentTreeReverseIndex) Reset()
//    L987-993: (c *contentTreeReverseIndex) Get(key any) contentNodeI
// ---------------------------------------------------------------------------
