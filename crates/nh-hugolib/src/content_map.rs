//! Port of `hugolib/content_map.go`.
//!
//! Owner: Wave B task T20 (hugolib-capture).

//! Go `hugolib/content_map.go`: `pageMap.AddFi` (parse content file -> page node in treePages;
//! non-content files -> `resourceSource` in treeResources), `cleanTreeKey`, taxonomy config view.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::{Arc, OnceLock};

use nh_common::Result;
use nh_common::herrors::Error;
use nh_common::hugio::OpenReadSeekCloser;
use nh_common::paths::pathparser::{Path, PathType};
use nh_hugofs::fileinfo::FileMetaInfo;
use nh_page::pagemeta::page_frontmatter::ResourceConfig;
use nh_resource::resourcetypes::Resource;

use crate::content_map_trees::ContentNode;
use crate::hugo_sites::HugoSites;
use crate::hugo_sites_build::BuildCfg;
use crate::page::PageId;
use crate::page__new::{NewPageMeta, new_page};

/// Go: `contentMapConfig`.
#[derive(Clone, Debug, Default)]
pub struct ContentMapConfig {
    pub lang: String,
    pub taxonomy_config: TaxonomiesConfigValues,
    pub taxonomy_disabled: bool,
    pub taxonomy_term_disabled: bool,
    pub page_disabled: bool,
    pub is_rebuild: bool,
}

impl ContentMapConfig {
    /// Go: `getTaxonomyConfig(s)` — the first view (sorted by plural) whose `pluralTreeKey` is a
    /// byte prefix of `s` (character level: `/tagsfoo` matches `/tags`).
    // Go: hugolib/content_map.go:getTaxonomyConfig
    pub fn get_taxonomy_config(&self, s: &str) -> ViewName {
        for n in &self.taxonomy_config.views {
            if s.starts_with(n.plural_tree_key.as_str()) {
                return n.clone();
            }
        }
        ViewName::default()
    }
}

/// Go: `taxonomiesConfigValues`.
#[derive(Clone, Debug, Default)]
pub struct TaxonomiesConfigValues {
    pub views: Vec<ViewName>,
    pub views_by_tree_key: BTreeMap<String, ViewName>,
}

/// Go: `taxonomiesConfig.Values()` (site.go) — the views sorted by plural (`sort.Slice`;
/// Go ranges over the config map in random order, so two singulars with the same plural come
/// out in random order: the port starts from the singulars in byte order).
// Go: hugolib/site.go:Values
pub fn taxonomies_config_values(t: &BTreeMap<String, String>) -> TaxonomiesConfigValues {
    let mut views: Vec<ViewName> = t
        .iter()
        .map(|(k, v)| ViewName {
            singular: k.clone(),
            plural: v.clone(),
            plural_tree_key: clean_tree_key(v),
        })
        .collect();
    go_sort::sort::slice(&mut views, |x, i, j| x[i].plural < x[j].plural);

    let mut views_by_tree_key = BTreeMap::new();
    for v in &views {
        views_by_tree_key.insert(v.plural_tree_key.clone(), v.clone());
    }

    TaxonomiesConfigValues {
        views,
        views_by_tree_key,
    }
}

/// Go: `resourceSource` — a non-content file of a bundle (image...) before it becomes a resource,
/// or (Go `r` holding a `*pageState`) a content file bundled in a leaf bundle.
#[derive(Clone)]
pub struct ResourceSource {
    pub lang_index: usize,
    /// Go `path` (nil for a bundled page, whose path is the page's).
    pub path: Option<Arc<Path>>,
    pub opener: Option<OpenReadSeekCloser>,
    pub fi: Option<FileMetaInfo>,
    pub rc: Option<ResourceConfig>,
    /// A bundled content file: the page created for it at capture (Go: `r` is that
    /// `*pageState`).
    pub page: Option<PageId>,
    /// Created in `assembleResources` (EN page creates; TH pages reuse via non-exact Shift).
    pub r: OnceLock<Arc<dyn Resource>>,
}

