//! Shared by the T16 (js-css-pipeline) oracle tests: the Rust side of
//! tools/go-oracle/nh-resource-transformers/t16support. A hermetic copy of the fixture site, its
//! resource spec with an exec helper, the script runner over the Rust clients (resources.Get,
//! resources.Concat, js.Build, toCSS, postCSS, minify, fingerprint) recording what the oracle
//! records, and the comparison.

#![allow(dead_code)] // Each test binary uses a part of it.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use go_value::{GoString, Map, MapType, Value};
use nh_config::config_provider::{Provider, config_section};
use nh_config::default_config_provider::DefaultConfigProvider;
use nh_helpers::pathspec::PathSpec;
use nh_hugofs::afero::Fs as AferoFs;
use nh_resource::resourcetypes::Resource;
use nh_resource_transformers::resource_factories::{bundler, create::create};
use nh_resource_transformers::resource_transformers::cssjs::postcss;
use nh_resource_transformers::resource_transformers::tocss::scss::tocss;
use nh_resource_transformers::resource_transformers::{integrity, js, minifier};
use nh_resources::resource_spec::Spec;
use serde_json::{Value as J, json};

pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// A gzipped JSON fixture under tests/fixtures.
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

/// Prints a visible skip line (stderr is not captured when written directly).
pub fn skip(why: &str) {
    let _ = writeln!(std::io::stderr(), "SKIPPED: {why}");
}

/// The pinned esbuild binary, or `None` (with a visible skip line) when
/// `NEOHUGO_ESBUILD_BINARY` is not set.
pub fn esbuild_binary() -> Option<String> {
    match std::env::var("NEOHUGO_ESBUILD_BINARY") {
        Ok(b) if !b.is_empty() => Some(b),
        _ => {
            skip(
                "NEOHUGO_ESBUILD_BINARY is not set (tools/esbuild/build.sh builds the pinned esbuild 0.25.6)",
            );
            None
        }
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
        let path =
            std::env::temp_dir().join(format!("nh-t16-rs-{prefix}-{}-{n}", std::process::id()));
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

/// Go: `t16support.CopySite(src, only, links)`: the copy is `<tmp>/site`; `_node_modules`
/// becomes `node_modules`.
pub fn copy_site(src: &Path, only: Option<&[String]>, links: &[(String, PathBuf)]) -> TempDir {
    let tmp = TempDir::new("site");
    let dst = tmp.path.join("site");
    copy_dir(src, &dst, src, only);
    for (name, target) in links {
        let p = dst.join(name);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::os::unix::fs::symlink(target, &p).unwrap();
    }
    tmp
}

fn copy_dir(dir: &Path, dst_root: &Path, src_root: &Path, only: Option<&[String]>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap())
        .collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        let p = e.path();
        let rel = p.strip_prefix(src_root).unwrap();
        let rel_s = rel.to_string_lossy().into_owned();
        if let Some(only) = only {
            let top = rel_s.split('/').next().unwrap();
            if !only.iter().any(|o| o == top) {
                continue;
            }
        }
        let rel_s = match rel_s.strip_prefix("_node_modules") {
            Some(rest) if rest.is_empty() || rest.starts_with('/') => format!("node_modules{rest}"),
            _ => rel_s,
        };
        let target = dst_root.join(rel_s);
        if e.file_type().unwrap().is_dir() {
            std::fs::create_dir_all(&target).unwrap();
            copy_dir(&p, dst_root, src_root, only);
        } else {
            std::fs::create_dir_all(target.parent().unwrap()).unwrap();
            std::fs::copy(&p, &target).unwrap();
        }
    }
}

/// A loaded site with the clients of the scripts.
pub struct Site {
    pub dir: String,
    pub spec: Arc<Spec>,
    pub publish: Arc<dyn AferoFs>,
    pub publish_dir: String,
    pub tmp: TempDir,
    pub log: Arc<Mutex<Vec<u8>>>,
    create: Arc<create::Client>,
    bundler: bundler::Client,
    js: js::build::Client,
    scss: tocss::Client,
    postcss: postcss::PostCssClient,
    minifier: minifier::Client,
    integrity: integrity::Client,
    results: HashMap<String, Arc<dyn Resource>>,
    published: HashSet<String>,
}

