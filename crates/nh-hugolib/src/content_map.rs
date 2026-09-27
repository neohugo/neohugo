//! Port of `hugolib/content_map.go`.
//!
//! Owner: Wave B task T20 (hugolib-capture).


//! Go `hugolib/content_map.go`: `pageMap.AddFi` (parse content file -> page node in treePages;
//! non-content files -> `resourceSource` in treeResources), `cleanTreeKey`, taxonomy config view.

use std::sync::Arc;

use nh_common::hugio::OpenReadSeekCloser;
use nh_common::paths::pathparser::Path;
use nh_common::Result;
use nh_hugofs::fileinfo::FileMetaInfo;
use nh_page::pagemeta::page_frontmatter::ResourceConfig;
use nh_resource::resourcetypes::Resource;

/// Go: `resourceSource` — a non-content file of a bundle (image...) before it becomes a resource.
#[derive(Clone)]
pub struct ResourceSource {
    pub lang_index: usize,
    pub path: Arc<Path>,
    pub opener: Option<OpenReadSeekCloser>,
    pub fi: Option<FileMetaInfo>,
    pub rc: Option<ResourceConfig>,
    /// Created in `assembleResources` (EN page creates; TH pages reuse via non-exact Shift).
    pub r: std::sync::OnceLock<Arc<dyn Resource>>,
}

/// Go: `cleanTreeKey(k)` — lower-case, leading `/`, no trailing `/`; home is `""`.
// Go: hugolib/content_map.go:cleanTreeKey
pub fn clean_tree_key(k: &str) -> String {
    todo!()
}

/// Go: `viewName` (taxonomy singular/plural + `pluralTreeKey`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ViewName {
    pub singular: String,
    pub plural: String,
    pub plural_tree_key: String,
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/content_map.go (493 lines; 7/23 funcs executed)
//   types: contentMapConfig, resourceSource, resourceSources
//    L67-70: (r resourceSource) clone() *resourceSource
// EX L72-74: (r *resourceSource) LangIndex() int
//    L76-78: (r *resourceSource) MarkStale()
//    L80-84: (r *resourceSource) resetBuildState()
// EX L86-89: (r *resourceSource) isPage() bool
//    L91-96: (r *resourceSource) GetIdentity() identity.Identity
//    L98-100: (r *resourceSource) ForEeachIdentity(f func(identity.Identity) bool) bool
//    L102-104: (r *resourceSource) Path() string
//    L106-108: (r *resourceSource) isContentNodeBranch() bool
//    L114-120: (n resourceSources) MarkStale()
//    L122-124: (n resourceSources) Path() string
//    L126-128: (n resourceSources) isContentNodeBranch() bool
//    L130-136: (n resourceSources) resetBuildState()
//    L138-145: (n resourceSources) GetIdentity() identity.Identity
//    L147-156: (n resourceSources) ForEeachIdentity(f func(identity.Identity) bool) bool
// EX L158-165: (cfg contentMapConfig) getTaxonomyConfig(s string) (v viewName)
// EX L167-183: (m *pageMap) insertPageWithLock(s string, p *pageState) (contentNodeI, contentNodeI, bool)
//    L185-191: (m *pageMap) insertResourceWithLock(s string, r contentNodeI) (contentNodeI, contentNodeI, bool)
// EX L193-199: (m *pageMap) insertResource(s string, r contentNodeI) (contentNodeI, contentNodeI, bool)
//    L201-214: (m *pageMap) handleDuplicateResourcePath(s string, updated, existing contentNodeI)
// EX L216-317: (m *pageMap) AddFi(fi hugofs.FileMetaInfo, buildConfig *BuildCfg) (pageCount uint64, resourceCount uint64, addErr error)
//    L319-471: (m *pageMap) addPagesFromGoTmplFi(fi hugofs.FileMetaInfo, buildConfig *BuildCfg) (pageCount uint64, resourceCount uint64, addErr error)
// EX L476-493: cleanTreeKey(elem ...string) string
// ---------------------------------------------------------------------------
