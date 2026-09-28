//! Port of `resources/page/page_paths.go`.
//!
//! Owner: Wave B task T11 (page-api-paths).
//!
//! Go `resources/page/page_paths.go`: `CreateTargetPaths` — THE function deciding every output
//! file name and link. Port exactly (sanitize per element, `%23`, `paths.PathEscape`, ugly/root/
//! prefix rules). Specs: content-model.md §9, output-publishing.md §4.

use std::sync::Arc;

use nh_common::kinds;
use nh_common::paths::path as paths;
use nh_common::paths::pathparser::Path;
use nh_helpers::pathspec::PathSpec;
use nh_media::output::output_format::{OutputFormat as Format, builtin_formats};

const SLASH: &str = "/";

/// Go: `page.TargetPathDescriptor`.
#[derive(Clone)]
pub struct TargetPathDescriptor {
    pub path_spec: Arc<PathSpec>,
    pub type_: Format,
    pub kind: String,
    pub path: Arc<Path>,
    pub section: Option<Arc<Path>>,
    /// For regular content pages this is either the slug or the base name of the file.
    pub base_name: String,
    /// Typically a language prefix added to file paths.
    pub prefix_file_path: String,
    /// Typically a language prefix added to links.
    pub prefix_link: String,
    /// Always use the prefix (sitemap, multihost).
    pub force_prefix: bool,
    /// From front matter `url`.
    pub url: String,
    /// Used to create paginator links, e.g. `/page/2`.
    pub addends: String,
    /// The expanded permalink if defined for the section, ready to use.
    pub expanded_permalink: String,
    pub ugly_urls: bool,
}

/// Go: `page.TargetPaths`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TargetPaths {
    /// Where to store the file on disk relative to the publish dir. OS slashes.
    pub target_filename: String,
    /// The directory to store sub-resources of this page (images...).
    pub sub_resource_base_target: String,
    /// The base for creating links to resources below this page.
    pub sub_resource_base_link: String,
    /// The relative permalink to this resources. Unix slashes.
    pub link: String,
}

impl TargetPaths {
    /// Go: `TargetPaths.RelPermalink(s)` = `s.PrependBasePath(p.Link, false)`.
    // Go: resources/page/page_paths.go:RelPermalink
    pub fn rel_permalink(&self, s: &PathSpec) -> String {
        s.prepend_base_path(&self.link, false)
    }

    /// Go: `PermalinkForOutputFormat(s, f)` — baseURL (with protocol for e.g. calendar) + Link.
    /// An invalid protocol gives "" (Go discards the error).
    // Go: resources/page/page_paths.go:PermalinkForOutputFormat
    pub fn permalink_for_output_format(&self, s: &PathSpec, f: &Format) -> String {
        let base_url = if !f.protocol.is_empty() {
            match s.cfg.base_url().with_protocol(&f.protocol) {
                Ok(b) => b,
                Err(_) => return String::new(),
            }
        } else {
            s.cfg.base_url()
        };
        let base_url_str = base_url.string();
        s.permalink_for_base_url(&self.link, base_url_str)
    }
}

/// The section path of a descriptor. Go's `Section` is a `*paths.Path` that is never nil when
/// `CreateTargetPaths` reads it (hugolib always sets it); a missing one here behaves like a
/// path whose `Base()` is `/`.
fn section_base(d: &TargetPathDescriptor) -> String {
    match &d.section {
        Some(s) => s.base(),
        None => SLASH.to_string(),
    }
}

