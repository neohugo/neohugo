//! Shared helpers of the nh-resources oracle tests: fixture loading, the typed value decoding
//! of `tools/go-oracle/nh-resources/rsupport` (`Enc`), a site's resource specs built like the
//! oracle's `LoadSite` (one per language, shared `SpecCommon` and memory cache, the publish dir
//! in memory, cold cache dirs), and the record of a resource's attributes (`Rec`).

#![allow(dead_code)]

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use go_value::{GoString, Map, MapType, Value};
use nh_config::config_provider::{AllProvider, Provider};
use nh_config::default_config_provider::DefaultConfigProvider;
use nh_helpers::pathspec::PathSpec;
use nh_hugofs::afero::Fs as AferoFs;
use nh_resources::resource_spec::Spec;
use nh_resources::transform::ResourceAdapter;
use serde_json::{Value as J, json};
use sha2::{Digest, Sha256};

pub const PLACEHOLDER: &str = "$ROOT";

/// Wraps the source file system of a test site.
pub type WrapFs<'a> = &'a dyn Fn(Arc<dyn AferoFs>) -> Arc<dyn AferoFs>;

/// The repository root.
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// Reads a gzipped JSON fixture of `tests/fixtures/<rel>`.
pub fn fixture(rel: &str) -> J {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(rel);
    let f = std::fs::File::open(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
    let mut s = String::new();
    flate2::read::GzDecoder::new(f)
        .read_to_string(&mut s)
        .unwrap();
    serde_json::from_str(&s).unwrap()
}

pub fn sha(b: &[u8]) -> String {
    let d = Sha256::digest(b);
    d.iter().map(|b| format!("{b:02x}")).collect()
}

/// Decodes `rsupport.Enc`.
pub fn dec(v: &J) -> Value {
    let t = v["t"].as_str().unwrap();
    let x = &v["v"];
    match t {
        "nil" => Value::Invalid,
        "nilmap" => Value::TypedNil(Arc::from("map[string]interface {}")),
        "string" => Value::string(x.as_str().unwrap()),
        "bool" => Value::Bool(x.as_bool().unwrap()),
        "int" => Value::int(x.as_i64().unwrap()),
        "int64" => Value::int64(x.as_i64().unwrap()),
        "uint64" => Value::Uint(
            x.as_str().unwrap().parse().unwrap(),
            go_value::UintKind::Uint64,
        ),
        "float64" => Value::Float(
            x.as_str().unwrap().parse().unwrap(),
            go_value::FloatKind::F64,
        ),
        "[]string" => Value::string_list(x.as_array().unwrap().iter().map(|s| s.as_str().unwrap())),
        "[]any" => Value::any_list(x.as_array().unwrap().iter().map(dec).collect()),
        "maps.Params" => Value::map(dec_map(x, MapType::Params)),
        "map" => Value::map(dec_map(x, MapType::StringAny)),
        other => panic!("unknown encoded type {other}"),
    }
}

pub fn dec_map(v: &J, ty: MapType) -> Map {
    let mut m = Map::new(ty);
    for e in v.as_array().unwrap() {
        let k = e[0].as_str().unwrap();
        m.entries.insert(GoString::from(k), dec(&e[1]));
    }
    m
}

/// A decoded `map[string]any` (`None` for a nil map).
pub fn dec_opt_map(v: &J) -> Option<Map> {
    match dec(v) {
        Value::Map(m) => Some((*m).clone()),
        _ => None,
    }
}

static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// A temporary directory, removed on drop.
pub struct TempDir {
    pub path: PathBuf,
}

impl TempDir {
    pub fn new(prefix: &str) -> TempDir {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!(
            "nh-resources-rs-{prefix}-{}-{n}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        TempDir {
            path: std::fs::canonicalize(&path).unwrap(),
        }
    }

    pub fn str(&self) -> String {
        self.path.to_string_lossy().into_owned()
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// A loaded site: one resource spec per language.
pub struct Site {
    pub dir: String,
    pub specs: Vec<Arc<Spec>>,
    pub langs: Vec<String>,
    pub publish: Arc<dyn AferoFs>,
    pub publish_dir: String,
    pub tmp: TempDir,
    pub log: Arc<Mutex<Vec<u8>>>,
}

/// Go: `rsupport.LoadSite(dir)` (the source file system can be wrapped, e.g. to poison
/// `resources/_gen`).
pub fn load_site(dir: &str, wrap_source: Option<WrapFs<'_>>) -> Site {
    let tmp = TempDir::new("site");
    load_site_in(dir, tmp, wrap_source)
}

pub fn load_site_in(dir: &str, tmp: TempDir, wrap_source: Option<WrapFs<'_>>) -> Site {
    let flags = DefaultConfigProvider::new();
    flags.set("workingDir", Value::string(dir));
    flags.set("noBuildLock", Value::Bool(true));
    flags.set("cacheDir", Value::string(format!("{}/cache", tmp.str())));
    flags.set(
        "resourceDir",
        Value::string(format!("{}/resources", tmp.str())),
    );

    let log_buf: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
    let sink: nh_common::loggers::LogSink = log_buf.clone();
    let logger = nh_common::loggers::Logger::with_options(nh_common::loggers::Options {
        level: nh_common::loggers::Level::Warn,
        std_out: Some(sink.clone()),
        std_err: Some(sink),
        ..Default::default()
    });

    let configs = nh_allconfig::load::load_config(nh_allconfig::load::ConfigSourceDescriptor {
        flags: Some(Arc::new(flags)),
        filename: format!("{dir}/hugo.toml"),
        config_dir: "config".to_string(),
        environ: vec!["NEOHUGO_ORACLE=1".to_string()],
        working_dir: dir.to_string(),
        getenv: Some(Arc::new(|_k: &str| String::new())),
        logger: Some(logger.clone()),
        ..Default::default()
    })
    .unwrap_or_else(|e| panic!("load config {dir}: {e}"));

    let publish_dir = configs.loading_info.base_config.publish_dir.clone();
    let fs_cfg = DefaultConfigProvider::new();
    fs_cfg.set("workingDir", Value::string(dir));
    fs_cfg.set("publishDir", Value::string(publish_dir.as_str()));
    let publish = nh_hugofs::afero::new_mem_map_fs();
    let mut source = nh_hugofs::afero::new_os_fs();
    if let Some(w) = wrap_source {
        source = w(source);
    }
    let hfs = nh_hugofs::fs::new_from_source_and_destination(source, publish.clone(), &fs_cfg);
    let mem_cache = nh_common::dynacache::Cache::new(Default::default());

    let mut specs: Vec<Arc<Spec>> = Vec::new();
    let mut langs = Vec::new();
    let mut first_ps: Option<Arc<PathSpec>> = None;
    let mut file_caches = None;
    for conf in configs.config_langs() {
        let ps = match &first_ps {
            None => {
                let ps = PathSpec::new(hfs.clone(), conf.clone()).unwrap();
                file_caches =
                    Some(nh_helpers::cache::filecache::filecache::new_caches(&ps).unwrap());
                first_ps = Some(ps.clone());
                ps
            }
            Some(first) => {
                PathSpec::new_with_base_fs(hfs.clone(), conf.clone(), Some(first.base_fs.clone()))
                    .unwrap()
            }
        };
        let common = specs.first().map(|s| s.common.clone());
        let spec = Spec::new(
            ps,
            common,
            file_caches.clone().unwrap(),
            &mem_cache,
            None,
            nh_config::hexec::Exec::new_with_env(Default::default(), dir, &[], None),
            Some(logger.clone()),
        )
        .unwrap();
        langs.push(conf.language().lang.clone());
        specs.push(spec);
    }

    let abs_publish = if publish_dir.starts_with('/') {
        publish_dir.clone()
    } else {
        format!("{dir}/{publish_dir}")
    };

    Site {
        dir: dir.to_string(),
        specs,
        langs,
        publish,
        publish_dir: abs_publish,
        tmp,
        log: log_buf,
    }
}

impl Site {
    /// Go: `Site.PublishedFiles()`: `path -> "len:sha256"`, relative to the publish dir.
    pub fn published_files(&self) -> J {
        let mut out = serde_json::Map::new();
        walk(self.publish.as_ref(), "/", &mut |p, b| {
            let rel = p
                .strip_prefix(&format!("{}/", self.publish_dir))
                .unwrap_or(p)
                .to_string();
            out.insert(rel, J::String(format!("{}:{}", b.len(), sha(b))));
        });
        J::Object(out)
    }

    /// The bytes of a published file (relative to the publish dir).
    pub fn published(&self, rel: &str) -> Option<Vec<u8>> {
        nh_hugofs::afero::read_file(
            self.publish.as_ref(),
            &format!("{}/{}", self.publish_dir, rel.trim_start_matches('/')),
        )
        .ok()
    }
}

fn walk(fs: &dyn AferoFs, dir: &str, f: &mut dyn FnMut(&str, &[u8])) {
    let Ok(entries) = nh_hugofs::afero::read_dir(fs, dir) else {
        return;
    };
    for e in entries {
        let p = if dir.ends_with('/') {
            format!("{dir}{}", e.name())
        } else {
            format!("{dir}/{}", e.name())
        };
        if e.is_dir() {
            walk(fs, &p, f);
        } else if let Ok(b) = nh_hugofs::afero::read_file(fs, &p) {
            f(&p, &b);
        }
    }
}

fn marshal(v: &Value) -> String {
    match go_json::marshal(v) {
        Ok(b) => String::from_utf8_lossy(&b).into_owned(),
        Err(e) => format!("ERR: {e}"),
    }
}

/// Go: `rsupport.Rec(r, withContent)`.
pub fn rec(r: &ResourceAdapter, with_content: bool) -> J {
    let mut o = serde_json::Map::new();
    o.insert("name".into(), json!(r.name()));
    o.insert("title".into(), json!(r.title()));
    o.insert("key".into(), json!(r.key()));
    o.insert("nameNormalized".into(), json!(r.name_normalized()));
    let mt = r.media_type();
    o.insert("mediaType".into(), json!(mt.typ));
    o.insert(
        "mediaTypeJSON".into(),
        json!(String::from_utf8_lossy(&mt.marshal_json_bytes().unwrap()).into_owned()),
    );
    o.insert("resourceType".into(), json!(r.resource_type()));
    o.insert("data".into(), json!(marshal(&r.data())));
    let params = match r.params() {
        Some(p) => Value::Map(p),
        None => Value::TypedNil(Arc::from("maps.Params")),
    };
    o.insert("params".into(), json!(marshal(&params)));
    if with_content {
        let ctx = nh_tpl::template::TplContext::default();
        match r.content(ctx.as_host()) {
            Err(e) => {
                o.insert("contentErr".into(), json!(e.message()));
            }
            Ok(v) => {
                let s = v.as_go_string().unwrap().as_bytes().to_vec();
                if mt.is_text() || mt.main_type == "text" || mt.sub_type == "json" {
                    o.insert("content".into(), json!(String::from_utf8_lossy(&s)));
                } else {
                    o.insert("contentSha".into(), json!(sha(&s)));
                }
                o.insert("contentLen".into(), json!(s.len()));
            }
        }
    }
    if r.resource_type() == "image" {
        match (r.width(), r.height()) {
            (Ok(w), Ok(h)) => {
                o.insert("width".into(), json!(w));
                o.insert("height".into(), json!(h));
            }
            (Err(e), _) | (_, Err(e)) => {
                o.insert("whPanic".into(), json!(e.message()));
            }
        }
    }
    o.insert("relPermalink".into(), json!(r.rel_permalink()));
    o.insert("permalink".into(), json!(r.permalink()));
    J::Object(o)
}

/// Compares two JSON objects key by key (`skip` keys are ignored); returns the differences.
pub fn diff(what: &str, want: &J, got: &J, skip: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    let (Some(w), Some(g)) = (want.as_object(), got.as_object()) else {
        if want != got {
            out.push(format!("{what}: want {want}, got {got}"));
        }
        return out;
    };
    let mut keys: Vec<&String> = w.keys().chain(g.keys()).collect();
    keys.sort();
    keys.dedup();
    for k in keys {
        if skip.contains(&k.as_str()) {
            continue;
        }
        if w.get(k) != g.get(k) {
            out.push(format!(
                "{what}.{k}: want {}, got {}",
                w.get(k).unwrap_or(&J::Null),
                g.get(k).unwrap_or(&J::Null)
            ));
        }
    }
    out
}

/// An opener of a repository file.
pub fn open_file(path: PathBuf) -> nh_common::hugio::OpenReadSeekCloser {
    Arc::new(move || {
        let f = std::fs::File::open(&path).map_err(|e| nh_common::Error::new(e.to_string()))?;
        Ok(Box::new(f) as Box<dyn nh_common::hugio::ReadSeekCloser>)
    })
}

/// `AllProvider` of a spec (for the minifier client).
pub fn cfg_of(spec: &Spec) -> Arc<dyn AllProvider> {
    spec.path_spec.cfg.clone()
}

pub fn provider_unused(_p: &dyn Provider) {}
