//! Support of the `resources` namespace tests (Wave B task T15): the fixtures of
//! tools/go-oracle/nh-resource-transformers (in crates/nh-resource-transformers/tests/fixtures),
//! a site's resource spec built like the oracle's `rsupport.LoadSite` (the publish dir in
//! memory, cold cache dirs, the site's security config), the namespace over
//! `Deps::for_tests(conf).with_resource_spec(..)`, the template store of the synthetic site
//! (stubs for Hugo's function names, T18's compare/fmt/collections namespaces for the functions
//! the ExecuteAsTemplate templates call), the script interpreter of `rtsupport` and the
//! record of a result (`rtsupport.Rec`).

#![allow(dead_code)]

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use go_value::{GoString, Map, MapType, Object, SliceType, Value};
use nh_config::config_provider::{Provider, config_section};
use nh_config::default_config_provider::DefaultConfigProvider;
use nh_deps::deps::Deps;
use nh_helpers::pathspec::PathSpec;
use nh_hugofs::afero::Fs as AferoFs;
use nh_resources::resource_spec::Spec;
use nh_tplimpl::engine::{FuncMap, TplFunc};
use nh_tplimpl::templatestore::{SiteOptions, StoreOptions, TemplateStore};
use serde_json::{Value as J, json};
use sha2::{Digest, Sha256};