impl fmt::Debug for ResourceSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResourceSource")
            .field("lang_index", &self.lang_index)
            .field("path", &self.path.as_ref().map(|p| p.path().to_string()))
            .field("page", &self.page)
            .finish()
    }
}

impl ResourceSource {
    /// Go: `resourceSource.clone()` (without the created resource).
    // Go: hugolib/content_map.go:clone
    pub fn clone_source(&self) -> ResourceSource {
        ResourceSource {
            lang_index: self.lang_index,
            path: self.path.clone(),
            opener: self.opener.clone(),
            fi: self.fi.clone(),
            rc: self.rc.clone(),
            page: self.page,
            r: OnceLock::new(),
        }
    }

    // Go: hugolib/content_map.go:LangIndex
    pub fn lang_index(&self) -> usize {
        self.lang_index
    }

    /// Go: `isPage()` — `r.r` is a `page.Page` (a bundled content file).
    // Go: hugolib/content_map.go:isPage
    pub fn is_page(&self) -> bool {
        self.page.is_some()
    }

    /// Go: `Path()` (Go panics on a bundled page's nil path).
    // Go: hugolib/content_map.go:Path
    pub fn path(&self) -> &str {
        self.path
            .as_ref()
            .expect("invalid memory address or nil pointer dereference")
            .path()
    }
}

/// Go: `cleanTreeKey(k)` — lower-case, leading `/`, no trailing `/`; home is `""`.
// Go: hugolib/content_map.go:cleanTreeKey
pub fn clean_tree_key(k: &str) -> String {
    clean_tree_key_elems(&[k])
}

/// Go: `cleanTreeKey(elem...)` — `path.Join` of several elements first.
// Go: hugolib/content_map.go:cleanTreeKey
pub fn clean_tree_key_elems(elem: &[&str]) -> String {
    let mut s = String::new();
    if !elem.is_empty() {
        s = elem[0].to_string();
        if elem.len() > 1 {
            s = go_path::path::join(elem);
        }
    }
    let s = trim_func_dot_slash_space(&s);
    let s = go_unicode::strings::to_lower_str(&nh_common::paths::path::sanitize(s)).into_owned();
    if s.is_empty() || s == "/" {
        return String::new();
    }
    if !s.starts_with('/') {
        return format!("/{s}");
    }
    s
}

/// Go: `strings.TrimFunc(s, trimCutsetDotSlashSpace)` (`.`, `/` and `unicode.IsSpace`).
fn trim_func_dot_slash_space(s: &str) -> &str {
    let f = |r: char| r == '.' || r == '/' || go_unicode::is_space(r as i32);
    s.trim_matches(f)
}

/// Go: `viewName` (taxonomy singular/plural + `pluralTreeKey`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ViewName {
    pub singular: String,
    pub plural: String,
    pub plural_tree_key: String,
}

impl ViewName {
    // Go: hugolib/content_map_page.go:IsZero
    pub fn is_zero(&self) -> bool {
        self.singular.is_empty()
    }
}

/// Go: `(m *pageMap) insertPageWithLock(s, p)` — `m` is `site_idx`'s page map (the inserts of
/// a build all go through the first site's).
// Go: hugolib/content_map.go:insertPageWithLock
pub fn insert_page_with_lock(
    h: &mut HugoSites,
    site_idx: usize,
    s: &str,
    p: PageId,
) -> (ContentNode, Option<ContentNode>, bool) {
    let lang = h.page(p).site_idx;
    let dims = h.sites[site_idx].page_map.dims;
    let (u, n, replaced) = h
        .page_trees
        .tree_pages
        .insert_into_values_dimension_with_lock(dims, s, ContentNode::Page(p, lang));

    if replaced && !h.is_rebuild() && h.sites[site_idx].conf.root.print_path_warnings {
        let mut message_detail = String::new();
        if let Some(ContentNode::Page(p1, _)) = &n
            && let Some(f) = &h.page(*p1).meta.f
        {
            message_detail = format!(" file: {}", go_strconv::quote(f.filename()));
        }
        if let ContentNode::Page(p2, _) = &u
            && let Some(f) = &h.page(*p2).meta.f
        {
            message_detail.push_str(&format!(" file: {}", go_strconv::quote(f.filename())));
        }

        h.sites[site_idx].deps.log.warnf(format!(
            "Duplicate content path: {}{message_detail}",
            go_strconv::quote(s)
        ));
    }

    (u, n, replaced)
}

