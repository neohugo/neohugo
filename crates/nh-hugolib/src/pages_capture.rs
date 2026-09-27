//! Port of `hugolib/pages_capture.go`.
//!
//! Owner: Wave B task T20 (hugolib-capture).


//! Go `hugolib/pages_capture.go`: walks the content component fs (ComponentFs order), applies
//! `IgnoreFile`, handles leaf bundles (`handleBundleLeaf`: other files become resources), branch
//! bundles, and calls `pageMap.AddFi` for each file. Walk sequentially; parsing may be parallel
//! (the tree is keyed; duplicates are dropped at walk time).

use nh_common::Result;

use crate::hugo_sites::HugoSites;

/// Go: `pagesCollector`.
pub struct PagesCollector<'a> {
    pub h: &'a mut HugoSites,
}

impl PagesCollector<'_> {
    // Go: hugolib/pages_capture.go:Collect
    pub fn collect(&mut self) -> Result<()> {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/pages_capture.go (425 lines; 5/5 funcs executed)
//   types: pagesCollector
// EX L38-60: newPagesCollector( ctx context.Context, h *HugoSites, sp *source.SourceSpec, logger loggers.Logger, infoLogger logg.LevelLogger, m *pageMap, buildC...
// EX L85-219: (c *pagesCollector) Collect() (collectErr error)
// EX L221-251: (c *pagesCollector) collectDir(dirPath *paths.Path, isDir bool, inFilter func(fim hugofs.FileMetaInfo) bool) error
// EX L253-376: (c *pagesCollector) collectDirDir(path string, root hugofs.FileMetaInfo, inFilter func(fim hugofs.FileMetaInfo) bool) error
// EX L378-425: (c *pagesCollector) handleBundleLeaf(dir, bundle hugofs.FileMetaInfo, inPath string, readdir []hugofs.FileMetaInfo) error
// ---------------------------------------------------------------------------
