//! Port of `hugolib/pagecollections.go`.
//!
//! Owner: Wave B task T21 (hugolib-assemble).

//! Go `hugolib/pagecollections.go`: `pageFinder` — GetPage ref normalisation (trailing `/`
//! trimmed, `.md` appended, `/_index.md` for `/`, PathParser base), tree lookup in the site's
//! language, reverse index by base name for bare refs.
//!
//! Go's `pageFinder` holds the site's `pageMap`; here it is the site index. The results are
//! `page.Page` values (`None` is Go's nil; `Site.GetPage` turns it into `page.NilPage`, T23).

use std::sync::Arc;

use nh_common::Result;
use nh_common::herrors::Error;
use nh_common::kinds;
use nh_common::paths::path as cpaths;
use nh_common::paths::pathparser::{Path, has_ext};
use nh_helpers::source::file_info::File;
use nh_hugofs::fileinfo::FileMetaInfo;
use nh_page::page::PageRef;

use crate::content_map_trees::ReverseIndexEntry;
use crate::hugo_sites::HugoSites;
use crate::page::{PageHandle, PageId, PageWrapper};

const DEFAULT_CONTENT_EXT: &str = ".md";

/// Go: `pageFinder` (the site's `pageMap`).
#[derive(Clone)]
pub struct PageFinder {
    pub h: Arc<HugoSites>,
    pub site_idx: usize,
}

/// Go: `newPageFinder(m)`.
// Go: hugolib/pagecollections.go:newPageFinder
pub fn new_page_finder(h: &Arc<HugoSites>, site_idx: usize) -> PageFinder {
    if site_idx >= h.sites.len() {
        panic!("must provide a pageMap");
    }
    PageFinder {
        h: h.clone(),
        site_idx,
    }
}

/// The page context of a lookup: what Go reads from `context page.Page`.
struct ContextPage {
    path_info: Option<Arc<Path>>,
    file: Option<Arc<File>>,
}

impl ContextPage {
    fn from_ref(p: &PageRef) -> ContextPage {
        if let Some(h) = p.0.as_any().downcast_ref::<PageHandle>() {
            let ps = h.state();
            return ContextPage {
                path_info: Some(ps.meta.path_info.clone()),
                file: ps.meta.f.clone(),
            };
        }
        ContextPage {
            path_info: Some(p.0.path_info()),
            file: p.0.file(),
        }
    }
}

impl PageFinder {
    fn page_ref(&self, id: PageId) -> PageRef {
        PageHandle {
            h: self.h.clone(),
            id,
            wrapper: PageWrapper::None,
        }
        .page_ref()
    }

    /// Go: `getPageRef(context, ref)` — resolves a Page from ref/relRef, with a slightly more
    /// comprehensive search path than getPage.
    // Go: hugolib/pagecollections.go:getPageRef
    pub fn get_page_ref(&self, context: Option<&PageRef>, r: &str) -> Result<Option<PageRef>> {
        let n = self.get_content_node(context, true, r)?;
        Ok(n.map(|id| self.page_ref(id)))
    }

    // Go: hugolib/pagecollections.go:getPage
    pub fn get_page(&self, context: Option<&PageRef>, r: &str) -> Result<Option<PageRef>> {
        let n = self.get_content_node(context, false, r)?;
        Ok(n.map(|id| self.page_ref(id)))
    }

    /// Go: `getPageOldVersion(kind, sections...)` (only used in tests).
    // Go: hugolib/pagecollections.go:getPageOldVersion
    pub fn get_page_old_version(&self, kind: &str, sections: &[&str]) -> Option<PageRef> {
        let refs = vec![kind.to_string(), go_path::path::join(sections)];
        self.get_page_for_refs(&refs).ok().flatten()
    }

