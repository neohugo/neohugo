//! Shared helpers of the nh-page oracle tests: fixture loading, Go string decoding, panic
//! capture, a test `config.AllProvider` (to rebuild the Go sites' PathSpecs), the PathParser
//! replay of the recorded callbacks, and the goval value decoder.

#![allow(dead_code)]

use std::any::Any;
use std::collections::{BTreeMap, HashMap};
use std::io::Read;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, Once};
use std::time::Duration;

use go_time::GoTimeExt;
use go_value::{FloatKind, GoString, IntKind, List, Map, SafeKind, UintKind, Value};
use nh_common::hreflect::{map_type_from_name, slice_type_from_name};
use nh_common::paths::pathparser::{Path, PathParser, PathType};
use nh_common::urls::BaseURL;
use nh_config::common_config::{BaseConfig, CommonDirs, Pagination};
use nh_config::config_provider::{AllProvider, ContentTypesProvider};
use nh_helpers::pathspec::PathSpec;
use nh_hugofs::modules::module::{Module, Modules};
use nh_langs::language::{Language, Languages};
use nh_media::media::media_type::{MediaType, SuffixInfo};
use nh_media::output::output_format::OutputFormat;
use serde_json::Value as J;

/// Reads a gzip-compressed JSON fixture below `tests/fixtures`.
pub fn fixture(rel: &str) -> J {
    let path = fixtures_dir().join(rel);
    let f = std::fs::File::open(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut s = String::new();
    flate2::read::GzDecoder::new(f)
        .read_to_string(&mut s)
        .unwrap();
    serde_json::from_str(&s).unwrap()
}

pub fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

/// The fixture files of a topic, sorted.
pub fn fixture_files(topic: &str) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(fixtures_dir().join(topic))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".json.gz"))
        .collect();
    v.sort();
    v
}

/// A Go string from the fixture: a JSON string or `{"hex": ...}`.
pub fn gostr(v: &J) -> Vec<u8> {
    match v {
        J::String(s) => s.as_bytes().to_vec(),
        J::Object(m) if m.contains_key("hex") => unhex(m["hex"].as_str().unwrap()),
        _ => panic!("not a Go string: {v}"),
    }
}

pub fn gostring(v: &J) -> String {
    String::from_utf8(gostr(v)).expect("UTF-8 Go string")
}

pub fn unhex(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

pub fn hex(b: &[u8]) -> String {
    b.iter().map(|c| format!("{c:02x}")).collect()
}

/// Encodes bytes like the Go oracle (`goval.Str`).
pub fn enc(b: &[u8]) -> J {
    match std::str::from_utf8(b) {
        Ok(s) => J::String(s.to_string()),
        Err(_) => serde_json::json!({ "hex": hex(b) }),
    }
}

static QUIET: Once = Once::new();

/// Runs `f`, turning a Rust panic into `Err(message)` (Go's `{"panic": ...}`).
pub fn catch<T>(f: impl FnOnce() -> T) -> Result<T, String> {
    QUIET.call_once(|| {
        let prev = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            if std::env::var("NH_PAGE_SHOW_PANICS").is_ok() {
                prev(info);
            }
        }));
    });
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).map_err(|p| {
        if let Some(s) = p.downcast_ref::<String>() {
            s.clone()
        } else if let Some(s) = p.downcast_ref::<&str>() {
            s.to_string()
        } else {
            "?".to_string()
        }
    })
}

static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// A unique working directory name (PathSpec needs one; nothing is written).
pub fn work_dir(prefix: &str) -> String {
    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    std::env::temp_dir()
        .join(format!("nh-page-rs-{prefix}-{}-{n}", std::process::id()))
        .to_string_lossy()
        .into_owned()
}

