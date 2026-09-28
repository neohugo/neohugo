//! Shared harness of the T22 tests (`content.rs`, `hookrec.rs`): the replaying
//! `TemplateExecutor`, the site setup after T20's `process` (what assembly computes and content
//! rendering reads), `initLazyProviders`/`shiftToOutputFormat` stand-ins (T21's).

#![allow(dead_code)]

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use go_value::Value;
use nh_common::Result;
use nh_common::herrors::Error;
use nh_hugolib::HugoSites;
use nh_hugolib::hugo_sites_build::BuildCfg;
use nh_hugolib::page::{PageId, PageLazy};
use nh_hugolib::page__output::PageOutput;
use nh_hugolib::page__paths::{PagePaths, TargetPathsHolder};
use nh_hugolib::page__per_output::PageContentOutput;
use nh_hugolib::template_exec::{ExecCall, ExecKind, TemplateExecutor};
use nh_media::output::output_format::{Formats, OutputFormat};
use nh_tpl::template::TplContext;
use nh_tplimpl::templatestore::TemplInfo;
use serde_json::{Value as J, json};

use crate::support::*;

/// A recorded execution.
#[derive(Clone)]
pub struct Rec {
    pub out: String,
    pub err: Option<String>,
    /// The template context's `is_in_goldmark` Go saw (set only for `{{% %}}` shortcodes and
    /// what runs inside them; render hooks get the caller's context unchanged).
    pub in_goldmark: bool,
}

pub type RecKey = (usize, String, String, u32);

/// The replaying `TemplateExecutor`: the ordinal of a call is the number of earlier calls with
/// the same (page, output format, kind), as the Go recorder counts them.
pub struct Replay {
    pub records: HashMap<RecKey, Rec>,
    pub used: Mutex<HashSet<RecKey>>,
    pub counts: Mutex<HashMap<(usize, String, String), u32>>,
    pub page_idx: Mutex<HashMap<PageId, usize>>,
    pub missing: Mutex<Vec<String>>,
    /// Go page id -> Rust page id: pids are only identities (shortcode placeholders, hugocontext
    /// markers), and recorded outputs may carry them (e.g. a `{{< ref >}}` placeholder in a link
    /// destination reaches the link hook).
    pub pids: HashMap<u64, u64>,
}

/// Rewrites the Go pids in shortcode placeholders (`HAHAHUGOSHORTCODE<pid>s`) and hugocontext
/// markers (`pid=<pid>`) of a recorded output to the port's.
pub fn translate_pids(out: &str, pids: &HashMap<u64, u64>) -> String {
    let mut s = out.to_string();
    for prefix in ["HAHAHUGOSHORTCODE", "pid="] {
        let mut res = String::with_capacity(s.len());
        let mut rest = s.as_str();
        while let Some(i) = rest.find(prefix) {
            res.push_str(&rest[..i + prefix.len()]);
            rest = &rest[i + prefix.len()..];
            let n = rest.bytes().take_while(|b| b.is_ascii_digit()).count();
            let digits = &rest[..n];
            match digits.parse::<u64>().ok().and_then(|d| pids.get(&d)) {
                Some(p) if n > 0 => res.push_str(&p.to_string()),
                _ => res.push_str(digits),
            }
            rest = &rest[n..];
        }
        res.push_str(rest);
        s = res;
    }
    s
}

pub fn kind_string(k: &ExecKind) -> String {
    match k {
        ExecKind::Hook(k) => k.to_string(),
        ExecKind::Shortcode(n) => format!("shortcode:{n}"),
        other => format!("{other:?}"),
    }
}

