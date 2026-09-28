//! Port of `hugolib/site_render.go`.
//!
//! Owner: Wave B task T24 (hugolib-build).

//! Go `hugolib/site_render.go`: `renderPages` (tree walk in key order; sequential), `pageRenderer`
//! (standalone filter, `renderResources`, resolve template, renderAndWritePage, renderPaginator),
//! `renderPaginator` (page/1 alias for HTML formats, then pagers 2..N with `current` advanced),
//! `renderAliases`, `renderMainLanguageRedirect` (`/en/index.html` -> baseURL).
//!
//! Go renders the pages of a pass with `GetNumWorkerMultiplier()` workers fed by the tree walk;
//! with more than one worker the publish order, and so the last writer of targets several pages
//! share (term collisions), is random. The port renders sequentially, in walk order, which is
//! Go's order with one worker (`HUGO_NUMWORKERMULTIPLIER=1`); the golden build equals that
//! single-worker output (HUGO_LAYER.md §7.1). A render error does not stop the pass: as in Go,
//! the errors are collected and the pass returns the one `pickOneAndLogTheRest` picks.

use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use go_value::Value;
use nh_common::Result;
use nh_common::herrors::{Error, improve_render_err};
use nh_common::kinds;
use nh_doctree::nodeshifttree::WalkConfig;
use nh_page::page::Page;
use nh_resource::resourcetypes::Resource;

use crate::alias::{publish_dest_alias, write_dest_alias};
use crate::hugo_sites::HugoSites;
use crate::hugo_sites_build::SiteRenderContext;
use crate::page::{PageId, handle_of};
use crate::site::Site;
use crate::template_exec::ExecKind;

/// Go: `(s *Site) render(ctx)` (site.go:1580-1613): FIRST `page.Clear()` (=
/// `nh_page::pages_cache::clear()`: drops the global sorted-pages cache before EVERY site/format
/// render), then aliases (first format of the first build), `renderPages`, and the main-language
/// redirect when the standalone-page context allows it.
// Go: hugolib/site.go:render
pub fn render_site(h: &Arc<HugoSites>, site_idx: usize, ctx: &SiteRenderContext) -> Result<()> {
    // Go: `page.Clear()` never fails.
    nh_page::page::clear();

    let s = &h.sites[site_idx];
    if ctx.out_idx == 0 && h.build_counter.load(Ordering::SeqCst) == 0 {
        // Note that even if disableAliases is set, the aliases themselves are
        // preserved on page. The motivation with this is to be able to generate
        // 301 redirects in a .htaccess file and similar using a custom output format.
        if !s.conf.root.disable_aliases {
            // Aliases must be rendered before pages.
            // Some sites, Hugo docs included, have faulty alias definitions that point
            // to itself or another real page. These will be overwritten in the next
            // step.
            render_aliases(h, site_idx)?;
        }
    }

    render_pages(h, site_idx, ctx)?;

    if !ctx.should_render_standalone_page("") {
        return Ok(());
    }

    render_main_language_redirect(h, site_idx)
}

/// The pages of site `site_idx` in `treePages` walk order (Go's `NodeShiftTreeWalker` over the
/// site's dimension; every visited node is a `*pageState`).
fn walk_pages(h: &HugoSites, site_idx: usize) -> Result<Vec<PageId>> {
    let cfg = WalkConfig {
        dims: h.sites[site_idx].page_map.dims,
        ..Default::default()
    };
    let mut ids = Vec::new();
    h.page_trees.tree_pages.walk(&cfg, |_w, _key, n, _match| {
        if let Some(id) = n.page_id() {
            ids.push(id);
        }
        Ok(false)
    })?;
    Ok(ids)
}

