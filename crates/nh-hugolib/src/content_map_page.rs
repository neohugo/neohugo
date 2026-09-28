//! Port of `hugolib/content_map_page.go`.
//!
//! Owner: Wave B task T21 (hugolib-assemble).

//! Go `hugolib/content_map_page.go` (queries + assembly; the tree types, the language shifter,
//! the `pageMap` struct and `newPageMap` are in `content_map_trees.rs`, owned by T20 because
//! capture inserts into them): page queries (`getPagesInSection`, `getPagesWithTerm` — note the shared cache key quirk for
//! term `.Pages`/`.RegularPages`), resources of a page, and the ASSEMBLY steps:
//! step 1 per site: addMissingTaxonomies, addMissingRootSections, addStandalonePages,
//! applyAggregates; step 2: removeShouldNotBuild, assembleTermsAndTranslations (term key =
//! pathparser Base of "/plural/value/_index.md" — NOT sanitized; LAST value wins for m.term),
//! applyAggregatesToTaxonomiesAndTerms; final: assembleResources; lazy CreateSiteTaxonomies.
//!
//! The assembly runs on `&mut HugoSites` (HUGO_LAYER.md §4.1). Walks that insert into or delete
//! from the walked tree use `NodeShiftTree::walk_mut` (Go's walk-under-mutation semantics, see
//! nh-doctree's PORTING.md); when the handle also creates pages (`h.newPage`, which needs the
//! whole `HugoSites`), the walked tree is moved out of `HugoSites` for the walk
//! ([`with_tree_pages`]; `new_page` never reads the trees).
//!
//! The "dates" events of the aggregation walks go through nh-doctree's `WalkContext`; its
//! listeners cannot borrow the page arena, so each listener records (listening page, event
//! source) and the updates are applied in that order once the events are handled: Go's
//! listeners only read the source's dates and write the listener's, so the result is the same.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

use go_value::{GoString, Time, Value};
use nh_common::Result;
use nh_common::herrors::Error;
use nh_common::kinds;
use nh_common::paths::path as cpaths;
use nh_doctree::dimensions::{Dimension, DimensionFlag};
use nh_doctree::nodeshifttree::{NodeShiftTree, WalkConfig};
use nh_doctree::support::{Event, WalkContext};
use nh_page::page::{PageRef, Pages};
use nh_page::page_matcher::Cascade;
use nh_page::pagemeta::page_frontmatter::PageConfig;
use nh_resource::resourcetypes::{Resource, Resources};

use crate::content_map::ResourceSource;
pub use crate::content_map_trees::{
    ContentNode, ContentNodeShifter, PageMap, PageTrees, WeightedContentNode,
};
use crate::hugo_sites::HugoSites;
use crate::page::{PageHandle, PageId, PageState, PageWrapper};
use crate::page__new::{NewPageMeta, new_page};

/// Go: `predicate.P[*pageState]`.
pub type PagePredicate = nh_common::predicate::P<PageState>;

/// Go: `pagePredicates`.
pub mod page_predicates {
    use super::PagePredicate;
    use nh_common::kinds;
    use std::sync::Arc;

    pub fn kind_page() -> PagePredicate {
        Arc::new(|p| p.meta.kind() == kinds::KIND_PAGE)
    }
    pub fn kind_section() -> PagePredicate {
        Arc::new(|p| p.meta.kind() == kinds::KIND_SECTION)
    }
    pub fn kind_home() -> PagePredicate {
        Arc::new(|p| p.meta.kind() == kinds::KIND_HOME)
    }
    pub fn kind_term() -> PagePredicate {
        Arc::new(|p| p.meta.kind() == kinds::KIND_TERM)
    }
    pub fn should_list_local() -> PagePredicate {
        Arc::new(|p| p.meta.should_list(false))
    }
    pub fn should_list_global() -> PagePredicate {
        Arc::new(|p| p.meta.should_list(true))
    }
    pub fn should_list_any() -> PagePredicate {
        Arc::new(|p| p.meta.should_list_any())
    }
    pub fn should_link() -> PagePredicate {
        Arc::new(|p| !p.meta.no_link())
    }

    /// Go: `p.And(ps...)`.
    pub fn and(p: PagePredicate, ps: Vec<PagePredicate>) -> PagePredicate {
        nh_common::predicate::and(Some(p), ps)
    }

    /// Go: `p.Or(ps...)`.
    pub fn or(p: PagePredicate, ps: Vec<PagePredicate>) -> PagePredicate {
        nh_common::predicate::or(Some(p), ps)
    }
}

/// Go: `pageMapQueryPagesBelowPath` (must be hashable in Go: its `Key()` is the cache key).
#[derive(Clone, Default)]
pub struct PageMapQueryPagesBelowPath {
    pub path: String,
    /// Additional identifier for this query. Used as part of the cache key.
    pub key_part: String,
    /// Page inclusion filter. May be nil.
    pub include: Option<PagePredicate>,
}

impl PageMapQueryPagesBelowPath {
    /// Go: `Key()` = Path + "/" + KeyPart.
    // Go: hugolib/content_map_page.go:Key
    pub fn key(&self) -> String {
        format!("{}/{}", self.path, self.key_part)
    }
}

impl std::fmt::Debug for PageMapQueryPagesBelowPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PageMapQueryPagesBelowPath")
            .field("path", &self.path)
            .field("key_part", &self.key_part)
            .field("include", &self.include.is_some())
            .finish()
    }
}

/// Go: `pageMapQueryPagesInSection`.
#[derive(Clone, Default)]
pub struct PageMapQueryPagesInSection {
    pub path: String,
    pub key_part: String,
    pub recursive: bool,
    pub include_self: bool,
    /// Go `Include` of the embedded `pageMapQueryPagesBelowPath` (nil = `ShouldListLocal`).
    pub include: Option<PagePredicate>,
}

impl std::fmt::Debug for PageMapQueryPagesInSection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PageMapQueryPagesInSection")
            .field("path", &self.path)
            .field("key_part", &self.key_part)
            .field("recursive", &self.recursive)
            .field("include_self", &self.include_self)
            .field("include", &self.include.is_some())
            .finish()
    }
}

impl PageMapQueryPagesInSection {
    /// Go: `Key()` = "gagesInSection/" + Path + "/" + KeyPart + "/" + Recursive + "/" + IncludeSelf.
    // Go: hugolib/content_map_page.go:Key
    pub fn key(&self) -> String {
        format!(
            "gagesInSection/{}/{}/{}/{}",
            self.path, self.key_part, self.recursive, self.include_self
        )
    }
}

/// The template handle of a page of the frozen sites.
fn handle(h: &Arc<HugoSites>, id: PageId, wrapper: PageWrapper) -> PageRef {
    PageHandle {
        h: h.clone(),
        id,
        wrapper,
    }
    .page_ref()
}

/// Go `n.(*pageState)` on a node of `treePages` (always a page there).
fn must_page(n: &ContentNode) -> PageId {
    n.page_id()
        .expect("interface conversion: contentNodeI is not *hugolib.pageState")
}

/// Go `n.isContentNodeBranch()` of a page node (`p.IsNode()`).
fn is_branch(pages: &[PageState], id: PageId) -> bool {
    pages[id.0 as usize].meta.is_node()
}