    /// Go: `getPageForRefs(ref...)` — an adapter func for the old API with Kind as first
    /// argument. This is invoked when you do .Site.GetPage. We drop the Kind and fails if
    /// there are more than 2 arguments, which would be ambiguous.
    // Go: hugolib/pagecollections.go:getPageForRefs
    pub fn get_page_for_refs(&self, r: &[String]) -> Result<Option<PageRef>> {
        let mut refs: Vec<&str> = Vec::new();
        for rr in r {
            // A common construct in the wild is
            // .Site.GetPage "home" "" or
            // .Site.GetPage "home" "/"
            if !rr.is_empty() && rr != "/" {
                refs.push(rr);
            }
        }

        if refs.len() > 2 {
            // This was allowed in Hugo <= 0.44, but we cannot support this with the
            // new API. This should be the most unusual case.
            return Err(Error::new(format!(
                "too many arguments to .Site.GetPage: [{}]. Use lookups on the form {{{{ .Site.GetPage \"/posts/mypage-md\" }}}}",
                r.join(" ")
            )));
        }

        let key: &str = if refs.is_empty() || refs[0] == kinds::KIND_HOME {
            "/"
        } else if refs.len() == 1 {
            if r.len() == 2 && refs[0] == kinds::KIND_SECTION {
                // This is an old style reference to the "Home Page section".
                // Typically fetched via {{ .Site.GetPage "section" .Section }}
                // See https://github.com/gohugoio/hugo/issues/4989
                "/"
            } else {
                refs[0]
            }
        } else {
            refs[1]
        };

        self.get_page(None, key)
    }

    // Go: hugolib/pagecollections.go:getContentNode
    fn get_content_node(
        &self,
        context: Option<&PageRef>,
        is_reflink: bool,
        r: &str,
    ) -> Result<Option<PageId>> {
        let context = context.map(ContextPage::from_ref);
        let mut r = cpaths::to_slash_trim_trailing(r);
        let in_ref = r.clone();
        if r.is_empty() {
            r = "/".to_string();
        }

        if has_ext(&r) {
            return self.get_content_node_for_ref(context.as_ref(), is_reflink, true, &in_ref, &r);
        }

        // We are always looking for a content file and having an extension greatly simplifies
        // the code that follows, even in the case where the extension does not match this one.
        if r == "/" {
            if let Some(n) = self.get_content_node_for_ref(
                context.as_ref(),
                is_reflink,
                false,
                &in_ref,
                &format!("/_index{DEFAULT_CONTENT_EXT}"),
            )? {
                return Ok(Some(n));
            }
        } else if r.ends_with("/index") {
            if let Some(n) = self.get_content_node_for_ref(
                context.as_ref(),
                is_reflink,
                false,
                &in_ref,
                &format!("{r}/index{DEFAULT_CONTENT_EXT}"),
            )? {
                return Ok(Some(n));
            }
            if let Some(n) = self.get_content_node_for_ref(
                context.as_ref(),
                is_reflink,
                false,
                &in_ref,
                &format!("{r}{DEFAULT_CONTENT_EXT}"),
            )? {
                return Ok(Some(n));
            }
        } else if let Some(n) = self.get_content_node_for_ref(
            context.as_ref(),
            is_reflink,
            false,
            &in_ref,
            &format!("{r}{DEFAULT_CONTENT_EXT}"),
        )? {
            return Ok(Some(n));
        }

        Ok(None)
    }

    // Go: hugolib/pagecollections.go:getContentNodeForRef
    fn get_content_node_for_ref(
        &self,
        context: Option<&ContextPage>,
        is_reflink: bool,
        had_extension: bool,
        in_ref: &str,
        r: &str,
    ) -> Result<Option<PageId>> {
        let s = &self.h.sites[self.site_idx];
        let content_path_parser = s.deps.conf.path_parser();

        if let Some(context) = context
            && !r.starts_with('/')
        {
            // Try the page-relative path first.
            // Branch pages: /mysection, "./mypage" => /mysection/mypage
            // Regular pages: /mysection/mypage.md, Path=/mysection/mypage, "./someotherpage" => /mysection/mypage/../someotherpage
            // Regular leaf bundles: /mysection/mypage/index.md, Path=/mysection/mypage, "./someotherpage" => /mysection/mypage/../someotherpage
            // Given the above, for regular pages we use the containing folder.
            let mut base_dir = String::new();
            if let Some(pi) = &context.path_info {
                if pi.is_branch_bundle() || (had_extension && r.starts_with("../")) {
                    base_dir = pi.dir().to_string();
                } else {
                    base_dir = pi.container_dir().to_string();
                }
            }

            let rel = go_path::path::join(&[base_dir.as_str(), r]);

            let (rel_path, _) = content_path_parser.parse_base_and_base_name_no_identifier(
                nh_common::files::COMPONENT_FOLDER_CONTENT,
                &rel,
            );

            if let Some(n) = self.get_content_node_from_path(&rel_path, r)? {
                return Ok(Some(n));
            }

            if had_extension
                && let Some(f) = &context.file
                && let Some(n) =
                    self.get_content_node_from_ref_reverse_lookup(in_ref, f.file_info())?
            {
                return Ok(Some(n));
            }
        }

        if r.starts_with('.') {
            // Page relative, no need to look further.
            return Ok(None);
        }

        let (rel_path, name_no_identifier) = content_path_parser
            .parse_base_and_base_name_no_identifier(nh_common::files::COMPONENT_FOLDER_CONTENT, r);

        if let Some(n) = self.get_content_node_from_path(&rel_path, r)? {
            return Ok(Some(n));
        }

        if had_extension
            && let Some(home) = s.home
            && let Some(f) = &self.h.page(home).meta.f
            && let Some(n) = self.get_content_node_from_ref_reverse_lookup(in_ref, f.file_info())?
        {
            return Ok(Some(n));
        }

        let mut do_simple_lookup = false;
        if is_reflink || context.is_none() {
            let slash_count = in_ref.matches('/').count();
            do_simple_lookup = slash_count == 0;
        }

        if !do_simple_lookup {
            return Ok(None);
        }

        let pm = &s.page_map;
        match pm.page_reverse_index.get(
            &name_no_identifier,
            &self.h.page_trees,
            &self.h.pages,
            pm.dims,
        ) {
            Some(ReverseIndexEntry::Ambiguous) => Err(Error::new(format!(
                "page reference {} is ambiguous",
                go_strconv::quote(in_ref)
            ))),
            Some(ReverseIndexEntry::Page(id)) => Ok(Some(id)),
            None => Ok(None),
        }
    }

