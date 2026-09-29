//! Port of `hugolib/hugo_sites_build.go`.
//!
//! Owner: Wave B task T24 (hugolib-build).

//! Go `hugolib/hugo_sites_build.go`: `Build(cfg)` = lock -> process -> assemble -> (freeze) ->
//! render -> writeBuildStats -> renderDeferred -> postProcess -> error check.
//!
//! RENDER ORDER (reproduces Go with HUGO_NUMWORKERMULTIPLIER=1, the canonical golden):
//! ```text
//! i := 0
//! for s in sites (en, th):                       h.current_site = s
//!   for (siteOutIdx, f) in s.render_formats:     (html, 404, json, robots, rss, sitemap, sitemapindex)
//!     sitesOutIdx = i; i += 1
//!     for s2 in sites: s2.prepare_pages_for_render(s == s2, sitesOutIdx)   // shiftToOutputFormat
//!     s.render(ctx):  aliases (first format, first build) -> pages in tree-key order (last writer
//!                     wins on duplicate targets) -> page/1 alias + pagers -> main-language redirect
//! ```
//! Then `hugo_stats.json` (merged + sorted collectors, JSON indent "  ", no HTML escape) is written
//! to the WORKING DIR, and only then postProcess replaces `__h_pp_l1_<id>_<Field>__e=` placeholders
//! in the recorded files (this is where toCSS | postCSS (purgecss reads hugo_stats.json) | minify |
//! fingerprint runs).
//!
//! Entry points: [`build`] (Go `HugoSites.Build`: the whole build of a mutable `HugoSites`,
//! returning the frozen sites), or the phases one by one: `process`, `assemble`,
//! `HugoSites::freeze`, [`render`], [`write_build_stats`], [`render_deferred`], [`post_process`].

use std::sync::Arc;
use std::sync::atomic::Ordering;

use go_json::{JsonField, JsonStruct};
use go_value::{SliceType, Value};
use nh_common::Result;
use nh_common::herrors::{Error, is_not_exist};
use nh_common::kinds;
use nh_common::loggers::Level;
use nh_hugofs::afero::{Fs, read_file, write_file};
use nh_publisher::html_elements_collector::HtmlElements;
use nh_publisher::publisher::{PublishStats, Publisher};
use nh_resources::postpub::postpub::{
    POST_PROCESS_PREFIX, POST_PROCESS_SUFFIX, PostPublishResource,
};

use crate::hugo_sites::HugoSites;
use crate::page::PageState;
use crate::site_render::render_site;

/// Go: `hugolib.BuildCfg` — build options. The fields for rebuilds and the server
/// (`WhatChanged`, `PartialReRender`, `ErrRecovery`, `RecentlyTouched`,
/// `ContentInclusionFilter`) are not ported: every build is a full, first build.
#[derive(Clone, Debug, Default)]
pub struct BuildCfg {
    /// Skip rendering. Useful for testing.
    pub skip_render: bool,
    /// Set when the build lock is already acquired.
    pub no_build_lock: bool,
}

impl BuildCfg {
    /// Go: `shouldRender(infol, p)` — whether the page's CURRENT output renders in this pass:
    /// not excluded by the segment filter, and rendered at most once per output unless reset
    /// (Go's fast render mode of the server re-renders recently touched URLs; not ported).
    // Go: hugolib/hugo_sites.go:shouldRender
    pub fn should_render(&self, h: &HugoSites, p: &PageState) -> bool {
        if p.skip_render(h) {
            return false;
        }

        let po = p.current_output();
        if !po.render_once.load(Ordering::SeqCst) {
            return true;
        }

        // The render state is incremented on render and reset when a related change is detected.
        // Note that this is set per output format.
        let should_render = po.render_state.load(Ordering::SeqCst) == 0;

        if !should_render {
            return false;
        }

        let fast_render_mode = h.sites[p.site_idx].deps.conf.fast_render_mode();

        if !fast_render_mode || h.build_counter.load(Ordering::SeqCst) == 0 {
            return should_render;
        }

        if !po.render {
            // Not be to rendered for this output format.
            return false;
        }

        // Go: the recently touched URLs of the server (`RecentlyTouched`) are not ported; a
        // rebuild never happens in a one-shot build.
        false
    }
}