impl PageMap {
    /// Go: `(m *pageMap) forEachPage(include, fn)` — `fn` returns true to stop the walk.
    // Go: hugolib/content_map_page.go:forEachPage
    pub fn for_each_page(
        h: &HugoSites,
        site_idx: usize,
        include: Option<&PagePredicate>,
        f: &mut dyn FnMut(&PageState) -> Result<bool>,
    ) -> Result<()> {
        let cfg = WalkConfig {
            dims: h.sites[site_idx].page_map.dims,
            ..Default::default()
        };
        h.page_trees.tree_pages.walk(&cfg, |_w, _key, n, _match| {
            if let Some(id) = n.page_id() {
                let p = h.page(id);
                if include.is_none_or(|inc| inc(p)) {
                    let terminate = f(p)?;
                    if terminate {
                        return Ok(true);
                    }
                }
            }
            Ok(false)
        })
    }

    /// Go: `forEeachPageIncludingBundledPages(include, fn)`.
    // Go: hugolib/content_map_page.go:forEeachPageIncludingBundledPages
    pub fn for_each_page_including_bundled_pages(
        h: &HugoSites,
        site_idx: usize,
        include: Option<&PagePredicate>,
        f: &mut dyn FnMut(&PageState) -> Result<bool>,
    ) -> Result<()> {
        Self::for_each_page(h, site_idx, include, f)?;

        let cfg = WalkConfig {
            dims: h.sites[site_idx].page_map.dims,
            ..Default::default()
        };
        h.page_trees
            .tree_resources
            .walk(&cfg, |_w, _key, n, _match| {
                if let ContentNode::Resource(rs) = n
                    && let Some(id) = rs.page
                {
                    let p = h.page(id);
                    if include.is_none_or(|inc| inc(p)) {
                        let terminate = f(p)?;
                        if terminate {
                            return Ok(true);
                        }
                    }
                }
                Ok(false)
            })
    }

    /// Go: `getOrCreatePagesFromCache(cache, key, create)` (nil cache = `cachePages1`).
    // Go: hugolib/content_map_page.go:getOrCreatePagesFromCache
    pub fn get_or_create_pages_from_cache(
        &self,
        cache: Option<&nh_common::dynacache::Partition<String, Pages>>,
        key: String,
        create: impl FnOnce(&String) -> Result<Pages>,
    ) -> Result<Pages> {
        let cache = cache.unwrap_or(&self.cache_pages1);
        cache.get_or_create(key, create)
    }

    /// Go: `getPagesInSection(q)` (sorted by default; recursive or not; include self).
    // Go: hugolib/content_map_page.go:getPagesInSection
    pub fn get_pages_in_section(
        h: &Arc<HugoSites>,
        site_idx: usize,
        q: &PageMapQueryPagesInSection,
    ) -> Pages {
        let cache_key = q.key();
        let m = &h.sites[site_idx].page_map;
        let dims = m.dims;

        let pages = m.get_or_create_pages_from_cache(None, cache_key, |_| {
            let prefix = cpaths::add_trailing_slash(&q.path);

            let mut pas: Vec<PageId> = Vec::new();
            let mut other_branch = String::new();

            let include = q
                .include
                .clone()
                .unwrap_or_else(page_predicates::should_list_local);

            let cfg = WalkConfig {
                dims,
                prefix,
                ..Default::default()
            };

            let res = h.page_trees.tree_pages.walk(&cfg, |w, key, n, _match| {
                let id = n.page_id();
                if q.recursive {
                    if let Some(id) = id
                        && include(h.page(id))
                    {
                        pas.push(id);
                    }
                    return Ok(false);
                }

                if let Some(id) = id
                    && include(h.page(id))
                {
                    pas.push(id);
                }

                if id.is_some_and(|id| is_branch(&h.pages, id)) {
                    let current_branch = format!("{key}/");
                    if other_branch.is_empty() || other_branch != current_branch {
                        w.skip_prefix(&current_branch);
                    }
                    other_branch = current_branch;
                }
                Ok(false)
            });

            let mut pas: Pages = pas
                .into_iter()
                .map(|id| handle(h, id, PageWrapper::None))
                .collect();

            if res.is_ok() {
                if q.include_self
                    && let Some(n) = h.page_trees.tree_pages.get(dims, &q.path)
                    && let Some(id) = n.page_id()
                    && include(h.page(id))
                {
                    pas.push(handle(h, id, PageWrapper::None));
                }
                nh_page::pages_sort::sort_by_default(&mut pas);
            }

            res.map(|_| pas)
        });

        pages.unwrap_or_else(|err| panic!("{}", err.message()))
    }

    /// Go: `getPagesWithTerm(q)` — entries under the term key, wrapped as `pageWithWeight0`.
    // Go: hugolib/content_map_page.go:getPagesWithTerm
    pub fn get_pages_with_term(
        h: &Arc<HugoSites>,
        site_idx: usize,
        q: &PageMapQueryPagesBelowPath,
    ) -> Pages {
        let key = q.key();
        let m = &h.sites[site_idx].page_map;
        let lang = m.dims[0];

        let v = m.cache_pages1.get_or_create(key, |_| {
            let mut pas: Pages = Vec::new();
            let include = q
                .include
                .clone()
                .unwrap_or_else(page_predicates::should_list_local);

            h.page_trees.tree_taxonomy_entries.walk_prefix(
                lang,
                &cpaths::add_trailing_slash(&q.path),
                &mut |_s, n| {
                    let p = h.page(n.n);
                    if !include(p) {
                        return Ok(false);
                    }
                    pas.push(handle(h, n.n, PageWrapper::Weight0(n.weight)));
                    Ok(false)
                },
            )?;

            nh_page::pages_sort::sort_by_default(&mut pas);

            Ok(pas)
        });

        v.unwrap_or_else(|err| panic!("{}", err.message()))
    }

    /// Go: `getTermsForPageInTaxonomy(path, taxonomy)` — the terms (as `pageWithOrdinal`) of
    /// the page `path` in the taxonomy, sorted.
    // Go: hugolib/content_map_page.go:getTermsForPageInTaxonomy
    pub fn get_terms_for_page_in_taxonomy(
        h: &Arc<HugoSites>,
        site_idx: usize,
        path: &str,
        taxonomy: &str,
    ) -> Pages {
        let prefix = cpaths::add_leading_slash(taxonomy);
        let m = &h.sites[site_idx].page_map;
        let lang = m.dims[0];

        let term_pages = m.cache_get_terms.get_or_create(prefix.clone(), |_| {
            let mut mm: BTreeMap<String, Pages> = BTreeMap::new();
            h.page_trees.tree_taxonomy_entries.walk_prefix(
                lang,
                &cpaths::add_trailing_slash(&prefix),
                &mut |_s, n| {
                    if let Some((term, ordinal)) = n.term {
                        mm.entry(h.page(n.n).meta.path()).or_default().push(handle(
                            h,
                            term,
                            PageWrapper::Ordinal(ordinal),
                        ));
                    }
                    Ok(false)
                },
            )?;

            // Sort the terms.
            for v in mm.values_mut() {
                nh_page::pages_sort::sort_by_default(v);
            }

            Ok(mm)
        });

        match term_pages {
            Ok(mm) => mm.get(path).cloned().unwrap_or_default(),
            Err(err) => panic!("{}", err.message()),
        }
    }

