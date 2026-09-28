//! T24 acceptance: the build driver, render loop and aliases against the Go oracle
//! `tools/go-oracle/nh-hugolib/build` (a real in-memory Go build per site, one render worker).
//!
//! For each recorded site the sites are created like the oracle does, the publish dir fs is
//! wrapped with a recorder, and `hugo_sites_build::build` runs the whole build with a STUB
//! `TemplateExecutor`:
//!
//! * page and pager executions write a line naming the execution (no file for the executions
//!   whose Go output was empty; Go's error text for the ones that failed), and call `.Paginator`
//!   for exactly the pages whose paginator Go initialised (the `pagers` records);
//! * alias executions run the real alias template (the func map is names-only, except a real
//!   `site`), so the alias bytes can be compared.
//!
//! Then, with Go's files created by templates ("tpl": resources published by `.RelPermalink`)
//! and by postProcess left out on both sides:
//!
//! * the publish order (every file created in the publish dir, duplicates included, so the last
//!   writer of targets several pages share) must be Go's;
//! * the bytes of every alias file must be Go's;
//! * the executions (kind, pager number, language, page, output format, template) must be Go's,
//!   and the paginators must have Go's number of pagers;
//! * the log and the build error must be Go's; with js.Build source roots, the jsconfig.json.
//!
//! `stats_*`: the HTML each site's publisher fed its elements collector in the Go build goes
//! through the port's collectors, and `build_stats_json` must give Go's `hugo_stats.json`.
//! `postprocess`: the recorded files with placeholders are post-processed with stub resources
//! returning Go's field values, and must equal Go's files.

mod support;

use std::collections::{HashMap, HashSet};
use std::io::{Read, Seek, SeekFrom, Write};
use std::sync::{Arc, Mutex};

use go_value::{HostCtx, Map, MapType, Value};
use nh_allconfig::load::{ConfigSourceDescriptor, load_config};
use nh_common::Result;
use nh_common::herrors::Error;
use nh_common::loggers::{Level, LogSink, Logger, Options};
use nh_config::config_provider::Provider;
use nh_config::default_config_provider::DefaultConfigProvider;
use nh_deps::deps::Deps;
use nh_hugofs::afero::{File, Fs};
use nh_hugofs::fileinfo::FileMetaInfo;
use nh_hugolib::alias::AliasPage;
use nh_hugolib::hugo_sites::{FuncMapFactory, HugoSites, NewHugoSitesCfg};
use nh_hugolib::hugo_sites_build::{BuildCfg, build, build_stats_json, post_process_file};
use nh_hugolib::page::PageHandle;
use nh_hugolib::template_exec::{ExecCall, ExecKind, TemplateExecutor};
use nh_publisher::html_elements_collector::{HtmlElementsCollector, HtmlElementsCollectorWriter};
use nh_publisher::publisher::PublishStats;
use nh_resource::resourcetypes::Resource;
use nh_resources::postpub::postpub::PostPublishResource;
use nh_tpl::template::TplContext;
use nh_tplimpl::templatestore::TemplInfo;
use serde_json::{Value as J, json};
use support::*;

// ---------------------------------------------------------------------------
// The publish dir recorder.

type Events = Arc<Mutex<Vec<(String, Arc<Mutex<Vec<u8>>>)>>>;

/// Wraps the publish dir: records every file created (Create, OpenFile with O_CREATE) and the
/// bytes written to it (the oracle's `nhRecFs`).
struct RecFs {
    inner: Arc<dyn Fs>,
    events: Events,
}

impl RecFs {
    fn created(&self, name: &str, f: Box<dyn File>) -> Box<dyn File> {
        let buf = Arc::new(Mutex::new(Vec::new()));
        let path = name.trim_start_matches('/').to_string();
        self.events.lock().unwrap().push((path, buf.clone()));
        Box::new(RecFile { inner: f, buf })
    }
}