/// Go: `siteRenderContext`.
#[derive(Clone, Debug, Default)]
pub struct SiteRenderContext {
    pub cfg: BuildCfg,
    pub language_idx: usize,
    /// Zero based index for all output formats combined.
    pub sites_out_idx: usize,
    /// Zero based index of the output format for the current site.
    pub out_idx: usize,
    pub multihost: bool,
}

impl SiteRenderContext {
    /// Go: `shouldRenderStandalonePage(kind)` — 404: once per site (outIdx 0); robots and
    /// sitemapindex: languageIdx 0 && outIdx 0; sitemap: outIdx 0.
    // Go: hugolib/site_render.go:shouldRenderStandalonePage
    pub fn should_render_standalone_page(&self, kind: &str) -> bool {
        if self.multihost || kind == kinds::KIND_SITEMAP {
            // 1 per site
            return self.out_idx == 0;
        }

        if kind == kinds::KIND_TEMPORARY || kind == kinds::KIND_STATUS_404 {
            // 1 for all output formats
            return self.out_idx == 0;
        }

        // 1 for all sites and output formats.
        self.language_idx == 0 && self.out_idx == 0
    }
}

/// Go: `HugoSites.Build(config)`: the whole build. Consumes the mutable HugoSites (process +
/// assemble), freezes it, renders, post-processes; returns the frozen sites.
///
/// Errors: like Go, the errors of the phases after assembly are sent to the error collector
/// (`render: ...`, `postProcess: ...`) and the build returns the one
/// `pickOneAndLogTheRest` picks; a `hugo_stats.json` write error returns at once; an ERROR log
/// line fails the build with `logged N error(s)`.
// Go: hugolib/hugo_sites_build.go:Build
pub fn build(mut h: HugoSites, cfg: BuildCfg) -> Result<Arc<HugoSites>> {
    let unlock = if !cfg.no_build_lock {
        Some(
            h.deps
                .path_spec()
                .base_fs
                .lock_build()
                .map_err(|err| err.wrap("failed to acquire a build lock"))?,
        )
    } else {
        None
    };

    h.deps.global_err_handler.start_error_collector();

    // Go: `s.state = siteStateInit`, `h.Metrics.Reset()`, `h.buildCounters`: no state in the
    // port (metrics and counters are not ported).

    // Go: `prepare()`: `init` (the build start listeners, `initSites` = `h.reset`: nothing to
    // reset in a first build), `process`, `assemble`.
    let mut prepare_err: Option<Error> = None;
    for s in &h.sites {
        s.deps.build_start_listeners.notify(&[]);
    }
    if let Err(err) = process(&mut h, &cfg) {
        prepare_err = Some(err.wrap("process"));
    } else if let Err(err) = assemble(&mut h, &cfg) {
        prepare_err = Some(err.wrap("assemble"));
    }
    if let Some(err) = &prepare_err {
        h.deps.send_error(err.clone());
    }

    // `s.state = siteStateReady`: the frozen sites.
    let h = h.freeze();

    let res = build_rest(&h, &cfg, prepare_err.is_none());

    // Go's deferred calls: the build end listeners, the build counter, the lock.
    let finish = |h: &Arc<HugoSites>| {
        for s in &h.sites {
            s.deps.build_end_listeners.notify(&[]);
        }
        h.build_counter.fetch_add(1, Ordering::SeqCst);
    };

    let errors = h.deps.global_err_handler.stop_error_collector();
    finish(&h);
    if let Some(unlock) = unlock {
        unlock();
    }
    res?;

    if let Some(err) = h.pick_one_and_log_the_rest(errors) {
        return Err(err);
    }

    if let Some(err) = h.fatal_error_handler.get_err() {
        return Err(err);
    }

    let error_count =
        h.deps.log.logg_count(Level::Error) + nh_common::loggers::log().logg_count(Level::Error);
    if error_count > 0 {
        return Err(Error::new(format!("logged {error_count} error(s)")));
    }

    Ok(h)
}