    /// Go: `getResourcesForPage(ps)` (non-exact: TH pages get the EN resource objects).
    // Go: hugolib/content_map_page.go:getResourcesForPage
    pub fn get_resources_for_page(h: &Arc<HugoSites>, p: PageId) -> Result<Resources> {
        let ps = h.page(p);
        let mut res: Resources = Vec::new();
        let _ = for_each_resource_in_page(
            &h.page_trees.tree_pages,
            &h.page_trees.tree_resources,
            &h.pages,
            h.sites[ps.site_idx].page_map.dims,
            ps,
            false,
            &mut |_key, n, _match| {
                if let Some(rs) = n.resource() {
                    if let Some(pid) = rs.page {
                        res.push(Arc::new(PageHandle {
                            h: h.clone(),
                            id: pid,
                            wrapper: PageWrapper::None,
                        }) as Arc<dyn Resource>);
                    } else if let Some(r) = rs.r.get() {
                        res.push(r.clone());
                    }
                }
                Ok(false)
            },
        );
        Ok(res)
    }

    /// Go: `getOrCreateResourcesForPage(ps)` — the page's resources (plus those of its
    /// `translationKey` translations), sorted, with the front matter `resources` metadata.
    // Go: hugolib/content_map_page.go:getOrCreateResourcesForPage
    pub fn get_or_create_resources_for_page(h: &Arc<HugoSites>, p: PageId) -> Resources {
        let ps = h.page(p);
        let mut key_page = ps.meta.path();
        if key_page == "/" {
            key_page = String::new();
        }
        let key = format!("{key_page}/get-resources-for-page");
        let m = &h.sites[ps.site_idx].page_map;
        let v = m.cache_resources.get_or_create(key, |_| {
            let mut res = Self::get_resources_for_page(h, p)?;

            let translation_key = &ps.meta.page_config.translation_key;
            if !translation_key.is_empty() {
                // This this should not be a very common case.
                // Merge in resources from the other languages.
                let translated_pages = h
                    .translation_key_pages
                    .get(translation_key)
                    .cloned()
                    .unwrap_or_default();
                for tp in translated_pages {
                    if tp == p {
                        continue;
                    }
                    // Make sure we query from the correct language root.
                    let res2 = Self::get_resources_for_page(h, tp)?;
                    // Add if Name not already in res.
                    for r in res2 {
                        let n = nh_resource::resourcetypes::name_normalized_or_name(&*r);
                        let found = res.iter().any(|r2| {
                            nh_resource::resourcetypes::name_normalized_or_name(&**r2) == n
                        });
                        if !found {
                            res.push(r);
                        }
                    }
                }
            }

            go_sort::stable_by(&mut res, resource_less);

            let resources_meta = &ps.meta.page_config.resources_meta;
            if !resources_meta.is_empty() {
                for r in res.iter_mut() {
                    *r = nh_resources::resource_metadata::clone_with_metadata_from_map_if_needed(
                        resources_meta,
                        r.clone(),
                    );
                }
                go_sort::stable_by(&mut res, resource_less);
            }

            Ok(res)
        });

        v.unwrap_or_else(|err| panic!("{}", err.message()))
    }

    /// Go: `CreateSiteTaxonomies(ctx)`.
    // Go: hugolib/content_map_page.go:CreateSiteTaxonomies
    pub fn create_site_taxonomies(
        h: &Arc<HugoSites>,
        site_idx: usize,
    ) -> Result<nh_page::taxonomy::TaxonomyList> {
        let m = &h.sites[site_idx].page_map;
        let mut taxonomies: BTreeMap<String, nh_page::taxonomy::Taxonomy> = BTreeMap::new();

        if m.cfg.taxonomy_disabled && m.cfg.taxonomy_term_disabled {
            return Ok(Arc::new(taxonomies));
        }

        let lang = m.dims[0];
        for view_name in &m.cfg.taxonomy_config.views {
            let key = &view_name.plural_tree_key;
            taxonomies.insert(view_name.plural.clone(), BTreeMap::new());
            let cfg = WalkConfig {
                dims: m.dims,
                prefix: cpaths::add_trailing_slash(key),
                ..Default::default()
            };
            h.page_trees.tree_pages.walk(&cfg, |_w, s, n, _match| {
                let id = must_page(n);
                let p = h.page(id);

                if p.meta.kind() == kinds::KIND_TERM {
                    if !p.meta.should_list(true) {
                        return Ok(false);
                    }
                    let Some(taxonomy) = taxonomies.get_mut(&view_name.plural) else {
                        return Err(Error::new(format!(
                            "missing taxonomy: {}",
                            view_name.plural
                        )));
                    };
                    if p.meta.term.is_empty() {
                        panic!("term is empty");
                    }
                    let k = go_unicode::strings::to_lower_str(&p.meta.term).into_owned();

                    h.page_trees.tree_taxonomy_entries.walk_prefix(
                        lang,
                        &cpaths::add_trailing_slash(s),
                        &mut |_ss, wn| {
                            let owner = wn.term.map(|(t, _)| handle(h, t, PageWrapper::None));
                            taxonomy.entry(k.clone()).or_default().push(
                                nh_page::weighted::WeightedPage::new(
                                    wn.weight,
                                    handle(h, wn.n, PageWrapper::None),
                                    owner,
                                ),
                            );
                            Ok(false)
                        },
                    )?;
                }

                Ok(false)
            })?;
        }

        for taxonomy in taxonomies.values_mut() {
            for v in taxonomy.values_mut() {
                nh_page::weighted::sort(v);
            }
        }

        Ok(Arc::new(taxonomies))
    }
}

/// Go's resource sort in `getOrCreateResourcesForPage` (`sort.SliceStable` with this `less`):
/// resource type, pages after other resources, pages by the default page sort, others by
/// name (never through the lazily publishing link methods).
fn resource_less(ri: &Arc<dyn Resource>, rj: &Arc<dyn Resource>) -> bool {
    if ri.resource_type() < rj.resource_type() {
        return true;
    }

    let p1 = ri.as_any().downcast_ref::<PageHandle>();
    let p2 = rj.as_any().downcast_ref::<PageHandle>();

    if p1.is_some() != p2.is_some() {
        // Pull pages behind other resources.
        return p2.is_some();
    }

    if let (Some(p1), Some(p2)) = (p1, p2) {
        return nh_page::pages_sort::default_page_sort(p1, p2);
    }

    // Make sure not to use RelPermalink or any of the other methods that
    // trigger lazy publishing.
    ri.name() < rj.name()
}

/// What `forEachResourceInPage` does with a resource of a branch page.
enum ResourceOwner {
    /// The page owns it: call the handler.
    Handle,
    /// Nothing owns it: stop the walk.
    Stop,
    /// Someone else owns it: skip this prefix.
    Skip(String),
}

/// The ownership loop of `forEachResourceInPage` for branch pages: "A resourceKey always
/// represents a filename with extension. A page key points to the logical path of a page,
/// which when sourced from the filesystem may represent a directory (bundles) or a single
/// content file (e.g. p1.md). So, to avoid any overlapping ambiguity, we start looking from
/// the owning directory."
fn resource_owner(
    tree_pages: &NodeShiftTree<ContentNode>,
    key_page: &str,
    resource_key: &str,
) -> ResourceOwner {
    let mut s = resource_key.to_string();
    loop {
        s = go_path::path::dir(&s);
        let Some(owner_key) = tree_pages.longest_prefix_all(&s) else {
            return ResourceOwner::Stop;
        };
        if owner_key == key_page {
            return ResourceOwner::Handle;
        }

        if s != owner_key && s.starts_with(owner_key.as_str()) {
            // Keep looking
            continue;
        }

        // Stop walking downwards, someone else owns this resource.
        return ResourceOwner::Skip(format!("{owner_key}/"));
    }
}