    // Go: hugolib/pagecollections.go:getContentNodeFromRefReverseLookup
    fn get_content_node_from_ref_reverse_lookup(
        &self,
        r: &str,
        fi: &FileMetaInfo,
    ) -> Result<Option<PageId>> {
        let s = &self.h.sites[self.site_idx];
        let meta = fi.meta();
        let mut dir = meta.filename.clone();
        if !fi.is_dir() {
            dir = go_path::filepath::dir(&meta.filename);
        }

        let real_filename = go_path::filepath::join(&[dir.as_str(), r]);

        let pcs = s
            .deps
            .path_spec()
            .base_fs
            .source_filesystems
            .content
            .reverse_lookup(&real_filename, true)?;

        // There may be multiple matches, but we will only use the first one.
        let pp = s.deps.conf.path_parser();
        for pc in pcs {
            let pi = pp.parse(&pc.component, &pc.path);
            if let Some(n) = self
                .h
                .page_trees
                .tree_pages
                .get(s.page_map.dims, &pi.base())
            {
                return Ok(n.page_id());
            }
        }
        Ok(None)
    }

    // Go: hugolib/pagecollections.go:getContentNodeFromPath
    fn get_content_node_from_path(&self, s: &str, _ref: &str) -> Result<Option<PageId>> {
        let site = &self.h.sites[self.site_idx];
        let n = self.h.page_trees.tree_pages.get(site.page_map.dims, s);
        Ok(n.and_then(|n| n.page_id()))
    }
}

/// Go: `(s *Site) GetPage(ref...)`'s lookup (`s.getPageForRefs`) for one ref in a context
/// page (`getPage(context, ref)` when `context` is set).
// Go: hugolib/pagecollections.go:getPage
pub fn get_page(
    h: &Arc<HugoSites>,
    site_idx: usize,
    context: Option<&PageRef>,
    ref_: &str,
) -> Result<Option<PageRef>> {
    new_page_finder(h, site_idx).get_page(context, ref_)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/pagecollections.go (256 lines; 7/9 funcs executed)
//   types: pageFinder
// OK L36-42: newPageFinder(m *pageMap) *pageFinder
// OK L46-56: (c *pageFinder) getPageRef(context page.Page, ref string) (page.Page, error)
// OK L58-67: (c *pageFinder) getPage(context page.Page, ref string) (page.Page, error)
// OK L70-74: (c *pageFinder) getPageOldVersion(kind string, sections ...string) page.Page
// OK L79-114: (c *pageFinder) getPageForRefs(ref ...string) (page.Page, error)
// OK L118-149: (c *pageFinder) getContentNode(context page.Page, isReflink bool, ref string) (contentNodeI, error)
// OK L151-222: (c *pageFinder) getContentNodeForRef(context page.Page, isReflink, hadExtension bool, inRef, ref string) (contentNodeI, error)
// OK L224-247: (c *pageFinder) getContentNodeFromRefReverseLookup(ref string, fi hugofs.FileMetaInfo) (contentNodeI, error)
// OK L249-256: (c *pageFinder) getContentNodeFromPath(s string, ref string) (contentNodeI, error)
// ---------------------------------------------------------------------------
