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
//!
//! Go's `Shape(d, v)` returns a shaped copy of the trees; in Rust a tree's shape is a value
//! (`nh_doctree::dimensions::Dimension`, here `PageMap::dims`) passed to the dimension-dependent
//! tree methods (see `crates/nh-doctree/PORTING.md`).

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, OnceLock};

use nh_common::Result;
use nh_common::dynacache::Partition;
use nh_doctree::dimensions::{DIMENSION_LANGUAGE, Dimension, DimensionFlag};
use nh_doctree::nodeshifttree::{NodeShiftTree, Shifter, WalkConfig};
use nh_doctree::treeshifttree::TreeShiftTree;
use nh_page::page::Pages;
use nh_resource::resourcetypes::Resources;

use crate::content_map::{ContentMapConfig, ResourceSource};
use crate::page::{PageId, PageState};

/// A tree node (Go `contentNodeI`): one node, or one per language (Go `contentNodeIs`,
/// `resourceSources`).
///
/// A page node carries the language index of its site (Go `p.s.languagei`), which the shifter
/// needs; the page itself lives in the `HugoSites.pages` arena.
#[derive(Clone, Debug)]
pub enum ContentNode {
    /// Go `*pageState`: the page and its site's language index.
    Page(PageId, usize),
    /// Go `*resourceSource`.
    Resource(Arc<ResourceSource>),
    /// Go `contentNodeIs`, indexed by language index (pages only in the Hugo trees).
    Pages(Vec<Option<PageId>>),
    /// Go `resourceSources`, indexed by language index.
    Resources(Vec<Option<Arc<ResourceSource>>>),
}

impl ContentNode {
    /// The page of a shifted page node (Go `n.(*pageState)`).
    pub fn page_id(&self) -> Option<PageId> {
        match self {
            ContentNode::Page(id, _) => Some(*id),
            _ => None,
        }
    }

    /// The resource of a shifted resource node (Go `n.(*resourceSource)`).
    pub fn resource(&self) -> Option<&Arc<ResourceSource>> {
        match self {
            ContentNode::Resource(r) => Some(r),
            _ => None,
        }
    }

    /// Go: `contentNodeIs.Path()` / `(*resourceSource).Path()` (`resourceSources.Path` panics
    /// in Go: "not supported"). Pages need the arena.
    pub fn resource_path(&self) -> Option<String> {
        match self {
            ContentNode::Resource(r) => r.path.as_ref().map(|p| p.path().to_string()),
            _ => None,
        }
    }
}

/// Go: `contentNodeShifter{numLanguages}`.
pub struct ContentNodeShifter {
    pub num_languages: usize,
}

impl ContentNodeShifter {
    /// Go: `contentNodeShifter.Delete(n, dimension)` on a node Go mutates in place.
    fn delete_slot<T: Clone>(v: &mut [Option<T>], lidx: usize) -> (Option<T>, bool, bool) {
        let deleted = v[lidx].take();
        // resource.MarkStale(deleted): no stale tracking in a one-shot build.
        let was_deleted = deleted.is_some();
        let is_empty = v.iter().all(|vv| vv.is_none());
        (deleted, was_deleted, is_empty)
    }
}

impl Shifter<ContentNode> for ContentNodeShifter {
    // Go: hugolib/content_map_page.go:ForEeachInDimension
    fn for_each_in_dimension(
        &self,
        n: &ContentNode,
        d: usize,
        f: &mut dyn FnMut(&ContentNode) -> bool,
    ) {
        if d != DIMENSION_LANGUAGE {
            panic!("only language dimension supported");
        }

        match n {
            ContentNode::Pages(vv) => {
                for (i, v) in vv.iter().enumerate() {
                    if let Some(id) = v
                        && f(&ContentNode::Page(*id, i))
                    {
                        return;
                    }
                }
            }
            // Go's default case: the node itself (a resourceSources node included).
            _ => {
                f(n);
            }
        }
    }