/// Go: `page.CreateTargetPaths(d)`.
// Go: resources/page/page_paths.go:CreateTargetPaths
pub fn create_target_paths(d: &TargetPathDescriptor) -> TargetPaths {
    let mut d = d.clone();
    let mut tp = TargetPaths::default();

    // Normalize all file Windows paths to simplify what's next (a no-op on unix).

    if !d.type_.root && !d.url.is_empty() && !d.url.starts_with('/') {
        // Treat this as a context relative URL
        d.force_prefix = true;
    }

    if !d.url.is_empty() {
        d.url = go_path::filepath::to_slash(&d.url).to_string();
        if d.url.contains("..") {
            d.url = go_path::path::join(&["/", d.url.as_str()]);
        }
    }

    if d.type_.root && !d.force_prefix {
        d.prefix_file_path = String::new();
        d.prefix_link = String::new();
    }

    let mut pb = PagePathBuilder::new(d);
    let d = &pb.d.clone();

    pb.full_suffix = d.type_.media_type.first_suffix.full_suffix.clone();

    // The top level index files, i.e. the home page etc., needs
    // the index base even when uglyURLs is enabled.
    let mut needs_base = true;

    pb.is_ugly = (d.ugly_urls || d.type_.ugly) && !d.type_.no_ugly;
    pb.base_name_same_as_type =
        !d.path.is_bundle() && !d.base_name.is_empty() && d.base_name == d.type_.base_name;
    let mut index_is_ugly_kind = d.kind == kinds::KIND_HOME
        || d.kind == kinds::KIND_SECTION
        || d.kind == kinds::KIND_TAXONOMY;
    index_is_ugly_kind = index_is_ugly_kind && pb.is_ugly;

    if d.expanded_permalink.is_empty() && pb.base_name_same_as_type {
        pb.is_ugly = true;
    }

    if !d.type_.path.is_empty() {
        pb.add(std::slice::from_ref(&d.type_.path));
    }

    let bf = builtin_formats();
    if d.type_ == bf.http_status_404_html || d.type_ == bf.sitemap || d.type_ == bf.robots_txt {
        pb.no_sub_resources = true;
    } else if d.kind != kinds::KIND_PAGE && d.url.is_empty() && section_base(d) != SLASH {
        if !d.expanded_permalink.is_empty() {
            pb.add(std::slice::from_ref(&d.expanded_permalink));
        } else {
            pb.add(&[section_base(d)]);
        }
        needs_base = false;
    }

    if d.kind != kinds::KIND_HOME && !d.url.is_empty() {
        pb.add(&paths::fields_slash(&d.url));

        if !d.addends.is_empty() {
            pb.add(std::slice::from_ref(&d.addends));
        }

        let has_dot = d.url.contains('.');
        let has_slash = d.url.ends_with('/');

        if has_slash || !has_dot {
            pb.add(&[format!("{}{}", d.type_.base_name, pb.full_suffix)]);
        } else if has_dot {
            pb.full_suffix = paths::ext(&d.url).to_string();
        }

        if pb.is_html_index() {
            pb.link_upper_offset = 1;
        }

        if d.force_prefix {
            // Prepend language prefix if not already set in URL
            if !d.prefix_file_path.is_empty()
                && !d.url.starts_with(&format!("/{}", d.prefix_file_path))
            {
                pb.prefix_path = d.prefix_file_path.clone();
            }

            if !d.prefix_link.is_empty() && !d.url.starts_with(&format!("/{}", d.prefix_link)) {
                pb.prefix_link = d.prefix_link.clone();
            }
        }
    } else if !kinds::is_branch(&d.kind) {
        if !d.expanded_permalink.is_empty() {
            pb.add(std::slice::from_ref(&d.expanded_permalink));
        } else {
            let dir = d.path.container_dir();
            if !dir.is_empty() {
                pb.add(&[dir.to_string()]);
            }
            if !d.base_name.is_empty() {
                pb.add(std::slice::from_ref(&d.base_name));
            } else {
                pb.add(&[d.path.base_name_no_identifier().to_string()]);
            }
        }

        if !d.addends.is_empty() {
            pb.add(std::slice::from_ref(&d.addends));
        }

        if pb.is_ugly {
            let s = pb.full_suffix.clone();
            pb.concat_last(&s);
        } else {
            pb.add(&[format!("{}{}", d.type_.base_name, pb.full_suffix)]);
        }

        if pb.is_html_index() {
            pb.link_upper_offset = 1;
        }

        if !d.prefix_file_path.is_empty() {
            pb.prefix_path = d.prefix_file_path.clone();
        }

        if !d.prefix_link.is_empty() {
            pb.prefix_link = d.prefix_link.clone();
        }
    } else {
        if !d.addends.is_empty() {
            pb.add(std::slice::from_ref(&d.addends));
        }

        needs_base = needs_base && d.addends.is_empty();

        if needs_base || (!pb.is_ugly || index_is_ugly_kind) {
            pb.add(&[format!("{}{}", d.type_.base_name, pb.full_suffix)]);
        } else {
            let s = pb.full_suffix.clone();
            pb.concat_last(&s);
        }

        if !index_is_ugly_kind && pb.is_html_index() {
            pb.link_upper_offset = 1;
        }

        if !d.prefix_file_path.is_empty() {
            pb.prefix_path = d.prefix_file_path.clone();
        }

        if !d.prefix_link.is_empty() {
            pb.prefix_link = d.prefix_link.clone();
        }
    }

    // if page URL is explicitly set in frontmatter,
    // preserve its value without sanitization
    if d.url.is_empty() {
        // Note: MakePathSanitized will lower case the path if
        // disablePathToLower isn't set.
        pb.sanitize();
    }

    let mut link = pb.link();

    let page_path = pb.path_file();

    tp.target_filename = go_path::filepath::from_slash(&page_path).to_string();
    if !pb.no_sub_resources {
        tp.sub_resource_base_target = pb.path_dir();
        tp.sub_resource_base_link = pb.link_dir();
    }

    // paths.{URL,Path}Escape rely on url.Parse which
    // will consider # a fragment identifier, so it and
    // and everything after it will be stripped from
    // `link`, so we need to escape it first.
    link = link.replace('#', "%23");

    if !d.url.is_empty() {
        tp.link = nh_common::paths::url::url_escape(&link);
    } else {
        // This is slightly faster for when we know we don't have any
        // query or scheme etc.
        tp.link = paths::path_escape(&link);
    }
    if tp.link.is_empty() {
        tp.link = "/".to_string();
    }

    tp
}

