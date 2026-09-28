//! Port of `hugolib/hugo_sites_build.go` (`assemble`) + `hugolib/site.go` (`initRenderFormats`,
//! `shouldBuild`).
//!
//! Owner: Wave B task T21 (hugolib-assemble).

//! Split from `hugo_sites_build.go`/`site.go` so that the assembly task (T21) owns the whole
//! second build phase and its acceptance test (the model after `assemble`) needs no later task.
//!
//! `assemble` (hugo_sites_build.go:274-348): for each site `assemblePagesStep1` (Go runs these in
//! parallel, the result is order-independent; run them in site order), then for each site
//! `assemblePagesStep2`, then `h.render_formats` = concatenation of every site's
//! `initRenderFormats()`, then for each site `assemblePagesStepFinal` (which calls
//! `shift_to_output_format(true, 0)` on every page: see page__init.rs, same task).

use nh_common::Result;

use crate::hugo_sites::HugoSites;
use crate::hugo_sites_build::BuildCfg;
use crate::site::Site;

/// Go: `HugoSites.assemble(ctx, l, bcfg)`.
// Go: hugolib/hugo_sites_build.go:assemble
pub fn assemble(h: &mut HugoSites, cfg: &BuildCfg) -> Result<()> {
    todo!()
}

impl Site {
    /// Go: `initRenderFormats()` — union of the pages' formats and the per-kind formats, sorted
    /// with `sort.Sort(output.Formats)`.
    // Go: hugolib/site.go:initRenderFormats
    pub fn init_render_formats(h: &mut HugoSites, idx: usize) -> Result<()> {
        todo!()
    }

    /// Go: `(s *Site) shouldBuild(p)` — drafts/future/expired against `htime.Now()` (the
    /// `--clock` time). Called only by the assembly walks (content_map_page.go:1582, 1903).
    // Go: hugolib/site.go:shouldBuild
    pub fn should_build(&self, p: &crate::page::PageState) -> bool {
        todo!()
    }
}

/// Go: `shouldBuild(buildFuture, buildExpired, buildDrafts, Draft, publishDate, expiryDate)`.
// Go: hugolib/site.go:shouldBuild
pub fn should_build(
    build_future: bool,
    build_expired: bool,
    build_drafts: bool,
    draft: bool,
    publish_date: &go_value::Time,
    expiry_date: &go_value::Time,
) -> bool {
    todo!()
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/hugo_sites_build.go (assemble only)
// EX L274-348: (h *HugoSites) assemble(ctx context.Context, l logg.LevelLogger, bcfg *BuildCfg) error
// Source: hugolib/site.go (initRenderFormats, shouldBuild only)
// EX L800-837: (s *Site) initRenderFormats()
// EX L1556-1562: (s *Site) shouldBuild(p page.Page) bool
// EX L1564-1578: shouldBuild(buildFuture bool, buildExpired bool, buildDrafts bool, Draft bool, publishDate time.Time, expiryDate time.Time, ) bool
// ---------------------------------------------------------------------------