    // Go: hugolib/content_map_page.go:Insert
    fn insert(
        &self,
        old: ContentNode,
        new: ContentNode,
    ) -> (ContentNode, Option<ContentNode>, bool) {
        match old {
            ContentNode::Page(old_id, old_lang) => {
                let ContentNode::Page(new_id, new_lang) = new else {
                    panic!("unknown type {}", type_name(&new));
                };
                if old_lang == new_lang {
                    // resource.MarkStale(old) when newp != old: no stale tracking.
                    return (new, Some(ContentNode::Page(old_id, old_lang)), true);
                }
                let mut is = vec![None; self.num_languages];
                is[new_lang] = Some(new_id);
                is[old_lang] = Some(old_id);
                (
                    ContentNode::Pages(is),
                    Some(ContentNode::Page(old_id, old_lang)),
                    false,
                )
            }
            ContentNode::Pages(mut vv) => {
                let ContentNode::Page(new_id, new_lang) = new else {
                    panic!("unknown type {}", type_name(&new));
                };
                let oldp = vv[new_lang];
                vv[new_lang] = Some(new_id);
                (
                    ContentNode::Pages(vv),
                    oldp.map(|id| ContentNode::Page(id, new_lang)),
                    oldp.is_some(),
                )
            }
            ContentNode::Resource(vv) => {
                let ContentNode::Resource(newp) = new else {
                    panic!("unknown type {}", type_name(&new));
                };
                if vv.lang_index == newp.lang_index {
                    return (
                        ContentNode::Resource(newp),
                        Some(ContentNode::Resource(vv)),
                        true,
                    );
                }
                let mut rs = vec![None; self.num_languages];
                let (nl, ol) = (newp.lang_index, vv.lang_index);
                rs[nl] = Some(newp);
                rs[ol] = Some(vv.clone());
                (
                    ContentNode::Resources(rs),
                    Some(ContentNode::Resource(vv)),
                    false,
                )
            }
            ContentNode::Resources(mut vv) => {
                let ContentNode::Resource(newp) = new else {
                    panic!("unknown type {}", type_name(&new));
                };
                let nl = newp.lang_index;
                let oldp = vv[nl].replace(newp);
                let updated = oldp.is_some();
                (
                    ContentNode::Resources(vv),
                    oldp.map(ContentNode::Resource),
                    updated,
                )
            }
        }
    }

    // Go: hugolib/content_map_page.go:InsertInto
    fn insert_into(
        &self,
        old: ContentNode,
        new: ContentNode,
        dimension: Dimension,
    ) -> (ContentNode, Option<ContentNode>, bool) {
        let langi = dimension[DIMENSION_LANGUAGE];
        match old {
            ContentNode::Page(old_id, old_lang) => {
                let ContentNode::Page(_, new_lang) = new else {
                    panic!("unknown type {}", type_name(&new));
                };
                if old_lang == new_lang && new_lang == langi {
                    return (new, Some(ContentNode::Page(old_id, old_lang)), true);
                }
                let mut is = vec![None; self.num_languages];
                is[old_lang] = Some(old_id);
                is[langi] = new.page_id();
                (
                    ContentNode::Pages(is),
                    Some(ContentNode::Page(old_id, old_lang)),
                    false,
                )
            }
            ContentNode::Pages(mut vv) => {
                let oldv = vv[langi];
                vv[langi] = new.page_id();
                (
                    ContentNode::Pages(vv),
                    oldv.map(|id| ContentNode::Page(id, langi)),
                    oldv.is_some(),
                )
            }
            ContentNode::Resources(mut vv) => {
                let ContentNode::Resource(newp) = new else {
                    panic!("interface conversion: {}", type_name(&new));
                };
                let oldv = vv[langi].replace(newp);
                let updated = oldv.is_some();
                (
                    ContentNode::Resources(vv),
                    oldv.map(ContentNode::Resource),
                    updated,
                )
            }
            ContentNode::Resource(vv) => {
                let ContentNode::Resource(newp) = new else {
                    panic!("unknown type {}", type_name(&new));
                };
                if vv.lang_index == newp.lang_index && newp.lang_index == langi {
                    return (
                        ContentNode::Resource(newp),
                        Some(ContentNode::Resource(vv)),
                        true,
                    );
                }
                let mut rs = vec![None; self.num_languages];
                rs[vv.lang_index] = Some(vv.clone());
                rs[langi] = Some(newp);
                (
                    ContentNode::Resources(rs),
                    Some(ContentNode::Resource(vv)),
                    false,
                )
            }
        }
    }

