//! Port of `hugolib/page__new.go`.
//!
//! Owner: Wave B task T20 (hugolib-capture).

//! Go `hugolib/page__new.go`: page creation from a content file or a synthetic meta (kind
//! detection: home / taxonomy (pluralTreeKey) / term (taxonomy prefix, character-level) /
//! section / page; site from the file language).
//!
//! Go creates the lazy page parts in `initLazyProviders` (a `lazy.Init` that runs on the first
//! `shiftToOutputFormat`); in Rust that is `PageState::lazy`, filled by T21's `init_page`.

use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};

use nh_common::Result;
use nh_common::herrors::Error;
use nh_common::kinds;
use nh_common::paths::pathparser::Path;
use nh_helpers::source::file_info::File;
use nh_media::output::output_format::OutputFormat;
use nh_page::pagemeta::page_frontmatter::PageConfig;

use crate::content_map::ViewName;
use crate::hugo_sites::HugoSites;
use crate::page::{PageId, PageState};
use crate::page__common::PageCommon;
use crate::page__content_parse::{new_cached_content, parse_front_matter};
use crate::page__meta::PageMeta;

/// Go: the `*pageMeta` handed to `h.newPage(m)`: what the caller sets before creation (a content
/// file: `f`, `pathInfo`, `bundled`; a page created during assembly: `pathInfo`, the site and a
/// `pageConfig` with its kind, a standalone output format, ...). Everything else is computed.
#[derive(Default)]
pub struct NewPageMeta {
    /// Go `pathInfo` (else the file's path info, or the front matter `path`).
    pub path_info: Option<Arc<Path>>,
    /// Go `f` (`source.NewFileInfo(fi)`).
    pub f: Option<Arc<File>>,
    /// Go `bundled`: a content file inside a leaf bundle.
    pub bundled: bool,
    /// Go `resourcePath`.
    pub resource_path: String,
    /// Go `term` (set by the caller for terms created during assembly).
    pub term: String,
    /// Go `singular`.
    pub singular: String,
    /// Go `m.s`: the site, when the caller knows it (else resolved from the language).
    pub site_idx: Option<usize>,
    /// Go `m.pageMetaParams.pageConfig` preset by the caller (e.g. its `Kind`; Go's zero
    /// `PageConfig{}` when `None`).
    pub page_config: Option<PageConfig>,
    /// Go `standaloneOutputFormat` (404, sitemap, robots...).
    pub standalone_output_format: Option<OutputFormat>,
}

/// Go: `h.newPage(m)` — `None` for a disabled page (a disabled language or kind).
// Go: hugolib/page__new.go:newPage
pub fn new_page(h: &mut HugoSites, m: NewPageMeta) -> Result<Option<PageId>> {
    let p = do_new_page(h, m)?;
    // (Go marks a partially created page stale on error: no stale tracking in the port.)

    if let Some(id) = p {
        let ps = h.page(id);
        let pth = ps.meta.path_info.clone();
        if ps.meta.is_home() && pth.is_leaf_bundle() {
            let mut msg = "Using %s in your content's root directory is usually incorrect for your home page. ".to_string();
            msg.push_str(
                "You should use %s instead. If you don't rename this file, your home page will be ",
            );
            msg.push_str("treated as a leaf bundle, meaning it won't be able to have any child pages or sections.");
            let p0 = pth.path_no_leading_slash().to_string();
            let msg = msg
                .replacen("%s", &p0, 1)
                .replacen("%s", &p0.replace("index", "_index"), 1);
            h.deps
                .log
                .warnidf(nh_common::constants::WARN_HOME_PAGE_IS_LEAF_BUNDLE, msg);
        }
    }

    Ok(p)
}