/// Go: `t16support.LoadSite(dir)`. `environ` is the process environment the exec helper
/// filters (Go: `os.Environ()`).
pub fn load_site(dir: &str, environ: &[String]) -> Site {
    let tmp = TempDir::new("cache");
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
    let source = nh_hugofs::afero::new_os_fs();
    let hfs = nh_hugofs::fs::new_from_source_and_destination(source, publish.clone(), &fs_cfg);
    let mem_cache = nh_common::dynacache::Cache::new(Default::default());

    let conf = configs.config_langs().into_iter().next().unwrap();
    let ps = PathSpec::new(hfs.clone(), conf.clone()).unwrap();
    let file_caches = nh_helpers::cache::filecache::filecache::new_caches(&ps).unwrap();
    let sc = (*config_section::<nh_config::security::security_config::Config>(
        conf.as_ref(),
        "security",
    ))
    .clone();
    let path_env = environ
        .iter()
        .find_map(|e| e.strip_prefix("PATH="))
        .map(str::to_string);
    let spec = Spec::new(
        ps.clone(),
        None,
        file_caches,
        &mem_cache,
        None,
        nh_config::hexec::Exec::new_with_env(sc, dir, environ, path_env),
        Some(logger.clone()),
    )
    .unwrap();

    let abs_publish = if publish_dir.starts_with('/') {
        publish_dir.clone()
    } else {
        format!("{dir}/{publish_dir}")
    };

    let assets = ps.base_fs.assets.clone();
    Site {
        dir: dir.to_string(),
        create: create::Client::new(spec.clone()).unwrap(),
        bundler: bundler::Client::new(spec.clone()),
        js: js::build::Client::new_default(spec.clone()),
        scss: tocss::Client::new(assets, spec.clone()).unwrap(),
        postcss: postcss::new_post_css_client(spec.clone()),
        minifier: minifier::Client::new(spec.clone()).unwrap(),
        integrity: integrity::Client::new(spec.clone()),
        spec,
        publish,
        publish_dir: abs_publish,
        tmp,
        log: log_buf,
        results: HashMap::new(),
        published: HashSet::new(),
    }
}

/// The process environment as `KEY=value` strings.
pub fn process_environ() -> Vec<String> {
    std::env::vars_os()
        .map(|(k, v)| format!("{}={}", k.to_string_lossy(), v.to_string_lossy()))
        .collect()
}

/// Decodes `rsupport.Enc`.
pub fn dec(v: &J) -> Value {
    let x = &v["v"];
    match v["t"].as_str().unwrap() {
        "nil" => Value::Invalid,
        "nilmap" => Value::TypedNil(Arc::from("map[string]interface {}")),
        "string" => Value::string(x.as_str().unwrap()),
        "bool" => Value::Bool(x.as_bool().unwrap()),
        "int" => Value::int(x.as_i64().unwrap()),
        "int64" => Value::int64(x.as_i64().unwrap()),
        "float64" => Value::Float(
            x.as_str().unwrap().parse().unwrap(),
            go_value::FloatKind::F64,
        ),
        "[]string" => Value::string_list(x.as_array().unwrap().iter().map(|s| s.as_str().unwrap())),
        "[]any" => Value::any_list(x.as_array().unwrap().iter().map(dec).collect()),
        "map" => Value::map(dec_map(x, MapType::StringAny)),
        "maps.Params" => Value::map(dec_map(x, MapType::Params)),
        "css.QuotedString" => Value::object(nh_common::types::css::QuotedString(GoString::from(
            x.as_str().unwrap(),
        ))),
        "css.UnquotedString" => Value::object(nh_common::types::css::UnquotedString(
            GoString::from(x.as_str().unwrap()),
        )),
        other => panic!("unknown encoded type {other}"),
    }
}

fn dec_map(v: &J, ty: MapType) -> Map {
    let mut m = Map::new(ty);
    for e in v.as_array().unwrap() {
        m.entries
            .insert(GoString::from(e[0].as_str().unwrap()), dec(&e[1]));
    }
    m
}