    /// The trees call [`Shifter::delete_in_place`]; this consuming form answers like Go for
    /// single-language nodes and for a multi-language node whose last slot is deleted.
    // Go: hugolib/content_map_page.go:Delete
    fn delete(
        &self,
        mut v: ContentNode,
        dimension: Dimension,
    ) -> (Option<ContentNode>, bool, bool) {
        self.delete_in_place(&mut v, dimension)
    }

    // Go: hugolib/content_map_page.go:Delete
    fn delete_in_place(
        &self,
        v: &mut ContentNode,
        dimension: Dimension,
    ) -> (Option<ContentNode>, bool, bool) {
        let lidx = dimension[0];
        match v {
            ContentNode::Pages(vv) => {
                let (deleted, was_deleted, is_empty) = Self::delete_slot(vv, lidx);
                (
                    deleted.map(|id| ContentNode::Page(id, lidx)),
                    was_deleted,
                    is_empty,
                )
            }
            ContentNode::Resources(vv) => {
                let (deleted, was_deleted, is_empty) = Self::delete_slot(vv, lidx);
                (deleted.map(ContentNode::Resource), was_deleted, is_empty)
            }
            ContentNode::Resource(r) => {
                if lidx != r.lang_index {
                    return (None, false, false);
                }
                (Some(v.clone()), true, true)
            }
            ContentNode::Page(_, lang) => {
                if lidx != *lang {
                    return (None, false, false);
                }
                (Some(v.clone()), true, true)
            }
        }
    }

    /// Pages: exact language only. Resources, non-exact: first non-nil language, flag 0.
    // Go: hugolib/content_map_page.go:Shift
    fn shift(
        &self,
        v: &ContentNode,
        dimension: Dimension,
        exact: bool,
    ) -> (Option<ContentNode>, bool, DimensionFlag) {
        let lidx = dimension[0];
        // How accurate is the match.
        let accuracy = DimensionFlag::LANGUAGE;
        match v {
            ContentNode::Pages(vv) => {
                if vv.is_empty() {
                    panic!("empty contentNodeIs");
                }
                if let Some(id) = vv[lidx] {
                    return (Some(ContentNode::Page(id, lidx)), true, accuracy);
                }
                (None, false, DimensionFlag(0))
            }
            ContentNode::Resources(vv) => {
                if let Some(r) = &vv[lidx] {
                    return (
                        Some(ContentNode::Resource(r.clone())),
                        true,
                        DimensionFlag::LANGUAGE,
                    );
                }
                if exact {
                    return (None, false, DimensionFlag(0));
                }
                // For non content resources, pick the first match.
                if let Some(r) = vv.iter().flatten().next() {
                    if r.is_page() {
                        return (None, false, DimensionFlag(0));
                    }
                    return (
                        Some(ContentNode::Resource(r.clone())),
                        true,
                        DimensionFlag(0),
                    );
                }
                (None, false, DimensionFlag(0))
            }
            ContentNode::Resource(r) => {
                if r.lang_index == lidx {
                    return (Some(v.clone()), true, DimensionFlag::LANGUAGE);
                }
                if !r.is_page() && !exact {
                    return (Some(v.clone()), true, DimensionFlag(0));
                }
                (None, false, DimensionFlag(0))
            }
            ContentNode::Page(_, lang) => {
                if *lang == lidx {
                    return (Some(v.clone()), true, DimensionFlag::LANGUAGE);
                }
                (None, false, DimensionFlag(0))
            }
        }
    }
}

/// Go's `%T` of a tree node (panic messages).
fn type_name(n: &ContentNode) -> &'static str {
    match n {
        ContentNode::Page(..) => "*hugolib.pageState",
        ContentNode::Resource(_) => "*hugolib.resourceSource",
        ContentNode::Pages(_) => "hugolib.contentNodeIs",
        ContentNode::Resources(_) => "hugolib.resourceSources",
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
    /// Go's `treePagesResources` (both trees) and `resourceTrees` (`treeResources`) are views
    /// the Rust callers take directly from the two fields. `treePagesFromTemplateAdapters`
    /// (content adapters) is not ported.
    // Go: hugolib/content_map_page.go:createMutableTrees
    pub fn new(num_languages: usize) -> PageTrees {
        let ns: Arc<dyn Shifter<ContentNode>> = Arc::new(ContentNodeShifter { num_languages });
        PageTrees {
            tree_pages: NodeShiftTree::new(ns.clone()),
            tree_resources: NodeShiftTree::new(ns),
            tree_taxonomy_entries: TreeShiftTree::new(num_languages),
        }
    }

    /// Go: `Shape(d, v)` — the shape of a site's view of the trees (Go returns shaped copies of
    /// the trees; Rust passes the shape to the tree methods).
    // Go: hugolib/content_map_page.go:Shape
    pub fn shape(&self, d: usize, v: usize) -> Dimension {
        self.tree_pages.shape(d, v)
    }
}