/// A test `config.AllProvider` with the values the PathSpec helpers read.
pub struct TestCfg {
    pub lang: String,
    pub languages: Languages,
    pub default_content_language: String,
    pub base_url: BaseURL,
    pub language_prefix: String,
    pub canonify_urls: bool,
    pub disable_path_to_lower: bool,
    pub remove_path_accents: bool,
    pub is_multihost: bool,
    pub working_dir: String,
    pub sections: BTreeMap<String, Arc<dyn Any + Send + Sync>>,
    /// The pagination config (T12's pagination test; `None`: unimplemented).
    pub pagination: Option<Pagination>,
}

impl TestCfg {
    pub fn new(working_dir: &str) -> TestCfg {
        let lang = Language::new("en", "en", "", Default::default()).unwrap();
        let mut sections: BTreeMap<String, Arc<dyn Any + Send + Sync>> = BTreeMap::new();
        let project: Modules = vec![Arc::new(Module {
            path: "project".to_string(),
            dir: working_dir.to_string(),
            config: Default::default(),
            config_filenames: Vec::new(),
            mounts: Vec::new(),
            owner: None,
            is_go_mod: false,
            vendor: false,
            version: String::new(),
            watch: false,
            cfg: None,
        })];
        sections.insert("allModules".to_string(), Arc::new(project));
        TestCfg {
            lang: "en".to_string(),
            languages: vec![lang],
            default_content_language: "en".to_string(),
            base_url: nh_common::urls::new_base_url_from_string("https://example.org/").unwrap(),
            language_prefix: String::new(),
            canonify_urls: false,
            disable_path_to_lower: false,
            remove_path_accents: false,
            is_multihost: false,
            working_dir: working_dir.to_string(),
            sections,
            pagination: None,
        }
    }
}

impl AllProvider for TestCfg {
    fn language(&self) -> Arc<Language> {
        self.languages
            .iter()
            .find(|l| l.lang == self.lang)
            .cloned()
            .unwrap_or_else(|| self.languages[0].clone())
    }
    fn languages(&self) -> Languages {
        self.languages.clone()
    }
    fn languages_default_first(&self) -> Languages {
        self.languages.clone()
    }
    fn language_prefix(&self) -> String {
        self.language_prefix.clone()
    }
    fn base_url(&self) -> BaseURL {
        self.base_url.clone()
    }
    fn base_url_live_reload(&self) -> BaseURL {
        self.base_url.clone()
    }
    fn path_parser(&self) -> Arc<PathParser> {
        Arc::new(nh_media::media::config::default_path_parser())
    }
    fn environment(&self) -> String {
        "production".to_string()
    }
    fn is_multihost(&self) -> bool {
        self.is_multihost
    }
    fn is_multilingual(&self) -> bool {
        self.languages.len() > 1
    }
    fn no_build_lock(&self) -> bool {
        true
    }
    fn base_config(&self) -> BaseConfig {
        BaseConfig {
            working_dir: self.working_dir.clone(),
            publish_dir: "public".to_string(),
            ..Default::default()
        }
    }
    fn dirs(&self) -> CommonDirs {
        CommonDirs {
            working_dir: self.working_dir.clone(),
            publish_dir: "public".to_string(),
            resource_dir: "resources".to_string(),
            ..Default::default()
        }
    }
    fn quiet(&self) -> bool {
        true
    }
    fn dirs_base(&self) -> CommonDirs {
        self.dirs()
    }
    fn content_types(&self) -> Arc<dyn ContentTypesProvider> {
        unimplemented!("content_types")
    }
    fn get_config_section(&self, name: &str) -> Arc<dyn Any + Send + Sync> {
        self.sections
            .get(name)
            .cloned()
            .unwrap_or_else(|| unimplemented!("config section {name}"))
    }
    fn get_config(&self) -> Arc<dyn Any + Send + Sync> {
        unimplemented!("get_config")
    }
    fn canonify_urls(&self) -> bool {
        self.canonify_urls
    }
    fn disable_path_to_lower(&self) -> bool {
        self.disable_path_to_lower
    }
    fn remove_path_accents(&self) -> bool {
        self.remove_path_accents
    }
    fn is_ugly_urls(&self, _section: &str) -> bool {
        false
    }
    fn default_content_language(&self) -> String {
        self.default_content_language.clone()
    }
    fn default_content_language_in_subdir(&self) -> bool {
        false
    }
    fn is_lang_disabled(&self, _lang: &str) -> bool {
        false
    }
    fn summary_length(&self) -> i64 {
        70
    }
    fn pagination(&self) -> Pagination {
        match &self.pagination {
            Some(p) => p.clone(),
            None => unimplemented!("pagination"),
        }
    }
    fn build_expired(&self) -> bool {
        false
    }
    fn build_future(&self) -> bool {
        false
    }
    fn build_drafts(&self) -> bool {
        false
    }
    fn running(&self) -> bool {
        false
    }
    fn watching(&self) -> bool {
        false
    }
    fn fast_render_mode(&self) -> bool {
        false
    }
    fn print_unused_templates(&self) -> bool {
        false
    }
    fn enable_missing_translation_placeholders(&self) -> bool {
        false
    }
    fn template_metrics(&self) -> bool {
        false
    }
    fn template_metrics_hints(&self) -> bool {
        false
    }
    fn print_i18n_warnings(&self) -> bool {
        false
    }
    fn create_title(&self, s: &str) -> String {
        s.to_string()
    }
    fn ignore_file(&self, _s: &str) -> bool {
        false
    }
    fn new_content_editor(&self) -> String {
        String::new()
    }
    fn timeout(&self) -> Duration {
        Duration::from_secs(30)
    }
    fn static_dirs(&self) -> Vec<String> {
        Vec::new()
    }
    fn ignored_logs(&self) -> std::collections::BTreeSet<String> {
        Default::default()
    }
    fn working_dir(&self) -> String {
        self.working_dir.clone()
    }
    fn enable_emoji(&self) -> bool {
        false
    }
}