/// Go: `renderPages(ctx)` — the pages of the site that should render (`BuildCfg.shouldRender`),
/// in tree walk order, through [`page_renderer`]; the render errors are collated.
// Go: hugolib/site_render.go:renderPages
pub fn render_pages(h: &Arc<HugoSites>, site_idx: usize, ctx: &SiteRenderContext) -> Result<()> {
    let cfg = &ctx.cfg;

    let mut results: Vec<Error> = Vec::new();

    for id in walk_pages(h, site_idx)? {
        let p = h.page(id);
        if cfg.should_render(h, p) {
            // Go: the walk stops feeding the workers once a fatal error closed `Done()`.
            if h.fatal_error_handler.done() {
                break;
            }
            page_renderer(ctx, h, site_idx, id, &mut results);
        }
    }

    if let Some(err) = Site::error_collator(h, results) {
        return Err(improve_render_err(err).wrap("failed to render pages"));
    }
    Ok(())
}

/// Go: `pageRenderer(ctx, s, pages, results, wg)` — one page: skip standalone pages outside
/// their pass, publish the bundle resources, resolve the template (a missing one is logged
/// except for standalone pages), render and write the page (`sitemapindex` gets the sites as
/// data), then its paginator pages when the page's paginator was initialised.
// Go: hugolib/site_render.go:pageRenderer
pub fn page_renderer(
    ctx: &SiteRenderContext,
    h: &Arc<HugoSites>,
    site_idx: usize,
    id: PageId,
    results: &mut Vec<Error>,
) {
    let s = &h.sites[site_idx];
    let p = h.page(id);

    if p.meta.is_standalone() && !ctx.should_render_standalone_page(p.meta.kind()) {
        return;
    }

    if p.meta.page_config.build.publish_resources
        && let Err(err) = p.render_resources(h)
    {
        s.deps
            .send_error(p.errorf(Some(err), "failed to render page resources"));
        return;
    }

    let po = p.current_output().clone();
    if !po.render {
        // Nothing more to do for this page.
        return;
    }

    let templ = match p.resolve_template(h) {
        Err(err) => {
            s.deps
                .send_error(p.errorf(Some(err), "failed to resolve template"));
            return;
        }
        Ok(None) => {
            if s.deps.log.level() <= nh_common::loggers::Level::Trace {
                s.deps.log.trace(format!(
                    "no layout for kind {} found",
                    go_strconv::quote(p.meta.kind())
                ));
            }
            // Don't emit warning for missing 404 etc. pages.
            if !p.meta.is_standalone() {
                log_missing_layout(h, site_idx, "", p.meta.layout(), p.meta.kind(), &po.f.name);
            }
            return;
        }
        Ok(Some(t)) => t,
    };

    let target_path = po.target_paths.paths.target_filename.clone();

    if s.deps.log.level() <= nh_common::loggers::Level::Trace {
        s.deps.log.trace(format!(
            "rendering outputFormat {} kind {} using layout {} to {}",
            go_strconv::quote(&po.f.name),
            go_strconv::quote(p.meta.kind()),
            go_strconv::quote(templ.name()),
            go_strconv::quote(&target_path)
        ));
    }

    let d: Value = if p.meta.kind() == kinds::KIND_SITEMAP_INDEX {
        crate::site::hugolib_sites_value(h)
    } else {
        handle_of(h, id).page_ref().to_value()
    };

    if let Err(err) = Site::render_and_write_page_with(
        h,
        site_idx,
        Some(&s.deps.path_spec().processing_stats.pages),
        &target_path,
        id,
        &d,
        &templ,
        ExecKind::Page,
    ) {
        results.push(err);
    }

    if let Some(paginator) = &po.paginator
        && paginator.current().is_some()
        && let Err(err) = render_paginator(h, site_idx, id, &templ)
    {
        results.push(err);
    }
}

/// Go: `logMissingLayout(name, layout, kind, outputFormat)` — a WARN (INFO for the optional
/// `404` layout by name) naming what has no layout.
// Go: hugolib/site_render.go:logMissingLayout
pub fn log_missing_layout(
    h: &HugoSites,
    site_idx: usize,
    name: &str,
    layout: &str,
    kind: &str,
    output_format: &str,
) {
    let log = &h.sites[site_idx].deps.log;
    // Go: infoOnMissingLayout (site.go): the 404 layout is very much optional in Hugo, but
    // we do look for it.
    let info = !name.is_empty() && name == "404";

    let err_msg = "You should create a template file which matches Hugo Layouts Lookup Rules for this combination.";
    let mut msg = String::from("found no layout file for");
    if !output_format.is_empty() {
        msg.push(' ');
        msg.push_str(&go_strconv::quote(output_format));
    }

    if !layout.is_empty() {
        msg.push_str(" for layout ");
        msg.push_str(&go_strconv::quote(layout));
    }

    if !kind.is_empty() {
        msg.push_str(" for kind ");
        msg.push_str(&go_strconv::quote(kind));
    }

    if !name.is_empty() {
        msg.push_str(" for ");
        msg.push_str(&go_strconv::quote(name));
    }

    msg.push_str(": ");
    msg.push_str(err_msg);

    if info {
        log.infof(msg);
    } else {
        log.warnf(msg);
    }
}