/// The part of `Build` after `prepare`: render, build stats, the path warnings, the deferred
/// templates, the unused templates and post processing. Only a `writeBuildStats` error returns
/// directly (as in Go); the others go to the error collector.
fn build_rest(h: &Arc<HugoSites>, cfg: &BuildCfg, prepared: bool) -> Result<()> {
    if !prepared {
        return Ok(());
    }

    if let Err(err) = render(h, cfg) {
        h.deps.send_error(err.wrap("render"));
    }

    // Make sure to write any build stats to disk first so it's available
    // to the post processors.
    write_build_stats(h)?;

    // We need to do this before render deferred.
    if let Err(err) = print_path_warnings_once(h) {
        h.deps.send_error(err.wrap("printPathWarnings"));
    }

    if let Err(err) = render_deferred(h) {
        h.deps.send_error(err.wrap("renderDeferred"));
    }

    // This needs to be done after the deferred rendering to get complete template usage coverage.
    if let Err(err) = print_unused_templates_once(h) {
        // Go's message says printPathWarnings here too.
        h.deps.send_error(err.wrap("printPathWarnings"));
    }

    if let Err(err) = post_process(h) {
        h.deps.send_error(err.wrap("postProcess"));
    }

    Ok(())
}

// `process` (T20) is in build_process.rs; `assemble` (T21) is in build_assemble.rs.
pub use crate::build_assemble::assemble;
pub use crate::build_process::process;

/// Go: `render(l, config)` — every site in language order, every render format of the site
/// (the segment filter may skip either): all sites' pages are shifted to the format
/// (`preparePagesForRender`), then the site renders (`Site.render`).
// Go: hugolib/hugo_sites_build.go:render
pub fn render(h: &Arc<HugoSites>, cfg: &BuildCfg) -> Result<()> {
    let mut site_render_context = SiteRenderContext {
        cfg: cfg.clone(),
        multihost: h.configs.is_multihost,
        ..Default::default()
    };

    let render_err = |err: Error| -> Error {
        // In Hugo 0.141.0 we replaced the special error handling for resources.GetRemote
        // with the more general try.
        let s = err.to_string();
        if s.contains("can't evaluate field Err in type") {
            if s.contains("resource.Resource") {
                return Error::new(format!(
                    "{s}: Resource.Err was removed in Hugo v0.141.0 and replaced with a new try keyword, see https://gohugo.io/functions/go-template/try/"
                ));
            } else if s.contains("template.HTML") {
                return Error::new(format!(
                    "{s}: the return type of transform.ToMath was changed in Hugo v0.141.0 and the error handling replaced with a new try keyword, see https://gohugo.io/functions/go-template/try/"
                ));
            }
        }
        err
    };

    let log = &h.deps.log;
    let mut i = 0;
    for (si, s) in h.sites.iter().enumerate() {
        let segment_filter = &s.conf.compiled().segment_filter;
        if segment_filter.should_exclude_coarse(&nh_allconfig::segments::SegmentMatcherFields {
            lang: s.language.lang.clone(),
            ..Default::default()
        }) {
            log.infof(format!(
                "skip language {} not matching segments set in --renderSegments",
                go_strconv::quote(&s.language.lang)
            ));
            continue;
        }

        site_render_context.language_idx = s.idx;
        h.current_site.store(si, Ordering::SeqCst);
        for (site_out_idx, render_format) in s.render_formats.0.iter().enumerate() {
            if segment_filter.should_exclude_coarse(&nh_allconfig::segments::SegmentMatcherFields {
                output: render_format.name.clone(),
                lang: s.language.lang.clone(),
                ..Default::default()
            }) {
                log.infof(format!(
                    "skip output format {} for language {} not matching segments set in --renderSegments",
                    go_strconv::quote(&render_format.name),
                    go_strconv::quote(&s.language.lang)
                ));
                continue;
            }

            // Go: `h.BuildState.StartStageRender(rc)` / `defer StopStageRender(rc)` group the
            // deferred executions of this pass; see `render_deferred`.
            let rc = nh_deps::deps::RenderingContext {
                site_idx: si,
                site_out_idx,
            };
            h.deps.build_state.start_stage_render(rc);
            let res = (|| -> Result<()> {
                site_render_context.out_idx = site_out_idx;
                site_render_context.sites_out_idx = i;
                i += 1;

                // Go: `case <-h.Done(): return nil`.
                if h.fatal_error_handler.done() {
                    return Ok(());
                }
                for s2 in 0..h.sites.len() {
                    h.prepare_pages_for_render(s2, si == s2, site_render_context.sites_out_idx)?;
                }
                if !cfg.skip_render {
                    // Go: `PartialReRender` (renderPages only) is a server feature (not ported).
                    render_site(h, si, &site_render_context).map_err(render_err)?;
                }
                Ok(())
            })();
            h.deps.build_state.stop_stage_render(rc);
            res?;
        }
    }

    Ok(())
}