impl TemplateExecutor for Replay {
    fn execute(
        &self,
        ctx: &TplContext,
        _templ: &Arc<TemplInfo>,
        w: &mut Vec<u8>,
        _data: &Value,
        call: &ExecCall,
    ) -> Result<()> {
        let page = call.page.expect("hooks and shortcodes have a page");
        let pi = *self
            .page_idx
            .lock()
            .unwrap()
            .get(&page)
            .unwrap_or_else(|| panic!("replay: page {page:?} is not in the fixture"));
        let kind = kind_string(&call.kind);
        let ck = (pi, call.output_format.clone(), kind.clone());
        let ordinal = {
            let mut c = self.counts.lock().unwrap();
            let n = c.entry(ck).or_insert(0);
            let o = *n;
            *n += 1;
            o
        };
        let key = (pi, call.output_format.clone(), kind.clone(), ordinal);
        match self.records.get(&key) {
            Some(r) => {
                if r.in_goldmark != ctx.is_in_goldmark {
                    self.missing.lock().unwrap().push(format!(
                        "replay: page {pi} format {} kind {kind} ordinal {ordinal}: is_in_goldmark {} (Go: {})",
                        call.output_format, ctx.is_in_goldmark, r.in_goldmark
                    ));
                }
                self.used.lock().unwrap().insert(key);
                w.extend_from_slice(translate_pids(&r.out, &self.pids).as_bytes());
                match &r.err {
                    Some(e) => Err(Error::new(e.clone())),
                    None => Ok(()),
                }
            }
            None => {
                let msg = format!(
                    "replay: no record for page {pi} format {} kind {kind} ordinal {ordinal}",
                    call.output_format
                );
                self.missing.lock().unwrap().push(msg.clone());
                Err(Error::new(msg))
            }
        }
    }
}

/// A string value as the fixture holds it (a long string may be recorded as its FNV hash).
pub fn str_j(want: &J, got: &[u8]) -> J {
    if want.is_object() && want.get("fnv").is_some() {
        return json!({"fnv": fnv(got), "len": got.len()});
    }
    J::String(String::from_utf8_lossy(got).into_owned())
}

pub fn value_bytes(v: &Value) -> Vec<u8> {
    match v {
        Value::String(s) | Value::Safe(_, s) => s.as_bytes().to_vec(),
        _ => panic!("not a string value: {v:?}"),
    }
}

/// Go's zero `targetPathsHolder`.
pub fn zero_target_paths() -> TargetPathsHolder {
    TargetPathsHolder {
        rel_url: String::new(),
        paths: Default::default(),
        output_format: nh_page::page_outputformat::OutputFormat::new(
            "",
            "",
            false,
            OutputFormat::default(),
        ),
    }
}

/// `initLazyProviders` (T21) for the harness: one output per render format name, a content
/// output on the first.
pub fn init_outputs(h: &HugoSites, id: PageId) {
    let ps = h.page(id);
    let mut created: HashMap<String, Arc<PageOutput>> = HashMap::new();
    let mut outputs = Vec::new();
    for (i, f) in h.render_formats.0.iter().enumerate() {
        if let Some(po) = created.get(&f.name) {
            outputs.push(po.clone());
            continue;
        }
        let po = Arc::new(PageOutput::new_with_target_paths(
            h,
            ps,
            zero_target_paths(),
            f.clone(),
            true,
        ));
        if i == 0 {
            po.set_content_provider(Some(PageContentOutput::new(&po, i).unwrap()));
        }
        outputs.push(po.clone());
        created.insert(f.name.clone(), po);
    }
    let first =
        nh_page::page_outputformat::OutputFormat::new("", "", true, h.render_formats.0[0].clone());
    let paths = PagePaths {
        output_formats: Default::default(),
        first_output_format: first,
        target_paths: BTreeMap::new(),
        target_path_descriptor: nh_page::page_paths::TargetPathDescriptor {
            path_spec: h.deps.path_spec().clone(),
            type_: h.render_formats.0[0].clone(),
            kind: ps.meta.kind().to_string(),
            path: ps.meta.path_info.clone(),
            section: None,
            base_name: String::new(),
            prefix_file_path: String::new(),
            prefix_link: String::new(),
            force_prefix: false,
            url: String::new(),
            addends: String::new(),
            expanded_permalink: String::new(),
            ugly_urls: false,
        },
    };
    assert!(ps.lazy.set(Ok(PageLazy { paths, outputs })).is_ok());
}

