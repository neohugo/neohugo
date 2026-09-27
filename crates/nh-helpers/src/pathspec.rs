//! Port of `helpers/pathspec.go`.
//!
//! Owner: Wave B task T08 (helpers-source-cache).


use std::sync::Arc;

use nh_common::Result;
use nh_config::config_provider::AllProvider;
use nh_hugofs::filesystems::basefs::BaseFs;
use nh_hugofs::fs::Fs;
use nh_hugofs::paths::Paths;

use crate::processing_stats::ProcessingStats;

/// Go: `helpers.PathSpec` — per-language path/URL helpers + the base filesystems.
#[derive(Clone)]
pub struct PathSpec {
    pub paths: Arc<Paths>,
    pub base_fs: Arc<BaseFs>,
    pub processing_stats: Arc<ProcessingStats>,
    pub fs: Fs,
    pub cfg: Arc<dyn AllProvider>,
}

impl PathSpec {
    // Go: helpers/pathspec.go:NewPathSpec
    pub fn new(fs: Fs, cfg: Arc<dyn AllProvider>) -> Result<Arc<PathSpec>> {
        todo!()
    }

    /// Go: `NewPathSpecWithBaseBaseFsProvided` — sites share the first site's BaseFs.
    // Go: helpers/pathspec.go:NewPathSpecWithBaseBaseFsProvided
    pub fn new_with_base_fs(fs: Fs, cfg: Arc<dyn AllProvider>, base_fs: Option<Arc<BaseFs>>) -> Result<Arc<PathSpec>> {
        todo!()
    }

    /// Go: `PermalinkForBaseURL(link, baseURL)` = baseURL + TrimPrefix(link, "/").
    // Go: helpers/pathspec.go:PermalinkForBaseURL
    pub fn permalink_for_base_url(&self, link: &str, base_url: &str) -> String {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: helpers/pathspec.go (78 lines; 3/3 funcs executed)
//   types: PathSpec
// EX L41-43: NewPathSpec(fs *hugofs.Fs, cfg config.AllProvider, logger loggers.Logger) (*PathSpec, error)
// EX L47-73: NewPathSpecWithBaseBaseFsProvided(fs *hugofs.Fs, cfg config.AllProvider, logger loggers.Logger, baseBaseFs *filesystems.BaseFs) (*PathSpec, error)
// EX L76-78: (p *PathSpec) PermalinkForBaseURL(link, baseURL string) string
// ---------------------------------------------------------------------------