impl Fs for RecFs {
    fn name(&self) -> &str {
        "RecFs"
    }
    fn embedded(&self) -> Option<&dyn Fs> {
        Some(self.inner.as_ref())
    }
    fn create(&self, name: &str) -> Result<Box<dyn File>> {
        let f = self.inner.create(name)?;
        Ok(self.created(name, f))
    }
    fn open_file(&self, name: &str, flag: i32, perm: u32) -> Result<Box<dyn File>> {
        let f = self.inner.open_file(name, flag, perm)?;
        if flag & nh_hugofs::afero::flags::O_CREATE == 0 {
            return Ok(f);
        }
        Ok(self.created(name, f))
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

struct RecFile {
    inner: Box<dyn File>,
    buf: Arc<Mutex<Vec<u8>>>,
}

impl Read for RecFile {
    fn read(&mut self, b: &mut [u8]) -> std::io::Result<usize> {
        self.inner.read(b)
    }
}

impl Write for RecFile {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        let n = self.inner.write(b)?;
        self.buf.lock().unwrap().extend_from_slice(&b[..n]);
        Ok(n)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

impl Seek for RecFile {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.inner.seek(pos)
    }
}

impl File for RecFile {
    fn name(&self) -> String {
        self.inner.name()
    }
    fn stat(&self) -> Result<FileMetaInfo> {
        self.inner.stat()
    }
    fn read_dir(&mut self, count: i32) -> Result<Vec<FileMetaInfo>> {
        self.inner.read_dir(count)
    }
    fn close(&mut self) -> Result<()> {
        self.inner.close()
    }
}

// ---------------------------------------------------------------------------
// The stub executor.

/// (kind, pager number, lang, path, format)
type Key = (String, u32, String, String, String);

struct Stub {
    /// Every site's deps (alias templates execute for real through the site's store).
    deps: Vec<Arc<Deps>>,
    empty: HashSet<Key>,
    errs: HashMap<Key, String>,
    pagers: HashSet<(String, String, String)>,
    records: Mutex<Vec<J>>,
    pager_records: Mutex<Vec<J>>,
}

fn page_of(v: &Value) -> Option<PageHandle> {
    let p = nh_page::page::page_from_value(v)?;
    p.0.as_any().downcast_ref::<PageHandle>().cloned()
}

impl TemplateExecutor for Stub {
    fn execute(
        &self,
        ctx: &TplContext,
        templ: &Arc<TemplInfo>,
        w: &mut Vec<u8>,
        data: &Value,
        call: &ExecCall,
    ) -> Result<()> {
        let (kind, n) = match &call.kind {
            ExecKind::Page => ("page", 0),
            ExecKind::Pager(n) => ("pager", *n),
            ExecKind::Alias => {
                let ap = data
                    .as_object()
                    .and_then(|o| o.as_any().downcast_ref::<AliasPage>())
                    .expect("alias data");
                let (lang, path, site_idx) = match &ap.page {
                    Some(p) => {
                        let ph = p.0.as_any().downcast_ref::<PageHandle>().unwrap();
                        let ps = ph.state();
                        (ps.meta.lang().to_string(), ps.meta.path(), ps.site_idx)
                    }
                    None => (String::new(), String::new(), 0),
                };
                self.records.lock().unwrap().push(json!({
                    "kind": "alias", "lang": lang, "path": path, "permalink": ap.permalink,
                }));
                return self.deps[site_idx]
                    .get_template_store()
                    .execute_with_context(ctx, templ, w, data);
            }
            other => panic!("unexpected execution {other:?}"),
        };
        let page = ctx
            .page
            .as_ref()
            .and_then(page_of)
            .expect("the page in the context");
        let ps = page.state();
        let lang = ps.meta.lang().to_string();
        let path = ps.meta.path();
        let format = call.output_format.clone();
        let name = templ.name();
        self.records.lock().unwrap().push(json!({
            "kind": kind, "n": n, "lang": lang, "path": path, "format": format, "templ": name,
        }));

        if kind == "page"
            && self
                .pagers
                .contains(&(lang.clone(), path.clone(), format.clone()))
        {
            let pv = ctx.page.clone().unwrap();
            let obj = pv.as_object().unwrap().clone();
            let v = obj
                .call_method(ctx.as_host(), "Paginator", &[])
                .expect("Paginator")
                .map_err(|e| Error::new(e.message().to_string()))?;
            let pager = nh_page::pagination::pager_from_value(&v).expect("a pager");
            self.pager_records.lock().unwrap().push(json!({
                "lang": lang, "path": path, "format": format, "total": pager.paginator.total_pages(),
            }));
        }

        let key: Key = (kind.to_string(), n, lang, path, format);
        if let Some(e) = self.errs.get(&key) {
            return Err(Error::new(e.clone()));
        }
        if self.empty.contains(&key) {
            return Ok(());
        }
        // A JSON string: valid output for every format's minifier (HTML/XML text, JSON).
        w.extend_from_slice(format!("\"{kind} {n} {} {} {name}\"", key.3, key.4).as_bytes());
        Ok(())
    }
}

fn key_of(r: &J) -> Key {
    (
        r["kind"].as_str().unwrap().to_string(),
        r["n"].as_u64().unwrap_or(0) as u32,
        r["lang"].as_str().unwrap().to_string(),
        r["path"].as_str().unwrap().to_string(),
        r["format"].as_str().unwrap_or("").to_string(),
    )
}

// ---------------------------------------------------------------------------

/// The oracle's bytes encoding (a string, or {"hex": ...}).
fn dec(v: &J) -> Vec<u8> {
    if let Some(s) = v.as_str() {
        return s.as_bytes().to_vec();
    }
    let h = v["hex"].as_str().unwrap();
    (0..h.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&h[i..i + 2], 16).unwrap())
        .collect()
}

/// The names-only func map with a real `site` (the embedded alias.html calls it).
fn func_map_factory() -> FuncMapFactory {
    let base = names_only_func_map_factory();
    Arc::new(move |d: &Arc<Deps>| {
        let mut m = base(d);
        let d2 = d.clone();
        let f: nh_tplimpl::engine::TplFunc = Arc::new(move |_ctx, _args| Ok(d2.site().to_value()));
        m.insert("site".to_string(), f);
        m
    })
}

struct BuiltSite {
    h: HugoSites,
    dir: String,
    log: Arc<Mutex<Vec<u8>>>,
    events: Events,
}

/// support::new_sites with the publish dir wrapped by the recorder and the alias func map.
fn new_sites_rec(site: &J, tmp: &std::path::Path) -> Result<BuiltSite> {
    let dir = write_site(site, tmp);
    let dirs = dir.to_str().unwrap().to_string();

    let flags = DefaultConfigProvider::new();
    flags.set("workingDir", Value::string(dirs.as_str()));
    flags.set("noBuildLock", Value::Bool(true));
    flags.set(
        "cacheDir",
        Value::string(tmp.join("_cache").to_str().unwrap()),
    );
    let cfg_log: LogSink = Arc::new(Mutex::new(Vec::<u8>::new()));
    let configs = load_config(ConfigSourceDescriptor {
        flags: Some(Arc::new(flags)),
        filename: dir.join("hugo.toml").to_str().unwrap().to_string(),
        environment: "production".to_string(),
        environ: vec!["NEOHUGO_ORACLE=1".to_string()],
        getenv: Some(Arc::new(|_k: &str| String::new())),
        logger: Some(Logger::with_options(Options {
            level: Level::Warn,
            std_out: Some(cfg_log.clone()),
            std_err: Some(cfg_log),
            ..Default::default()
        })),
        ..Default::default()
    })?;

    let log = Arc::new(Mutex::new(Vec::<u8>::new()));
    let sink: LogSink = log.clone();
    let logger = Logger::with_options(Options {
        level: Level::Warn,
        std_out: Some(sink.clone()),
        std_err: Some(sink),
        distinct_level: Some(Level::Warn),
        ..Default::default()
    });

    let mut fs = nh_hugofs::fs::new_from(
        nh_hugofs::afero::new_os_fs(),
        &configs.loading_info.base_config,
    );
    let events: Events = Arc::new(Mutex::new(Vec::new()));
    fs.publish_dir = Arc::new(RecFs {
        inner: fs.publish_dir.clone(),
        events: events.clone(),
    });
    let h = HugoSites::new(NewHugoSitesCfg {
        configs: Arc::new(configs),
        fs,
        log: logger,
        func_map_factory: Some(func_map_factory()),
    })?;
    Ok(BuiltSite {
        h,
        dir: dirs,
        log,
        events,
    })
}

fn run_case(name: &str) {
    let fx = fixture(&format!("build/{name}.json.gz"));
    let tmp = TempDir::new(&format!("build-{name}"));
    let b = new_sites_rec(&fx["site"], &tmp.0).expect("new sites");
    let BuiltSite {
        mut h,
        dir,
        log,
        events,
    } = b;
    let norm = |s: &str| s.replace(&dir, "/SITE");

    let mut empty = HashSet::new();
    let mut errs = HashMap::new();
    for r in fx["renders"].as_array().unwrap() {
        if r["kind"] == "alias" {
            continue;
        }
        if let Some(e) = r["err"].as_str() {
            // The executor's error: the text inside Go's `render of "<page>" failed: `.
            let inner = match e.find("\" failed: ") {
                Some(i) if e.starts_with("render of \"") => &e[i + "\" failed: ".len()..],
                _ => e,
            };
            errs.insert(key_of(r), inner.replace("/SITE", &dir));
        } else if r["empty"] == true {
            empty.insert(key_of(r));
        }
    }
    let pagers: HashSet<(String, String, String)> = fx["pagers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| {
            (
                p["lang"].as_str().unwrap().to_string(),
                p["path"].as_str().unwrap().to_string(),
                p["format"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    let stub = Arc::new(Stub {
        deps: h.sites.iter().map(|s| s.deps.clone()).collect(),
        empty,
        errs,
        pagers,
        records: Mutex::new(Vec::new()),
        pager_records: Mutex::new(Vec::new()),
    });
    h.template_executor = Some(stub.clone());

    if let Some(roots) = fx["jsconfig"]["roots"].as_array() {
        for r in roots {
            h.deps
                .resource_spec()
                .common
                .post_build_assets
                .js_config_builder
                .add_source_root(&r.as_str().unwrap().replace("/SITE", &dir));
        }
    }

    let res = build(h, BuildCfg::default());

    let mut diffs: Vec<String> = Vec::new();

    // The build error.
    let got_err = res.as_ref().err().map(|e| norm(&e.to_string()));
    let want_err = fx["err"].as_str().map(|s| s.to_string());
    if got_err != want_err {
        diffs.push(format!(
            "build error:\n  want {want_err:?}\n  got  {got_err:?}"
        ));
    }

    // The publish order and the alias bytes.
    let want_events: Vec<&J> = fx["events"].as_array().unwrap().iter().collect();
    let tpl_paths: HashSet<&str> = want_events
        .iter()
        .filter(|e| e["phase"] == "tpl")
        .map(|e| e["path"].as_str().unwrap())
        .collect();
    let want: Vec<&J> = want_events
        .into_iter()
        .filter(|e| e["phase"] == "" && !tpl_paths.contains(e["path"].as_str().unwrap()))
        .collect();
    let got_events = events.lock().unwrap().clone();
    let got: Vec<(String, Vec<u8>)> = got_events
        .into_iter()
        .filter(|(p, _)| !tpl_paths.contains(p.as_str()))
        .map(|(p, b)| (p, b.lock().unwrap().clone()))
        .collect();
    let mut aliases = 0;
    for i in 0..want.len().max(got.len()) {
        let w = want.get(i).map(|e| e["path"].as_str().unwrap().to_string());
        let g = got.get(i).map(|e| e.0.clone());
        if w != g {
            diffs.push(format!("publish order [{i}]: want {w:?} got {g:?}"));
            break;
        }
        if let Some(wb) = want[i].get("bytes") {
            aliases += 1;
            let wb = dec(wb);
            let gb = norm(&String::from_utf8_lossy(&got[i].1)).into_bytes();
            if wb != gb {
                diffs.push(format!(
                    "alias bytes of {} [{i}]:\n  want {:?}\n  got  {:?}",
                    got[i].0,
                    String::from_utf8_lossy(&wb),
                    String::from_utf8_lossy(&gb)
                ));
            }
        }
    }

    // The executions.
    let want_renders: Vec<J> = fx["renders"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| {
            if r["kind"] == "alias" {
                json!({"kind": "alias", "lang": r["lang"], "path": r["path"], "permalink": r["permalink"]})
            } else {
                json!({"kind": r["kind"], "n": r["n"], "lang": r["lang"], "path": r["path"], "format": r["format"], "templ": r["templ"]})
            }
        })
        .collect();
    let got_renders: Vec<J> = stub
        .records
        .lock()
        .unwrap()
        .iter()
        .map(|r| {
            let mut r = r.clone();
            if let Some(p) = r.get("permalink").and_then(|p| p.as_str()) {
                r["permalink"] = J::String(norm(p));
            }
            r
        })
        .collect();
    if let Some(d) = first_diff(
        "renders",
        &J::Array(want_renders.clone()),
        &J::Array(got_renders),
    ) {
        diffs.push(d);
    }
    let got_pagers = J::Array(stub.pager_records.lock().unwrap().clone());
    if let Some(d) = first_diff("pagers", &fx["pagers"], &got_pagers) {
        diffs.push(d);
    }

    // The log.
    let got_log: Vec<String> = {
        let b = log.lock().unwrap().clone();
        norm(&String::from_utf8_lossy(&b))
            .split('\n')
            .filter(|l| !l.is_empty())
            .map(|l| l.to_string())
            .collect()
    };
    if let Some(d) = first_diff("log", &fx["log"], &json!(got_log)) {
        diffs.push(d);
    }

    // jsconfig.json.
    if let Some(want) = fx["jsconfig"].get("json") {
        let got = std::fs::read(format!("{dir}/assets/jsconfig.json")).unwrap_or_default();
        if dec(want) != got {
            diffs.push(format!(
                "jsconfig.json:\n  want {:?}\n  got  {:?}",
                String::from_utf8_lossy(&dec(want)),
                String::from_utf8_lossy(&got)
            ));
        }
    }

    assert!(
        diffs.is_empty(),
        "{name}: {} difference(s):\n{}",
        diffs.len(),
        diffs.join("\n")
    );
    eprintln!(
        "{name}: {} files in order, {aliases} alias files, {} executions, {} paginators",
        want.len(),
        want_renders.len(),
        fx["pagers"].as_array().unwrap().len()
    );
}

macro_rules! build_cases {
    ($($f:ident => $n:expr),* $(,)?) => {
        $(#[test] fn $f() { run_case($n); })*
    };
}

build_cases! {
    build_asm_build => "asm-build",
    build_asm_cascade => "asm-cascade",
    build_asm_flags => "asm-flags",
    build_asm_i18n => "asm-i18n",
    build_asm_multihost => "asm-multihost",
    build_asm_taxo => "asm-taxo",
    build_asm_ugly => "asm-ugly",
    build_content => "content",
    build_contentdir => "contentdir",
    build_docs => "docs",
    build_edge_tree => "edge-tree",
    build_homeleaf => "homeleaf",
    build_nokinds => "nokinds",
    build_seeksnack => "seeksnack",
    build_shortcodes => "shortcodes",
    build_synthetic => "synthetic",
    build_testsite => "testsite",
    build_aliases => "build-aliases",
    build_custom_alias => "build-custom-alias",
    build_disable => "build-disable",
    build_multihost => "build-multihost",
    build_collide => "build-collide",
    build_stats_notags => "build-stats-notags",
    build_stats_classes => "build-stats-classes",
    build_postprocess => "build-postprocess",
    build_errors => "build-errors",
}

// ---------------------------------------------------------------------------
// hugo_stats.json from the recorded collector inputs.

fn run_stats(name: &str) {
    let fx = fixture(&format!("build/{name}.json.gz"));
    let st = &fx["stats"];
    let tmp = TempDir::new(&format!("stats-{name}"));
    // The site's build stats config, from the site's config.
    let b = new_sites_rec(&fx["site"], &tmp.0).expect("new sites");
    let conf = b.h.deps.resource_spec().build_config().build_stats.clone();
    let mut stats: Vec<PublishStats> = Vec::new();
    let mut inputs = 0;
    for lang in st["langs"].as_array().unwrap() {
        let mut c = HtmlElementsCollector::new(conf.clone());
        for input in st["inputs"][lang.as_str().unwrap()].as_array().unwrap() {
            inputs += 1;
            let mut w = HtmlElementsCollectorWriter::new(&mut c);
            w.write(&dec(input));
        }
        stats.push(PublishStats {
            html_elements: c.get_html_elements(),
        });
    }
    let got = build_stats_json(&stats).unwrap();
    let want = dec(&st["json"]);
    assert_eq!(
        String::from_utf8_lossy(&got),
        String::from_utf8_lossy(&want),
        "{name}: hugo_stats.json"
    );
    assert_eq!(got, want);
    eprintln!("{name}: {inputs} collector inputs, {} bytes", want.len());
}

#[test]
fn stats_build_aliases() {
    run_stats("build-aliases");
}
#[test]
fn stats_build_collide() {
    run_stats("build-collide");
}
#[test]
fn stats_build_stats_notags() {
    run_stats("build-stats-notags");
}
#[test]
fn stats_build_stats_classes() {
    run_stats("build-stats-classes");
}
#[test]
fn stats_build_postprocess() {
    run_stats("build-postprocess");
}
#[test]
fn stats_seeksnack() {
    run_stats("seeksnack");
}

#[test]
fn stats_nil_lists_and_empty() {
    // No site with a collector: every list is nil (null), as Go encodes nil slices.
    let got = build_stats_json(&[PublishStats::default(), PublishStats::default()]).unwrap();
    assert_eq!(
        String::from_utf8(got).unwrap(),
        "{\n  \"htmlElements\": {\n    \"tags\": null,\n    \"classes\": null,\n    \"ids\": null\n  }\n}\n"
    );
}

// ---------------------------------------------------------------------------
// postProcess on the recorded files with stub resources.

/// A resource returning the field values Go's PostProcess delegate returned.
struct StubResource {
    fields: HashMap<String, String>,
    media_type: nh_media::media::media_type::MediaType,
}

impl StubResource {
    fn f(&self, k: &str) -> String {
        self.fields.get(k).cloned().unwrap_or_default()
    }
}

impl Resource for StubResource {
    fn resource_type(&self) -> String {
        self.f("ResourceType")
    }
    fn media_type(&self) -> nh_media::media::media_type::MediaType {
        self.media_type.clone()
    }
    fn permalink(&self) -> String {
        self.f("Permalink")
    }
    fn rel_permalink(&self) -> String {
        self.f("RelPermalink")
    }
    fn data(&self) -> Value {
        let mut m = Map::new(MapType::StringAny);
        m.insert(
            "Integrity",
            Value::string(self.f("Data.Integrity").as_str()),
        );
        Value::map(m)
    }
    fn name(&self) -> String {
        self.f("Name")
    }
    fn title(&self) -> String {
        self.f("Title")
    }
    fn params(&self) -> Arc<Map> {
        Arc::new(Map::new(MapType::Params))
    }
    fn key(&self) -> String {
        self.f("RelPermalink")
    }
    fn content(&self, _ctx: HostCtx<'_>) -> Option<Result<Value>> {
        Some(Ok(Value::string(self.f("Content").as_str())))
    }
    fn tpl_type_name(&self) -> std::borrow::Cow<'_, str> {
        std::borrow::Cow::Borrowed("*resources.resourceAdapter")
    }
    fn tpl_has_method(&self, _name: &str) -> bool {
        false
    }
    fn tpl_call_method(
        &self,
        _ctx: HostCtx<'_>,
        _name: &str,
        _args: &[Value],
    ) -> Option<nh_common::object::GoResult<Value>> {
        None
    }
    fn to_value(self: Arc<Self>) -> Value {
        Value::Invalid
    }
    fn as_any_arc(self: Arc<Self>) -> Arc<dyn std::any::Any + Send + Sync> {
        self
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[test]
fn postprocess() {
    let fx = fixture("build/build-postprocess.json.gz");
    let pp = &fx["postprocess"];
    let types = nh_media::media::config::default_types();
    let mut resources = Vec::new();
    let mut want_fields = Vec::new();
    for r in pp["resources"].as_array().unwrap() {
        let prefix = r["prefix"].as_str().unwrap();
        let id: i64 = prefix
            .trim_start_matches("__h_pp_l1_")
            .trim_end_matches('_')
            .parse()
            .unwrap();
        let mut fields = HashMap::new();
        for (k, v) in r["fields"].as_object().unwrap() {
            fields.insert(k.clone(), String::from_utf8(dec(v)).unwrap());
        }
        let media_type = types.get_by_type(&fields["MediaType"]).expect("media type");
        want_fields.push((prefix.to_string(), fields.clone()));
        let stub: Arc<dyn Resource> = Arc::new(StubResource { fields, media_type });
        let ppr = PostPublishResource::new(id, stub);
        assert_eq!(ppr.prefix, prefix);
        resources.push(ppr);
    }

    // Every recorded field through GetFieldString (the MediaType ones read the media type).
    for (prefix, fields) in &want_fields {
        let r = resources.iter().find(|r| &r.prefix == prefix).unwrap();
        for (k, v) in fields {
            if k == "MediaType" {
                continue;
            }
            let got = r
                .get_field_string(&format!("{prefix}{k}__e="))
                .expect("known")
                .expect("no error");
            assert_eq!(&got, v, "{prefix}{k}");
        }
    }

    let fs = nh_hugofs::afero::new_mem_map_fs();
    let files = pp["files"].as_array().unwrap();
    for f in files {
        nh_hugofs::afero::write_file(
            fs.as_ref(),
            f["path"].as_str().unwrap(),
            &dec(&f["pre"]),
            0o666,
        )
        .unwrap();
    }
    for f in files {
        let p = f["path"].as_str().unwrap();
        post_process_file(fs.as_ref(), p, &resources).unwrap();
        let got = nh_hugofs::afero::read_file(fs.as_ref(), p).unwrap();
        assert_eq!(
            String::from_utf8_lossy(&got),
            String::from_utf8_lossy(&dec(&f["post"])),
            "{p}"
        );
    }
    eprintln!(
        "postprocess: {} files, {} resources",
        files.len(),
        resources.len()
    );
}

#[test]
fn postprocess_edge_cases() {
    // A placeholder of an unknown resource is skipped; a prefix without a suffix is skipped;
    // an unknown accessor is a Go panic.
    let stub: Arc<dyn Resource> = Arc::new(StubResource {
        fields: HashMap::from([("RelPermalink".to_string(), "/x.css".to_string())]),
        media_type: nh_media::media::config::default_types()
            .get_by_type("text/css")
            .unwrap(),
    });
    let rs = vec![PostPublishResource::new(7, stub)];
    let fs = nh_hugofs::afero::new_mem_map_fs();
    let cases: &[(&str, Option<&str>)] = &[
        (
            "a __h_pp_l1_7_RelPermalink__e= b __h_pp_l1_8_RelPermalink__e= c",
            Some("a /x.css b __h_pp_l1_8_RelPermalink__e= c"),
        ),
        ("nothing", Some("nothing")),
        ("__h_pp_l1_7_Unknown__e=", None),
        // Go: m = Index(suffix) + len(suffix) = 3 without a suffix: the 3 bytes are no field.
        (
            "x__h_pp_l1 and __h_pp_l1_",
            Some("x__h_pp_l1 and __h_pp_l1_"),
        ),
    ];
    for (i, (input, want)) in cases.iter().enumerate() {
        let p = format!("f{i}.html");
        nh_hugofs::afero::write_file(fs.as_ref(), &p, input.as_bytes(), 0o666).unwrap();
        let res = post_process_file(fs.as_ref(), &p, &rs);
        match want {
            Some(w) => {
                res.unwrap();
                let got = nh_hugofs::afero::read_file(fs.as_ref(), &p).unwrap();
                assert_eq!(String::from_utf8(got).unwrap(), *w);
            }
            None => assert!(res.is_err(), "{input}: want an error"),
        }
    }
}

// ---------------------------------------------------------------------------
// targetPathAlias (Go's TestTargetPathHTMLRedirectAlias table, off Windows).

#[test]
fn target_path_alias_table() {
    let log = Logger::new(Level::Warn, Default::default());
    let tests: &[(&str, &str, bool)] = &[
        ("", "", false),
        ("s", "s/index.html", true),
        ("/", "", false),
        ("alias 1", "alias 1/index.html", true),
        ("alias 2/", "alias 2/index.html", true),
        ("alias 3.html", "alias 3.html", true),
        ("alias4.html", "alias4.html", true),
        ("/alias 5.html", "alias 5.html", true),
        ("/трям.html", "трям.html", true),
        ("../../../../tmp/passwd", "", false),
        ("/foo/../../../../tmp/passwd", "tmp/passwd/index.html", true),
        ("foo/../../../../tmp/passwd", "", false),
        ("C:\\Windows", "C:\\Windows/index.html", true),
        ("/trailing-space /", "trailing-space /index.html", true),
        ("/trailing-period./", "trailing-period./index.html", true),
        ("/tab\tseparated/", "tab\tseparated/index.html", true),
        (
            "/chrome/?p=help&ctx=keyboard#topic=3227046",
            "chrome/?p=help&ctx=keyboard#topic=3227046/index.html",
            true,
        ),
        ("/LPT1/Printer/", "LPT1/Printer/index.html", true),
    ];
    for (value, expected, err_is_nil) in tests {
        let res = nh_hugolib::alias::target_path_alias_with(&log, false, value);
        assert_eq!(res.is_ok(), *err_is_nil, "{value:?}: {res:?}");
        if let Ok(p) = res {
            assert_eq!(&p, expected, "{value:?}");
        }
    }
    // Go's error texts; the root is allowed for the main-language redirect.
    let err = |v: &str| {
        nh_hugolib::alias::target_path_alias_with(&log, false, v)
            .unwrap_err()
            .to_string()
    };
    assert_eq!(err(""), "alias \"\" is an empty string");
    assert_eq!(err("/"), "alias \"/\" resolves to website root directory");
    assert_eq!(
        err("../x"),
        "alias \"../x\" traverses outside the website root directory"
    );
    assert_eq!(
        nh_hugolib::alias::target_path_alias_with(&log, true, "/").unwrap(),
        "/index.html"
    );
}