/// Go: `pageMap` (per site): the site's view of the shared trees + query caches.
///
/// The caches are dynacache partitions (Go `dynacache.GetOrCreatePartition`): `get_or_create`
/// runs the create function WITHOUT holding the partition lock, and the first stored value wins.
/// The queries themselves are `impl PageMap` blocks in `content_map_page.rs` (T21).
pub struct PageMap {
    pub site_idx: usize,
    /// Go `pageTrees.Shape(0, i)`: this site's shape of the shared trees.
    pub dims: Dimension,
    /// Used for simple page lookups by name, e.g. "mypage.md" or "mypage".
    pub page_reverse_index: ContentTreeReverseIndex,
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
    pub cache_content_rendered:
        Partition<String, Arc<Result<crate::page__content::ContentSummary>>>,
    /// Go `cacheContentPlain` (same key scheme).
    pub cache_content_plain:
        Partition<String, Arc<Result<crate::page__content::ContentPlainPlainWords>>>,
    /// Go `contentTableOfContents` (same key scheme).
    pub cache_content_toc:
        Partition<String, Arc<Result<crate::page__content::ContentTableOfContents>>>,
    /// Go `cfg contentMapConfig`.
    pub cfg: ContentMapConfig,
}

impl PageMap {
    /// Go: `(m *pageMap) Reset()` (invoked on rebuilds: resets the reverse index).
    // Go: hugolib/content_map_page.go:Reset
    pub fn reset(&mut self) {
        self.page_reverse_index.reset();
    }
}

/// Go: `newPageMap(i, s, mcache, pageTrees)` (partition names as in Go, `i` = site index;
/// `cfg` is the site's `contentMapConfig`, see `content_map::new_content_map_config`).
// Go: hugolib/content_map_page.go:newPageMap
pub fn new_page_map(site_idx: usize, cfg: ContentMapConfig) -> PageMap {
    let mut dims: Dimension = [0];
    dims[DIMENSION_LANGUAGE] = site_idx;
    PageMap {
        site_idx,
        dims,
        page_reverse_index: new_content_tree_reverse_index(),
        cache_pages1: Partition::new(format!("/pag1/{site_idx}")),
        cache_pages2: Partition::new(format!("/pag2/{site_idx}")),
        cache_resources: Partition::new(format!("/ress/{site_idx}")),
        cache_get_terms: Partition::new(format!("/gett/{site_idx}")),
        cache_content_rendered: Partition::new(format!("/cont/ren/{site_idx}")),
        cache_content_plain: Partition::new(format!("/cont/pla/{site_idx}")),
        cache_content_toc: Partition::new(format!("/cont/toc/{site_idx}")),
        cfg,
    }
}

/// An entry of the reverse index: a page, or Go's `ambiguousContentNode` (several pages share
/// the name).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReverseIndexEntry {
    Page(PageId),
    Ambiguous,
}

/// Go: `contentTreeReverseIndex` — `BaseNameNoIdentifier()` -> page, built lazily (once) by
/// walking `treePages` in the site's language.
#[derive(Default)]
pub struct ContentTreeReverseIndex {
    mm: OnceLock<HashMap<String, ReverseIndexEntry>>,
}

/// Go: `newContentTreeTreverseIndex(init)` with `newPageMap`'s init func (the walk is in
/// [`ContentTreeReverseIndex::get`]).
// Go: hugolib/content_map_page.go:newContentTreeTreverseIndex
pub fn new_content_tree_reverse_index() -> ContentTreeReverseIndex {
    ContentTreeReverseIndex::default()
}

impl ContentTreeReverseIndex {
    // Go: hugolib/content_map_page.go:Reset
    pub fn reset(&mut self) {
        self.mm = OnceLock::new();
    }

