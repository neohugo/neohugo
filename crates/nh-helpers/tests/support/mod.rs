//! Shared helpers of the nh-helpers oracle tests: fixture loading, temporary directories, Go
//! string decoding, panic capture and a test `config.AllProvider`.

#![allow(dead_code)]

pub mod goval;

use std::any::Any;
use std::collections::{BTreeMap, HashMap};
use std::io::Read;
use std::path::{Path as StdPath, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, Once};
use std::time::Duration;

use nh_common::paths::pathparser::PathParser;
use nh_common::urls::BaseURL;
use nh_config::common_config::{BaseConfig, CommonDirs, Pagination};
use nh_config::config_provider::{AllProvider, ContentTypesProvider};
use nh_hugofs::modules::module::{Module, Modules};
use nh_langs::language::{Language, Languages};
use serde_json::Value as J;

/// Reads a gzip-compressed JSON fixture.
pub fn load_fixture(path: &StdPath) -> J {
    let f = std::fs::File::open(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut s = String::new();
    flate2::read::GzDecoder::new(f)
        .read_to_string(&mut s)
        .unwrap();
    serde_json::from_str(&s).unwrap()
}

pub fn fixture(topic: &str, name: &str) -> J {
    load_fixture(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join(topic)
            .join(name),
    )
}

/// A Go string from the fixture: a JSON string or `{"hex": ...}`.
pub fn gostr(v: &J) -> Vec<u8> {
    match v {
        J::String(s) => s.as_bytes().to_vec(),
        J::Object(m) if m.contains_key("hex") => unhex(m["hex"].as_str().unwrap()),
        _ => panic!("not a Go string: {v}"),
    }
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

/// Encodes bytes like the Go oracle (`hsupport.Str`).
pub fn enc(b: &[u8]) -> J {
    match std::str::from_utf8(b) {
        Ok(s) => J::String(s.to_string()),
        Err(_) => serde_json::json!({ "hex": hex(b) }),
    }
}

static QUIET: Once = Once::new();

/// Runs `f`, returning Go's `{"panic": msg}` shape for a Rust panic.
pub fn call(f: impl FnOnce() -> String) -> J {
    QUIET.call_once(|| {
        let prev = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            if std::env::var("NH_HELPERS_SHOW_PANICS").is_ok() {
                prev(info);
            }
        }));
    });
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(s) => enc(s.as_bytes()),
        Err(p) => {
            let msg = if let Some(s) = p.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = p.downcast_ref::<&str>() {
                s.to_string()
            } else {
                "?".to_string()
            };
            serde_json::json!({ "panic": msg })
        }
    }
}

static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// A temporary directory removed on drop.
pub struct TempDir {
    pub path: PathBuf,
}

impl TempDir {
    pub fn new(prefix: &str) -> TempDir {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let path =
            std::env::temp_dir().join(format!("nh-helpers-rs-{prefix}-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        TempDir { path }
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

/// A test `config.AllProvider` with the values the helpers read.
pub struct TestCfg {
    pub lang: String,
    pub languages: Languages,
    pub base_url: BaseURL,
    pub language_prefix: String,
    pub canonify_urls: bool,
    pub disable_path_to_lower: bool,
    pub remove_path_accents: bool,
    pub is_multihost: bool,
    pub working_dir: String,
    pub publish_dir: String,
    pub resource_dir: String,
    pub cache_dir: String,
    pub sections: BTreeMap<String, Arc<dyn Any + Send + Sync>>,
    pub ignore_file: Option<IgnoreFileFn>,
}

/// The `ignoreFile` predicate a test installs on [`TestCfg`].
pub type IgnoreFileFn = Arc<dyn Fn(&str) -> bool + Send + Sync>;

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
        sections.insert(
            "markup".to_string(),
            Arc::new(nh_markup::markup_config::default_config()),
        );
        TestCfg {
            lang: "en".to_string(),
            languages: vec![lang],
            base_url: nh_common::urls::new_base_url_from_string("https://example.org/").unwrap(),
            language_prefix: String::new(),
            canonify_urls: false,
            disable_path_to_lower: false,
            remove_path_accents: false,
            is_multihost: false,
            working_dir: working_dir.to_string(),
            publish_dir: "public".to_string(),
            resource_dir: "resources".to_string(),
            cache_dir: String::new(),
            sections,
            ignore_file: None,
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
            publish_dir: self.publish_dir.clone(),
            cache_dir: self.cache_dir.clone(),
            ..Default::default()
        }
    }
    fn dirs(&self) -> CommonDirs {
        CommonDirs {
            working_dir: self.working_dir.clone(),
            publish_dir: self.publish_dir.clone(),
            resource_dir: self.resource_dir.clone(),
            cache_dir: self.cache_dir.clone(),
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
        "en".to_string()
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
        unimplemented!("pagination")
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
    fn ignore_file(&self, s: &str) -> bool {
        self.ignore_file.as_ref().is_some_and(|f| f(s))
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

pub type Misses = Arc<Mutex<Vec<String>>>;

/// Rebuilds a recorded PathParser: the language index and the recorded callback answers (a call
/// Go never made is a miss).
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