/// Go: `renderDeferred(l)` — executes the `templates.Defer` blocks whose placeholders
/// (`__hdeferred/`) were published, per rendering pass (Go ranges over a map of the passes, so
/// its order is random; the port uses the render order). Before a pass's files are handled,
/// every site's pages are shifted to the pass's output format — with the pass's SITE-LOCAL
/// output index, exactly as Go calls `preparePagesForRender(s == s2, rc.SiteOutIdx)`.
// Go: hugolib/hugo_sites_build.go:renderDeferred
pub fn render_deferred(h: &Arc<HugoSites>) -> Result<()> {
    for (rc, de) in h.deps.build_state.deferred_executions_grouped() {
        if de.filenames().is_empty() {
            continue;
        }
        for s2 in 0..h.sites.len() {
            h.prepare_pages_for_render(s2, rc.site_idx == s2, rc.site_out_idx)?;
        }
        execute_deferred_templates(h, rc.site_idx, &de)
            .map_err(nh_common::herrors::improve_render_err)?;
    }
    Ok(())
}

/// Go: `(s *Site) executeDeferredTemplates(de)` — for every recorded file (Go: `numWorkers`
/// workers over an unordered map; the port handles the files one by one in sorted order, each
/// file on its own), replaces each `__hdeferred/…__d=` placeholder with the result of its
/// deferred execution (executed once, with the context and data recorded by `DoDefer`), and
/// writes the file back when something changed. Go's panics (unknown id or template) are errors.
// Go: hugolib/hugo_sites_build.go:executeDeferredTemplates
fn execute_deferred_templates(
    h: &Arc<HugoSites>,
    site_idx: usize,
    de: &nh_deps::deps::DeferredExecutions,
) -> Result<()> {
    let s = &h.sites[site_idx];
    let publish_fs = s.deps.path_spec().base_fs.publish_fs.clone();
    let prefix = nh_tpl::template::HUGO_DEFERRED_TEMPLATE_PREFIX.as_bytes();
    let suffix = nh_tpl::template::HUGO_DEFERRED_TEMPLATE_SUFFIX.as_bytes();

    let handle_file = |filename: &str| -> Result<()> {
        let mut content = read_file(publish_fs.as_ref(), filename)?;

        let mut k: usize = 0;
        let mut changed = false;

        while k < content.len() {
            let Some(l) = bytes_index(&content[k..], prefix) else {
                break;
            };
            // Go: `bytes.Index(...) + len(suffix)` (len(suffix)-1 when not found).
            let m = match bytes_index(&content[k + l..], suffix) {
                Some(i) => i + suffix.len(),
                None => suffix.len() - 1,
            };

            let (low, high) = (k + l, k + l + m);
            if high > content.len() {
                return Err(Error::new(format!(
                    "runtime error: slice bounds out of range [:{high}] with capacity {}",
                    content.len()
                )));
            }

            let id = String::from_utf8_lossy(&content[low..high]).into_owned();

            let Some(deferred) = de.get(&id) else {
                return Err(Error::new(format!(
                    "deferred execution with id {} not found",
                    go_strconv::quote(&id)
                )));
            };
            let result = {
                let mut state = deferred.result.lock().unwrap_or_else(|e| e.into_inner());
                if state.is_none() {
                    let store = s
                        .template_store
                        .get()
                        .ok_or_else(|| Error::new("template store not initialised".to_string()))?;
                    let Some(ti) = store.lookup_by_path(&deferred.template_path) else {
                        return Err(Error::new(format!(
                            "template {} not found",
                            go_strconv::quote(&deferred.template_path)
                        )));
                    };
                    let mut buf = Vec::new();
                    store.execute_with_context(&deferred.ctx, &ti, &mut buf, &deferred.data)?;
                    *state = Some(buf);
                }
                state.clone().unwrap_or_default()
            };

            let mut next = Vec::with_capacity(content.len() + result.len());
            next.extend_from_slice(&content[..low]);
            next.extend_from_slice(&result);
            next.extend_from_slice(&content[high..]);
            content = next;
            changed = true;

            k += result.len();
        }

        if changed {
            return write_file(publish_fs.as_ref(), filename, &content, 0o666);
        }

        Ok(())
    };

    for filename in de.filenames() {
        handle_file(&filename)?;
    }
    Ok(())
}