/// Go: `pagePathBuilder` (Go pools these; the port allocates one per call, see `last`).
struct PagePathBuilder {
    els: Vec<String>,

    d: TargetPathDescriptor,

    // Builder state.
    is_ugly: bool,
    base_name_same_as_type: bool,
    no_sub_resources: bool,
    /// File suffix including any ".".
    full_suffix: String,
    prefix_link: String,
    prefix_path: String,
    link_upper_offset: usize,
}

impl PagePathBuilder {
    // Go: resources/page/page_paths.go:getPagePathBuilder
    fn new(d: TargetPathDescriptor) -> PagePathBuilder {
        PagePathBuilder {
            els: Vec::new(),
            d,
            is_ugly: false,
            base_name_same_as_type: false,
            no_sub_resources: false,
            full_suffix: String::new(),
            prefix_link: String::new(),
            prefix_path: String::new(),
            link_upper_offset: 0,
        }
    }

    // Go: resources/page/page_paths.go:Add
    fn add(&mut self, el: &[String]) {
        // Filter empty and slashes.
        for e in el {
            if !e.is_empty() && e != SLASH {
                self.els.push(e.clone());
            }
        }
    }

    // Go: resources/page/page_paths.go:ConcatLast
    fn concat_last(&mut self, s: &str) {
        if self.els.is_empty() {
            self.add(&[s.to_string()]);
            return;
        }
        let n = self.els.len();
        let mut old = self.els[n - 1].clone();
        if old.is_empty() {
            self.els[n - 1] = s.to_string();
            return;
        }
        if old.ends_with('/') {
            old.pop();
        }
        self.els[n - 1] = old + s;
    }

    // Go: resources/page/page_paths.go:IsHtmlIndex
    fn is_html_index(&self) -> bool {
        self.last() == "index.html"
    }

    /// Go: `Last()` — `""` for a nil `els`. A pooled builder has an empty, non-nil `els` and Go
    /// then panics (index -1); the port always answers `""` (see PORTING.md).
    // Go: resources/page/page_paths.go:Last
    fn last(&self) -> &str {
        match self.els.last() {
            Some(s) => s,
            None => "",
        }
    }

    // Go: resources/page/page_paths.go:Link
    fn link(&self) -> String {
        let mut link = self.path(self.link_upper_offset);

        if self.base_name_same_as_type
            && let Some(l) = link.strip_suffix(self.d.base_name.as_str())
        {
            link = l.to_string();
        }

        if !self.prefix_link.is_empty() {
            link = format!("/{}{link}", self.prefix_link);
        }

        if self.link_upper_offset > 0 && !link.ends_with('/') {
            link.push('/');
        }

        link
    }