/// The repository root.
pub fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// Reads a gzipped JSON fixture of crates/nh-resource-transformers/tests/fixtures/<rel>.
pub fn fixture(rel: &str) -> J {
    let p = repo_root()
        .join("crates/nh-resource-transformers/tests/fixtures")
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

static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// A temporary directory, removed on drop.
pub struct TempDir {
    pub path: PathBuf,
}

impl TempDir {
    pub fn new(prefix: &str) -> TempDir {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!(
            "nh-tplfuncs-resources-{prefix}-{}-{n}",
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

/// A loaded site: the first language's resource spec.
pub struct Site {
    pub dir: String,
    pub spec: Arc<Spec>,
    pub publish: Arc<dyn AferoFs>,
    pub publish_dir: String,
    pub tmp: TempDir,
    pub log: Arc<Mutex<Vec<u8>>>,
}

/// Go: `rsupport.LoadSite(dir)` + the exec helper `rtsupport.Namespace` sets.
pub fn load_site(dir: &str) -> Site {
    let tmp = TempDir::new("site");
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
    let spec = Spec::new(
        ps,
        None,
        file_caches,
        &mem_cache,
        None,
        nh_config::hexec::Exec::new_with_env(sc, dir, &[], None),
        Some(logger.clone()),
    )
    .unwrap();

    let abs_publish = if publish_dir.starts_with('/') {
        publish_dir.clone()
    } else {
        format!("{dir}/{publish_dir}")
    };

    Site {
        dir: dir.to_string(),
        spec,
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

    /// The getresource file cache dir (Go `:cacheDir/:project/filecache/getresource`).
    pub fn getresource_dir(&self) -> PathBuf {
        let project = Path::new(&self.dir).file_name().unwrap().to_owned();
        self.tmp
            .path
            .join("cache")
            .join(project)
            .join("filecache/getresource")
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

/// The deps of a site (resource spec only; the template store when given).
pub fn deps(site: &Site, store: Option<TemplateStore>) -> Arc<Deps> {
    let d = Deps::for_tests(site.spec.path_spec.cfg.clone()).with_resource_spec(site.spec.clone());
    if let Some(s) = store {
        let _ = d.template_store.set(s);
    }
    Arc::new(d)
}

fn stub(name: &str) -> TplFunc {
    let name = name.to_string();
    Arc::new(move |_ctx, _args| Err(go_value::Error::new(format!("stub function {name} called"))))
}

/// The template store of the site: its layouts, the embedded templates, stubs for every Hugo
/// function name, and the real compare/fmt/collections namespaces (T18).
pub fn store(site: &Site, func_names: &[String]) -> TemplateStore {
    let spec = &site.spec;
    let conf = spec
        .path_spec
        .cfg
        .get_config()
        .downcast::<nh_allconfig::allconfig::Config>()
        .unwrap();
    let mut funcs = FuncMap::new();
    for n in func_names {
        funcs.insert(n.clone(), stub(n));
    }
    let d = Arc::new(Deps::for_tests(spec.path_spec.cfg.clone()));
    for ns in [
        nh_tplfuncs::compare::init::namespace(&d),
        nh_tplfuncs::fmt::init::namespace(&d),
        nh_tplfuncs::collections::init::namespace(&d),
    ] {
        funcs.insert(ns.name.to_string(), ns.context.clone());
        for (alias, f) in ns.aliases {
            funcs.insert(alias, f);
        }
    }
    let opts = StoreOptions {
        fs: spec.path_spec.base_fs.source_filesystems.layouts.fs.clone(),
        log: None,
        path_parser: spec.path_spec.cfg.path_parser(),
        output_formats: spec.output_formats(),
        media_types: spec.media_types(),
        default_content_language: spec.path_spec.cfg.default_content_language(),
        default_output_format: conf.root.default_output_format.clone(),
        taxonomy_singular_plural: conf.taxonomies.clone(),
        watching: false,
        render_hooks: conf.markup.goldmark.render_hooks.clone(),
        named_types: Arc::new(nh_common::object::NamedTypeRegistry::new()),
    };
    TemplateStore::new(
        opts,
        SiteOptions {
            site: Arc::new(std::sync::OnceLock::new()),
            template_funcs: Arc::new(funcs),
        },
    )
    .unwrap_or_else(|e| panic!("store: {e}"))
}

fn marshal(v: &Value) -> String {
    match go_json::marshal(v) {
        Ok(b) => String::from_utf8_lossy(&b).into_owned(),
        Err(e) => format!("ERR: {e}"),
    }
}

/// Go: `rsupport.Rec(r, true)`.
pub fn rec_res(r: &dyn nh_resource::resourcetypes::Resource) -> J {
    let mut o = serde_json::Map::new();
    o.insert("name".into(), json!(r.name()));
    o.insert("title".into(), json!(r.title()));
    o.insert("key".into(), json!(r.key()));
    if let Some(n) = r.name_normalized() {
        o.insert("nameNormalized".into(), json!(n));
    }
    let mt = r.media_type();
    o.insert("mediaType".into(), json!(mt.typ));
    o.insert(
        "mediaTypeJSON".into(),
        json!(String::from_utf8_lossy(&mt.marshal_json_bytes().unwrap()).into_owned()),
    );
    o.insert("resourceType".into(), json!(r.resource_type()));
    o.insert("data".into(), json!(marshal(&r.data())));
    o.insert("params".into(), json!(marshal(&Value::Map(r.params()))));
    let ctx = nh_tpl::template::TplContext::default();
    if let Some(c) = r.content(ctx.as_host()) {
        match c {
            Err(e) => {
                o.insert("contentErr".into(), json!(e.message()));
                if e.message().contains("this operation is not supported") {
                    // A resource over a directory: Go panics reading it (the oracle's record
                    // stops there), the port returns the error.
                    return J::Object(o);
                }
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
    if r.resource_type() == "image"
        && let Some(a) = r
            .as_any()
            .downcast_ref::<nh_resources::transform::ResourceAdapter>()
    {
        match (a.width(), a.height()) {
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

const PP_FIELDS: &[&str] = &[
    "Content",
    "RelPermalink",
    "Permalink",
    "Name",
    "Title",
    "ResourceType",
    "Data.Integrity",
    "MediaType.Type",
    "MediaType.SubType",
    "MediaType.Suffixes",
];

fn rec_pp(p: &nh_resources::postpub::postpub::PostPublishResource) -> J {
    let content = p.content().ok().and_then(|v| {
        v.as_go_string()
            .map(|s| String::from_utf8_lossy(s.as_bytes()).into_owned())
    });
    let mut fields = serde_json::Map::new();
    for f in PP_FIELDS {
        let rel = p.rel_permalink();
        let pattern = format!("{}{f}__e=", &rel[..rel.len() - "RelPermalink__e=".len()]);
        let v = match p.get_field_string(&pattern) {
            None => json!({"s": "", "ok": false}),
            Some(Ok(s)) => json!({"s": s, "ok": true}),
            Some(Err(e)) => json!({"panic": e.message()}),
        };
        fields.insert(f.to_string(), v);
    }
    json!({"pp": {
        "relPermalink": p.rel_permalink(),
        "permalink": p.permalink(),
        "name": p.name(),
        "title": p.title(),
        "resourceType": p.resource_type(),
        "content": content,
        "data": marshal(&p.data()),
        "mediaType": marshal(&p.media_type()),
        "fields": fields,
    }})
}

/// Go: `rtsupport.Rec(v)`.
pub fn rec(v: &Value) -> J {
    match v {
        Value::Invalid => json!({"nil": true}),
        Value::TypedNil(t) if &**t == nh_resource::resourcetypes::RESOURCE_TYPE => {
            json!({"nil": true})
        }
        Value::TypedNil(t) if &**t == nh_resource::resourcetypes::RESOURCES_TYPE => {
            json!({"nilList": true})
        }
        Value::List(l) if matches!(&l.ty, SliceType::Named(n) if &**n == "resource.Resources") => {
            let items: Vec<J> = l
                .items
                .iter()
                .map(|x| {
                    let r = nh_resource::resourcetypes::resource_from_value_any(x).unwrap();
                    json!({"name": r.name(), "key": r.key(), "mediaType": r.media_type().typ, "resourceType": r.resource_type()})
                })
                .collect();
            json!({ "list": items })
        }
        v => {
            if let Some(p) = nh_resources::postpub::postpub::post_publish_from_value(v) {
                return rec_pp(&p);
            }
            if let Some(r) = nh_resource::resourcetypes::resource_from_value_any(v) {
                return json!({"res": rec_res(r.as_ref())});
            }
            json!({"other": v.go_type_name()})
        }
    }
}

/// Runs the scripts of a fixture (`rtsupport.RunSite`) over the namespace; returns the
/// mismatches.
pub fn run_cases(
    ns: &nh_tplfuncs::resources::resources::Namespace,
    cases: &J,
    vars: &mut HashMap<String, Value>,
    check: &mut dyn FnMut(&str, &J, &J, &J),
) {
    let ctx = nh_tpl::template::TplContext::default();
    for c in cases.as_array().unwrap() {
        let name = c["name"].as_str().unwrap();
        let results = c["results"].as_array().unwrap();
        for (i, st) in c["steps"].as_array().unwrap().iter().enumerate() {
            let op = st["op"].as_str().unwrap();
            let args: Vec<Value> = st["args"]
                .as_array()
                .map(|a| a.iter().map(|x| dec_arg(x, vars)).collect())
                .unwrap_or_default();
            let got = if op == "rec" {
                rec(&args[0])
            } else {
                match ns
                    .call_method(ctx.as_host(), op, &args)
                    .unwrap_or_else(|| panic!("no method {op}"))
                {
                    Err(e) => json!({"err": e.message()}),
                    Ok(v) => {
                        if let Some(a) = st.get("as").and_then(J::as_str) {
                            vars.insert(a.to_string(), v.clone());
                        }
                        rec(&v)
                    }
                }
            };
            check(&format!("{name}[{i}] {op}"), st, &results[i], &got);
        }
    }
}

fn dec_arg(a: &J, vars: &HashMap<String, Value>) -> Value {
    match a["t"].as_str().unwrap() {
        "ref" => vars
            .get(a["v"].as_str().unwrap())
            .cloned()
            .unwrap_or(Value::Invalid),
        "refs" | "anyrefs" => {
            let items: Vec<Value> = a["v"]
                .as_array()
                .unwrap()
                .iter()
                .map(|n| {
                    vars.get(n.as_str().unwrap())
                        .cloned()
                        .unwrap_or(Value::Invalid)
                })
                .collect();
            if a["t"] == "refs" {
                if items.is_empty() {
                    // Go's nil resource.Resources.
                    return Value::TypedNil(Arc::from(nh_resource::resourcetypes::RESOURCES_TYPE));
                }
                Value::list(
                    SliceType::Named(Arc::from(nh_resource::resourcetypes::RESOURCES_TYPE)),
                    items,
                )
            } else {
                Value::any_list(items)
            }
        }
        _ => dec(a),
    }
}

/// Compares two JSON objects key by key; returns the differences.
pub fn diff(what: &str, want: &J, got: &J) -> Vec<String> {
    let mut out = Vec::new();
    match (want.as_object(), got.as_object()) {
        (Some(w), Some(g)) => {
            let mut keys: Vec<&String> = w.keys().chain(g.keys()).collect();
            keys.sort();
            keys.dedup();
            for k in keys {
                let (wv, gv) = (w.get(k).unwrap_or(&J::Null), g.get(k).unwrap_or(&J::Null));
                if wv.is_object() && gv.is_object() {
                    out.extend(diff(&format!("{what}.{k}"), wv, gv));
                } else if wv != gv {
                    out.push(format!("{what}.{k}: want {wv}, got {gv}"));
                }
            }
        }
        _ => {
            if want != got {
                out.push(format!("{what}: want {want}, got {got}"));
            }
        }
    }
    out
}

/// Runs `f` on a thread with a large stack (the minifiers and the template engine recurse like
/// Go, whose goroutine stacks grow to 1 GB).
pub fn big_stack(f: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(512 << 20)
        .spawn(f)
        .unwrap()
        .join()
        .unwrap();
}