/// Go: `printPathWarningsOnce()` — with `printPathWarnings`, the publish fs layers that count
/// duplicate target paths report them. Only the counting fs the Go commands install for that
/// flag reports duplicates; no publish fs of the port does, so there is nothing to print.
// Go: hugolib/hugo_sites_build.go:printPathWarningsOnce
pub fn print_path_warnings_once(h: &Arc<HugoSites>) -> Result<()> {
    let _ = h.configs.base.root.print_path_warnings;
    Ok(())
}

/// Go: `printUnusedTemplatesOnce()` — with `printUnusedTemplates`, a WARN per template never
/// executed.
// Go: hugolib/hugo_sites_build.go:printUnusedTemplatesOnce
pub fn print_unused_templates_once(h: &Arc<HugoSites>) -> Result<()> {
    if h.configs.base.root.print_unused_templates {
        let unused_templates = h.deps.get_template_store().unused_templates();
        for unused_template in unused_templates {
            let path = unused_template
                .path_info
                .as_ref()
                .map(|p| p.path())
                .unwrap_or_default();
            match &unused_template.fi {
                Some(fi) => h.deps.log.warnf(format!(
                    "Template {path} is unused, source {}",
                    go_strconv::quote(&fi.meta().filename)
                )),
                None => h.deps.log.warnf(format!("Template {path} is unused")),
            }
        }
    }
    Ok(())
}

/// Go: `writeBuildStats()` -> `<workingDir>/hugo_stats.json` (skip write if bytes equal).
// Go: hugolib/hugo_sites_build.go:writeBuildStats
pub fn write_build_stats(h: &Arc<HugoSites>) -> Result<()> {
    if !h.deps.resource_spec().build_config().build_stats.enabled() {
        return Ok(());
    }

    let stats: Vec<PublishStats> = h
        .sites
        .iter()
        .map(|s| s.publisher.publish_stats())
        .collect();
    let js = build_stats_json(&stats)?;

    let filename = go_path::filepath::join(&[
        h.configs.loading_info.base_config.working_dir.as_str(),
        nh_common::files::FILENAME_HUGO_STATS_JSON,
    ]);

    let os = nh_hugofs::fs::os();
    if let Ok(existing_content) = read_file(os.as_ref(), &filename) {
        // Check if the content has changed.
        if existing_content == js {
            return Ok(());
        }
    }

    // Make sure it's always written to the OS fs.
    write_file(os.as_ref(), &filename, &js, 0o666)?;

    // Write to the destination as well if it's a in-memory fs.
    let fs = h.deps.fs();
    if !nh_hugofs::fs::is_os_fs(fs.source.as_ref()) {
        write_file(fs.working_dir_writable.as_ref(), &filename, &js, 0o666)?;
    }

    // Go: `dynacacheGCFilenameIfNotWatchedAndDrainMatching(filename)` drops cache entries of
    // resources that depend on hugo_stats.json (a rebuild concern: nothing depends on the file
    // before postProcess in a first build).

    Ok(())
}