/// Go: `(m *pageMap) forEachResourceInPage(ps, lockType, exact, handle)` over the site's shape
/// `dims` (read-only walk; `assembleResources` has its own mutating walk).
// Go: hugolib/content_map_page.go:forEachResourceInPage
pub fn for_each_resource_in_page(
    tree_pages: &NodeShiftTree<ContentNode>,
    tree_resources: &NodeShiftTree<ContentNode>,
    pages: &[PageState],
    dims: Dimension,
    ps: &PageState,
    exact: bool,
    handle: &mut dyn FnMut(&str, &ContentNode, DimensionFlag) -> Result<bool>,
) -> Result<()> {
    let _ = pages;
    let mut key_page = ps.meta.path();
    if key_page == "/" {
        key_page = String::new();
    }
    let prefix = cpaths::add_trailing_slash(&ps.meta.path());
    let is_branch = ps.meta.is_node();

    let cfg = WalkConfig {
        dims,
        prefix,
        exact,
        ..Default::default()
    };

    tree_resources.walk(&cfg, |rw, resource_key, n, m| {
        if is_branch {
            match resource_owner(tree_pages, &key_page, resource_key) {
                ResourceOwner::Handle => {}
                ResourceOwner::Stop => return Ok(true),
                ResourceOwner::Skip(p) => {
                    rw.skip_prefix(&p);
                    return Ok(false);
                }
            }
        }
        handle(resource_key, n, m)
    })
}

/// Go: `(t *pageTrees) DeletePageAndResourcesBelow(ss...)` on a site's shaped trees (T20's
/// `content_map_trees.rs` checklist, L223-232: implemented here, its only caller is
/// `removeShouldNotBuild`).
// Go: hugolib/content_map_page.go:DeletePageAndResourcesBelow
pub fn delete_page_and_resources_below(t: &mut PageTrees, dims: Dimension, ss: &[String]) {
    for s in ss {
        t.tree_resources
            .delete_prefix(dims, &cpaths::add_trailing_slash(s));
        t.tree_pages.delete(dims, s);
    }
}

/// Runs `f` with `treePages` moved out of `h`, so that a walk handle can both modify the
/// walked tree and create pages (`new_page` never reads the trees).
fn with_tree_pages<R>(
    h: &mut HugoSites,
    f: impl FnOnce(&mut HugoSites, &mut NodeShiftTree<ContentNode>) -> R,
) -> R {
    let placeholder = NodeShiftTree::new(Arc::new(ContentNodeShifter {
        num_languages: h.sites.len().max(1),
    }));
    let mut tree = std::mem::replace(&mut h.page_trees.tree_pages, placeholder);
    let r = f(h, &mut tree);
    h.page_trees.tree_pages = tree;
    r
}

/// A recorded "dates" event delivery: (listening page, event source).
type DatesLog = Arc<Mutex<Vec<(PageId, PageId, bool, bool)>>>;

const EVENT_DATES: &str = "dates";

/// Go: `sitePagesAssembler` — the assembly steps run on `&mut HugoSites` before freezing.
pub struct SitePagesAssembler<'a> {
    pub h: &'a mut HugoSites,
    pub site_idx: usize,
}

