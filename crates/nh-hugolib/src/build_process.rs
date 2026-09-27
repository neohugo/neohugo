//! Port of `hugolib/hugo_sites_build.go` (the `process` step only).
//!
//! Owner: Wave B task T20 (hugolib-capture).


//! Split from `hugo_sites_build.go` so that the capture task (T20) owns the whole first build
//! phase and its acceptance test (tree dumps after `process`) needs no later task:
//! `process` -> `processFull` -> `processFiles` -> `pagesCollector.Collect` (pages_capture.rs).
//! Rebuild paths (`processPartial*`, content adapters) are not ported (one-shot build).

use nh_common::Result;

use crate::hugo_sites::HugoSites;
use crate::hugo_sites_build::BuildCfg;

/// Go: `HugoSites.process(ctx, l, config, init, events...)` — a full build always takes
/// `processFull`.
// Go: hugolib/hugo_sites_build.go:process
pub fn process(h: &mut HugoSites, cfg: &BuildCfg) -> Result<()> {
    todo!()
}

/// Go: `processFull` -> `processFiles` (a SourceSpec over the content fs, then
/// `newPagesCollector(...).Collect()` with `h.sites[0]`'s page map for inserts).
// Go: hugolib/hugo_sites_build.go:processFiles
pub fn process_files(h: &mut HugoSites, cfg: &BuildCfg) -> Result<()> {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/hugo_sites_build.go (process part; the rest is in hugo_sites_build.rs)
// EX L255-270: (h *HugoSites) process(ctx context.Context, l logg.LevelLogger, config *BuildCfg, init func(config *BuildCfg) error, events ...fsnotify.Event) error
// EX L1193-1199: (h *HugoSites) processFull(ctx context.Context, l logg.LevelLogger, config *BuildCfg) (err error)
// EX L1243-1260: (s *HugoSites) processFiles(ctx context.Context, l logg.LevelLogger, buildConfig *BuildCfg, filenames ...pathChange) error
// ---------------------------------------------------------------------------