/// The bytes of `hugo_stats.json` for the sites' publish stats: the HTML elements merged in
/// site order (append + unique), sorted, encoded like Go's `json.Encoder` with
/// `SetEscapeHTML(false)` and `SetIndent("", "  ")` (trailing newline; nil lists are `null`).
// Go: hugolib/hugo_sites_build.go:writeBuildStats
pub fn build_stats_json(stats: &[PublishStats]) -> Result<Vec<u8>> {
    let mut html_elements = HtmlElements::default();
    for s in stats {
        html_elements.merge(&s.html_elements);
    }

    html_elements.sort();

    fn list(v: &Option<Vec<String>>) -> Value {
        match v {
            Some(v) => Value::list(
                SliceType::String,
                v.iter().map(|s| Value::string(s.as_str())).collect(),
            ),
            None => Value::TypedNil(Arc::from("[]string")),
        }
    }
    let elements = JsonStruct::new(
        "publisher.HTMLElements",
        vec![
            JsonField::new("tags", list(&html_elements.tags)),
            JsonField::new("classes", list(&html_elements.classes)),
            JsonField::new("ids", list(&html_elements.ids)),
        ],
    );
    let stats = JsonStruct::new(
        "publisher.PublishStats",
        vec![JsonField::new("htmlElements", Value::object(elements))],
    );

    let mut enc = go_json::Encoder::new(Vec::new());
    enc.set_escape_html(false);
    enc.set_indent("", "  ");
    enc.encode(&Value::object(stats))
        .map_err(|err| Error::new(err.to_string()))?;
    Ok(enc.into_inner())
}

/// Go: `postProcess(l)` — jsconfig.json (if any roots) + placeholder replacement in
/// `BuildState.GetFilenamesWithPostPrefix()` files (sorted); rewrite a file only if changed.
// Go: hugolib/hugo_sites_build.go:postProcess
pub fn post_process(h: &Arc<HugoSites>) -> Result<()> {
    let rs = h.deps.resource_spec();

    // This will only be set when js.Build have been triggered with
    // imports that resolves to the project or a module.
    // Write a jsconfig.json file to the project's /asset directory
    // to help JS IntelliSense in VS Code etc.
    if !rs.build_config().no_js_config_in_assets {
        let handle_js_config = |fi: &nh_hugofs::fileinfo::FileMetaInfo| {
            let m = fi.meta();
            if !m.is_project {
                return;
            }

            if let Some(b) = rs
                .common
                .post_build_assets
                .js_config_builder
                .build(&m.source_root)
            {
                let filename = go_path::filepath::join(&[m.source_root.as_str(), "jsconfig.json"]);
                // Go: in server mode the file is added to skipRebuildForFilenames.
                // Make sure it's  written to the OS fs as this is used by
                // editors.
                if let Err(err) = write_file(nh_hugofs::fs::os().as_ref(), &filename, &b, 0o666) {
                    h.deps
                        .log
                        .warnf(format!("Failed to write jsconfig.json: {err}"));
                }
            }
        };

        match h
            .deps
            .path_spec()
            .base_fs
            .source_filesystems
            .assets
            .fs
            .stat("")
        {
            Err(err) => {
                if !is_not_exist(&err) {
                    h.deps
                        .log
                        .warnf(format!("Failed to resolve jsconfig.json dir: {err}"));
                }
            }
            Ok(fi) => handle_js_config(&fi),
        }
    }

    let to_post_process: Vec<Arc<PostPublishResource>> = rs
        .common
        .post_build_assets
        .post_process_resources
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .values()
        .cloned()
        .collect();

    if to_post_process.is_empty() {
        // Nothing more to do.
        return Ok(());
    }

    let publish_fs = h.deps.path_spec().base_fs.publish_fs.clone();
    let filenames = h.deps.build_state.get_filenames_with_post_prefix();
    // Go runs the files on GetNumWorkerMultiplier() workers and returns the first error.
    let mut first_err: Option<Error> = None;
    for filename in &filenames {
        if let Err(err) = post_process_file(publish_fs.as_ref(), filename, &to_post_process)
            && first_err.is_none()
        {
            first_err = Some(err);
        }
    }

    // Prepare for a new build (the sites share the post build assets).
    for _s in &h.sites {
        rs.common
            .post_build_assets
            .post_process_resources
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
    }

    match first_err {
        Some(err) => Err(err),
        None => Ok(()),
    }
}

