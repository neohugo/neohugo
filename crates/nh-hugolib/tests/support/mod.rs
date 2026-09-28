//! Shared helpers of the nh-hugolib oracle tests: the fixtures of
//! `tools/go-oracle/nh-hugolib/*`, recreating an oracle site in a temporary directory, loading
//! its config and creating the `HugoSites` like the Go oracle does (hsupport.New), with the
//! names-only func map (every function name of `tplimplinit.CreateFuncMap` bound to a stub),
//! and the typed value encoding of `tools/go-oracle/nh-common/goval`.

#![allow(dead_code)]

use std::io::Read;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use go_value::{FloatKind, Map, MapType, Value};
use nh_allconfig::load::{ConfigSourceDescriptor, load_config};
use nh_common::loggers::{Level, LogSink, Logger, Options};
use nh_config::config_provider::Provider;
use nh_config::default_config_provider::DefaultConfigProvider;
use nh_hugolib::hugo_sites::{FuncMapFactory, HugoSites, NewHugoSitesCfg};
use nh_tplimpl::engine::{FuncMap, TplFunc};
use serde_json::{Value as J, json};

/// The repository root (the fixtures reference repository files).
pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Reads `tests/fixtures/<rel>` (gunzipped when it ends in `.gz`).
pub fn fixture(rel: &str) -> J {
    let path = format!("{}/tests/fixtures/{rel}", env!("CARGO_MANIFEST_DIR"));
    let raw = std::fs::read(&path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let text = if rel.ends_with(".gz") {
        let mut s = String::new();
        flate2::read::GzDecoder::new(&raw[..])
            .read_to_string(&mut s)
            .unwrap();
        s
    } else {
        String::from_utf8(raw).unwrap()
    };
    serde_json::from_str(&text).unwrap()
}

/// The names-only func map: every name of Go's `tplimplinit.CreateFuncMap` (the `funcnames`
/// fixture) bound to a stub that fails when called. The template store only needs the names.
pub fn names_only_func_map_factory() -> FuncMapFactory {
    let fx = fixture("funcnames/funcnames.json.gz");
    let names: Vec<String> = fx["funcNames"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n.as_str().unwrap().to_string())
        .collect();
    Arc::new(move |_d| {
        let mut m = FuncMap::new();
        for n in &names {
            let name = n.clone();
            let f: TplFunc = Arc::new(move |_ctx, _args| {
                Err(go_value::Error::new(format!("stub function {name} called")))
            });
            m.insert(n.clone(), f);
        }
        m
    })
}

/// FNV-1a 64 as 16 hex digits (hsupport.FNV).
pub fn fnv(b: &[u8]) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for &c in b {
        h ^= c as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}")
}

/// A temporary directory removed on drop.
pub struct TempDir(pub PathBuf);