    /// Go: `Get(key)` — `mm.InitAndGet`: the first call builds the index with `newPageMap`'s
    /// init func (walk `treePages` shaped `dims`; for each page add
    /// `PathInfo().BaseNameNoIdentifier()`; a second page with the same name makes it
    /// ambiguous).
    // Go: hugolib/content_map_page.go:Get
    pub fn get(
        &self,
        key: &str,
        trees: &PageTrees,
        pages: &[PageState],
        dims: Dimension,
    ) -> Option<ReverseIndexEntry> {
        let mm = self.mm.get_or_init(|| {
            let mut mm: HashMap<String, ReverseIndexEntry> = HashMap::new();
            let mut add = |k: &str, n: PageId| match mm.get(k) {
                Some(existing) if *existing != ReverseIndexEntry::Ambiguous => {
                    mm.insert(k.to_string(), ReverseIndexEntry::Ambiguous);
                }
                Some(_) => {}
                None => {
                    mm.insert(k.to_string(), ReverseIndexEntry::Page(n));
                }
            };
            let cfg = WalkConfig {
                dims,
                ..Default::default()
            };
            let res = trees.tree_pages.walk(&cfg, |_w, _s, n, _match| {
                let Some(id) = n.page_id() else {
                    panic!("interface conversion: contentNodeI is not *hugolib.pageState");
                };
                let p = &pages[id.0 as usize];
                add(p.meta.path_info.base_name_no_identifier(), id);
                Ok(false)
            });
            if let Err(err) = res {
                panic!("{}", err.message());
            }
            mm
        });
        mm.get(key).copied()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/content_map_page.go lines 88-259 and 636-994 (the rest is in content_map_page.rs)
//   types: pageMap, pageTrees, weightedContentNode, buildStateReseter, contentNodeI, contentNodeIs,
//          contentNodeShifter, contentTreeReverseIndex
// OK L112-114: (m *pageMap) Reset()
//    L147-191: (t *pageTrees) collectAndMarkStaleIdentities(p *paths.Path) []identity.Identity
//    L194-198: (t *pageTrees) collectIdentitiesSurrounding(key string, maxSamplesPerTree int) []identity.Identity
//    L200-221: (t *pageTrees) collectIdentitiesSurroundingIn(key string, maxSamples int, tree *doctree.NodeShiftTree[contentNodeI]) []identity.Identity
//    L223-232: (t *pageTrees) DeletePageAndResourcesBelow(ss ...string)
// OK L235-243: (t pageTrees) Shape(d, v int) *pageTrees
// OK L245-254: (t *pageTrees) createMutableTrees()
//    L659-661: (n contentNodeIs) Path() string
//    L663-665: (n contentNodeIs) isContentNodeBranch() bool
//    L667-669: (n contentNodeIs) GetIdentity() identity.Identity
//    L671-680: (n contentNodeIs) ForEeachIdentity(f func(identity.Identity) bool) bool
//    L682-688: (n contentNodeIs) resetBuildState()
//    L690-694: (n contentNodeIs) MarkStale()
// OK L700-744: (s *contentNodeShifter) Delete(n contentNodeI, dimension doctree.Dimension) (contentNodeI, bool, bool)
// OK L746-792: (s *contentNodeShifter) Shift(n contentNodeI, dimension doctree.Dimension, exact bool) (contentNodeI, bool, doctree.DimensionFlag)
// OK L794-811: (s *contentNodeShifter) ForEeachInDimension(n contentNodeI, d int, f func(contentNodeI) bool)
// OK L813-852: (s *contentNodeShifter) InsertInto(old, new contentNodeI, dimension doctree.Dimension) (contentNodeI, contentNodeI, bool)
// OK L854-911: (s *contentNodeShifter) Insert(old, new contentNodeI) (contentNodeI, contentNodeI, bool)
// OK L913-969: newPageMap(i int, s *Site, mcache *dynacache.Cache, pageTrees *pageTrees) *pageMap
// OK L971-976: newContentTreeTreverseIndex(init func(get func(key any) (contentNodeI, bool), set func(key any, val contentNodeI))) *contentTreeReverseIndex
// OK L983-985: (c *contentTreeReverseIndex) Reset()
// OK L987-993: (c *contentTreeReverseIndex) Get(key any) contentNodeI
// ---------------------------------------------------------------------------
