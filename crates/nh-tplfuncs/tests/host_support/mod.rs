//! Support of the host namespace tests (Wave B task T19): the sites of
//! `tools/go-oracle/nh-tplfuncs/host`, built with nh-hugolib (process + assemble + freeze, the
//! func map of `nh_tplfuncs::tplimplinit::create_func_map`), the calls of the recorded cases
//! (namespace methods and func map functions, with page, page list and nested call arguments)
//! and the encoding of their results in the oracle's typed JSON.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use go_value::{FloatKind, HostCtx, Object, Value};
use nh_allconfig::load::{ConfigSourceDescriptor, load_config};
use nh_common::loggers::{Level, LogSink, Logger, Options};
use nh_config::config_provider::Provider;
use nh_config::default_config_provider::DefaultConfigProvider;
use nh_deps::deps::Deps;
use nh_hugolib::hugo_sites::{HugoSites, NewHugoSitesCfg};
use nh_page::page::{PageRef, page_from_value};
use nh_page::site::SiteRef;
use nh_tpl::template::TplContext;
use nh_tplimpl::engine::FuncMap;
use serde_json::{Value as J, json};

/// Reads a gzip-compressed JSON fixture of `tests/fixtures/host`.
pub fn fixture(rel: &str) -> J {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/host")
        .join(rel);
    let f = std::fs::File::open(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut s = String::new();
    flate2::read::GzDecoder::new(f)
        .read_to_string(&mut s)
        .unwrap();
    serde_json::from_str(&s).unwrap()
}

/// The topic files of the fixture directory (without the site file), sorted.
pub fn topics() -> Vec<String> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/host");
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".json.gz") && n != "sites.json.gz")
        .map(|n| n.trim_end_matches(".json.gz").to_string())
        .collect();
    v.sort();
    v
}

static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// A temporary directory, removed on drop.
pub struct TempDir(pub PathBuf);