impl TempDir {
    pub fn new(name: &str) -> TempDir {
        static N: AtomicUsize = AtomicUsize::new(0);
        let p = std::env::temp_dir().join(format!(
            "nh-hugolib-{name}-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        TempDir(p)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Writes the site recorded in a fixture (`site`: name, toml, files) into `tmp/<name>`; a
/// repository file must still have its recorded hash. Returns the site directory.
pub fn write_site(site: &J, tmp: &std::path::Path) -> PathBuf {
    let name = site["name"].as_str().unwrap();
    let dir = tmp.join(name);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("hugo.toml"), site["toml"].as_str().unwrap()).unwrap();
    let root = repo_root();
    for f in site["files"].as_array().unwrap() {
        let p = f["path"].as_str().unwrap();
        let content: Vec<u8> = if let Some(repo) = f["repo"].as_str() {
            let b = std::fs::read(root.join(repo)).unwrap_or_else(|e| panic!("{repo}: {e}"));
            assert_eq!(
                fnv(&b),
                f["fnv"].as_str().unwrap(),
                "{repo} changed since the fixture was recorded: regenerate it"
            );
            b
        } else {
            f["content"].as_str().unwrap().as_bytes().to_vec()
        };
        let fp = dir.join(p);
        std::fs::create_dir_all(fp.parent().unwrap()).unwrap();
        std::fs::write(fp, content).unwrap();
    }
    dir
}

/// A site whose `HugoSites` is created.
pub struct Built {
    pub h: HugoSites,
    pub dir: String,
    pub log: Arc<Mutex<Vec<u8>>>,
}

impl Built {
    /// Replaces the site directory with "/SITE" (hsupport's Norm).
    pub fn norm(&self, s: &str) -> String {
        s.replace(&self.dir, "/SITE")
    }

    /// The non-empty lines of the HugoSites log, normalised.
    pub fn log_lines(&self) -> Vec<String> {
        let b = self.log.lock().unwrap().clone();
        let s = self.norm(&String::from_utf8_lossy(&b));
        s.split('\n')
            .filter(|l| !l.is_empty())
            .map(|l| l.to_string())
            .collect()
    }
}

/// hsupport.New: writes the site into `tmp`, loads its config like a build (`production`,
/// no process environment, the caches below `tmp/_cache`) and creates the `HugoSites` with
/// the names-only func map; the HugoSites logs (warn level, distinct) to `Built::log`.
pub fn new_sites(site: &J, tmp: &std::path::Path) -> nh_common::Result<Built> {
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

    let fs = nh_hugofs::fs::new_from(
        nh_hugofs::afero::new_os_fs(),
        &configs.loading_info.base_config,
    );
    let h = HugoSites::new(NewHugoSitesCfg {
        configs: Arc::new(configs),
        fs,
        log: logger,
        func_map_factory: Some(names_only_func_map_factory()),
    })?;
    Ok(Built { h, dir: dirs, log })
}

// ---------------------------------------------------------------------------
// The typed value encoding of tools/go-oracle/nh-common/goval.

/// A string as goval encodes it.
pub fn str_enc(b: &[u8]) -> J {
    match std::str::from_utf8(b) {
        Ok(s) => J::String(s.to_string()),
        Err(_) => json!({"hex": b.iter().map(|c| format!("{c:02x}")).collect::<String>()}),
    }
}

fn fbits_enc(f: f64) -> String {
    format!("{:016x}", f.to_bits())
}

/// Encodes a value like goval.Encode.
pub fn encode(v: &Value) -> J {
    match v {
        Value::Invalid => json!({"t": "nil"}),
        Value::TypedNil(t) => json!({"t": format!("nil:{t}")}),
        Value::Bool(b) => json!({"t": "bool", "v": b}),
        Value::Int(i, k) => json!({"t": k.go_name(), "v": i.to_string()}),
        Value::Uint(u, k) => json!({"t": k.go_name(), "v": u.to_string()}),
        Value::Float(f, FloatKind::F64) => json!({"t": "float64", "v": fbits_enc(*f)}),
        Value::Float(f, FloatKind::F32) => json!({"t": "float32", "v": fbits_enc(*f)}),
        Value::String(s) => json!({"t": "string", "s": str_enc(s.as_bytes())}),
        Value::Safe(k, s) => json!({"t": k.go_name(), "s": str_enc(s.as_bytes())}),
        Value::Time(t) => {
            use go_time::GoTimeExt;
            let (abbr, off) = t.zone();
            json!({
                "t": "time.Time", "unix": t.go_unix(), "nsec": t.nanosecond(),
                "loc": go_time::location_string(t.loc.as_ref()), "abbr": abbr, "off": off,
            })
        }
        Value::List(l) => {
            json!({"t": l.ty.go_name(), "items": l.items.iter().map(encode).collect::<Vec<_>>()})
        }
        Value::Map(m) => encode_map(m),
        Value::Object(o) => {
            if let Some(s) = o.go_string() {
                return json!({"t": o.type_name(), "s": str_enc(s.as_bytes())});
            }
            if let Some(u) = o.underlying() {
                return json!({"t": "named", "name": o.type_name(), "under": encode(&u)});
            }
            panic!("cannot encode {v:?}")
        }
    }
}

/// Encodes a map like goval.Encode.
pub fn encode_map(m: &Map) -> J {
    json!({
        "t": m.ty.go_name(),
        "entries": m.entries.iter().map(|(k, v)| json!([str_enc(k.as_bytes()), encode(v)])).collect::<Vec<_>>(),
    })
}

/// A `maps.Params` map (for Go values that are `maps.Params` whatever the Rust map type).
pub fn encode_params(m: &Map) -> J {
    let mut m = m.clone();
    m.ty = MapType::Params;
    encode_map(&m)
}

/// The first difference between two JSON values (a path and both sides), if any.
pub fn first_diff(path: &str, want: &J, got: &J) -> Option<String> {
    match (want, got) {
        (J::Object(a), J::Object(b)) => {
            let mut keys: Vec<&String> = a.keys().chain(b.keys()).collect();
            keys.sort();
            keys.dedup();
            for k in keys {
                let (x, y) = (a.get(k).unwrap_or(&J::Null), b.get(k).unwrap_or(&J::Null));
                if let Some(d) = first_diff(&format!("{path}.{k}"), x, y) {
                    return Some(d);
                }
            }
            None
        }
        (J::Array(a), J::Array(b)) => {
            for i in 0..a.len().max(b.len()) {
                let (x, y) = (a.get(i).unwrap_or(&J::Null), b.get(i).unwrap_or(&J::Null));
                if let Some(d) = first_diff(&format!("{path}[{i}]"), x, y) {
                    return Some(d);
                }
            }
            None
        }
        _ => {
            if want == got {
                None
            } else {
                Some(format!("{path}:\n  want {want}\n  got  {got}"))
            }
        }
    }
}