/// Go: `(m *pageMap) insertResource(s, r)`.
// Go: hugolib/content_map.go:insertResource
pub fn insert_resource(
    h: &mut HugoSites,
    site_idx: usize,
    s: &str,
    r: ContentNode,
) -> (ContentNode, Option<ContentNode>, bool) {
    let dims = h.sites[site_idx].page_map.dims;
    let updated = r.clone();
    let (u, n, replaced) = h
        .page_trees
        .tree_resources
        .insert_into_values_dimension(dims, s, r);
    if replaced {
        handle_duplicate_resource_path(h, site_idx, s, &updated, n.as_ref());
    }
    (u, n, replaced)
}

/// Go: `(m *pageMap) handleDuplicateResourcePath(s, updated, existing)`.
// Go: hugolib/content_map.go:handleDuplicateResourcePath
fn handle_duplicate_resource_path(
    h: &HugoSites,
    site_idx: usize,
    s: &str,
    updated: &ContentNode,
    existing: Option<&ContentNode>,
) {
    if h.is_rebuild() || !h.sites[site_idx].conf.root.print_path_warnings {
        return;
    }
    let mut message_detail = String::new();
    if let Some(ContentNode::Resource(r1)) = existing
        && let Some(fi) = &r1.fi
    {
        message_detail = format!(" file: {}", go_strconv::quote(&fi.meta().filename));
    }
    if let ContentNode::Resource(r2) = updated
        && let Some(fi) = &r2.fi
    {
        message_detail.push_str(&format!(
            " file: {}",
            go_strconv::quote(&fi.meta().filename)
        ));
    }

    h.sites[site_idx].deps.log.warnf(format!(
        "Duplicate resource path: {}{message_detail}",
        go_strconv::quote(s)
    ));
}