impl SitePagesAssembler<'_> {
    fn dims(&self) -> Dimension {
        self.h.sites[self.site_idx].page_map.dims
    }

    // Go: hugolib/content_map_page.go:assemblePagesStep1
    pub fn assemble_pages_step1(&mut self) -> Result<()> {
        self.add_missing_taxonomies()?;
        self.add_missing_root_sections()?;
        self.add_standalone_pages()?;
        self.apply_aggregates()?;
        Ok(())
    }

    // Go: hugolib/content_map_page.go:assemblePagesStep2
    pub fn assemble_pages_step2(&mut self) -> Result<()> {
        self.remove_should_not_build()?;
        self.assemble_terms_and_translations()?;
        self.apply_aggregates_to_taxonomies_and_terms()?;
        Ok(())
    }

    // Go: hugolib/content_map_page.go:assemblePagesStepFinal
    pub fn assemble_pages_step_final(&mut self) -> Result<()> {
        self.assemble_resources()?;
        Ok(())
    }

    // Go: hugolib/content_map_page.go:addMissingTaxonomies
    fn add_missing_taxonomies(&mut self) -> Result<()> {
        let i = self.site_idx;
        let cfg = self.h.sites[i].page_map.cfg.clone();
        if cfg.taxonomy_disabled && cfg.taxonomy_term_disabled {
            return Ok(());
        }
        let dims = self.dims();
        let pp = self.h.deps.conf.path_parser();

        for view_name in &cfg.taxonomy_config.views {
            let key = &view_name.plural_tree_key;
            if self.h.page_trees.tree_pages.get(dims, key).is_none() {
                let path_info = pp.parse(
                    nh_common::files::COMPONENT_FOLDER_CONTENT,
                    &format!("{key}/_index.md"),
                );
                let m = NewPageMeta {
                    site_idx: Some(i),
                    path_info: Some(Arc::new(path_info)),
                    page_config: Some(page_config_with_kind(kinds::KIND_TAXONOMY)),
                    singular: view_name.singular.clone(),
                    ..Default::default()
                };
                let Some(p) = new_page(self.h, m)? else {
                    continue;
                };
                self.h.page_trees.tree_pages.insert_into_values_dimension(
                    dims,
                    key,
                    ContentNode::Page(p, i),
                );
            }
        }

        Ok(())
    }

    // Go: hugolib/content_map_page.go:addMissingRootSections
    fn add_missing_root_sections(&mut self) -> Result<()> {
        let i = self.site_idx;
        let dims = self.dims();
        let pp = self.h.deps.conf.path_parser();
        let site_lang = self.h.sites[i].language.lang.clone();

        let mut has_home = false;
        let mut home: Option<PageId> = None;

        // Add missing root sections.
        let mut seen: BTreeSet<String> = BTreeSet::new();
        let cfg = WalkConfig {
            dims,
            ..Default::default()
        };
        with_tree_pages(self.h, |h, tree| {
            tree.walk_mut(&cfg, |w, s, n, _match| {
                let id = must_page(n);
                let ps = h.page(id);

                if ps.meta.lang() != site_lang {
                    panic!(
                        "lang mismatch: {}: {} != {}",
                        go_strconv::quote(s),
                        ps.meta.lang(),
                        site_lang
                    );
                }

                if s.is_empty() {
                    has_home = true;
                    home = Some(id);
                    return Ok(false);
                }

                match ps.meta.kind() {
                    kinds::KIND_PAGE | kinds::KIND_SECTION => {
                        // OK
                    }
                    _ => {
                        // Skip taxonomy nodes etc.
                        return Ok(false);
                    }
                }

                let p = ps.meta.path_info.clone();
                let section = p.section();
                if section.is_empty() || seen.contains(section) {
                    return Ok(false);
                }
                seen.insert(section.to_string());

                // Try to preserve the original casing if possible.
                let section_unnormalized = p.unnormalized().section();
                let pth = pp.parse(
                    nh_common::files::COMPONENT_FOLDER_CONTENT,
                    &format!("/{section_unnormalized}/_index.md"),
                );
                let nn = w.tree().get(dims, &pth.base());

                if nn.is_none() {
                    let m = NewPageMeta {
                        site_idx: Some(i),
                        path_info: Some(Arc::new(pth)),
                        ..Default::default()
                    };
                    if let Some(np) = new_page(h, m)? {
                        let base = h.page(np).meta.path_info.base();
                        w.tree_mut().insert_into_values_dimension(
                            dims,
                            &base,
                            ContentNode::Page(np, i),
                        );
                    }
                }

                // /a/b, we don't need to walk deeper.
                if s.matches('/').count() > 1 {
                    w.skip_prefix(&format!("{s}/"));
                }

                Ok(false)
            })
        })?;

        if !has_home {
            let p = pp.parse(nh_common::files::COMPONENT_FOLDER_CONTENT, "/_index.md");
            let m = NewPageMeta {
                site_idx: Some(i),
                path_info: Some(Arc::new(p)),
                page_config: Some(page_config_with_kind(kinds::KIND_HOME)),
                ..Default::default()
            };
            if let Some(n) = new_page(self.h, m)? {
                let base = self.h.page(n).meta.path_info.base();
                self.h
                    .page_trees
                    .tree_pages
                    .insert_into_values_dimension_with_lock(dims, &base, ContentNode::Page(n, i));
                home = Some(n);
            }
        }

        self.h.sites[i].home = home;

        Ok(())
    }

    /// Create the fixed output pages, e.g. sitemap.xml, if not already there.
    // Go: hugolib/content_map_page.go:addStandalonePages
    fn add_standalone_pages(&mut self) -> Result<()> {
        let i = self.site_idx;
        let dims = self.dims();
        let conf = self.h.sites[i].conf.clone();
        let sconf = self.h.sites[i].deps.conf.clone();
        let pp = self.h.deps.conf.path_parser();
        let bf = nh_media::output::output_format::builtin_formats();

        let mut add_standalone = |h: &mut HugoSites,
                                  key: &str,
                                  kind: &str,
                                  f: nh_media::output::output_format::OutputFormat|
         -> Result<()> {
            if !sconf.is_multihost()
                && (kind == kinds::KIND_SITEMAP_INDEX || kind == kinds::KIND_ROBOTS_TXT)
            {
                // Only one for all languages.
                if i != 0 {
                    return Ok(());
                }
            }

            if !conf.is_kind_enabled(kind) || h.page_trees.tree_pages.has(dims, key) {
                return Ok(());
            }

            let m = NewPageMeta {
                site_idx: Some(i),
                path_info: Some(Arc::new(pp.parse(
                    nh_common::files::COMPONENT_FOLDER_CONTENT,
                    &format!("{key}{}", f.media_type.first_suffix.full_suffix),
                ))),
                page_config: Some(page_config_with_kind(kind)),
                standalone_output_format: Some(f),
                ..Default::default()
            };

            if let Some(p) = new_page(h, m)? {
                h.page_trees.tree_pages.insert_into_values_dimension(
                    dims,
                    key,
                    ContentNode::Page(p, i),
                );
            }
            Ok(())
        };

        add_standalone(
            self.h,
            "/404",
            kinds::KIND_STATUS_404,
            bf.http_status_404_html.clone(),
        )?;

        if conf.root.enable_robots_txt && (i == 0 || sconf.is_multihost()) {
            add_standalone(
                self.h,
                "/_robots",
                kinds::KIND_ROBOTS_TXT,
                bf.robots_txt.clone(),
            )?;
        }

        let sitemap_enabled = self
            .h
            .sites
            .iter()
            .any(|s| s.conf.is_kind_enabled(kinds::KIND_SITEMAP));

        if sitemap_enabled {
            let mut of = bf.sitemap.clone();
            if !conf.sitemap.filename.is_empty() {
                of.base_name = cpaths::filename(&conf.sitemap.filename);
            }
            add_standalone(self.h, "/_sitemap", kinds::KIND_SITEMAP, of)?;

            let skip_sitemap_index = sconf.is_multihost()
                || (!sconf.default_content_language_in_subdir() && !sconf.is_multilingual());
            if !skip_sitemap_index {
                let mut of = bf.sitemap_index.clone();
                if !conf.sitemap.filename.is_empty() {
                    of.base_name = cpaths::filename(&conf.sitemap.filename);
                }
                add_standalone(self.h, "/_sitemapindex", kinds::KIND_SITEMAP_INDEX, of)?;
            }
        }

        Ok(())
    }

    /// Calculate and apply aggregate values to the page tree (e.g. dates, cascades).
    // Go: hugolib/content_map_page.go:applyAggregates
    fn apply_aggregates(&mut self) -> Result<()> {
        let i = self.site_idx;
        let dims = self.dims();
        // Go: a map; see the main sections choice below.
        let mut section_page_count: BTreeMap<String, i64> = BTreeMap::new();

        self.h.sites[i].lastmod = Time::zero();

        let mut wc: WalkContext<PageId, Option<Arc<Cascade>>> = WalkContext::default();
        let log: DatesLog = Arc::new(Mutex::new(Vec::new()));

        {
            let HugoSites {
                sites,
                pages,
                page_trees,
                ..
            } = &mut *self.h;
            let site = &sites[i];
            let site_cascade: Option<Arc<Cascade>> = site
                .conf
                .cascade
                .as_ref()
                .map(|c| Arc::new(c.config.clone()));
            let tree_pages = &page_trees.tree_pages;
            let tree_resources = &page_trees.tree_resources;

            let cfg = WalkConfig {
                dims,
                ..Default::default()
            };
            tree_pages.walk(&cfg, |_pw, key_page, n, _match| {
                let id = must_page(n);

                if pages[id.0 as usize].meta.kind() == kinds::KIND_TERM {
                    // Delay this until they're created.
                    return Ok(false);
                }

                if pages[id.0 as usize].meta.is_page() {
                    let root_section = pages[id.0 as usize].meta.section().to_string();
                    *section_page_count.entry(root_section).or_insert(0) += 1;
                }

                // Handle cascades first to get any default dates set.
                let cascade: Option<Arc<Cascade>> = if key_page.is_empty() {
                    // Home page gets it's cascade from the site config.
                    if pages[id.0 as usize]
                        .meta
                        .page_config
                        .cascade_compiled
                        .is_none()
                    {
                        // Pass the site cascade downwards.
                        wc.data.insert(key_page, site_cascade.clone());
                    }
                    site_cascade.clone()
                } else {
                    wc.data
                        .longest_prefix(&cpaths::dir(key_page))
                        .and_then(|(_, d)| d.clone())
                };

                // (Go: the rebuild branch: post hooks that detect changed dates; not ported.)

                // Combine the cascade map with front matter.
                {
                    let ps = &mut pages[id.0 as usize];
                    let content = ps.content.clone();
                    ps.meta
                        .set_meta_post(site, content.as_deref(), cascade.as_deref())?;
                }

                let ps = &pages[id.0 as usize];
                let branch = ps.meta.is_node();
                if branch {
                    if let Some(cc) = &ps.meta.page_config.cascade_compiled {
                        // Pass it down.
                        wc.data.insert(key_page, Some(Arc::new(cc.clone())));
                    }

                    let was_zero_dates = ps.meta.page_config.dates.is_all_dates_zero();
                    let is_home = ps.meta.is_home();
                    if was_zero_dates || is_home {
                        let log = log.clone();
                        wc.add_event_listener(
                            EVENT_DATES,
                            key_page,
                            Box::new(move |e: &mut Event<PageId>| {
                                log.lock().unwrap_or_else(|e| e.into_inner()).push((
                                    id,
                                    e.source,
                                    was_zero_dates,
                                    is_home,
                                ));
                            }),
                        );
                    }
                }

                // Send the date info up the tree.
                wc.send_event(Event::new(EVENT_DATES, key_page, id));

                let is_branch = branch;
                let rcfg = WalkConfig {
                    dims,
                    prefix: format!("{key_page}/"),
                    ..Default::default()
                };

                tree_resources.walk(&rcfg, |rw, resource_key, rn, _match| {
                    if is_branch {
                        let owner_key = tree_pages
                            .longest_prefix(dims, resource_key, true, None)
                            .map(|(k, _)| k)
                            .unwrap_or_default();
                        if owner_key != key_page {
                            // Stop walking downwards, someone else owns this resource.
                            rw.skip_prefix(&format!("{owner_key}/"));
                            return Ok(false);
                        }
                    }
                    let rs = rn.resource().expect(
                        "interface conversion: contentNodeI is not *hugolib.resourceSource",
                    );
                    if let Some(pid) = rs.page {
                        let rel_path = pages[pid.0 as usize]
                            .meta
                            .path_info
                            .base_rel(&pages[id.0 as usize].meta.path_info);
                        // Apply cascade (if set) to the page.
                        let cascade = wc
                            .data
                            .longest_prefix(resource_key)
                            .and_then(|(_, d)| d.clone());
                        let pr = &mut pages[pid.0 as usize];
                        pr.meta.resource_path = rel_path;
                        let content = pr.content.clone();
                        pr.meta
                            .set_meta_post(site, content.as_deref(), cascade.as_deref())?;
                    }

                    Ok(false)
                })?;

                Ok(false)
            })?;
        }

        wc.handle_events_and_hooks()?;
        self.apply_dates_log(&log);

        let conf = self.h.sites[i].conf.clone();
        if !conf.compiled().is_main_sections_set() {
            let mut main_section = String::new();
            let mut maxcount = 0;
            // Go ranges over a map: with several sections sharing the highest count the
            // chosen one is random; the port takes the first in byte order.
            for (section, counter) in &section_page_count {
                if !section.is_empty() && *counter > maxcount {
                    main_section = section.clone();
                    maxcount = *counter;
                }
            }
            conf.compiled().set_main_sections(vec![main_section]);
        }

        Ok(())
    }

    /// Applies the recorded "dates" deliveries of `applyAggregates` in handling order: a branch
    /// whose dates were all zero takes the later dates of the event source
    /// (`UpdateDateAndLastmodAndPublishDateIfAfter`); the home page also tracks the site's
    /// `lastmod`.
    fn apply_dates_log(&mut self, log: &DatesLog) {
        let entries = std::mem::take(&mut *log.lock().unwrap_or_else(|e| e.into_inner()));
        for (target, source, was_zero_dates, is_home) in entries {
            let sd = self.h.page(source).meta.page_config.dates.clone();
            if was_zero_dates {
                self.h
                    .page_mut(target)
                    .meta
                    .page_config
                    .dates
                    .update_date_and_lastmod_and_publish_date_if_after(&sd);
            }

            if is_home {
                let site_idx = self.h.page(target).site_idx;
                let own = self.h.page(target).meta.page_config.dates.lastmod.clone();
                let site = &mut self.h.sites[site_idx];
                if own.after(&site.lastmod) {
                    site.lastmod = own;
                }
                if sd.lastmod.after(&site.lastmod) {
                    site.lastmod = sd.lastmod.clone();
                }
            }
        }
    }

    // Go: hugolib/content_map_page.go:applyAggregatesToTaxonomiesAndTerms
    fn apply_aggregates_to_taxonomies_and_terms(&mut self) -> Result<()> {
        let i = self.site_idx;
        let dims = self.dims();
        let lang = dims[0];
        let mut wc: WalkContext<PageId, Option<Arc<Cascade>>> = WalkContext::default();
        let log: DatesLog = Arc::new(Mutex::new(Vec::new()));

        let views = self.h.sites[i].page_map.cfg.taxonomy_config.views.clone();

        {
            let HugoSites {
                sites,
                pages,
                page_trees,
                ..
            } = &mut *self.h;
            let site = &sites[i];
            let PageTrees {
                tree_pages,
                tree_taxonomy_entries,
                ..
            } = page_trees;

            for view_name in &views {
                let key = &view_name.plural_tree_key;
                let cfg = WalkConfig {
                    dims,
                    // We also want to include the root taxonomy nodes, so no trailing slash.
                    prefix: key.clone(),
                    ..Default::default()
                };
                tree_pages.walk_mut(&cfg, |pw, s, n, _match| {
                    let id = must_page(n);
                    let kind = pages[id.0 as usize].meta.kind().to_string();
                    if kind != kinds::KIND_TERM {
                        // The other kinds were handled in applyAggregates.
                        if let Some(cc) = &pages[id.0 as usize].meta.page_config.cascade_compiled {
                            // Pass it down.
                            wc.data.insert(s, Some(Arc::new(cc.clone())));
                        }
                    }

                    if kind != kinds::KIND_TERM && kind != kinds::KIND_TAXONOMY {
                        // Already handled.
                        return Ok(false);
                    }

                    if kind == kinds::KIND_TERM {
                        let cascade = wc.data.longest_prefix(s).and_then(|(_, d)| d.clone());
                        {
                            let p = &mut pages[id.0 as usize];
                            let content = p.content.clone();
                            p.meta
                                .set_meta_post(site, content.as_deref(), cascade.as_deref())?;
                        }
                        if !site.should_build(&pages[id.0 as usize]) {
                            pw.tree_mut().delete(dims, s);
                            tree_taxonomy_entries.delete_prefix(&cpaths::add_trailing_slash(s));
                        } else {
                            tree_taxonomy_entries.walk_prefix(
                                lang,
                                &cpaths::add_trailing_slash(s),
                                &mut |ss, wn| {
                                    // Send the date info up the tree.
                                    wc.send_event(Event::new(EVENT_DATES, ss, wn.n));
                                    Ok(false)
                                },
                            )?;
                        }
                    }

                    // Send the date info up the tree.
                    wc.send_event(Event::new(EVENT_DATES, s, id));

                    if pages[id.0 as usize]
                        .meta
                        .page_config
                        .dates
                        .is_all_dates_zero()
                    {
                        let log = log.clone();
                        wc.add_event_listener(
                            EVENT_DATES,
                            s,
                            Box::new(move |e: &mut Event<PageId>| {
                                log.lock()
                                    .unwrap_or_else(|e| e.into_inner())
                                    .push((id, e.source, true, false));
                            }),
                        );
                    }

                    Ok(false)
                })?;
            }
        }

        wc.handle_events_and_hooks()?;
        self.apply_dates_log(&log);

        Ok(())
    }

    // Go: hugolib/content_map_page.go:assembleTermsAndTranslations
    fn assemble_terms_and_translations(&mut self) -> Result<()> {
        let i = self.site_idx;
        let cmcfg = self.h.sites[i].page_map.cfg.clone();
        if cmcfg.taxonomy_term_disabled {
            return Ok(());
        }

        let dims = self.dims();
        let lang = dims[0];
        let views = cmcfg.taxonomy_config.views.clone();
        let pp = self.h.sites[i].deps.conf.path_parser();
        let log = self.h.sites[i].deps.log.clone();

        let cfg = WalkConfig {
            dims,
            ..Default::default()
        };
        with_tree_pages(self.h, |h, pages_tree| {
            pages_tree.walk_mut(&cfg, |w, s, n, _match| {
                let id = must_page(n);

                if h.page(id).meta.no_link() {
                    return Ok(false);
                }

                let mut s = s.to_string();
                let params = h.page(id).meta.page_config.params.clone();
                for view_name in &views {
                    let Some(vals) = term_values(&crate::page__meta::get_param(
                        params.as_ref(),
                        &view_name.plural,
                        false,
                    )) else {
                        continue;
                    };

                    let wv = crate::page__meta::get_param_to_lower(
                        params.as_ref(),
                        &format!("{}_weight", view_name.plural),
                    );
                    let weight = match nh_common::cast::caste::to_int_e(&wv) {
                        Ok(w) => w,
                        Err(_) => {
                            log.warnf(format!(
                                "Unable to convert taxonomy weight {} to int for {}",
                                String::from_utf8_lossy(&go_fmt::sprintf(
                                    "%#v",
                                    std::slice::from_ref(&wv)
                                )),
                                go_strconv::quote(h.page(id).meta.path())
                            ));
                            // weight will equal zero, so let the flow continue
                            0
                        }
                    };

                    for (vi, v) in vals.iter().enumerate() {
                        if v.is_empty() {
                            continue;
                        }
                        let v = String::from_utf8_lossy(v.as_bytes()).into_owned();
                        let view_term_key = format!("/{}/{}", view_name.plural, v);
                        let pi = pp.parse(
                            nh_common::files::COMPONENT_FOLDER_CONTENT,
                            &format!("{view_term_key}/_index.md"),
                        );
                        let term = match w.tree().get(dims, &pi.base()) {
                            None => {
                                // (Go: in server mode a new tag marks the taxonomy changed.)
                                let m = NewPageMeta {
                                    term: v.clone(),
                                    singular: view_name.singular.clone(),
                                    site_idx: Some(i),
                                    path_info: Some(Arc::new(pi.clone())),
                                    page_config: Some(page_config_with_kind(kinds::KIND_TERM)),
                                    ..Default::default()
                                };
                                let Some(np) = new_page(h, m)? else {
                                    continue;
                                };
                                let base = h.page(np).meta.path_info.base();
                                w.tree_mut().insert_into_values_dimension(
                                    dims,
                                    &base,
                                    ContentNode::Page(np, i),
                                );
                                must_page(
                                    &w.tree()
                                        .get(dims, &base)
                                        .expect("term inserted into the page tree"),
                                )
                            }
                            Some(tn) => {
                                let tid = must_page(&tn);
                                let m = &mut h.page_mut(tid).meta;
                                m.term = v.clone();
                                m.singular = view_name.singular.clone();
                                tid
                            }
                        };

                        if s.is_empty() {
                            // Consider making this the real value.
                            s = "/".to_string();
                        }

                        let key = format!("{}{}", pi.base(), s);

                        h.page_trees.tree_taxonomy_entries.insert(
                            lang,
                            &key,
                            WeightedContentNode {
                                weight,
                                n: id,
                                term: Some((term, vi as i64)),
                            },
                        );
                    }
                }

                Ok(false)
            })
        })
    }

    // Go: hugolib/content_map_page.go:assembleResources
    fn assemble_resources(&mut self) -> Result<()> {
        let i = self.site_idx;
        let dims = self.dims();

        // The walk of the pages tree does not modify it (only the resources tree), so its
        // visits are taken first; each page is then handled in walk order.
        let mut visits: Vec<PageId> = Vec::new();
        let cfg = WalkConfig {
            dims,
            ..Default::default()
        };
        self.h
            .page_trees
            .tree_pages
            .walk(&cfg, |_w, _s, n, _match| {
                visits.push(must_page(n));
                Ok(false)
            })?;

        for id in visits {
            // This is a little out of place, but is conveniently put here.
            // Check if translationKey is set by user.
            // This is to support the manual way of setting the translationKey in front matter.
            let tk = self.h.page(id).meta.page_config.translation_key.clone();
            if !tk.is_empty() {
                self.h.translation_key_pages.entry(tk).or_default().push(id);
            }

            // Prepare resources for this page.
            let _ = self.h.page(id).shift_to_output_format(&*self.h, true, 0);
            let target_paths = self
                .h
                .page(id)
                .lazy
                .get()
                .and_then(|r| r.as_ref().ok())
                .map(|l| {
                    let idx = self
                        .h
                        .page(id)
                        .current_output_idx
                        .load(std::sync::atomic::Ordering::SeqCst);
                    l.outputs[idx].target_paths.paths.clone()
                })
                .unwrap_or_default();
            let mut base_target = target_paths.sub_resource_base_target.clone();

            let site = &self.h.sites[i];
            let ps = self.h.page(id);
            let mut duplicate_resource_files = true;
            if ps.meta.page_config.content_media_type.is_markdown() {
                duplicate_resource_files = site.conf.markup.goldmark.duplicate_resource_files;
            }

            let multihost = site.deps.conf.is_multihost();
            duplicate_resource_files = duplicate_resource_files || multihost;

            let rspec = site.deps.resource_spec().clone();
            let target_language_base_path = if multihost {
                site.language.lang.clone()
            } else {
                site.deps.conf.language_prefix()
            };
            let publish_resources = ps.meta.page_config.build.publish_resources;
            let ps_path_info = ps.meta.path_info.clone();
            let mut key_page = ps.meta.path();
            if key_page == "/" {
                key_page = String::new();
            }
            let prefix = cpaths::add_trailing_slash(&ps.meta.path());
            let is_branch = ps.meta.is_node();

            let rcfg = WalkConfig {
                dims,
                prefix,
                exact: !duplicate_resource_files,
                ..Default::default()
            };

            let PageTrees {
                tree_pages,
                tree_resources,
                ..
            } = &mut self.h.page_trees;
            let tree_pages = &*tree_pages;

            tree_resources.walk_mut(&rcfg, |rw, resource_key, n, m| {
                if is_branch {
                    match resource_owner(tree_pages, &key_page, resource_key) {
                        ResourceOwner::Handle => {}
                        ResourceOwner::Stop => return Ok(true),
                        ResourceOwner::Skip(p) => {
                            rw.skip_prefix(&p);
                            return Ok(false);
                        }
                    }
                }

                let mut rs: Arc<ResourceSource> = n
                    .resource()
                    .expect("interface conversion: contentNodeI is not *hugolib.resourceSource")
                    .clone();
                if !m.has(DimensionFlag::LANGUAGE) {
                    // We got an alternative language version.
                    // Clone this and insert it into the tree.
                    rs = Arc::new(rs.clone_source());
                    rw.tree_mut().insert_into_current_dimension(
                        dims,
                        resource_key,
                        ContentNode::Resource(rs.clone()),
                    );
                }
                if rs.page.is_some() || rs.r.get().is_some() {
                    return Ok(false);
                }

                let rs_path = rs
                    .path
                    .as_ref()
                    .expect("invalid memory address or nil pointer dereference");
                let rel_path_original =
                    rs_path.unnormalized().path_rel(ps_path_info.unnormalized());
                let rel_path = rs_path.base_rel(&ps_path_info);

                let mut target_base_paths: Vec<String> = Vec::new();
                if multihost {
                    base_target = target_paths.sub_resource_base_link.clone();
                    // In multihost we need to publish to the lang sub folder.
                    target_base_paths = vec![target_language_base_path.clone()];
                }

                if rs.rc.is_some() {
                    // Go: resource configs come from content adapters (`_content.gotmpl`),
                    // which capture already rejects.
                    return Err(Error::new(
                        "neohugo-rs: content adapters (_content.gotmpl) are not supported",
                    ));
                }

                let filename = rs
                    .fi
                    .as_ref()
                    .map(|fi| fi.meta().filename.clone())
                    .unwrap_or_default();

                let rd = nh_resources::resource::ResourceSourceDescriptor {
                    open_read_seek_closer: rs.opener.clone(),
                    path: rs.path.clone(),
                    // Use the original path for the target path, so the links can be guessed.
                    target_path: rel_path_original.clone(),
                    target_base_paths,
                    base_path_rel_permalink: target_paths.sub_resource_base_link.clone(),
                    base_path_target_path: base_target.clone(),
                    source_filename_or_path: filename,
                    name_normalized: rel_path,
                    name_original: rel_path_original,
                    // Go: `rs.rc.ContentMediaType`, the zero media type without a resource
                    // config.
                    media_type: None,
                    lazy_publish: !publish_resources,
                    ..Default::default()
                };

                let r = rspec.new_resource(rd)?;
                let _ = rs.r.set(r);
                Ok(false)
            })?;
        }

        Ok(())
    }

    /// Remove any leftover node that we should not build for some reason (draft, expired,
    /// scheduled in the future). Note that for the home and section kinds we just disable the
    /// nodes to preserve the structure.
    // Go: hugolib/content_map_page.go:removeShouldNotBuild
    fn remove_should_not_build(&mut self) -> Result<()> {
        let i = self.site_idx;
        let dims = self.dims();
        let mut keys: Vec<String> = Vec::new();
        {
            let HugoSites {
                sites,
                pages,
                page_trees,
                ..
            } = &mut *self.h;
            let s = &sites[i];
            let cfg = WalkConfig {
                dims,
                ..Default::default()
            };
            page_trees.tree_pages.walk(&cfg, |_w, key, n, _match| {
                let id = must_page(n);
                let p = &mut pages[id.0 as usize];
                if !s.should_build(p) {
                    match p.meta.kind() {
                        kinds::KIND_HOME | kinds::KIND_SECTION | kinds::KIND_TAXONOMY => {
                            // We need to keep these for the structure, but disable
                            // them so they don't get listed/rendered.
                            p.meta.page_config.build.disable();
                        }
                        _ => keys.push(key.to_string()),
                    }
                }
                Ok(false)
            })?;
        }

        if keys.is_empty() {
            return Ok(());
        }

        delete_page_and_resources_below(&mut self.h.page_trees, dims, &keys);

        Ok(())
    }
}

