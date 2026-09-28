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
///
/// Go embeds `*paths.Paths` and `*filesystems.BaseFs`; the port derefs to [`Paths`] and keeps
/// the base filesystems in `base_fs`.
#[derive(Clone)]
pub struct PathSpec {
    pub paths: Arc<Paths>,
    pub base_fs: Arc<BaseFs>,
    pub processing_stats: Arc<ProcessingStats>,
    pub fs: Fs,
    pub cfg: Arc<dyn AllProvider>,
}

impl std::ops::Deref for PathSpec {
    type Target = Paths;
    fn deref(&self) -> &Paths {
        &self.paths
    }
}

impl PathSpec {
    /// Go: `NewPathSpec(fs, cfg, logger)`.
    // Go: helpers/pathspec.go:NewPathSpec
    pub fn new(fs: Fs, cfg: Arc<dyn AllProvider>) -> Result<Arc<PathSpec>> {
        Self::new_with_base_fs(fs, cfg, None)
    }

    /// Go: `NewPathSpecWithBaseBaseFsProvided` — sites share the first site's BaseFs.
    // Go: helpers/pathspec.go:NewPathSpecWithBaseBaseFsProvided
    pub fn new_with_base_fs(
        fs: Fs,
        cfg: Arc<dyn AllProvider>,
        base_fs: Option<Arc<BaseFs>>,
    ) -> Result<Arc<PathSpec>> {
        let p = Paths::new(fs.clone(), cfg.clone())?;

        let bfs = BaseFs::new_with_base(&p, None, base_fs.as_deref())?;

        let name = p.lang();
        Ok(Arc::new(PathSpec {
            paths: Arc::new(p),
            base_fs: bfs,
            fs,
            cfg,
            processing_stats: Arc::new(ProcessingStats::new(&name)),
        }))
    }

    /// Go: `PermalinkForBaseURL(link, baseURL)` = baseURL + TrimPrefix(link, "/").
    // Go: helpers/pathspec.go:PermalinkForBaseURL
    pub fn permalink_for_base_url(&self, link: &str, base_url: &str) -> String {
        format!("{base_url}{}", link.strip_prefix('/').unwrap_or(link))
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: helpers/pathspec.go (78 lines; 3/3 funcs executed)
//   types: PathSpec
// OK L41-43: NewPathSpec(fs *hugofs.Fs, cfg config.AllProvider, logger loggers.Logger) (*PathSpec, error)
// OK L47-73: NewPathSpecWithBaseBaseFsProvided(fs *hugofs.Fs, cfg config.AllProvider, logger loggers.Logger, baseBaseFs *filesystems.BaseFs) (*PathSpec, error)
// OK L76-78: (p *PathSpec) PermalinkForBaseURL(link, baseURL string) string
// ---------------------------------------------------------------------------