/// Go: `(m *pageMap) AddFi(fi, buildConfig)` — `m` is `site_idx`'s page map. Returns the page
/// and resource counts.
// Go: hugolib/content_map.go:AddFi
pub fn add_fi(
    h: &mut HugoSites,
    site_idx: usize,
    fi: &FileMetaInfo,
    build_config: &BuildCfg,
) -> Result<(u64, u64)> {
    let _ = build_config;
    let mut page_count = 0u64;
    let mut resource_count = 0u64;
    if fi.is_dir() {
        return Ok((page_count, resource_count));
    }

    let meta = fi.meta();
    let pi = meta
        .path_info
        .clone()
        .unwrap_or_else(|| panic!("no path info for {:?}", meta.filename));

    match pi.path_type() {
        PathType::File | PathType::ContentResource => {
            h.sites[site_idx].deps.log.trace(format!(
                "insert resource: {}",
                go_strconv::quote(&meta.filename)
            ));
            // insertResource(fi)
            resource_count += 1;
            let mut key = pi.base();

            let rs = if pi.is_content() {
                // Create the page now as we need it at assembly time.
                // The other resources are created if needed.
                let page_resource = new_page(
                    h,
                    NewPageMeta {
                        f: Some(nh_helpers::source::file_info::File::new(fi.clone())),
                        path_info: Some(pi.clone()),
                        bundled: true,
                        ..Default::default()
                    },
                )?;
                let Some(page_resource) = page_resource else {
                    // Disabled page.
                    return Ok((page_count, resource_count));
                };
                key = h.page(page_resource).meta.path_info.base();
                ResourceSource {
                    lang_index: h.page(page_resource).site_idx,
                    path: None,
                    opener: None,
                    fi: None,
                    rc: None,
                    page: Some(page_resource),
                    r: OnceLock::new(),
                }
            } else {
                let fim = fi.clone();
                let opener: OpenReadSeekCloser = Arc::new(move || {
                    let f = fim.meta().open()?;
                    Ok(Box::new(f) as Box<dyn nh_common::hugio::ReadSeekCloser>)
                });
                ResourceSource {
                    lang_index: usize::try_from(meta.lang_index).unwrap_or(0),
                    path: Some(pi.clone()),
                    opener: Some(opener),
                    fi: Some(fi.clone()),
                    rc: None,
                    page: None,
                    r: OnceLock::new(),
                }
            };

            let _ = insert_resource(h, site_idx, &key, ContentNode::Resource(Arc::new(rs)));
        }
        PathType::ContentData => {
            // Go: addPagesFromGoTmplFi (content adapters).
            return Err(Error::new(
                "neohugo-rs: content adapters (_content.gotmpl) are not supported",
            ));
        }
        _ => {
            h.sites[site_idx].deps.log.trace(format!(
                "insert bundle: {}",
                go_strconv::quote(&meta.filename)
            ));

            page_count += 1;

            // A content file.
            let p = new_page(
                h,
                NewPageMeta {
                    f: Some(nh_helpers::source::file_info::File::new(fi.clone())),
                    path_info: Some(pi.clone()),
                    bundled: false,
                    ..Default::default()
                },
            )?;
            let Some(p) = p else {
                // Disabled page.
                return Ok((page_count, resource_count));
            };

            let key = h.page(p).meta.path_info.base();
            insert_page_with_lock(h, site_idx, &key, p);
        }
    }
    Ok((page_count, resource_count))
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/content_map.go (493 lines; 7/23 funcs executed)
//   types: contentMapConfig, resourceSource, resourceSources
// OK L67-70: (r resourceSource) clone() *resourceSource
// OK L72-74: (r *resourceSource) LangIndex() int
//    L76-78: (r *resourceSource) MarkStale()
//    L80-84: (r *resourceSource) resetBuildState()
// OK L86-89: (r *resourceSource) isPage() bool
//    L91-96: (r *resourceSource) GetIdentity() identity.Identity
//    L98-100: (r *resourceSource) ForEeachIdentity(f func(identity.Identity) bool) bool
// OK L102-104: (r *resourceSource) Path() string
//    L106-108: (r *resourceSource) isContentNodeBranch() bool
//    L114-120: (n resourceSources) MarkStale()
//    L122-124: (n resourceSources) Path() string
//    L126-128: (n resourceSources) isContentNodeBranch() bool
//    L130-136: (n resourceSources) resetBuildState()
//    L138-145: (n resourceSources) GetIdentity() identity.Identity
//    L147-156: (n resourceSources) ForEeachIdentity(f func(identity.Identity) bool) bool
// OK L158-165: (cfg contentMapConfig) getTaxonomyConfig(s string) (v viewName)
// OK L167-183: (m *pageMap) insertPageWithLock(s string, p *pageState) (contentNodeI, contentNodeI, bool)
//    L185-191: (m *pageMap) insertResourceWithLock(s string, r contentNodeI) (contentNodeI, contentNodeI, bool)
// OK L193-199: (m *pageMap) insertResource(s string, r contentNodeI) (contentNodeI, contentNodeI, bool)
// OK L201-214: (m *pageMap) handleDuplicateResourcePath(s string, updated, existing contentNodeI)
// OK L216-317: (m *pageMap) AddFi(fi hugofs.FileMetaInfo, buildConfig *BuildCfg) (pageCount uint64, resourceCount uint64, addErr error)
//    L319-471: (m *pageMap) addPagesFromGoTmplFi(fi hugofs.FileMetaInfo, buildConfig *BuildCfg) (pageCount uint64, resourceCount uint64, addErr error)  [STUB: unsupported error]
// OK L476-493: cleanTreeKey(elem ...string) string
// ---------------------------------------------------------------------------