impl TempDir {
    pub fn new(prefix: &str) -> TempDir {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let p = std::env::temp_dir().join(format!(
            "nh-tplfuncs-host-{prefix}-{}-{n}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        TempDir(std::fs::canonicalize(&p).unwrap())
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

/// A site language the cases run against (the oracle's `env`).
pub struct Env {
    pub name: String,
    pub d: Arc<Deps>,
    pub site: SiteRef,
    pub func_map: FuncMap,
    pub dir: String,
}

/// The built sites.
pub struct Sites {
    pub tmp: TempDir,
    pub envs: BTreeMap<String, Env>,
    pub hs: Vec<Arc<HugoSites>>,
    pub logs: Vec<(String, Arc<Mutex<Vec<u8>>>)>,
}

/// Writes a recorded site into `tmp/<name>` and returns its directory.
fn write_site(site: &J, tmp: &Path) -> PathBuf {
    let name = site["name"].as_str().unwrap();
    let dir = tmp.join(name);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("hugo.toml"), site["toml"].as_str().unwrap()).unwrap();
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    for (k, v) in site["files"].as_object().unwrap() {
        files.push((k.clone(), v.as_str().unwrap().as_bytes().to_vec()));
    }
    for (k, v) in site["bin"].as_object().unwrap() {
        files.push((k.clone(), unhex(v.as_str().unwrap())));
    }
    for (p, b) in files {
        let fp = dir.join(p);
        std::fs::create_dir_all(fp.parent().unwrap()).unwrap();
        std::fs::write(fp, b).unwrap();
    }
    dir
}

/// Builds every site of the fixture like the oracle (hugolib without rendering): the config
/// loaded as by a build, `HugoSites::new` with the real func map, process, assemble, freeze.
pub fn build_sites(fx: &J) -> Sites {
    nh_page::page::init();
    let tmp = TempDir::new("sites");
    let mut envs = BTreeMap::new();
    let mut hs = Vec::new();
    let mut logs = Vec::new();
    for site in fx["sites"].as_array().unwrap() {
        let name = site["name"].as_str().unwrap().to_string();
        let dir = write_site(site, &tmp.0);
        let dirs = dir.to_str().unwrap().to_string();

        let flags = DefaultConfigProvider::new();
        flags.set("workingDir", Value::string(dirs.as_str()));
        flags.set("noBuildLock", Value::Bool(true));
        flags.set(
            "cacheDir",
            Value::string(tmp.0.join("_cache").to_str().unwrap()),
        );
        // One log for the config and the sites, as the oracle's (the config logger becomes the
        // global logger of deprecations: the last site's).
        let log = Arc::new(Mutex::new(Vec::<u8>::new()));
        let sink: LogSink = log.clone();
        let new_logger = || {
            Logger::with_options(Options {
                level: Level::Warn,
                std_out: Some(sink.clone()),
                std_err: Some(sink.clone()),
                distinct_level: Some(Level::Warn),
                ..Default::default()
            })
        };
        let configs = load_config(ConfigSourceDescriptor {
            flags: Some(Arc::new(flags)),
            filename: dir.join("hugo.toml").to_str().unwrap().to_string(),
            environment: "production".to_string(),
            environ: vec!["NEOHUGO_ORACLE=1".to_string()],
            getenv: Some(Arc::new(|_k: &str| String::new())),
            logger: Some(new_logger()),
            ..Default::default()
        })
        .unwrap_or_else(|e| panic!("{name}: load config: {e}"));
        let logger = new_logger();
        let fs = nh_hugofs::fs::new_from(
            nh_hugofs::afero::new_os_fs(),
            &configs.loading_info.base_config,
        );
        let mut h = HugoSites::new(NewHugoSitesCfg {
            configs: Arc::new(configs),
            fs,
            log: logger,
            func_map_factory: Some(Arc::new(nh_tplfuncs::tplimplinit::create_func_map)),
        })
        .unwrap_or_else(|e| panic!("{name}: new sites: {e}"));
        let cfg = nh_hugolib::hugo_sites_build::BuildCfg::default();
        nh_hugolib::build_process::process(&mut h, &cfg)
            .unwrap_or_else(|e| panic!("{name}: process: {e}"));
        nh_hugolib::build_assemble::assemble(&mut h, &cfg)
            .unwrap_or_else(|e| panic!("{name}: assemble: {e}"));
        let h = h.freeze();
        // The oracle's build ended (its error collector stopped); the oracle restarts the
        // collector, so that the errors the cases send are collected, not logged.
        h.sites[0].deps.global_err_handler.start_error_collector();
        for (i, s) in h.sites.iter().enumerate() {
            let d = s.deps.clone();
            let env_name = format!("{name}/{i}");
            envs.insert(
                env_name.clone(),
                Env {
                    name: env_name,
                    site: d.site().clone(),
                    func_map: nh_tplfuncs::tplimplinit::create_func_map(&d),
                    d,
                    dir: dirs.clone(),
                },
            );
        }
        hs.push(h);
        logs.push((name, log));
    }
    Sites {
        tmp,
        envs,
        hs,
        logs,
    }
}

/// Calls `m` ("ns.Method" or "f:name") on the env.
pub fn invoke(env: &Env, ctx: HostCtx<'_>, m: &str, args: &[J]) -> go_value::Result<Value> {
    let mut vals = Vec::new();
    for a in args {
        vals.push(
            resolve(env, ctx, a)
                .map_err(|e| go_value::Error::new(format!("argument: {}", e.message())))?,
        );
    }
    if let Some(name) = m.strip_prefix("f:") {
        let Some(f) = env.func_map.get(name) else {
            return Err(go_value::Error::new(format!("no func {name}")));
        };
        return f(ctx, &vals);
    }
    let (ns, meth) = m.split_once('.').unwrap();
    let nsf = env
        .func_map
        .get(ns)
        .unwrap_or_else(|| panic!("no namespace {ns}"));
    let recv = nsf(ctx, &[])?;
    let Value::Object(o) = recv else {
        panic!("namespace {ns} is {recv:?}");
    };
    match o.call_method(ctx, meth, &vals) {
        Some(r) => r,
        None => Err(go_value::Error::new(format!("no method {meth}"))),
    }
}

/// The value of an argument.
pub fn resolve(env: &Env, ctx: HostCtx<'_>, a: &J) -> go_value::Result<Value> {
    match a["t"].as_str().unwrap() {
        "@page" => {
            let p = a["path"].as_str().unwrap();
            match env.site.0.get_page(&[p.to_string()]) {
                Ok(Some(p)) => Ok(p.to_value()),
                Ok(None) => Err(go_value::Error::new(format!("no page {p}"))),
                Err(e) => Err(e.into()),
            }
        }
        "@pages" => Ok(match a["which"].as_str().unwrap() {
            "regular" => nh_page::page::pages_to_value(&env.site.0.regular_pages()),
            "all" => nh_page::page::pages_to_value(&env.site.0.pages()),
            w => panic!("{w}"),
        }),
        "@call" => invoke(
            env,
            ctx,
            a["m"].as_str().unwrap(),
            a["a"].as_array().unwrap(),
        ),
        _ => Ok(crate::support::decode_value(a, &[])),
    }
}

/// The context of a case.
pub fn case_ctx(env: &Env, c: &J) -> TplContext {
    let ctx = TplContext::default();
    match c.get("ctx").and_then(|x| x.get("page")) {
        Some(p) => {
            let p = env
                .site
                .0
                .get_page(&[p.as_str().unwrap().to_string()])
                .unwrap()
                .unwrap();
            ctx.with_page(p.to_value())
        }
        None => ctx,
    }
}

fn fbits_enc(f: f64) -> String {
    format!("{:016x}", f.to_bits())
}

fn sha(b: &[u8]) -> String {
    use sha2::Digest;
    let d = sha2::Sha256::digest(b);
    d.iter().map(|b| format!("{b:02x}")).collect()
}

fn s(b: &[u8]) -> J {
    crate::support::str_enc(b)
}

/// Encodes a result like the oracle's encoder.
pub fn encode(env: &Env, v: &Value) -> J {
    use go_time::GoTimeExt;
    match v {
        Value::Invalid => json!({"t": "nil"}),
        Value::TypedNil(t) => json!({"t": format!("nil:{t}")}),
        Value::Bool(b) => json!({"t": "bool", "v": b}),
        Value::Int(i, k) => json!({"t": k.go_name(), "v": i.to_string()}),
        Value::Uint(u, k) => json!({"t": k.go_name(), "v": u.to_string()}),
        Value::Float(f, FloatKind::F64) => json!({"t": "float64", "v": fbits_enc(*f)}),
        Value::Float(f, FloatKind::F32) => json!({"t": "float32", "v": fbits_enc(*f)}),
        Value::String(x) => json!({"t": "string", "s": s(x.as_bytes())}),
        Value::Safe(k, x) => json!({"t": k.go_name(), "s": s(x.as_bytes())}),
        Value::Time(t) => {
            let (abbr, off) = t.zone();
            json!({
                "t": "time.Time", "unix": t.go_unix(), "nsec": t.nanosecond(),
                "loc": go_time::location_string(t.loc.as_ref()), "abbr": abbr, "off": off,
            })
        }
        Value::List(l) => {
            if matches!(&l.ty, go_value::SliceType::Named(n) if &**n == "[]fs.FileInfo") {
                return json!({"t": "fileinfos", "items": l.items.iter().map(|i| {
                    let e = encode(env, i);
                    let mut o = json!({"name": e["name"], "dir": e["dir"]});
                    if e["dir"] == json!(false) {
                        o["size"] = e["size"].clone();
                    }
                    o
                }).collect::<Vec<_>>()});
            }
            json!({"t": l.ty.go_name(), "items": l.items.iter().map(|i| encode(env, i)).collect::<Vec<_>>()})
        }
        Value::Map(m) => json!({
            "t": m.ty.go_name(),
            "entries": m.entries.iter().map(|(k, v)| json!([s(k.as_bytes()), encode(env, v)])).collect::<Vec<_>>(),
        }),
        Value::Object(o) => encode_object(env, v, o),
    }
}

fn call0(o: &Arc<dyn Object>, name: &str) -> Value {
    o.call_method(&(), name, &[])
        .unwrap_or_else(|| panic!("no method {name}"))
        .unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn encode_object(env: &Env, v: &Value, o: &Arc<dyn Object>) -> J {
    use nh_tplfuncs::*;
    if let Some(p) = page_from_value(v) {
        return json!({"t": o.type_name(), "page": s(p.0.path().as_bytes()), "lang": p.0.lang()});
    }
    if let Some(site) = v.downcast::<SiteRef>() {
        return json!({"t": "*page.siteWrapper", "site": site.0.language().lang, "title": s(site.0.title().as_bytes())});
    }
    if let Some(r) = nh_resource::resourcetypes::resource_from_value_any(v) {
        let mt = r.media_type().typ.clone();
        let mut out = serde_json::Map::new();
        out.insert("t".into(), json!(r.tpl_type_name()));
        out.insert("name".into(), s(r.name().as_bytes()));
        out.insert("mt".into(), json!(mt));
        let rel = r.rel_permalink();
        out.insert("rel".into(), s(rel.as_bytes()));
        if let Ok(b) = nh_hugofs::afero::read_file(env.d.fs().publish_dir.as_ref(), &rel) {
            let b = go_unicode::bytes::replace(&b, env.dir.as_bytes(), b"/SITE", -1);
            out.insert("pub".into(), json!(sha(&b)));
        }
        if !mt.starts_with("image/")
            && let Some(c) = r.content(&TplContext::default())
        {
            match c {
                Ok(c) => {
                    out.insert("content".into(), s(&go_fmt::sprint(&[c])));
                }
                Err(e) => {
                    out.insert("contentErr".into(), s(e.message().as_bytes()));
                }
            }
        }
        if mt.starts_with("image/") {
            for (k, m) in [("w", "Width"), ("h", "Height")] {
                if let Some(Ok(Value::Int(n, _))) = r.tpl_call_method(&(), m, &[]) {
                    out.insert(k.into(), json!(n));
                }
            }
        }
        return J::Object(out);
    }
    let t = o.type_name().into_owned();
    match t.as_str() {
        "neohugo.HugoInfo" => {
            return json!({
                "t": "neohugo.HugoInfo", "env": s(o.field("Environment").unwrap().as_go_string().unwrap().as_bytes()),
                "prod": call0(o, "IsProduction") == Value::Bool(true), "dev": call0(o, "IsDevelopment") == Value::Bool(true),
                "server": call0(o, "IsServer") == Value::Bool(true), "multilingual": call0(o, "IsMultilingual") == Value::Bool(true),
                "multihost": call0(o, "IsMultihost") == Value::Bool(true),
            });
        }
        "paths.DirFile" => {
            let d = v.downcast::<path::path::DirFile>().unwrap();
            return json!({"t": t, "dir": s(d.dir.as_bytes()), "file": s(d.file.as_bytes())});
        }
        "*url.URL" => {
            let u = &v.downcast::<urls::urls::UrlObject>().unwrap().0;
            return json!({
                "t": t, "s": s(&u.string()), "scheme": s(&u.scheme), "opaque": s(&u.opaque), "host": s(&u.host),
                "path": s(&u.path), "rawPath": s(&u.raw_path), "rawQuery": s(&u.raw_query), "fragment": s(&u.fragment),
                "abs": u.is_abs(), "user": u.user.is_some(),
            });
        }
        "image.Config" => {
            let c = v.downcast::<images::images::ImageConfig>().unwrap();
            return json!({"t": t, "w": c.width, "h": c.height});
        }
        "*os.fileStat" => {
            let f = &v.downcast::<os::os::FileInfoObject>().unwrap().0;
            let mut o = json!({"t": "fileinfo", "name": s(f.name().as_bytes()), "dir": f.is_dir()});
            if !f.is_dir() {
                o["size"] = json!(f.size);
            }
            return o;
        }
        "*tpl.CurrentTemplateInfo" => {
            let c = &v
                .downcast::<templates::templates::CurrentTemplateInfoObject>()
                .unwrap()
                .0;
            return json!({"t": t, "name": c.name, "filename": c.filename, "level": c.level});
        }
        "images.filter" => {
            let f = v.downcast::<nh_images::filters::FilterObject>().unwrap();
            return json!({"t": t, "vals": encode(env, &f.vals), "f": f.filter_type_name});
        }
        _ => {}
    }
    if let Some(u) = o.underlying() {
        return json!({"t": "named", "name": t, "under": encode(env, &u)});
    }
    json!({"t": format!("other:{t}"), "s": s(&go_fmt::sprint(std::slice::from_ref(v)))})
}

/// Replaces memory addresses (0x + 6 or more hex digits) with 0xADDR.
pub fn mask_addrs(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'0' && i + 1 < b.len() && b[i + 1] == b'x' {
            let n = b[i + 2..]
                .iter()
                .take_while(|c| matches!(c, b'0'..=b'9' | b'a'..=b'f'))
                .count();
            if n >= 6 {
                out.push_str("0xADDR");
                i += 2 + n;
                continue;
            }
        }
        let ch = s[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

/// The result of a case: `{"ok": value}` or `{"err": text}`, with the site directory replaced
/// by /SITE and the addresses masked.
pub fn result(env: &Env, r: &go_value::Result<Value>) -> J {
    let mut j = match r {
        Ok(v) => json!({"ok": encode(env, v)}),
        Err(e) => json!({"err": s(e.message().as_bytes())}),
    };
    mask(&mut j, &env.dir, "");
    j
}

/// The oracle's masks: addresses in the `err` and `s` strings, the site directory everywhere.
fn mask(j: &mut J, dir: &str, key: &str) {
    match j {
        J::String(x) => {
            let mut y = x.replace(&format!("{dir}/"), "/SITE/");
            if let Some(prefix) = y.strip_suffix(dir) {
                y = format!("{prefix}/SITE");
            }
            if key == "err" || key == "s" {
                y = mask_addrs(&y);
            }
            *x = y;
        }
        J::Array(a) => a.iter_mut().for_each(|x| mask(x, dir, "")),
        J::Object(m) => m.iter_mut().for_each(|(k, x)| mask(x, dir, k)),
        _ => {}
    }
}

#[allow(dead_code)]
fn _unused(_: PageRef) {}