/// `shiftToOutputFormat(true, idx)` (T21) for the harness (the rendering site's pages).
pub fn shift_rendering(h: &HugoSites, id: PageId, idx: usize) {
    let ps = h.page(id);
    let lazy = ps.lazy.get().unwrap().as_ref().unwrap();
    let idx = if lazy.outputs.len() == 1 { 0 } else { idx };
    ps.current_output_idx.store(idx, Ordering::SeqCst);
    let po = &lazy.outputs[idx];
    let mut cp = po.pco();
    if cp.is_none() && ps.can_reuse_page_output_content() {
        // Look for content to reuse.
        for (i, o) in lazy.outputs.iter().enumerate() {
            if i == idx {
                continue;
            }
            if let Some(c) = o.pco() {
                cp = Some(c);
                break;
            }
        }
    }
    let cp = match cp {
        Some(cp) => cp,
        None => PageContentOutput::new(po, idx).unwrap(),
    };
    po.set_content_provider(Some(cp));
}

pub fn formats_by_names(h: &HugoSites, site: usize, names: &[J]) -> Formats {
    let all = &h.sites[site]
        .conf
        .output_formats
        .as_ref()
        .expect("output formats")
        .config;
    Formats(
        names
            .iter()
            .map(|n| {
                let n = n.as_str().unwrap();
                all.get_by_name(n)
                    .unwrap_or_else(|| panic!("output format {n}"))
            })
            .collect(),
    )
}

/// A site set up for content rendering: T20's `process`, the assembly results of the fixture,
/// the replay installed, frozen, the page outputs created.
pub struct Setup {
    pub h: Arc<HugoSites>,
    /// The captured page of each fixture page.
    pub ids: Vec<PageId>,
    pub replay: Arc<Replay>,
    pub nested: usize,
    pub n_records: usize,
    pub dir: String,
    pub _tmp: TempDir,
}

impl Setup {
    pub fn norm(&self, s: &str) -> String {
        s.replace(&self.dir, "/SITE")
    }

    /// Every recorded execution must have been replayed, and every replayed one recorded.
    /// Returns the number of replayed records.
    pub fn check_replay(&self, name: &str) -> usize {
        let missing = self.replay.missing.lock().unwrap().clone();
        let used = self.replay.used.lock().unwrap().clone();
        let mut unused: Vec<String> = self
            .replay
            .records
            .keys()
            .filter(|k| !used.contains(*k))
            .map(|k| format!("{k:?}"))
            .collect();
        unused.sort();
        assert!(missing.is_empty(), "{name}: missing records: {missing:#?}");
        assert!(unused.is_empty(), "{name}: unused records: {unused:#?}");
        used.len()
    }
}

