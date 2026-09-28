//! Port of `hugolib/page__paths.go`.
//!
//! Owner: Wave B task T21 (hugolib-assemble).

//! Go `hugolib/page__paths.go`: `createTargetPathDescriptor` (prefixes: "" for en, "th" for th;
//! sitemap always in a language subdir; BaseName = slug / standalone basename / BaseNameNoIdentifier)
//! and per-format target paths + `page.OutputFormats`. Owned by T21 because `initLazyProviders`
//! (page__init.rs, run during `assembleResources`) needs `newPagePaths`.

use std::collections::BTreeMap;

use nh_common::Result;
use nh_page::page_outputformat::{OutputFormat as PageOutputFormat, OutputFormats};
use nh_page::page_paths::{TargetPathDescriptor, TargetPaths};

/// Go: `targetPathsHolder`.
#[derive(Clone, Debug)]
pub struct TargetPathsHolder {
    pub rel_url: String,
    pub paths: TargetPaths,
    pub output_format: PageOutputFormat,
}

/// Go: `pagePaths`.
#[derive(Clone)]
pub struct PagePaths {
    pub output_formats: OutputFormats,
    pub first_output_format: PageOutputFormat,
    /// Format name -> target paths.
    pub target_paths: BTreeMap<String, TargetPathsHolder>,
    pub target_path_descriptor: TargetPathDescriptor,
}

/// Go: `newPagePaths(ps)`.
// Go: hugolib/page__paths.go:newPagePaths
pub fn new_page_paths(p: &crate::page::PageHandle) -> Result<PagePaths> {
    todo!()
}

/// Go: `createTargetPathDescriptor(p)`.
// Go: hugolib/page__paths.go:createTargetPathDescriptor
pub fn create_target_path_descriptor(p: &crate::page::PageHandle) -> Result<TargetPathDescriptor> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/page__paths.go (181 lines; 3/3 funcs executed)
//   types: pagePaths
// EX L26-97: newPagePaths(ps *pageState) (pagePaths, error)
// EX L107-109: (l pagePaths) OutputFormats() page.OutputFormats
// EX L111-181: createTargetPathDescriptor(p *pageState) (page.TargetPathDescriptor, error)
// ---------------------------------------------------------------------------
