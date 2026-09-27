//! Port of `resources/page/page_paths.go`.
//!
//! Owner: Wave B task T11 (page-api-paths).


//! Go `resources/page/page_paths.go`: `CreateTargetPaths` — THE function deciding every output
//! file name and link. Port exactly (sanitize per element, `%23`, `paths.PathEscape`, ugly/root/
//! prefix rules). Specs: content-model.md §9, output-publishing.md §4.

use std::sync::Arc;

use nh_common::paths::pathparser::Path;
use nh_helpers::pathspec::PathSpec;
use nh_media::output::output_format::OutputFormat as Format;

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
        todo!()
    }

    /// Go: `PermalinkForOutputFormat(s, f)` — baseURL (with protocol for e.g. calendar) + Link.
    // Go: resources/page/page_paths.go:PermalinkForOutputFormat
    pub fn permalink_for_output_format(&self, s: &PathSpec, f: &Format) -> String {
        todo!()
    }
}

/// Go: `page.CreateTargetPaths(d)`.
// Go: resources/page/page_paths.go:CreateTargetPaths
pub fn create_target_paths(d: &TargetPathDescriptor) -> TargetPaths {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: resources/page/page_paths.go (462 lines; 16/17 funcs executed)
//   types: TargetPathDescriptor, TargetPaths, pagePathBuilder
// EX L90-92: (p TargetPaths) RelPermalink(s *helpers.PathSpec) string
// EX L94-107: (p TargetPaths) PermalinkForOutputFormat(s *helpers.PathSpec, f output.Format) string
// EX L109-293: CreateTargetPaths(d TargetPathDescriptor) (tp TargetPaths)
// EX L311-323: (p *pagePathBuilder) Add(el ...string)
// EX L325-339: (p *pagePathBuilder) ConcatLast(s string)
// EX L341-343: (p *pagePathBuilder) IsHtmlIndex() bool
// EX L345-350: (p *pagePathBuilder) Last() string
// EX L352-368: (p *pagePathBuilder) Link() string
// EX L370-382: (p *pagePathBuilder) LinkDir() string
// EX L384-391: (p *pagePathBuilder) Path(upperOffset int) string
// EX L393-399: (p *pagePathBuilder) PathDir() string
// EX L401-420: (p *pagePathBuilder) PathDirBase() string
// EX L422-428: (p *pagePathBuilder) PathFile() string
//    L430-432: (p *pagePathBuilder) Prepend(el ...string)
// EX L434-438: (p *pagePathBuilder) Sanitize()
// EX L446-450: getPagePathBuilder(d TargetPathDescriptor) *pagePathBuilder
// EX L452-462: putPagePathBuilder(b *pagePathBuilder)
// ---------------------------------------------------------------------------
