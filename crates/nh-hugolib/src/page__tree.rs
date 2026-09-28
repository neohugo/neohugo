//! Port of `hugolib/page__tree.go`.
//!
//! Owner: Wave B task T21 (hugolib-assemble).

//! Go `hugolib/page__tree.go`: Parent (LongestPrefix of ContainerDir to a branch; home -> nil),
//! CurrentSection, FirstSection, InSection, IsAncestor/IsDescendant, Sections, SectionsEntries.
//!
//! The navigation works on the page arena (`*_id` functions, usable during assembly: the
//! target path descriptor needs `CurrentSection`) and on handles (the template API).

use std::sync::Arc;

use go_value::Value;
use nh_common::kinds;
use nh_common::paths::path as cpaths;
use nh_doctree::nodeshifttree::WalkConfig;
use nh_page::page::{PageRef, Pages};

use crate::content_map_trees::ContentNode;
use crate::hugo_sites::HugoSites;
use crate::page::{PageHandle, PageId, PageWrapper};

fn page_of(n: &ContentNode) -> PageId {
    n.page_id()
        .expect("interface conversion: contentNodeI is not *hugolib.pageState")
}

/// Go: `pageTree.CurrentSection()` on the arena: the page itself for branch kinds, else the
/// closest branch above its directory (home for root pages).
// Go: hugolib/page__tree.go:CurrentSection
pub fn current_section_id(h: &HugoSites, id: PageId) -> Option<PageId> {
    let p = h.page(id);
    if kinds::is_branch(p.meta.kind()) {
        return Some(id);
    }

    let s = &h.sites[p.site_idx];
    let dir = p.meta.path_info.dir();
    if dir == "/" {
        return s.home;
    }

    let is_branch = |n: &ContentNode| n.page_id().is_some_and(|id| h.page(id).meta.is_node());
    if let Some((_, n)) =
        h.page_trees
            .tree_pages
            .longest_prefix(s.page_map.dims, dir, true, Some(&is_branch))
    {
        return Some(page_of(&n));
    }

    panic!(
        "CurrentSection not found for {} in lang {}",
        go_strconv::quote(p.meta.path()),
        p.meta.lang()
    );
}

/// Go: `pageTree.FirstSection()` on the arena (`None` is Go's nil).
// Go: hugolib/page__tree.go:FirstSection
pub fn first_section_id(h: &HugoSites, id: PageId) -> Option<PageId> {
    let p = h.page(id);
    let site = &h.sites[p.site_idx];
    let mut s = p.meta.path_info.dir().to_string();
    if s == "/" {
        return site.home;
    }

    let is_branch = |n: &ContentNode| n.page_id().is_some_and(|id| h.page(id).meta.is_node());
    loop {
        let (k, n) = h.page_trees.tree_pages.longest_prefix(
            site.page_map.dims,
            &s,
            true,
            Some(&is_branch),
        )?;

        // /blog
        if k.matches('/').count() < 2 {
            return Some(page_of(&n));
        }

        if s.is_empty() {
            return None;
        }

        s = cpaths::dir(&s);
    }
}

/// Go: `pageTree.Parent()` on the arena (`None` for the home page, Go's nil).
// Go: hugolib/page__tree.go:Parent
pub fn parent_id(h: &HugoSites, id: PageId) -> Option<PageId> {
    let p = h.page(id);
    if p.meta.is_home() {
        return None;
    }
    let site = &h.sites[p.site_idx];

    let mut dir = p.meta.path_info.container_dir().to_string();

    if dir.is_empty() {
        return site.home;
    }

    loop {
        let Some((_, n)) =
            h.page_trees
                .tree_pages
                .longest_prefix(site.page_map.dims, &dir, true, None)
        else {
            return site.home;
        };
        let nid = page_of(&n);
        if p.meta.bundled || h.page(nid).meta.is_node() {
            return Some(nid);
        }
        dir = cpaths::dir(&dir);
    }
}

/// Go: `pageTree.SectionsPath()` = `CurrentSection().Path()`.
// Go: hugolib/page__tree.go:SectionsPath
pub fn sections_path_id(h: &HugoSites, id: PageId) -> String {
    match current_section_id(h, id) {
        Some(cs) => h.page(cs).meta.path(),
        // Go dereferences the nil home page.
        None => panic!("invalid memory address or nil pointer dereference"),
    }
}

/// Go: `pageTree.SectionsEntries()` (`None` is Go's nil slice).
// Go: hugolib/page__tree.go:SectionsEntries
pub fn sections_entries_id(h: &HugoSites, id: PageId) -> Option<Vec<String>> {
    let sp = sections_path_id(h, id);
    if sp == "/" {
        return None;
    }
    let entries: Vec<String> = sp[1.min(sp.len())..]
        .split('/')
        .map(|s| s.to_string())
        .collect();
    if entries.is_empty() {
        return None;
    }
    Some(entries)
}

fn to_ref(p: &PageHandle, id: PageId) -> PageRef {
    PageHandle {
        h: p.h.clone(),
        id,
        wrapper: PageWrapper::None,
    }
    .page_ref()
}

/// The `Path()` of a tree navigation argument that is a content node (Go
/// `other.(contentNodeI)`): a page of these sites.
fn content_node_path(p: &PageHandle, other: &Value) -> Option<String> {
    let o = nh_page::page::page_from_value(other)?;
    let h = o.0.as_any().downcast_ref::<PageHandle>()?;
    if !Arc::ptr_eq(&h.h, &p.h) {
        return None;
    }
    Some(h.state().meta.path())
}

