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

use std::sync::Arc;

use nh_common::Result;
use nh_page::page::Pages;
use nh_resource::resourcetypes::Resources;

pub use crate::content_map_trees::{
    ContentNode, ContentNodeShifter, PageMap, PageTrees, WeightedContentNode,
};
use crate::hugo_sites::HugoSites;
use crate::page::PageId;

/// Go: `pageMapQueryPagesInSection`.
#[derive(Clone, Debug)]
pub struct PageMapQueryPagesInSection {
    pub path: String,
    pub key_part: String,
    pub recursive: bool,
    pub include_self: bool,
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

impl PageMap {
    /// Go: `getPagesInSection(q)` (sorted by default; recursive or not; include self).
    // Go: hugolib/content_map_page.go:getPagesInSection
    pub fn get_pages_in_section(
        h: &Arc<HugoSites>,
        site_idx: usize,
        q: &PageMapQueryPagesInSection,
    ) -> Pages {
        todo!()
    }

    /// Go: `getPagesWithTerm(q)` — entries under the term key, wrapped as `pageWithWeight0`.
    // Go: hugolib/content_map_page.go:getPagesWithTerm
    pub fn get_pages_with_term(
        h: &Arc<HugoSites>,
        site_idx: usize,
        path: &str,
        key_part: &str,
    ) -> Pages {
        todo!()
    }

    /// Go: `getResourcesForPage(ps)` (non-exact: TH pages get the EN resource objects).
    // Go: hugolib/content_map_page.go:getResourcesForPage
    pub fn get_resources_for_page(h: &Arc<HugoSites>, p: PageId) -> Result<Resources> {
        todo!()
    }

    /// Go: `CreateSiteTaxonomies(ctx)`.
    // Go: hugolib/content_map_page.go:CreateSiteTaxonomies
    pub fn create_site_taxonomies(
        h: &Arc<HugoSites>,
        site_idx: usize,
    ) -> Result<nh_page::taxonomy::TaxonomyList> {
        todo!()
    }
}

/// Go: `sitePagesAssembler` — the assembly steps run on `&mut HugoSites` before freezing.
pub struct SitePagesAssembler<'a> {
    pub h: &'a mut HugoSites,
    pub site_idx: usize,
}

impl SitePagesAssembler<'_> {
    // Go: hugolib/content_map_page.go:assemblePagesStep1
    pub fn assemble_pages_step1(&mut self) -> Result<()> {
        todo!()
    }
    // Go: hugolib/content_map_page.go:assemblePagesStep2
    pub fn assemble_pages_step2(&mut self) -> Result<()> {
        todo!()
    }
    // Go: hugolib/content_map_page.go:assemblePagesStepFinal
    pub fn assemble_pages_step_final(&mut self) -> Result<()> {
        todo!()
    }
    // Go: hugolib/content_map_page.go:addMissingTaxonomies
    fn add_missing_taxonomies(&mut self) -> Result<()> {
        todo!()
    }
    // Go: hugolib/content_map_page.go:addMissingRootSections
    fn add_missing_root_sections(&mut self) -> Result<()> {
        todo!()
    }
    // Go: hugolib/content_map_page.go:addStandalonePages
    fn add_standalone_pages(&mut self) -> Result<()> {
        todo!()
    }
    // Go: hugolib/content_map_page.go:applyAggregates
    fn apply_aggregates(&mut self) -> Result<()> {
        todo!()
    }
    // Go: hugolib/content_map_page.go:removeShouldNotBuild
    fn remove_should_not_build(&mut self) -> Result<()> {
        todo!()
    }
    // Go: hugolib/content_map_page.go:assembleTermsAndTranslations
    fn assemble_terms_and_translations(&mut self) -> Result<()> {
        todo!()
    }
    // Go: hugolib/content_map_page.go:applyAggregatesToTaxonomiesAndTerms
    fn apply_aggregates_to_taxonomies_and_terms(&mut self) -> Result<()> {
        todo!()
    }
    // Go: hugolib/content_map_page.go:assembleResources
    fn assemble_resources(&mut self) -> Result<()> {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/content_map_page.go (2209 lines; 30/51 funcs executed) — lines 88-259 and 636-994
//          are in content_map_trees.rs (T20)
//   types: pageMapQueryPagesInSection, pageMapQueryPagesBelowPath, sitePagesAssembler, viewName
// EX L268-270: (q pageMapQueryPagesInSection) Key() string
// EX L285-287: (q pageMapQueryPagesBelowPath) Key() string
// EX L291-311: (m *pageMap) forEachPage(include predicate.P[*pageState], fn func(p *pageState) (bool, error)) error
// EX L313-340: (m *pageMap) forEeachPageIncludingBundledPages(include predicate.P[*pageState], fn func(p *pageState) (bool, error)) error
// EX L342-350: (m *pageMap) getOrCreatePagesFromCache( cache *dynacache.Partition[string, page.Pages], key string, create func(string) (page.Pages, error), ) (pag...
// EX L352-415: (m *pageMap) getPagesInSection(q pageMapQueryPagesInSection) page.Pages
// EX L417-452: (m *pageMap) getPagesWithTerm(q pageMapQueryPagesBelowPath) page.Pages
//    L454-483: (m *pageMap) getTermsForPageInTaxonomy(path, taxonomy string) page.Pages
// EX L485-538: (m *pageMap) forEachResourceInPage( ps *pageState, lockType doctree.LockType, exact bool, handle func(resourceKey string, n contentNodeI, match doc...
// EX L540-550: (m *pageMap) getResourcesForPage(ps *pageState) (resource.Resources, error)
// EX L552-634: (m *pageMap) getOrCreateResourcesForPage(ps *pageState) resource.Resources
//    L1001-1065: (m *pageMap) debugPrint(prefix string, maxLevel int, w io.Writer)
//    L1067-1102: (h *HugoSites) dynacacheGCFilenameIfNotWatchedAndDrainMatching(filename string)
//    L1104-1118: (h *HugoSites) dynacacheGCCacheBuster(cachebuster func(s string) bool)
//    L1120-1280: (h *HugoSites) resolveAndClearStateForIdentities( ctx context.Context, l logg.LevelLogger, cachebuster func(s string) bool, changes []identity.Iden...
//    L1284-1382: (h *HugoSites) resolveAndResetDependententPageOutputs(ctx context.Context, changes []identity.Identity) (int, int, error)
// EX L1385-1544: (sa *sitePagesAssembler) applyAggregates() error
// EX L1546-1633: (sa *sitePagesAssembler) applyAggregatesToTaxonomiesAndTerms() error
// EX L1635-1733: (sa *sitePagesAssembler) assembleTermsAndTranslations() error
// EX L1735-1854: (sa *sitePagesAssembler) assembleResources() error
// EX L1856-1870: (sa *sitePagesAssembler) assemblePagesStep1(ctx context.Context) error
// EX L1872-1884: (sa *sitePagesAssembler) assemblePagesStep2() error
// EX L1886-1891: (sa *sitePagesAssembler) assemblePagesStepFinal() error
// EX L1895-1927: (sa *sitePagesAssembler) removeShouldNotBuild() error
// EX L1930-2005: (sa *sitePagesAssembler) addStandalonePages() error
// EX L2007-2101: (sa *sitePagesAssembler) addMissingRootSections() error
// EX L2103-2134: (sa *sitePagesAssembler) addMissingTaxonomies() error
// EX L2136-2199: (m *pageMap) CreateSiteTaxonomies(ctx context.Context) error
// EX L2207-2209: (v viewName) IsZero() bool
// ---------------------------------------------------------------------------
