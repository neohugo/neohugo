//! Capture and assembly into the [`Model`] (docs/rust-port/REWRITE_PLAN.md §2.4, phases A4–B2).
//!
//! [`load_model`] reads the content and data of a project and builds the page arena:
//!
//! 1. **Capture** (A4, parallel over files): discovery through the `Vfs`, front matter split and
//!    decoded into folded [`Params`], the capture overrides `kind`, `lang` and `path`, the page's
//!    own `cascade`. `data::load` builds `.Site.Data` alongside.
//! 2. **Tree** (B1): every page gets its language, key and kind (home, section, taxonomy, term
//!    or page) and enters its language's [`SiteTree`]; content files inside leaf bundles are
//!    [`PageRole::Bundled`] pages, other bundle files [`BundleResource`]s. Keys claimed twice keep
//!    the first file, with a warning.
//! 3. **Cascade → meta → dates → filter** (B2): the cascade handed down each tree
//!    ([`CascadeIndex`]), then [`neohugo_page::meta_from_params`] with the language's date
//!    sources and time zone (parallel over pages), then drafts, future and expired content
//!    against the build clock.
//!
//! Auto nodes (missing home, root sections, taxonomies and terms, standalone pages), URLs,
//! relations, taxonomies and translations are the next phases (T23b): they build on
//! [`SiteModel::cascade`], [`meta::cascaded_params`] and [`Model::bundle_owner`].

#![forbid(unsafe_code)]

mod capture;
mod cascade;
pub mod data;
mod filter;
pub mod meta;
mod tree;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use neohugo_base::diag::Diagnostic;
use neohugo_base::paths::ContentKey;
use neohugo_base::{Clock, IdVec, LangIdx, Map, PageId, PageKind, Params, ResourceId};
use neohugo_config::{Config, ContentFilter};
use neohugo_page::{PageError, PageMeta};
use neohugo_vfs::{FileRef, PathInfo, Vfs, VfsError};

pub use capture::SourceFile;
pub use cascade::CascadeIndex;
pub use data::{Data, DataError};
pub use tree::{PageRole, SiteTree};

use filter::Verdict;

/// A page of the model: a content file, or (from T23b) a page Hugo makes itself.
#[derive(Clone, Debug)]
pub struct Page {
    pub id: PageId,
    pub lang: LangIdx,
    pub kind: PageKind,
    pub role: PageRole,
    /// The key in the language's tree (`.Path` without the leading slash); a bundled page's
    /// key keeps its file extension (`post/notes.md`).
    pub key: ContentKey,
    /// `None` for pages without a content file.
    pub source: Option<SourceFile>,
    /// Typed front matter after the cascade, with the dates resolved and, for switched-off
    /// nodes, the build policy turned off.
    pub meta: PageMeta,
}

impl Page {
    /// The front matter params (after the cascade).
    #[must_use]
    pub fn params(&self) -> &Params {
        &self.meta.params
    }
}

/// A file inside a bundle directory that is not a page of its own: bundle images and data, and
/// the content files of leaf bundles (then `page` is the bundled page).
#[derive(Clone, Debug)]
pub struct BundleResource {
    /// The resource's key: its path with extension (`blog/post/cover.jpg`).
    pub key: ContentKey,
    /// The language of the file (file name, else mount, else the default language).
    pub lang: LangIdx,
    pub file: FileRef,
    pub info: PathInfo,
    pub page: Option<PageId>,
}

/// One language's part of the model.
#[derive(Clone, Debug)]
pub struct SiteModel {
    pub lang: LangIdx,
    /// The language's pages of their own.
    pub tree: SiteTree,
    /// The language's bundle files by key (files of other languages are not in it).
    pub resources: BTreeMap<ContentKey, ResourceId>,
    /// The cascade handed down the tree.
    pub cascade: CascadeIndex,
}

/// The site model: every page of every language in one arena.
#[derive(Clone, Debug)]
pub struct Model {
    pub config: Arc<Config>,
    pub pages: IdVec<PageId, Page>,
    pub sites: IdVec<LangIdx, SiteModel>,
    pub bundle_resources: IdVec<ResourceId, BundleResource>,
    /// `.Site.Data` (keys as written).
    pub data: Arc<Map>,
    /// Warnings and non-fatal errors of loading, in phase order.
    pub diagnostics: Vec<Diagnostic>,
}

impl Model {
    /// The page `id`.
    #[must_use]
    pub fn page(&self, id: PageId) -> &Page {
        &self.pages[id]
    }

    /// The index page of a bundled page's bundle: in the page's language, else in the first
    /// language (by weight) that has one.
    #[must_use]
    pub fn bundle_owner(&self, id: PageId) -> Option<PageId> {
        let p = &self.pages[id];
        let PageRole::Bundled { bundle } = &p.role else {
            return None;
        };
        self.sites[p.lang]
            .tree
            .get(bundle)
            .or_else(|| self.sites.iter().find_map(|s| s.tree.get(bundle)))
    }
}

/// Which unpublished content a build includes, and its "now".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LoadModelOptions {
    pub clock: Clock,
    pub content: ContentFilter,
}