/// Go: `h.doNewPage(m)`.
// Go: hugolib/page__new.go:doNewPage
fn do_new_page(h: &mut HugoSites, nm: NewPageMeta) -> Result<Option<PageId>> {
    let NewPageMeta {
        path_info,
        f,
        bundled,
        resource_path,
        term,
        singular,
        site_idx,
        page_config,
        standalone_output_format,
    } = nm;

    // `m.pathInfo` may still be nil here (it is resolved below); a placeholder until then.
    let path_info_set = path_info.is_some();
    let mut m = PageMeta::new(path_info.unwrap_or_default());
    m.f = f;
    m.bundled = bundled;
    m.resource_path = resource_path;
    m.term = term;
    m.singular = singular;
    m.standalone_output_format = standalone_output_format;
    m.page_config = page_config.unwrap_or_default();
    if m.page_config.params.is_none() {
        m.page_config.params = Some(go_value::Map::new(go_value::MapType::Params));
    }

    let pid = h.page_id_counter.fetch_add(1, Ordering::SeqCst) + 1;
    let pi = parse_front_matter(&m, pid)?;

    // Go `h.Conf`: the first site's config.
    let conf = h.deps.conf.clone();
    if let Err(err) = m.set_meta_pre(&pi, &h.deps.log, &*conf) {
        return Err(wrap_error(&m, path_info_set, err));
    }
    if !m.page_config.lang.is_empty() && conf.is_lang_disabled(&m.page_config.lang) {
        return Ok(None);
    }

    let mut path_info_set = path_info_set;
    if !m.page_config.path.is_empty() {
        let mut s = m.page_config.path.clone();
        // Paths from content adapters should never have any extension.
        if m.page_config.is_from_content_adapter || !nh_common::paths::pathparser::has_ext(&s) {
            let mut is_branch = false;
            let mut is_branch_set = false;
            let mut ext = m.page_config.content_media_type.first_suffix.suffix.clone();
            if !m.page_config.kind.is_empty() {
                is_branch = kinds::is_branch(&m.page_config.kind);
                is_branch_set = true;
            }

            if !m.page_config.is_from_content_adapter {
                if path_info_set {
                    if !is_branch_set {
                        is_branch = m.path_info.is_branch_bundle();
                    }
                    if !m.path_info.ext().is_empty() {
                        ext = m.path_info.ext().to_string();
                    }
                } else if let Some(f) = &m.f
                    && let Some(pi) = &f.file_info().meta().path_info
                {
                    if !is_branch_set {
                        is_branch = pi.is_branch_bundle();
                    }
                    if !pi.ext().is_empty() {
                        ext = pi.ext().to_string();
                    }
                }
            }

            if is_branch {
                s.push_str(&format!("/_index.{ext}"));
            } else {
                s.push_str(&format!("/index.{ext}"));
            }
        }
        m.path_info = Arc::new(
            conf.path_parser()
                .parse(nh_common::files::COMPONENT_FOLDER_CONTENT, &s),
        );
        path_info_set = true;
    } else if !path_info_set {
        if let Some(f) = &m.f
            && let Some(pi) = &f.file_info().meta().path_info
        {
            m.path_info = pi.clone();
            path_info_set = true;
        }

        if !path_info_set {
            panic!("missing pathInfo in page meta");
        }
    }

    // Identify the Site/language to associate this Page with.
    let site_idx = match site_idx {
        Some(i) => i,
        None => {
            let lang = if !m.page_config.lang.is_empty() {
                m.page_config.lang.clone()
            } else if let Some(f) = &m.f {
                f.file_info().meta().lang.clone()
            } else {
                m.path_info.lang().to_string()
            };

            match h.resolve_site(&lang) {
                Some(i) => i,
                None => {
                    return Err(Error::new(format!(
                        "no site found for language {}",
                        go_strconv::quote(&lang)
                    )));
                }
            }
        }
    };
    m.lang = h.sites[site_idx].language.lang.clone();

    let mut tc = ViewName::default();
    // Identify Page Kind.
    if m.page_config.kind.is_empty() {
        m.page_config.kind = kinds::KIND_SECTION.to_string();
        if m.path_info.base() == "/" {
            m.page_config.kind = kinds::KIND_HOME.to_string();
        } else if m.path_info.is_branch_bundle() {
            // A section, taxonomy or term.
            tc = h.sites[site_idx]
                .page_map
                .cfg
                .get_taxonomy_config(&m.path());
            if !tc.is_zero() {
                // Either a taxonomy or a term.
                if tc.plural_tree_key == m.path() {
                    m.page_config.kind = kinds::KIND_TAXONOMY.to_string();
                } else {
                    m.page_config.kind = kinds::KIND_TERM.to_string();
                }
            }
        } else if m.f.is_some() {
            m.page_config.kind = kinds::KIND_PAGE.to_string();
        }
    }

    if m.page_config.kind == kinds::KIND_TERM || m.page_config.kind == kinds::KIND_TAXONOMY {
        if tc.is_zero() {
            tc = h.sites[site_idx]
                .page_map
                .cfg
                .get_taxonomy_config(&m.path());
        }
        if tc.is_zero() {
            return Err(Error::new(format!(
                "no taxonomy configuration found for {}",
                go_strconv::quote(m.path())
            )));
        }
        m.singular = tc.singular.clone();
        if m.page_config.kind == kinds::KIND_TERM {
            let base = m.path_info.unnormalized().base();
            m.term = nh_common::paths::path::trim_leading(
                base.strip_prefix(tc.plural_tree_key.as_str())
                    .unwrap_or(&base),
            );
        }
    }

    if m.page_config.kind == kinds::KIND_PAGE
        && !h.sites[site_idx].conf.is_kind_enabled(&m.page_config.kind)
    {
        return Ok(None);
    }

    // Parse the rest of the page content.
    let content = {
        let s = &h.sites[site_idx];
        let store = s.deps.get_template_store();
        let enable_inline_shortcodes = s.deps.exec_helper().sec().enable_inline_shortcodes;
        new_cached_content(
            &m,
            store,
            enable_inline_shortcodes,
            s.conf.root.enable_emoji,
            pi,
        )
        .map_err(|err| wrap_error(&m, true, err))?
    };

    if m.f.is_some() {
        // Go: gitInfoForPage / codeownersForPage.
        h.git_info_for_page()?;
    }

    let id = PageId(u32::try_from(h.pages.len()).expect("too many pages"));
    let ps = PageState {
        id,
        pid,
        site_idx,
        meta: m,
        common: PageCommon::default(),
        lazy: OnceLock::new(),
        current_output_idx: AtomicUsize::new(0),
        page_output_template_variations_state: AtomicU32::new(0),
        content: Some(Arc::new(content)),
    };
    h.pages.push(ps);

    Ok(Some(id))
}

/// Go: `(p *pageMeta) wrapError(err, sourceFs)` — the page path, or the file info, as context.
// Go: hugolib/page.go:wrapError
fn wrap_error(m: &PageMeta, path_info_set: bool, err: Error) -> Error {
    match &m.f {
        None => {
            let path = if path_info_set {
                m.path()
            } else {
                String::new()
            };
            // No more details to add.
            err.wrap(go_strconv::quote(&path))
        }
        Some(f) => nh_hugofs::fileinfo::add_file_info_to_error(err, f.file_info()),
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/page__new.go (265 lines; 2/2 funcs executed)
// OK L37-52: (h *HugoSites) newPage(m *pageMeta) (*pageState, *paths.Path, error)
// OK L54-265: (h *HugoSites) doNewPage(m *pageMeta) (*pageState, *paths.Path, error)
// ---------------------------------------------------------------------------