    // Go: resources/page/page_paths.go:LinkDir
    fn link_dir(&self) -> String {
        if self.no_sub_resources {
            return String::new();
        }

        let mut path_dir = self.path_dir_base();

        if !self.prefix_link.is_empty() {
            path_dir = format!("/{}{path_dir}", self.prefix_link);
        }

        path_dir
    }

    // Go: resources/page/page_paths.go:Path
    fn path(&self, upper_offset: usize) -> String {
        let mut upper = self.els.len();
        if upper_offset > 0 {
            // Go slices els[:len-offset]; offset is 0 or 1 and set only after an element was
            // added, so this never underflows.
            upper -= upper_offset;
        }
        let els: Vec<&str> = self.els[..upper].iter().map(|s| s.as_str()).collect();
        let pth = go_path::path::join(&els);
        paths::add_leading_slash(&pth)
    }

    // Go: resources/page/page_paths.go:PathDir
    fn path_dir(&self) -> String {
        let mut dir = self.path_dir_base();
        if !self.prefix_path.is_empty() {
            dir = format!("/{}{dir}", self.prefix_path);
        }
        dir
    }

    // Go: resources/page/page_paths.go:PathDirBase
    fn path_dir_base(&self) -> String {
        if self.no_sub_resources {
            return String::new();
        }

        let mut dir = self.path(0);
        let is_index = self
            .last()
            .starts_with(&format!("{}.", self.d.type_.base_name));

        if is_index {
            dir = paths::dir(&dir);
        } else if let Some(d) = dir.strip_suffix(self.full_suffix.as_str()) {
            dir = d.to_string();
        }

        if dir == "/" {
            dir = String::new();
        }

        dir
    }

    // Go: resources/page/page_paths.go:PathFile
    fn path_file(&self) -> String {
        let mut dir = self.path(0);
        if !self.prefix_path.is_empty() {
            dir = format!("/{}{dir}", self.prefix_path);
        }
        dir
    }

    // Go: resources/page/page_paths.go:Prepend
    #[allow(dead_code)]
    fn prepend(&mut self, el: &[String]) {
        let mut v = el.to_vec();
        v.append(&mut self.els);
        self.els = v;
    }

    // Go: resources/page/page_paths.go:Sanitize
    fn sanitize(&mut self) {
        for el in self.els.iter_mut() {
            *el = self.d.path_spec.make_path_sanitized(el);
        }
    }
}

// Go: resources/page/page_paths.go:putPagePathBuilder
// (The pool is not ported: every call gets a fresh builder.)

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/page/page_paths.go (462 lines; 16/17 funcs executed)
//   types: TargetPathDescriptor, TargetPaths, pagePathBuilder
// OK L90-92: (p TargetPaths) RelPermalink(s *helpers.PathSpec) string
// OK L94-107: (p TargetPaths) PermalinkForOutputFormat(s *helpers.PathSpec, f output.Format) string
// OK L109-293: CreateTargetPaths(d TargetPathDescriptor) (tp TargetPaths)
// OK L311-323: (p *pagePathBuilder) Add(el ...string)
// OK L325-339: (p *pagePathBuilder) ConcatLast(s string)
// OK L341-343: (p *pagePathBuilder) IsHtmlIndex() bool
// OK L345-350: (p *pagePathBuilder) Last() string
// OK L352-368: (p *pagePathBuilder) Link() string
// OK L370-382: (p *pagePathBuilder) LinkDir() string
// OK L384-391: (p *pagePathBuilder) Path(upperOffset int) string
// OK L393-399: (p *pagePathBuilder) PathDir() string
// OK L401-420: (p *pagePathBuilder) PathDirBase() string
// OK L422-428: (p *pagePathBuilder) PathFile() string
// OK L430-432: (p *pagePathBuilder) Prepend(el ...string)
// OK L434-438: (p *pagePathBuilder) Sanitize()
// OK L446-450: getPagePathBuilder(d TargetPathDescriptor) *pagePathBuilder
// OK L452-462: putPagePathBuilder(b *pagePathBuilder)
// ---------------------------------------------------------------------------