/// Rebuilds a Go site's PathSpec from `psupport.PathSpecDump` and checks it against the Go
/// results recorded with it.
pub fn build_path_spec(d: &J) -> Arc<PathSpec> {
    build_path_spec_with_pagination(d, None)
}

/// [`build_path_spec`] with a pagination config (T12).
pub fn build_path_spec_with_pagination(d: &J, pagination: Option<Pagination>) -> Arc<PathSpec> {
    let wd = work_dir("ps");
    let mut cfg = TestCfg::new(&wd);
    cfg.pagination = pagination;
    let lang = d["lang"].as_str().unwrap().to_string();
    cfg.base_url =
        nh_common::urls::new_base_url_from_string(d["baseURL"].as_str().unwrap()).unwrap();
    cfg.language_prefix = d["languagePrefix"].as_str().unwrap().to_string();
    cfg.canonify_urls = d["canonifyURLs"].as_bool().unwrap();
    cfg.disable_path_to_lower = d["disablePathToLower"].as_bool().unwrap();
    cfg.remove_path_accents = d["removePathAccents"].as_bool().unwrap();
    cfg.is_multihost = d["isMultihost"].as_bool().unwrap();
    cfg.languages = d["languages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| Language::new(l.as_str().unwrap(), "en", "", Default::default()).unwrap())
        .collect();
    cfg.default_content_language = cfg.languages[0].lang.clone();
    cfg.lang = lang;
    let fs = nh_hugofs::fs::new_from(nh_hugofs::afero::new_mem_map_fs(), &cfg.base_config());
    let ps = PathSpec::new(fs, Arc::new(cfg)).unwrap();
    assert_eq!(ps.get_base_path(false), d["getBasePath"].as_str().unwrap());
    assert_eq!(
        ps.get_base_path(true),
        d["getBasePathRel"].as_str().unwrap()
    );
    assert_eq!(
        ps.get_target_language_base_path(),
        d["targetLanguageBasePath"].as_str().unwrap()
    );
    ps
}

/// Rebuilds a `media.Type` from `psupport.MediaTypeDump`.
pub fn media_type(d: &J) -> MediaType {
    let s = |k: &str| d[k].as_str().unwrap().to_string();
    let mut m = MediaType::from_string(&s("type")).unwrap_or_default();
    m.typ = s("type");
    m.main_type = s("mainType");
    m.sub_type = s("subType");
    m.delimiter = s("delimiter");
    m.first_suffix = SuffixInfo {
        suffix: s("suffix"),
        full_suffix: s("fullSuffix"),
    };
    m.suffixes_csv = s("suffixesCSV");
    assert_eq!(m.mime_suffix(), s("mimeSuffix"), "mimeSuffix of {d}");
    m
}

/// Rebuilds an `output.Format` from `psupport.FormatDump` and checks the equality with the
/// built-in formats that `CreateTargetPaths` tests.
pub fn output_format(d: &J) -> OutputFormat {
    let s = |k: &str| d[k].as_str().unwrap().to_string();
    let b = |k: &str| d[k].as_bool().unwrap();
    let f = OutputFormat {
        name: s("name"),
        media_type: media_type(&d["mediaType"]),
        path: s("path"),
        base_name: s("baseName"),
        rel: s("rel"),
        protocol: s("protocol"),
        is_plain_text: b("isPlainText"),
        is_html: b("isHTML"),
        no_ugly: b("noUgly"),
        ugly: b("ugly"),
        not_alternative: b("notAlternative"),
        root: b("root"),
        permalinkable: b("permalinkable"),
        weight: d["weight"].as_i64().unwrap(),
    };
    let bf = nh_media::output::output_format::builtin_formats();
    assert_eq!(f == bf.http_status_404_html, b("is404"), "is404 of {d}");
    assert_eq!(f == bf.sitemap, b("isSitemap"), "isSitemap of {d}");
    assert_eq!(f == bf.robots_txt, b("isRobots"), "isRobots of {d}");
    f
}

pub type Misses = Arc<Mutex<Vec<String>>>;

/// Rebuilds a recorded PathParser (`hsupport.Recorder.Describe`): the language index and the
/// recorded callback answers (a call Go never made is a miss).
pub fn build_parser(d: &J, misses: &Misses) -> PathParser {
    let language_index = d["languageIndex"].as_object().map(|m| {
        m.iter()
            .map(|(k, v)| (k.clone(), v.as_u64().unwrap() as usize))
            .collect::<BTreeMap<_, _>>()
    });

    let log1 = |k: &str| -> HashMap<String, bool> {
        d[k].as_array()
            .unwrap()
            .iter()
            .map(|e| (e[0].as_str().unwrap().to_string(), e[1].as_bool().unwrap()))
            .collect()
    };
    let of: HashMap<(String, String), bool> = d["outputFormatLog"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| {
            (
                (
                    e[0].as_str().unwrap().to_string(),
                    e[1].as_str().unwrap().to_string(),
                ),
                e[2].as_bool().unwrap(),
            )
        })
        .collect();
    let ce = log1("contentExtLog");
    let ld = log1("langDisabledLog");

    let mut pp = PathParser {
        language_index,
        ..Default::default()
    };
    if d["isLangDisabled"].as_bool().unwrap() {
        let m = misses.clone();
        pp.is_lang_disabled = Some(Arc::new(move |l: &str| {
            ld.get(l).copied().unwrap_or_else(|| {
                m.lock().unwrap().push(format!("IsLangDisabled({l:?})"));
                false
            })
        }));
    }
    if d["isOutputFormat"].as_bool().unwrap() {
        let m = misses.clone();
        pp.is_output_format = Some(Arc::new(move |n: &str, e: &str| {
            of.get(&(n.to_string(), e.to_string()))
                .copied()
                .unwrap_or_else(|| {
                    m.lock()
                        .unwrap()
                        .push(format!("IsOutputFormat({n:?}, {e:?})"));
                    false
                })
        }));
    }
    if d["isContentExt"].as_bool().unwrap() {
        let m = misses.clone();
        pp.is_content_ext = Some(Arc::new(move |e: &str| {
            ce.get(e).copied().unwrap_or_else(|| {
                m.lock().unwrap().push(format!("IsContentExt({e:?})"));
                false
            })
        }));
    }
    pp
}

/// `paths.Type` from its Go `String()`.
pub fn path_type(s: &str) -> PathType {
    [
        PathType::File,
        PathType::ContentResource,
        PathType::ContentSingle,
        PathType::Leaf,
        PathType::Branch,
        PathType::ContentData,
        PathType::Markup,
        PathType::Shortcode,
        PathType::Partial,
        PathType::Baseof,
    ]
    .into_iter()
    .find(|t| t.string() == s)
    .unwrap_or_else(|| panic!("path type {s}"))
}

/// Rebuilds the `*paths.Path` values of a `psupport.PathTable` with the replayed parser and
/// checks the accessors Go recorded.
pub fn build_paths(entries: &J, pp: &PathParser) -> Vec<Arc<Path>> {
    entries
        .as_array()
        .unwrap()
        .iter()
        .map(|e| {
            let mut p = pp.parse(e["component"].as_str().unwrap(), &gostring(&e["input"]));
            if e["retype"].as_bool().unwrap() {
                p = p.for_type(path_type(e["type"].as_str().unwrap()));
            }
            if e["unnormalizedOf"].as_bool().unwrap() {
                p = p.unnormalized().clone();
            }
            let c = &e["checks"];
            let got = serde_json::json!({
                "isBundle": p.is_bundle(),
                "base": p.base(),
                "containerDir": p.container_dir(),
                "baseNameNoIdentifier": p.base_name_no_identifier(),
                "path": p.path(),
                "type": p.path_type().string(),
                "section": p.section(),
            });
            for (k, v) in got.as_object().unwrap() {
                assert_eq!(&c[k], v, "path {e}: {k}");
            }
            if let Some(u) = c.get("unBaseNameNoIdentifier") {
                assert_eq!(
                    u.as_str().unwrap(),
                    p.unnormalized().base_name_no_identifier(),
                    "path {e}: unnormalized"
                );
            }
            Arc::new(p)
        })
        .collect()
}

/// Decodes a `goval.Encode` value into a `go_value::Value` (go-toml local dates become
/// nh-parser's objects, parsed from their string form).
pub fn decode(v: &J) -> Value {
    let t = v["t"].as_str().unwrap();
    if let Some(ty) = t.strip_prefix("nil:") {
        return Value::TypedNil(Arc::from(ty));
    }
    let int = |k: IntKind| Value::Int(v["v"].as_str().unwrap().parse().unwrap(), k);
    let uint = |k: UintKind| Value::Uint(v["v"].as_str().unwrap().parse().unwrap(), k);
    match t {
        "nil" => Value::Invalid,
        "bool" => Value::Bool(v["v"].as_bool().unwrap()),
        "int" => int(IntKind::Int),
        "int8" => int(IntKind::Int8),
        "int16" => int(IntKind::Int16),
        "int32" => int(IntKind::Int32),
        "int64" => int(IntKind::Int64),
        "uint" => uint(UintKind::Uint),
        "uint8" => uint(UintKind::Uint8),
        "uint64" => uint(UintKind::Uint64),
        "float64" => Value::Float(
            f64::from_bits(u64::from_str_radix(v["v"].as_str().unwrap(), 16).unwrap()),
            FloatKind::F64,
        ),
        "string" => Value::String(GoString::from(gostr(&v["s"]))),
        "template.HTML" => Value::Safe(SafeKind::Html, GoString::from(gostr(&v["s"]))),
        "time.Time" => Value::Time(decode_time(v)),
        _ if t.starts_with("toml.") => {
            let s = v["s"].as_str().unwrap();
            let doc = format!("x = {s}\n");
            let m = nh_parser::metadecoders::toml::unmarshal_to_map(doc.as_bytes()).unwrap();
            m.get(b"x").unwrap().clone()
        }
        _ if v.get("items").is_some() => Value::List(Arc::new(List::new(
            slice_type_from_name(t),
            v["items"].as_array().unwrap().iter().map(decode).collect(),
        ))),
        _ if v.get("entries").is_some() => {
            let mut m = Map::new(map_type_from_name(t));
            for e in v["entries"].as_array().unwrap() {
                m.insert(GoString::from(gostr(&e[0])), decode(&e[1]));
            }
            Value::map(m)
        }
        _ => panic!("cannot decode {v}"),
    }
}

/// The location of a goval time: UTC, a loaded zone, or a fixed zone.
pub fn location(name: &str, abbr: &str, off: i64) -> Arc<go_value::Location> {
    match name {
        "UTC" => go_time::utc(),
        "" => go_time::fixed_zone(abbr, off),
        _ => match go_time::load_location(name) {
            Ok(l) => l,
            Err(_) => go_time::fixed_zone(abbr, off),
        },
    }
}

pub fn decode_time(v: &J) -> go_value::Time {
    let loc = location(
        v["loc"].as_str().unwrap(),
        v["abbr"].as_str().unwrap(),
        v["off"].as_i64().unwrap(),
    );
    go_time::unix(v["unix"].as_i64().unwrap(), v["nsec"].as_i64().unwrap()).in_loc(&loc)
}

/// Encodes a value like `goval.Encode` (the subset the front matter results use).
pub fn encode(v: &Value) -> J {
    match v {
        Value::Invalid => serde_json::json!({"t": "nil"}),
        Value::TypedNil(t) => serde_json::json!({"t": format!("nil:{t}")}),
        Value::Bool(b) => serde_json::json!({"t": "bool", "v": b}),
        Value::Int(i, k) => serde_json::json!({"t": k.go_name(), "v": i.to_string()}),
        Value::Uint(i, k) => serde_json::json!({"t": k.go_name(), "v": i.to_string()}),
        Value::Float(f, k) => {
            serde_json::json!({"t": match k { FloatKind::F64 => "float64", FloatKind::F32 => "float32" }, "v": format!("{:016x}", f.to_bits())})
        }
        Value::String(s) => serde_json::json!({"t": "string", "s": enc(s.as_bytes())}),
        Value::Safe(_, s) => serde_json::json!({"t": v.go_type_name(), "s": enc(s.as_bytes())}),
        Value::Time(t) => encode_time(t),
        Value::List(l) => {
            serde_json::json!({"t": v.go_type_name(), "items": l.items.iter().map(encode).collect::<Vec<_>>()})
        }
        Value::Map(m) => {
            serde_json::json!({"t": v.go_type_name(), "entries": m.entries.iter().map(|(k, v)| serde_json::json!([enc(k.as_bytes()), encode(v)])).collect::<Vec<_>>()})
        }
        Value::Object(o) => {
            let t = o.type_name().into_owned();
            if t.starts_with("toml.") {
                serde_json::json!({"t": t, "s": o.go_string().map(|s| s.to_str_lossy().into_owned()).unwrap_or_default()})
            } else {
                serde_json::json!({"t": t})
            }
        }
    }
}

pub fn encode_time(t: &go_value::Time) -> J {
    let (abbr, off) = t.zone();
    serde_json::json!({
        "t": "time.Time",
        "unix": t.go_unix(),
        "nsec": t.nanosecond(),
        "loc": t.go_location().name,
        "abbr": abbr,
        "off": off,
    })
}