/// Go: `renderPaginator(p, templ)` — must run after the owning page was rendered: the `page/1`
/// alias (HTML formats, unless `pagination.disableAliases`), then pagers 2..N, each rendered
/// with the paginator's `current` moved to it.
// Go: hugolib/site_render.go:renderPaginator
pub fn render_paginator(
    h: &Arc<HugoSites>,
    site_idx: usize,
    p: PageId,
    templ: &Arc<nh_tplimpl::templatestore::TemplInfo>,
) -> Result<()> {
    let s = &h.sites[site_idx];
    let pagination = s.deps.conf.pagination();
    let paginate_path = &pagination.path;

    let ps = h.page(p);
    let po = ps.current_output().clone();
    let Some(mut d) = ps.common.target_path_descriptor.get().cloned() else {
        // Go: a zero descriptor (a page without output formats never renders).
        return Err(Error::new(format!(
            "invalid paginator state for {}",
            go_strconv::quote(ps.path_or_title())
        )));
    };
    let f = ps.output_format().clone();
    d.type_ = f.clone();

    let paginator = po.paginator.as_ref();
    let current = paginator.and_then(|pg| pg.current());
    let Some(current) = current.filter(|c| Arc::ptr_eq(c, &c.first())) else {
        // Go panics here.
        return Err(Error::new(format!(
            "invalid paginator state for {}",
            go_strconv::quote(ps.path_or_title())
        )));
    };
    let paginator = paginator.expect("paginator");

    let handle = handle_of(h, p);
    if f.is_html && !pagination.disable_aliases {
        // Write alias for page 1
        d.addends = format!("/{paginate_path}/1");
        let target_paths = nh_page::page_paths::create_target_paths(&d);

        write_dest_alias(
            h,
            site_idx,
            &target_paths.target_filename,
            &handle.permalink(),
            &f,
            Some(handle.page_ref()),
        )?;
    }

    // Render pages for the rest
    let page_value = handle.page_ref().to_value();
    let mut next = current.next();
    while let Some(current) = next {
        paginator.set_current(Some(current.clone()));
        d.addends = format!("/{paginate_path}/{}", current.page_number());
        let target_paths = nh_page::page_paths::create_target_paths(&d);

        Site::render_and_write_page_with(
            h,
            site_idx,
            Some(&s.deps.path_spec().processing_stats.paginator_pages),
            &target_paths.target_filename,
            p,
            &page_value,
            templ,
            ExecKind::Pager(current.page_number() as u32),
        )?;

        next = current.next();
    }

    Ok(())
}