/// Sets up the site of fixture `fx` (see [`Setup`]).
pub fn setup(name: &str, fx: &J) -> Setup {
    let tmp = TempDir::new(&format!("content-{name}"));
    let mut b =
        new_sites(&fx["site"], &tmp.0).unwrap_or_else(|e| panic!("{name}: {}", e.message()));
    if let Some(err) = fx["err"].as_str() {
        panic!("{name}: the Go oracle failed: {err}");
    }
    let dump = &fx["dump"];

    nh_hugolib::build_process::process(&mut b.h, &BuildCfg::default())
        .unwrap_or_else(|e| panic!("{name}: process: {}", e.message()));

    // The sites' render formats (T21's initRenderFormats).
    let mut global = Vec::new();
    for (i, s) in dump["sites"].as_array().unwrap().iter().enumerate() {
        let f = formats_by_names(&b.h, i, s["renderFormats"].as_array().unwrap());
        global.extend(f.0.iter().cloned());
        b.h.sites[i].render_formats = f;
    }
    b.h.render_formats = Formats(global);
    let want_global: Vec<String> = dump["renderFormats"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n.as_str().unwrap().to_string())
        .collect();
    let got_global: Vec<String> =
        b.h.render_formats
            .0
            .iter()
            .map(|f| f.name.clone())
            .collect();
    assert_eq!(want_global, got_global, "{name}: render formats");

    // Match the fixture pages to the captured pages and set what assembly computes.
    let pages = dump["pages"].as_array().unwrap();
    let mut ids: Vec<PageId> = Vec::new();
    let mut page_idx: HashMap<PageId, usize> = HashMap::new();
    let mut pids: HashMap<u64, u64> = HashMap::new();
    for (pi, p) in pages.iter().enumerate() {
        let file = p["file"].as_str().unwrap();
        let lang = p["lang"].as_str().unwrap();
        let found =
            b.h.pages
                .iter()
                .find(|ps| {
                    ps.meta.lang() == lang
                        && ps
                            .meta
                            .f
                            .as_ref()
                            .is_some_and(|f| b.norm(f.filename()) == file)
                })
                .map(|ps| ps.id)
                .unwrap_or_else(|| panic!("{name}: page {file} ({lang}) not captured"));
        ids.push(found);
        page_idx.insert(found, pi);
        pids.insert(p["pid"].as_u64().unwrap(), b.h.page(found).pid);
        let site = p["site"].as_u64().unwrap() as usize;
        let media_types = b.h.sites[site]
            .conf
            .media_types
            .as_ref()
            .expect("media types")
            .config
            .clone();
        let ps = b.h.page_mut(found);
        assert_eq!(ps.site_idx, site, "{name}: {file}: site");
        let pc = &mut ps.meta.page_config;
        pc.kind = p["kind"].as_str().unwrap().to_string();
        pc.type_ = p["type"].as_str().unwrap().to_string();
        pc.layout = p["layout"].as_str().unwrap().to_string();
        pc.content.markup = p["markup"].as_str().unwrap().to_string();
        pc.content_media_type = media_types
            .get_by_type(p["mediaType"].as_str().unwrap())
            .unwrap_or_else(|| panic!("media type {}", p["mediaType"]));
        pc.summary = p["summary"].as_str().unwrap().to_string();
        pc.is_cjk_language = p["isCJK"].as_bool().unwrap();
        pc.title = p["title"].as_str().unwrap().to_string();
    }

    // The replay.
    let mut records = HashMap::new();
    let mut nested = 0usize;
    for r in dump["records"].as_array().unwrap() {
        if r["nested"].as_bool().unwrap() {
            nested += 1;
            continue;
        }
        let pi = r["page"].as_i64().unwrap();
        assert!(
            pi >= 0,
            "{name}: a top-level execution for a page without a file: {r}"
        );
        let key = (
            pi as usize,
            r["format"].as_str().unwrap().to_string(),
            r["kind"].as_str().unwrap().to_string(),
            r["ordinal"].as_u64().unwrap() as u32,
        );
        let rec = Rec {
            out: r["out"].as_str().unwrap().to_string(),
            err: r["err"].as_str().map(|s| s.to_string()),
            in_goldmark: r["inGoldmark"].as_bool().unwrap(),
        };
        assert!(
            records.insert(key, rec).is_none(),
            "{name}: duplicate record"
        );
    }
    let n_records = records.len();
    let replay = Arc::new(Replay {
        records,
        used: Mutex::new(HashSet::new()),
        counts: Mutex::new(HashMap::new()),
        page_idx: Mutex::new(page_idx),
        missing: Mutex::new(Vec::new()),
        pids,
    });
    b.h.template_executor = Some(replay.clone());

    let dir = b.dir.clone();
    let h = b.h.freeze();

    for &id in &ids {
        init_outputs(&h, id);
    }

    Setup {
        h,
        ids,
        replay,
        nested,
        n_records,
        dir,
        _tmp: tmp,
    }
}