/// A step's options: `None` for a nil map.
fn opts(step: &J) -> Option<Map> {
    match dec(&step["opts"]) {
        Value::Map(m) => Some((*m).clone()),
        _ => None,
    }
}

impl Site {
    /// Go: `Site.Norm` (inline source maps are decoded first).
    pub fn norm(&self, v: &str) -> String {
        use base64::Engine;
        static RE: std::sync::LazyLock<nh_common::goregexp::Regexp> =
            std::sync::LazyLock::new(|| {
                nh_common::goregexp::Regexp::must_compile(
                    r"data:application/json;base64,([A-Za-z0-9+/=]+)",
                )
            });
        let v = RE.replace_all_func(v.as_bytes(), |m| {
            let b64 = &m[b"data:application/json;base64,".len()..];
            match base64::engine::general_purpose::STANDARD.decode(b64) {
                Ok(b) => [
                    b"data:application/json;base64,DECODED(".as_slice(),
                    &b,
                    b")",
                ]
                .concat(),
                Err(_) => m.to_vec(),
            }
        });
        let v = String::from_utf8_lossy(&v).into_owned();
        let parent = Path::new(&self.dir)
            .parent()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        v.replace(&self.dir, "$SITE").replace(&parent, "$TMP")
    }

    /// An error text, normalized, with Go's order of a wrapped positioned error (see
    /// [`canon_wrapped_file_error`]).
    fn err_text(&self, e: &nh_common::Error) -> String {
        canon_wrapped_file_error(&self.norm(&e.to_string()))
    }

    /// Go: `Site.Run` of one case.
    pub fn run(&mut self, c: &J) -> J {
        let ctx = nh_tpl::template::TplContext::default();
        let name = c["name"].as_str().unwrap().to_string();
        let mut rec = serde_json::Map::new();
        rec.insert("name".into(), json!(name));
        let mut r: Option<Arc<dyn Resource>> = None;
        for (i, st) in c["steps"].as_array().unwrap().iter().enumerate() {
            let cur = r.clone();
            let res: nh_common::Result<Arc<dyn Resource>> = match st["op"].as_str().unwrap() {
                "get" => {
                    let p = st["path"].as_str().unwrap();
                    self.create.get(p).and_then(|x| {
                        x.ok_or_else(|| {
                            nh_common::Error::new(format!("resource {} not found", go_quote(p)))
                        })
                    })
                }
                "ref" => Ok(self.results[st["name"].as_str().unwrap()].clone()),
                "concat" => {
                    let rs: Vec<Arc<dyn Resource>> = st["refs"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|n| self.results[n.as_str().unwrap()].clone())
                        .collect();
                    self.bundler.concat(st["target"].as_str().unwrap(), &rs)
                }
                "js" => self.js.process(&ctx, cur.unwrap(), opts(st).as_ref()),
                "tocss" => match tocss::decode_options(opts(st).as_ref()) {
                    Ok(o) => self.scss.to_css(&ctx, cur.unwrap(), o),
                    Err(e) => Err(e),
                },
                "postcss" => self.postcss.process(&ctx, cur.unwrap(), opts(st).as_ref()),
                "minify" => self.minifier.minify(&ctx, cur.unwrap()),
                "fingerprint" => self.integrity.fingerprint(&ctx, cur.unwrap(), ""),
                op => panic!("unknown op {op}"),
            };
            match res {
                Ok(x) => r = Some(x),
                Err(e) => {
                    rec.insert("stepErr".into(), json!(self.err_text(&e)));
                    rec.insert("stepErrAt".into(), json!(i));
                    return J::Object(rec);
                }
            }
        }
        let r = r.unwrap();
        self.results.insert(name, r.clone());
        let adapter = nh_resources::transform::resource_adapter(&r);
        if let Some(a) = &adapter {
            rec.insert("transformationKey".into(), json!(a.transformation_key()));
        }
        let content = match &adapter {
            Some(a) => a.content(ctx.as_host()),
            None => Err(nh_common::Error::new("not an adapter")),
        };
        match content {
            Err(e) => {
                rec.insert("contentErr".into(), json!(self.err_text(&e)));
            }
            Ok(v) => {
                let s = v.as_go_string().unwrap().to_str_lossy().into_owned();
                rec.insert("content".into(), json!(self.norm(&s)));
            }
        }
        rec.insert("mediaType".into(), json!(r.media_type().typ));
        let data = go_json::marshal(&r.data())
            .map(|b| String::from_utf8_lossy(&b).into_owned())
            .unwrap_or_else(|e| format!("ERR: {e}"));
        rec.insert("data".into(), json!(data));
        if c["links"].as_bool().unwrap_or(false) {
            rec.insert("relPermalink".into(), json!(r.rel_permalink()));
        }
        rec.insert("published".into(), self.new_published());
        J::Object(rec)
    }