/// Go: `renderAliases()` — shell pages that redirect: for every rendered page with aliases, per
/// HTML output format (one per format `Path`), relative aliases resolved against the page's
/// directory, absolute ones below the format's path, `.html` added in ugly-URL sections, the
/// language prefixed on multihost sites.
// Go: hugolib/site_render.go:renderAliases
pub fn render_aliases(h: &Arc<HugoSites>, site_idx: usize) -> Result<()> {
    let s = &h.sites[site_idx];
    for id in walk_pages(h, site_idx)? {
        let p = h.page(id);

        // We cannot alias a page that's not rendered.
        if p.meta.no_link() || p.skip_render(h) {
            continue;
        }

        let handle = handle_of(h, id);
        let aliases = handle.aliases();
        if aliases.is_empty() {
            continue;
        }

        let mut path_seen: HashSet<String> = HashSet::new();
        for of in handle.output_formats().iter() {
            if !of.format.is_html {
                continue;
            }

            let f = &of.format;

            if path_seen.contains(&f.path) {
                continue;
            }
            path_seen.insert(f.path.clone());

            let plink = of.permalink().to_string();

            for a in &aliases {
                let is_relative = !a.starts_with('/');

                let mut a = if is_relative {
                    // Make alias relative, where "." will be on the
                    // same directory level as the current page.
                    let base_path = go_path::path::join(&[
                        p.current_output()
                            .target_paths
                            .paths
                            .sub_resource_base_link
                            .as_str(),
                        "..",
                    ]);
                    go_path::path::join(&[base_path.as_str(), a.as_str()])
                } else {
                    // Make sure AMP and similar doesn't clash with regular aliases.
                    go_path::path::join(&[f.path.as_str(), a.as_str()])
                };

                if (s.conf.compiled().is_ugly_url_section)(&handle.section())
                    && !a.ends_with(".html")
                {
                    a.push_str(".html");
                }

                let lang = handle.lang();

                if h.configs.is_multihost && !a.starts_with(&format!("/{lang}")) {
                    // These need to be in its language root.
                    a = go_path::path::join(&[lang.as_str(), a.as_str()]);
                }

                write_dest_alias(h, site_idx, &a, &plink, f, Some(handle.page_ref()))?;
            }
        }
    }
    Ok(())
}

/// Go: `renderMainLanguageRedirect()` — a redirect to the main language home, which lives in a
/// sub folder (`/en/` -> alias at the root) or not (alias at `/en/` -> the root).
// Go: hugolib/site_render.go:renderMainLanguageRedirect
pub fn render_main_language_redirect(h: &Arc<HugoSites>, site_idx: usize) -> Result<()> {
    let s = &h.sites[site_idx];
    if s.conf.root.disable_default_language_redirect {
        return Ok(());
    }
    // Go reads `s.h.Conf` (the first site's config).
    let hconf = &h.deps.conf;
    if hconf.is_multihost()
        || (!hconf.default_content_language_in_subdir() && !hconf.is_multilingual())
    {
        // No need for a redirect
        return Ok(());
    }

    let html = s
        .conf
        .output_formats
        .as_ref()
        .and_then(|n| n.config.get_by_name("html"));
    if let Some(html) = html {
        let main_lang = &s.conf.root.default_content_language;
        if s.conf.root.default_content_language_in_subdir {
            let main_lang_url = s.deps.path_spec().abs_url(&format!("{main_lang}/"), false);
            s.deps.log.debugf(format!(
                "Write redirect to main language {main_lang}: {main_lang_url}"
            ));
            publish_dest_alias(h, site_idx, true, "/", &main_lang_url, &html, None)?;
        } else {
            let main_lang_url = s.deps.path_spec().abs_url("", false);
            s.deps.log.debugf(format!(
                "Write redirect to main language {main_lang}: {main_lang_url}"
            ));
            publish_dest_alias(h, site_idx, true, main_lang, &main_lang_url, &html, None)?;
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/site_render.go (367 lines; 6/7 funcs executed)
//   types: siteRenderContext
// OK L54-67: (s siteRenderContext) shouldRenderStandalonePage(kind string) bool  [hugo_sites_build.rs]
// OK L70-120: (s *Site) renderPages(ctx *siteRenderContext) error
// OK L122-191: pageRenderer( ctx *siteRenderContext, s *Site, pages <-chan *pageState, results chan<- error, wg *sync.WaitGroup, )
// OK L193-225: (s *Site) logMissingLayout(name, layout, kind, outputFormat string)
// OK L228-267: (s *Site) renderPaginator(p *pageState, templ *tplimpl.TemplInfo) error
// OK L270-335: (s *Site) renderAliases() error
// OK L339-367: (s *Site) renderMainLanguageRedirect() error
// Source: hugolib/site.go (render only; the rest of site.go is in site.rs, hugo_sites.rs, build_assemble.rs, page__per_output.rs)
// OK L1580-1613: (s *Site) render(ctx *siteRenderContext) (err error)
// ---------------------------------------------------------------------------
