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

use std::collections::BTreeSet;

use go_value::Time;
use nh_common::Result;
use nh_common::kinds;
use nh_doctree::nodeshifttree::WalkConfig;
use nh_media::output::output_format::Formats;

use crate::content_map_page::SitePagesAssembler;
use crate::hugo_sites::HugoSites;
use crate::hugo_sites_build::BuildCfg;
use crate::site::Site;

/// Go: `HugoSites.assemble(ctx, l, bcfg)`.
///
/// A full build always needs the pages assembly (`bcfg.WhatChanged.needsPagesAssembly`); the
/// rebuild branch and the `resolveAndClearStateForIdentities` calls (server mode: the change
/// sets are always empty in a one-shot build) are not ported.
// Go: hugolib/hugo_sites_build.go:assemble
pub fn assemble(h: &mut HugoSites, cfg: &BuildCfg) -> Result<()> {
    let _ = cfg;
    h.translation_key_pages.clear();

    // Go runs step 1 of every site in parallel (h.workersSite); with
    // HUGO_NUMWORKERMULTIPLIER=1 (and in the port) they run in site order.
    for i in 0..h.sites.len() {
        SitePagesAssembler {
            h: &mut *h,
            site_idx: i,
        }
        .assemble_pages_step1()?;
    }

    for i in 0..h.sites.len() {
        SitePagesAssembler {
            h: &mut *h,
            site_idx: i,
        }
        .assemble_pages_step2()?;
    }

    h.render_formats = Formats::default();
    for i in 0..h.sites.len() {
        Site::init_render_formats(h, i)?;
        let f = h.sites[i].render_formats.0.clone();
        h.render_formats.0.extend(f);
    }

    for i in 0..h.sites.len() {
        SitePagesAssembler {
            h: &mut *h,
            site_idx: i,
        }
        .assemble_pages_step_final()?;
    }

    Ok(())
}

impl Site {
    /// Go: `initRenderFormats()` — union of the pages' formats and the per-kind formats, sorted
    /// with `sort.Sort(output.Formats)`.
    // Go: hugolib/site.go:initRenderFormats
    pub fn init_render_formats(h: &mut HugoSites, idx: usize) -> Result<()> {
        let mut format_set: BTreeSet<String> = BTreeSet::new();
        let mut formats = Formats::default();

        let cfg = WalkConfig {
            dims: h.sites[idx].page_map.dims,
            ..Default::default()
        };
        let pages = &h.pages;
        h.page_trees.tree_pages.walk(&cfg, |_w, _key, n, _match| {
            if let Some(id) = n.page_id() {
                let p = &pages[id.0 as usize];
                for f in &p.meta.page_config.configured_output_formats.0 {
                    if !format_set.contains(&f.name) {
                        formats.0.push(f.clone());
                        format_set.insert(f.name.clone());
                    }
                }
            }
            Ok(false)
        })?;

        // Add the per kind configured output formats
        let conf = h.sites[idx].conf.clone();
        for kind in kinds::ALL_KINDS_IN_PAGES {
            if let Some(site_formats) = conf.compiled().kind_output_formats.get(kind) {
                for f in &site_formats.0 {
                    if !format_set.contains(&f.name) {
                        formats.0.push(f.clone());
                        format_set.insert(f.name.clone());
                    }
                }
            }
        }

        formats.sort();
        h.sites[idx].render_formats = formats;
        Ok(())
    }

    /// Go: `(s *Site) shouldBuild(p)` — drafts/future/expired against `htime.Now()` (the
    /// `--clock` time). Called only by the assembly walks (content_map_page.go:1582, 1903).
    // Go: hugolib/site.go:shouldBuild
    pub fn should_build(&self, p: &crate::page::PageState) -> bool {
        if !self.conf.is_kind_enabled(p.meta.kind()) {
            return false;
        }
        let c = &self.deps.conf;
        should_build(
            c.build_future(),
            c.build_expired(),
            c.build_drafts(),
            p.meta.draft(),
            &p.meta.publish_date(),
            &p.meta.expiry_date(),
        )
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
    if !build_drafts && draft {
        return false;
    }
    let hnow: Time = nh_common::htime::now();
    if !build_future && !publish_date.is_zero() && publish_date.after(&hnow) {
        return false;
    }
    if !build_expired && !expiry_date.is_zero() && expiry_date.before(&hnow) {
        return false;
    }
    true
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/hugo_sites_build.go (assemble only)
// OK L274-348: (h *HugoSites) assemble(ctx context.Context, l logg.LevelLogger, bcfg *BuildCfg) error
// Source: hugolib/site.go (initRenderFormats, shouldBuild only)
// OK L800-837: (s *Site) initRenderFormats()
// OK L1556-1562: (s *Site) shouldBuild(p page.Page) bool
// OK L1564-1578: shouldBuild(buildFuture bool, buildExpired bool, buildDrafts bool, Draft bool, publishDate time.Time, expiryDate time.Time, ) bool
// ---------------------------------------------------------------------------