// Go: hugolib/page__tree.go:IsAncestor
pub fn is_ancestor(p: &PageHandle, other: &Value) -> bool {
    let Some(n) = content_node_path(p, other) else {
        return false;
    };
    let pp = p.state().meta.path();
    if n == pp {
        return false;
    }
    n.starts_with(&cpaths::add_trailing_slash(&pp))
}

// Go: hugolib/page__tree.go:IsDescendant
pub fn is_descendant(p: &PageHandle, other: &Value) -> bool {
    let Some(n) = content_node_path(p, other) else {
        return false;
    };
    let pp = p.state().meta.path();
    if n == pp {
        return false;
    }
    pp.starts_with(&cpaths::add_trailing_slash(&n))
}

/// Go: `CurrentSection()` (never nil in Go but for a site without a home page).
// Go: hugolib/page__tree.go:CurrentSection
pub fn current_section(p: &PageHandle) -> PageRef {
    match current_section_id(&p.h, p.id) {
        Some(id) => to_ref(p, id),
        None => panic!("invalid memory address or nil pointer dereference"),
    }
}

/// Go: `FirstSection()` (`None` is Go's nil `page.Page`).
// Go: hugolib/page__tree.go:FirstSection
pub fn first_section(p: &PageHandle) -> Option<PageRef> {
    first_section_id(&p.h, p.id).map(|id| to_ref(p, id))
}

/// Go: `InSection(other)`.
// Go: hugolib/page__tree.go:InSection
pub fn in_section(p: &PageHandle, other: &Value) -> bool {
    if matches!(other, Value::Invalid | Value::TypedNil(_)) {
        return false;
    }
    let Some(o) = nh_page::page::page_from_value(other) else {
        return false;
    };
    let Some(cs) = o.0.current_section() else {
        return false;
    };
    let mine = current_section_id(&p.h, p.id);
    let Some(oh) = cs.0.as_any().downcast_ref::<PageHandle>() else {
        return false;
    };
    Arc::ptr_eq(&oh.h, &p.h) && Some(oh.id) == mine
}

/// Go: `Parent()` (`None` is Go's nil `page.Page`, e.g. for the home page).
// Go: hugolib/page__tree.go:Parent
pub fn parent(p: &PageHandle) -> Option<PageRef> {
    parent_id(&p.h, p.id).map(|id| to_ref(p, id))
}

/// Go: `Ancestors()`.
// Go: hugolib/page__tree.go:Ancestors
pub fn ancestors(p: &PageHandle) -> Pages {
    let mut ancestors: Pages = Vec::new();
    let mut parent = parent_id(&p.h, p.id);
    while let Some(id) = parent {
        ancestors.push(to_ref(p, id));
        parent = parent_id(&p.h, id);
    }
    ancestors
}

/// Go: `Sections()` — the sections directly below the page, sorted.
// Go: hugolib/page__tree.go:Sections
pub fn sections(p: &PageHandle) -> Pages {
    let h = &p.h;
    let ps = p.state();
    let s = ps.meta.path();
    let prefix = cpaths::add_trailing_slash(&s);
    let site = &h.sites[ps.site_idx];

    let mut ids: Vec<PageId> = Vec::new();
    let mut current_branch_prefix = String::new();
    let cfg = WalkConfig {
        dims: site.page_map.dims,
        prefix,
        ..Default::default()
    };
    let res = h.page_trees.tree_pages.walk(&cfg, |w, ss, n, _match| {
        let Some(id) = n.page_id() else {
            return Ok(false);
        };
        let np = h.page(id);
        if !np.meta.is_node() {
            return Ok(false);
        }
        if current_branch_prefix.is_empty() || !ss.starts_with(&current_branch_prefix) {
            if np.meta.is_section() && np.meta.should_list(false) && parent_id(h, id) == Some(p.id)
            {
                ids.push(id);
            } else {
                w.skip_prefix(&format!("{ss}/"));
            }
        }
        current_branch_prefix = format!("{ss}/");
        Ok(false)
    });
    if let Err(err) = res {
        panic!("{}", err.message());
    }

    let mut pages: Pages = ids.into_iter().map(|id| to_ref(p, id)).collect();
    nh_page::pages_sort::sort_by_default(&mut pages);
    pages
}

/// Go: `pageTree.Page()` — the page itself (unwrapped).
// Go: hugolib/page__tree.go:Page
pub fn page(p: &PageHandle) -> PageRef {
    to_ref(p, p.id)
}

/// Go: `SectionsEntries()`.
// Go: hugolib/page__tree.go:SectionsEntries
pub fn sections_entries(p: &PageHandle) -> Option<Vec<String>> {
    sections_entries_id(&p.h, p.id)
}

/// Go: `SectionsPath()`.
// Go: hugolib/page__tree.go:SectionsPath
pub fn sections_path(p: &PageHandle) -> String {
    sections_path_id(&p.h, p.id)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/page__tree.go (203 lines; 3/11 funcs executed)
//   types: pageTree
// OK L33-44: (pt pageTree) IsAncestor(other any) bool
// OK L46-57: (pt pageTree) IsDescendant(other any) bool
// OK L59-75: (pt pageTree) CurrentSection() page.Page
// OK L77-101: (pt pageTree) FirstSection() page.Page
// OK L103-114: (pt pageTree) InSection(other any) bool
// OK L116-137: (pt pageTree) Parent() page.Page
// OK L139-147: (pt pageTree) Ancestors() page.Pages
// OK L149-183: (pt pageTree) Sections() page.Pages
// OK L185-187: (pt pageTree) Page() page.Page
// OK L189-199: (p pageTree) SectionsEntries() []string
// OK L201-203: (p pageTree) SectionsPath() string
// ---------------------------------------------------------------------------
