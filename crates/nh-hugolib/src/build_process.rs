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
use crate::pages_capture::new_pages_collector;

/// Go: `HugoSites.process(ctx, l, config, init, events...)` — a full build always takes
/// `processFull` (file events and remote changes are rebuilds: not ported).
// Go: hugolib/hugo_sites_build.go:process
pub fn process(h: &mut HugoSites, cfg: &BuildCfg) -> Result<()> {
    process_full(h, cfg)
}

/// Go: `processFull`.
// Go: hugolib/hugo_sites_build.go:processFull
pub fn process_full(h: &mut HugoSites, cfg: &BuildCfg) -> Result<()> {
    process_files(h, cfg).map_err(|err| err.wrap("readAndProcessContent"))
}

/// Go: `processFiles` — `source.NewSourceSpec(s.PathSpec, buildConfig.ContentInclusionFilter,
/// s.Content.Fs)` (only its content fs is used), then `newPagesCollector(...).Collect()` with
/// `h.Sites[0].pageMap` for the inserts (any page map would do). `ContentInclusionFilter` is a
/// server feature and always nil here.
// Go: hugolib/hugo_sites_build.go:processFiles
pub fn process_files(h: &mut HugoSites, cfg: &BuildCfg) -> Result<()> {
    let content_fs = h
        .deps
        .path_spec()
        .base_fs
        .source_filesystems
        .content
        .fs
        .clone();
    let logger = h.deps.log.clone();

    // For inserts, we can pick an arbitrary pageMap.
    let mut c = new_pages_collector(h, content_fs, logger, 0, cfg.clone());

    c.collect()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/hugo_sites_build.go (process part; the rest is in hugo_sites_build.rs)
// OK L255-270: (h *HugoSites) process(ctx context.Context, l logg.LevelLogger, config *BuildCfg, init func(config *BuildCfg) error, events ...fsnotify.Event) error
// OK L1193-1199: (h *HugoSites) processFull(ctx context.Context, l logg.LevelLogger, config *BuildCfg) (err error)
// OK L1243-1260: (s *HugoSites) processFiles(ctx context.Context, l logg.LevelLogger, buildConfig *BuildCfg, filenames ...pathChange) error
// ---------------------------------------------------------------------------