/// Go: `(s *Site) Taxonomies()`'s lazy init (`s.init.taxonomies`, T23 owns the method):
/// `CreateSiteTaxonomies` once per site. Go ignores the init error (the taxonomies created so
/// far are kept); the port keeps an empty list then.
pub fn site_taxonomies(h: &Arc<HugoSites>, site_idx: usize) -> nh_page::taxonomy::TaxonomyList {
    h.sites[site_idx]
        .taxonomies
        .get_or_init(|| PageMap::create_site_taxonomies(h, site_idx).unwrap_or_default())
        .clone()
}

/// A `PageConfig` with only its kind set (Go `&pagemeta.PageConfig{PageConfigEarly:
/// pagemeta.PageConfigEarly{Kind: kind}}`).
fn page_config_with_kind(kind: &str) -> PageConfig {
    PageConfig {
        kind: kind.to_string(),
        ..Default::default()
    }
}

/// Go: `types.ToStringSlicePreserveString(getParam(ps, plural, false))` with Go's nil result
/// (`None`): a nil value, a conversion error, and `cast.ToStringSliceE` of an empty non-`[]string`
/// slice (which appends to a nil slice) are nil; an empty `[]string` is not.
fn term_values(v: &Value) -> Option<Vec<GoString>> {
    match v {
        Value::Invalid | Value::TypedNil(_) => None,
        Value::List(l) if l.items.is_empty() => {
            (l.ty == go_value::SliceType::String).then(Vec::new)
        }
        _ => nh_common::types::convert::to_string_slice_preserve_string_e(v).ok(),
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/content_map_page.go (2209 lines; 30/51 funcs executed) — lines 88-259 and 636-994
//          are in content_map_trees.rs (T20)
//   types: pageMapQueryPagesInSection, pageMapQueryPagesBelowPath, sitePagesAssembler, viewName
// OK L268-270: (q pageMapQueryPagesInSection) Key() string
// OK L285-287: (q pageMapQueryPagesBelowPath) Key() string
// OK L291-311: (m *pageMap) forEachPage(include predicate.P[*pageState], fn func(p *pageState) (bool, error)) error
// OK L313-340: (m *pageMap) forEeachPageIncludingBundledPages(include predicate.P[*pageState], fn func(p *pageState) (bool, error)) error
// OK L342-350: (m *pageMap) getOrCreatePagesFromCache( cache *dynacache.Partition[string, page.Pages], key string, create func(string) (page.Pages, error), ) (pag...
// OK L352-415: (m *pageMap) getPagesInSection(q pageMapQueryPagesInSection) page.Pages
// OK L417-452: (m *pageMap) getPagesWithTerm(q pageMapQueryPagesBelowPath) page.Pages
// OK L454-483: (m *pageMap) getTermsForPageInTaxonomy(path, taxonomy string) page.Pages
// OK L485-538: (m *pageMap) forEachResourceInPage( ps *pageState, lockType doctree.LockType, exact bool, handle func(resourceKey string, n contentNodeI, match doc...
// OK L540-550: (m *pageMap) getResourcesForPage(ps *pageState) (resource.Resources, error)
// OK L552-634: (m *pageMap) getOrCreateResourcesForPage(ps *pageState) resource.Resources
//    L1001-1065: (m *pageMap) debugPrint(prefix string, maxLevel int, w io.Writer)
//    L1067-1102: (h *HugoSites) dynacacheGCFilenameIfNotWatchedAndDrainMatching(filename string)
//    L1104-1118: (h *HugoSites) dynacacheGCCacheBuster(cachebuster func(s string) bool)
//    L1120-1280: (h *HugoSites) resolveAndClearStateForIdentities( ctx context.Context, l logg.LevelLogger, cachebuster func(s string) bool, changes []identity.Iden...
//    L1284-1382: (h *HugoSites) resolveAndResetDependententPageOutputs(ctx context.Context, changes []identity.Identity) (int, int, error)
// OK L1385-1544: (sa *sitePagesAssembler) applyAggregates() error
// OK L1546-1633: (sa *sitePagesAssembler) applyAggregatesToTaxonomiesAndTerms() error
// OK L1635-1733: (sa *sitePagesAssembler) assembleTermsAndTranslations() error
// OK L1735-1854: (sa *sitePagesAssembler) assembleResources() error
// OK L1856-1870: (sa *sitePagesAssembler) assemblePagesStep1(ctx context.Context) error
// OK L1872-1884: (sa *sitePagesAssembler) assemblePagesStep2() error
// OK L1886-1891: (sa *sitePagesAssembler) assemblePagesStepFinal() error
// OK L1895-1927: (sa *sitePagesAssembler) removeShouldNotBuild() error
// OK L1930-2005: (sa *sitePagesAssembler) addStandalonePages() error
// OK L2007-2101: (sa *sitePagesAssembler) addMissingRootSections() error
// OK L2103-2134: (sa *sitePagesAssembler) addMissingTaxonomies() error
// OK L2136-2199: (m *pageMap) CreateSiteTaxonomies(ctx context.Context) error
// OK L2207-2209: (v viewName) IsZero() bool  [ported by T20 in content_map.rs, where the type lives]
// ---------------------------------------------------------------------------