impl LoadModelOptions {
    /// The configuration's `buildDrafts`/`buildFuture`/`buildExpired` at `clock`.
    #[must_use]
    pub fn from_config(cfg: &Config, clock: Clock) -> Self {
        Self {
            clock,
            content: cfg.content,
        }
    }
}

/// Why the model could not be built.
#[derive(Debug, thiserror::Error)]
pub enum ModelError {
    #[error(transparent)]
    Vfs(#[from] VfsError),
    #[error("{path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("{path}: {message}")]
    FrontMatter { path: PathBuf, message: String },
    #[error("{path}: {source}")]
    Page {
        path: PathBuf,
        #[source]
        source: PageError,
    },
    #[error("{0}: content adapters (_content.gotmpl) are not supported")]
    ContentAdapter(PathBuf),
    #[error("{path}: no taxonomy is configured for {key:?}")]
    NoTaxonomy { path: PathBuf, key: String },
    #[error("site cascade: {0}")]
    SiteCascade(#[source] PageError),
    #[error(transparent)]
    Data(#[from] DataError),
}

/// Builds the model of `cfg`'s project: phases A4–B2 (see the crate docs).
///
/// # Errors
/// A content or data file that cannot be read or decoded, invalid front matter (reserved keys
/// of the wrong shape, a bad cascade or date configuration), a content adapter, or a
/// taxonomy kind without a taxonomy.
pub fn load_model(cfg: Arc<Config>, vfs: &Vfs, o: &LoadModelOptions) -> Result<Model, ModelError> {
    let (captured, data) = rayon::join(|| capture::capture(&cfg, vfs), || data::load(vfs));
    let assembly = tree::place(&cfg, captured?)?;
    let data = data?;
    let cascades = meta::cascade_indexes(&cfg, &assembly)?;
    let (metas, meta_diags) = meta::metas(&cfg, &assembly, &cascades)?;

    let mut diagnostics = assembly.diagnostics;
    diagnostics.extend(meta_diags);

    // Filter: removed pages take the bundle files below them (same language) with them.
    let mut removed: Vec<(LangIdx, ContentKey)> = Vec::new();
    let mut keep = vec![true; assembly.pages.len()];
    let mut metas: Vec<Option<PageMeta>> = metas.into_iter().map(Some).collect();
    for (i, p) in assembly.pages.iter().enumerate() {
        if p.role != PageRole::Standalone {
            continue;
        }
        let site = &cfg.sites[p.page.lang];
        let Some(meta) = metas[i].as_mut() else {
            continue;
        };
        match o.verdict(p.kind, !site.disable_kinds.contains(p.kind), meta) {
            Verdict::Build => {}
            Verdict::Disable => meta.build = filter::DISABLED,
            Verdict::Remove => {
                keep[i] = false;
                removed.push((p.page.lang, p.key.clone()));
            }
        }
    }
    let below_removed = |lang: LangIdx, key: &ContentKey| {
        removed
            .iter()
            .any(|(l, k)| *l == lang && k != key && key.starts_with_segments(k))
    };

    let mut pages: IdVec<PageId, Page> = IdVec::new();
    let mut bundle_resources: IdVec<ResourceId, BundleResource> = IdVec::new();
    let mut sites: IdVec<LangIdx, SiteModel> = cascades
        .into_iter()
        .enumerate()
        .map(|(i, cascade)| SiteModel {
            lang: <LangIdx as neohugo_base::Idx>::from_index(i),
            tree: SiteTree::default(),
            resources: BTreeMap::new(),
            cascade,
        })
        .collect();
    for (i, p) in assembly.pages.into_iter().enumerate() {
        let lang = p.page.lang;
        let bundled = p.role != PageRole::Standalone;
        if !keep[i] || (bundled && below_removed(lang, &p.key)) {
            continue;
        }
        let Some(meta) = metas[i].take() else {
            continue;
        };
        let id = pages.next_id();
        let source = p.page.source;
        if bundled {
            let rid = bundle_resources.push(BundleResource {
                key: p.key.clone(),
                lang,
                file: source.file.clone(),
                info: source.file_info.clone(),
                page: Some(id),
            });
            sites[lang].resources.insert(p.key.clone(), rid);
        } else {
            sites[lang].tree.insert(p.key.clone(), id);
        }
        pages.push(Page {
            id,
            lang,
            kind: p.kind,
            role: p.role,
            key: p.key,
            source: Some(source),
            meta,
        });
    }
    for r in assembly.resources {
        if below_removed(r.lang, &r.info.key) {
            continue;
        }
        let key = r.info.key.clone();
        let rid = bundle_resources.push(BundleResource {
            key: key.clone(),
            lang: r.lang,
            file: r.file,
            info: r.info,
            page: None,
        });
        sites[r.lang].resources.insert(key, rid);
    }

    diagnostics.extend(data.diagnostics);
    Ok(Model {
        config: cfg,
        pages,
        sites,
        bundle_resources,
        data: Arc::new(data.map),
        diagnostics,
    })
}