    fn new_published(&mut self) -> J {
        let mut files = BTreeMap::new();
        walk(self.publish.as_ref(), "/", &mut |p, b| {
            files.insert(p.to_string(), b.to_vec());
        });
        let mut out = serde_json::Map::new();
        for (p, b) in files {
            if !self.published.insert(p.clone()) {
                continue;
            }
            let rel = p
                .strip_prefix(&format!("{}/", self.publish_dir))
                .unwrap_or(&p)
                .to_string();
            out.insert(rel, json!(self.norm(&String::from_utf8_lossy(&b))));
        }
        J::Object(out)
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

/// Known cross-crate text difference (reported to T01/T14): Go wraps a transformation error with
/// `fmt.Errorf("%s: %w", prefix, fileErr)`, which reads `PREFIX: "file:line:col": msg`, while
/// nh-common's `Error::wrap` keeps the position in front: `"file:line:col": PREFIX: msg`. This
/// puts the Rust text in Go's order; nothing else is rewritten.
pub fn canon_wrapped_file_error(s: &str) -> String {
    static RE: std::sync::LazyLock<nh_common::goregexp::Regexp> = std::sync::LazyLock::new(|| {
        nh_common::goregexp::Regexp::must_compile(
            r#"(?s)^("[^"]*:\d+:\d+"): ([A-Z0-9-]+: failed to transform "(?:[^"\\]|\\.)*" \([^)]*\)): (.*)$"#,
        )
    });
    RE.replace_all_string(s, "$2: $1: $3")
}

fn go_quote(s: &str) -> String {
    go_strconv::quote(s)
}

/// Compares the oracle's records with the Rust ones; returns the differences (one line each,
/// long values shortened around the first differing byte).
pub fn diff_results(topic: &str, want: &J, got: &[J]) -> Vec<String> {
    let want = want.as_array().unwrap();
    let mut out = Vec::new();
    if want.len() != got.len() {
        out.push(format!(
            "{topic}: {} records, got {}",
            want.len(),
            got.len()
        ));
    }
    for (w, g) in want.iter().zip(got) {
        let name = w["name"].as_str().unwrap_or("?");
        let (wo, go) = (w.as_object().unwrap(), g.as_object().unwrap());
        let mut keys: Vec<&String> = wo.keys().chain(go.keys()).collect();
        keys.sort();
        keys.dedup();
        for k in keys {
            let (a, b) = (wo.get(k), go.get(k));
            if a != b {
                let (sa, sb) = (
                    a.map(|v| v.to_string()).unwrap_or_default(),
                    b.map(|v| v.to_string()).unwrap_or_default(),
                );
                let at = sa
                    .bytes()
                    .zip(sb.bytes())
                    .position(|(x, y)| x != y)
                    .unwrap_or(sa.len().min(sb.len()));
                let from = sa.floor_char_boundary(at.saturating_sub(200));
                out.push(format!(
                    "{topic}/{name}.{k}: first difference at byte {at}:\n  want …{}\n  got  …{}",
                    shorten_str(&sa[from..]),
                    shorten_str(&sb[sb.floor_char_boundary(from)..])
                ));
            }
        }
    }
    out
}

fn shorten_str(s: &str) -> String {
    if s.len() > 600 {
        format!("{}…({} bytes)", &s[..s.floor_char_boundary(600)], s.len())
    } else {
        s.to_string()
    }
}