/// Go: `handleFile(filename)` of `postProcess` — replaces every `__h_pp_l1…__e=` placeholder of
/// the file with the value of the first resource that knows it (`GetFieldString`), and writes
/// the file back only when something changed. A Go panic (an unknown field accessor, a
/// placeholder without its suffix) is an error here.
// Go: hugolib/hugo_sites_build.go:postProcess
pub fn post_process_file(
    fs: &dyn Fs,
    filename: &str,
    to_post_process: &[Arc<PostPublishResource>],
) -> Result<()> {
    let mut content = read_file(fs, filename)?;

    let mut k: usize = 0;
    let mut changed = false;

    let prefix = POST_PROCESS_PREFIX.as_bytes();
    let suffix = POST_PROCESS_SUFFIX.as_bytes();

    loop {
        let Some(l) = bytes_index(&content[k..], prefix) else {
            break;
        };
        // Go: `bytes.Index(...) + len(suffix)`, which is len(suffix)-1 when not found.
        let m = match bytes_index(&content[k + l..], suffix) {
            Some(i) => i + suffix.len(),
            None => suffix.len() - 1,
        };

        let (low, high) = (k + l, k + l + m);
        if high > content.len() {
            // Go: a slice bounds panic.
            return Err(Error::new(format!(
                "runtime error: slice bounds out of range [:{high}] with capacity {}",
                content.len()
            )));
        }

        let field = String::from_utf8_lossy(&content[low..high]).into_owned();

        let mut forward = l + m;

        for r in to_post_process {
            match r.get_field_string(&field) {
                None => {}
                Some(Err(err)) => return Err(err),
                Some(Ok(v)) => {
                    let mut next = Vec::with_capacity(content.len() + v.len());
                    next.extend_from_slice(&content[..low]);
                    next.extend_from_slice(v.as_bytes());
                    next.extend_from_slice(&content[high..]);
                    content = next;
                    changed = true;
                    forward = v.len();
                    break;
                }
            }
        }

        k += forward;
    }

    if changed {
        return write_file(fs, filename, &content, 0o666);
    }

    Ok(())
}

/// Go: `bytes.Index(s, sep)` (`None` for -1).
fn bytes_index(s: &[u8], sep: &[u8]) -> Option<usize> {
    if sep.is_empty() {
        return Some(0);
    }
    s.windows(sep.len()).position(|w| w == sep)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: hugolib/hugo_sites_build.go (process* -> build_process.rs (T20), assemble -> build_assemble.rs (T21)) (1260 lines; 11/20 funcs executed)
//   types: pathChange
// OK L60-223: (h *HugoSites) Build(config BuildCfg, events ...fsnotify.Event) error
// OK L228-231: (h *HugoSites) initSites(config *BuildCfg) error  [h.reset: nothing to reset in a first build]
//    L233-251: (h *HugoSites) initRebuild(config *BuildCfg) error  [rebuilds: not ported]
// OK L351-438: (h *HugoSites) render(l logg.LevelLogger, config *BuildCfg) error
// OK L440-469: (h *HugoSites) renderDeferred(l logg.LevelLogger) error
// OK L471-558: (s *Site) executeDeferredTemplates(de *deps.DeferredExecutions) error
// OK L561-579: (h *HugoSites) printPathWarningsOnce() error
// OK L582-597: (h *HugoSites) printUnusedTemplatesOnce() error
// OK L600-717: (h *HugoSites) postProcess(l logg.LevelLogger) error
// OK L719-775: (h *HugoSites) writeBuildStats() error
//    L788-790: (p pathChange) isStructuralChange() bool
//    L792-801: (h *HugoSites) processPartialRebuildChanges(ctx context.Context, l logg.LevelLogger, config *BuildCfg) error
//    L804-1180: (h *HugoSites) processPartialFileEvents(ctx context.Context, l logg.LevelLogger, config *BuildCfg, init func(config *BuildCfg) error, events []fsno...
//    L1182-1191: (h *HugoSites) LogServerAddresses()
//    L1201-1217: (s *Site) handleContentAdapterChanges(bi pagesfromdata.BuildInfo, buildConfig *BuildCfg)
//    L1219-1241: (h *HugoSites) processContentAdaptersOnRebuild(ctx context.Context, buildConfig *BuildCfg) error
// ---------------------------------------------------------------------------
