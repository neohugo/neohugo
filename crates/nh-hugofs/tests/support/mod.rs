//! Shared helpers of the nh-hugofs oracle tests: fixture loading, tree recreation, the recorded
//! modules and a test `AllProvider` (the seam described in PORTING.md).

#![allow(dead_code)]

use std::any::Any;
use std::collections::{BTreeMap, HashMap};
use std::io::Read;
use std::path::{Path as StdPath, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use nh_common::paths::pathparser::PathParser;
use nh_common::urls::BaseURL;
use nh_config::common_config::{BaseConfig, CommonDirs, Pagination};
use nh_config::config_provider::{AllProvider, ContentTypesProvider};
use nh_hugofs::modules::config::Mount;
use nh_hugofs::modules::module::{Module, Modules};
use nh_langs::language::{Language, Languages};
use serde_json::Value as J;

pub const PLACEHOLDER: &str = "$ROOT";

/// Reads a gzip-compressed JSON fixture.
pub fn load_fixture(path: &StdPath) -> J {
    let f = std::fs::File::open(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let mut s = String::new();
    flate2::read::GzDecoder::new(f)
        .read_to_string(&mut s)
        .unwrap();
    serde_json::from_str(&s).unwrap()
}

pub fn fixture_dir(topic: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(topic)
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
            std::env::temp_dir().join(format!("nhfs-rs-{prefix}-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        TempDir { path }
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

/// Recreates a recorded tree below root (entries in sorted order, like the oracle).
pub fn materialize(root: &StdPath, tree: &J) {
    let mut nodes: Vec<&J> = tree.as_array().unwrap().iter().collect();
    nodes.sort_by(|a, b| a["p"].as_str().unwrap().cmp(b["p"].as_str().unwrap()));
    for n in nodes {
        let p = root.join(n["p"].as_str().unwrap());
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        match n["t"].as_str().unwrap() {
            "d" => std::fs::create_dir_all(&p).unwrap(),
            "f" => std::fs::write(&p, n["c"].as_str().unwrap_or("")).unwrap(),
            "l" => std::os::unix::fs::symlink(n["l"].as_str().unwrap(), &p).unwrap(),
            t => panic!("node type {t}"),
        }
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

fn strings(v: &J) -> Vec<String> {
    match v {
        J::Null => Vec::new(),
        J::Array(a) => a.iter().map(|s| s.as_str().unwrap().to_string()).collect(),
        _ => panic!("strings: {v}"),
    }
}

/// The recorded `modules.Modules` (what T09's `Collect` produces in a build).
pub fn build_modules(mods: &J, root: &str) -> Modules {
    let unroot = |s: &str| s.replace(PLACEHOLDER, root);
    let mut out: Modules = Vec::new();
    for m in mods.as_array().unwrap() {
        let mounts = m["mounts"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|mnt| Mount {
                        source: unroot(mnt["source"].as_str().unwrap()),
                        target: mnt["target"].as_str().unwrap().to_string(),
                        lang: mnt["lang"].as_str().unwrap().to_string(),
                        include_files: strings(&mnt["includeFiles"]),
                        exclude_files: strings(&mnt["excludeFiles"]),
                        disable_watch: mnt["disableWatch"].as_bool().unwrap(),
                    })
                    .collect()
            })
            .unwrap_or_default();
        let owner_path = m["owner"].as_str().unwrap();
        let owner = if owner_path.is_empty() {
            None
        } else {
            Some(
                out.iter()
                    .find(|o| o.path == owner_path)
                    .cloned()
                    .unwrap_or_else(|| panic!("owner {owner_path}")),
            )
        };
        out.push(Arc::new(Module {
            path: m["path"].as_str().unwrap().to_string(),
            dir: unroot(m["dir"].as_str().unwrap()),
            config: Default::default(),
            config_filenames: Vec::new(),
            mounts,
            owner,
            is_go_mod: false,
            vendor: false,
            version: String::new(),
            watch: m["watch"].as_bool().unwrap(),
            cfg: None,
        }));
    }
    out
}

/// A test `config.AllProvider` with the recorded values basefs and paths read.
pub struct TestCfg {
    pub working_dir: String,
    pub publish_dir: String,
    pub resource_dir: String,
    pub default_content_language: String,
    pub is_multihost: bool,
    pub no_build_lock: bool,
    pub languages: Languages,
    pub path_parser: Arc<PathParser>,
    pub modules: Arc<Modules>,
}

impl TestCfg {
    pub fn from_fixture(header: &J, root: &str, misses: &Misses) -> TestCfg {
        let c = &header["config"];
        let dcl = c["defaultContentLanguage"].as_str().unwrap().to_string();
        let languages: Languages = strings(&c["languages"])
            .iter()
            .map(|l| Language::new(l, &dcl, "", Default::default()).unwrap())
            .collect();
        TestCfg {
            working_dir: root.to_string(),
            publish_dir: c["publishDir"].as_str().unwrap().to_string(),
            resource_dir: c["resourceDir"].as_str().unwrap().to_string(),
            default_content_language: dcl,
            is_multihost: c["isMultihost"].as_bool().unwrap(),
            no_build_lock: c["noBuildLock"].as_bool().unwrap(),
            languages,
            path_parser: Arc::new(build_parser(&header["parser"], misses)),
            modules: Arc::new(build_modules(&header["modules"], root)),
        }
    }
}

impl AllProvider for TestCfg {
    fn language(&self) -> Arc<Language> {
        self.languages[0].clone()
    }
    fn languages(&self) -> Languages {
        self.languages.clone()
    }
    fn languages_default_first(&self) -> Languages {
        self.languages.clone()
    }
    fn language_prefix(&self) -> String {
        String::new()
    }
    fn base_url(&self) -> BaseURL {
        unimplemented!("base_url")
    }
    fn base_url_live_reload(&self) -> BaseURL {
        unimplemented!("base_url_live_reload")
    }
    fn path_parser(&self) -> Arc<PathParser> {
        self.path_parser.clone()
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
        self.no_build_lock
    }
    fn base_config(&self) -> BaseConfig {
        BaseConfig {
            working_dir: self.working_dir.clone(),
            publish_dir: self.publish_dir.clone(),
            ..Default::default()
        }
    }
    fn dirs(&self) -> CommonDirs {
        CommonDirs {
            working_dir: self.working_dir.clone(),
            publish_dir: self.publish_dir.clone(),
            resource_dir: self.resource_dir.clone(),
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
        match name {
            "allModules" => self.modules.clone(),
            _ => unimplemented!("config section {name}"),
        }
    }
    fn get_config(&self) -> Arc<dyn Any + Send + Sync> {
        unimplemented!("get_config")
    }
    fn canonify_urls(&self) -> bool {
        false
    }
    fn disable_path_to_lower(&self) -> bool {
        false
    }
    fn remove_path_accents(&self) -> bool {
        false
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
