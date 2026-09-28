//! Shared helpers of the nh-allconfig oracle tests: fixture loading, the case trees recreated in
//! a temporary directory, the CLI-like descriptor, and the dump of the loaded configs in the
//! format of `tools/go-oracle/nh-allconfig/load/dump.go`.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use go_value::{Map, MapType, Value};
use nh_allconfig::allconfig::{Config, Configs};
use nh_allconfig::load::{ConfigSourceDescriptor, load_config};
use nh_allconfig::segments::SegmentMatcherFields;
use nh_config::common_config::CommonDirs;
use nh_config::config_provider::{AllProvider, Provider};
use nh_config::default_config_provider::DefaultConfigProvider;
use nh_hugofs::modules::module::Module;
use serde_json::{Value as J, json};

pub const PLACEHOLDER: &str = "$ROOT";

pub const UGLY_PROBES: &[&str] = &["", "posts", "blog", "Posts", "docs", "news"];
pub const IGNORE_PROBES: &[&str] = &[
    "content/a.md",
    "content/draft/b.md",
    "/abs/c.txt",
    "x.tmp",
    "static/.DS_Store",
    "a/b~",
    "#x#",
];
pub const TITLE_PROBES: &[&str] = &[
    "hello world",
    "a tale of two cities",
    "THE END",
    "the lord of the rings",
    "über café",
];
pub const URL_PROBES: &[&str] = &[
    "https://example.org/a.json",
    "http://localhost/x",
    "https://gohugo.io/",
    "file:///tmp/a",
];
pub const OUTPUT_KINDS: &[&str] = &[
    "home",
    "page",
    "section",
    "taxonomy",
    "term",
    "rss",
    "sitemap",
    "robotstxt",
    "404",
    "sitemapindex",
    "status404",
    "taxonomyterm",
];

pub fn segment_probes() -> Vec<SegmentMatcherFields> {
    let f = |kind: &str, lang: &str, path: &str, output: &str| SegmentMatcherFields {
        kind: kind.into(),
        lang: lang.into(),
        path: path.into(),
        output: output.into(),
    };
    vec![
        f("page", "en", "/docs/a", "html"),
        f("home", "th", "/", "rss"),
        f("section", "en", "/blog", "json"),
        f("term", "fr", "/tags/x", "html"),
        f("", "en", "", ""),
        f("", "", "", "html"),
        f("", "", "/docs/a/b", ""),
        f("", "", "", ""),
    ]
}

/// Reads a gzip-compressed JSON fixture.
pub fn load_fixture(path: &Path) -> J {
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

/// A temporary directory removed on drop (its canonical path, like Go's EvalSymlinks).
pub struct TempDir {
    pub path: PathBuf,
}

impl TempDir {
    pub fn new(prefix: &str) -> TempDir {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!(
            "nh-allconfig-rs-{prefix}-{}-{n}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        let path = std::fs::canonicalize(&path).unwrap();
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

/// A JSON value from the fixture as a Go config value (strings, bools, numbers as int64 or
/// float64, lists, maps), with "$ROOT" replaced.
fn go_value(v: &J, tmp: &str) -> Value {
    match v {
        J::Null => Value::Invalid,
        J::Bool(b) => Value::Bool(*b),
        J::Number(n) => match n.as_i64() {
            Some(i) => Value::int64(i),
            None => Value::float64(n.as_f64().unwrap()),
        },
        J::String(s) => Value::string(s.replace(PLACEHOLDER, tmp)),
        J::Array(a) => Value::any_list(a.iter().map(|x| go_value(x, tmp)).collect()),
        J::Object(o) => {
            let mut m = Map::new(MapType::StringAny);
            for (k, x) in o {
                m.insert(k.as_str(), go_value(x, tmp));
            }
            Value::map(m)
        }
    }
}

/// A recreated case: the temporary root and the load result.
pub struct Loaded {
    pub tmp: TempDir,
    pub root: String,
    pub result: nh_common::Result<Configs>,
    /// The log output (INFO and above).
    pub log: Arc<std::sync::Mutex<Vec<u8>>>,
}

/// Recreates the case tree and runs `load_config` like the oracle (commands'
/// `ConfigFromProvider` flags provider).
pub fn load_case(c: &J) -> Loaded {
    let tmp = TempDir::new("load");
    let tmps = tmp.str();
    let site = c["site"].as_str().unwrap_or("site");
    let root = tmp.path.join(site);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(tmp.path.join("xdg")).unwrap();
    let files = c["files"].as_object().unwrap();
    let mut names: Vec<&String> = files.keys().collect();
    names.sort();
    for n in names {
        let p = root.join(n);
        if n.ends_with('/') {
            std::fs::create_dir_all(&p).unwrap();
            continue;
        }
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        let content = files[n].as_str().unwrap().replace(PLACEHOLDER, &tmps);
        std::fs::write(&p, content).unwrap();
    }
    let root_s = root.to_string_lossy().into_owned();

    // The process environment of GetCacheDir.
    let mut proc_env: BTreeMap<String, String> = BTreeMap::new();
    if let Some(pe) = c["procEnv"].as_object() {
        for (k, v) in pe {
            proc_env.insert(k.clone(), v.as_str().unwrap().replace(PLACEHOLDER, &tmps));
        }
    }
    let getenv = Arc::new(move |k: &str| proc_env.get(k).cloned().unwrap_or_default());

    let environment = c["environment"].as_str().unwrap().to_string();
    let flags = DefaultConfigProvider::new();
    flags.set("renderToMemory", Value::Bool(false));
    flags.set("environment", Value::string(environment.as_str()));
    let mut internal = Map::new(MapType::StringAny);
    for k in ["running", "watch", "verbose", "fastRenderMode"] {
        internal.insert(k, Value::Bool(false));
    }
    flags.set("internal", Value::map(internal));
    if let Some(fl) = c["flags"].as_object() {
        let mut keys: Vec<&String> = fl.keys().collect();
        keys.sort();
        for k in keys {
            flags.set(k, go_value(&fl[k], &tmps));
        }
    }
    if !flags.is_set("workingDir") {
        flags.set("workingDir", Value::string(root_s.as_str()));
    }

    let environ: Vec<String> = c["environ"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|e| e.as_str().unwrap().replace(PLACEHOLDER, &tmps))
                .collect()
        })
        .unwrap_or_default();
    let config_dir = match c["configDir"].as_str() {
        Some(s) if !s.is_empty() => s.to_string(),
        _ => "config".to_string(),
    };

    let log_buf: Arc<std::sync::Mutex<Vec<u8>>> = Arc::new(std::sync::Mutex::new(Vec::new()));
    let log: nh_common::loggers::LogSink = log_buf.clone();
    let result = load_config(ConfigSourceDescriptor {
        flags: Some(Arc::new(flags)),
        filename: c["filename"]
            .as_str()
            .unwrap_or("")
            .replace(PLACEHOLDER, &tmps),
        config_dir,
        environment,
        environ,
        ignore_module_does_not_exist: c["ignoreModuleDoesNotExist"].as_bool().unwrap_or(false),
        getenv: Some(getenv),
        logger: Some(nh_common::loggers::Logger::with_options(
            nh_common::loggers::Options {
                level: nh_common::loggers::Level::Info,
                std_out: Some(log.clone()),
                std_err: Some(log.clone()),
                ..Default::default()
            },
        )),
        ..Default::default()
    });

    Loaded {
        tmp,
        root: root_s,
        result,
        log: log_buf,
    }
}

pub struct Dumper {
    pub tmp: String,
}

impl Dumper {
    pub fn r(&self, s: &str) -> String {
        s.replace(&self.tmp, PLACEHOLDER)
    }

    fn strs(&self, s: &[String]) -> J {
        J::Array(s.iter().map(|x| J::String(self.r(x))).collect())
    }

    fn strs_nil(&self, s: &[String]) -> J {
        if s.is_empty() { J::Null } else { self.strs(s) }
    }

    fn marshal(&self, v: &Value) -> String {
        let b = go_json::marshal(v).expect("marshal");
        self.r(&String::from_utf8(b).expect("utf-8"))
    }

    pub fn configs(&self, confs: &Configs) -> J {
        let mut out = serde_json::Map::new();
        out.insert(
            "configFiles".into(),
            self.strs_nil(&confs.loading_info.config_files),
        );
        let bc = &confs.loading_info.base_config;
        out.insert(
            "baseConfig".into(),
            json!({
                "workingDir": self.r(&bc.working_dir),
                "cacheDir": self.r(&bc.cache_dir),
                "themesDir": self.r(&bc.themes_dir),
                "publishDir": self.r(&bc.publish_dir),
            }),
        );
        out.insert("isMultihost".into(), J::Bool(confs.is_multihost));
        let langs: Vec<String> = confs.languages.iter().map(|l| l.lang.clone()).collect();
        let langs_df: Vec<String> = confs
            .languages_default_first
            .iter()
            .map(|l| l.lang.clone())
            .collect();
        out.insert("languages".into(), self.strs_nil(&langs));
        out.insert("languagesDefaultFirst".into(), self.strs_nil(&langs_df));
        out.insert(
            "languageConfigSlice".into(),
            J::Array(
                language_slice_keys(confs)
                    .into_iter()
                    .map(J::String)
                    .collect(),
            ),
        );
        out.insert("base".into(), self.config(&confs.base));
        let mut lcm = serde_json::Map::new();
        for (k, c) in &confs.language_config_map {
            if Arc::ptr_eq(c, &confs.base) {
                lcm.insert(k.clone(), json!({"sameAsBase": true}));
                continue;
            }
            lcm.insert(k.clone(), self.config(c));
        }
        out.insert("languageConfigs".into(), J::Object(lcm));
        out.insert(
            "configLangs".into(),
            J::Array(
                confs
                    .config_langs()
                    .iter()
                    .map(|cl| self.config_lang(confs, cl.as_ref()))
                    .collect(),
            ),
        );
        out.insert(
            "modules".into(),
            J::Array(confs.modules.iter().map(|m| self.module(m)).collect()),
        );
        out.insert(
            "mountsDump".into(),
            J::String(self.r(&nh_allconfig::json::mounts_dump(confs).unwrap())),
        );
        let mut dumps = serde_json::Map::new();
        let mut langs: Vec<String> = vec![String::new()];
        langs.extend(confs.language_config_map.keys().cloned());
        for lang in langs {
            for zero in [false, true] {
                let s = nh_allconfig::json::config_dump(confs, &lang, zero).unwrap();
                dumps.insert(format!("{lang}|{zero}"), J::String(self.r(&s)));
            }
        }
        out.insert("configDumps".into(), J::Object(dumps));
        let root = nh_allconfig::load::strip_merge(&confs.loading_info.cfg.get(""));
        out.insert("providerRoot".into(), J::String(self.marshal(&root)));
        J::Object(out)
    }

    pub fn config(&self, c: &Config) -> J {
        let mut out = serde_json::Map::new();
        out.insert(
            "json".into(),
            J::String(self.marshal(&nh_allconfig::json::config_value(c))),
        );
        let cc = c.compiled();
        let bool_keys = |s: &std::collections::BTreeSet<String>| -> J {
            if s.is_empty() {
                J::Null
            } else {
                J::Array(s.iter().map(|k| J::String(format!("{k}=true"))).collect())
            }
        };
        let mut comp = serde_json::Map::new();
        comp.insert("timeout".into(), json!(cc.timeout.0));
        comp.insert("baseURL".into(), J::String(self.r(cc.base_url.string())));
        comp.insert(
            "baseURLWithPath".into(),
            J::String(self.r(&cc.base_url.with_path)),
        );
        comp.insert(
            "baseURLBasePath".into(),
            J::String(cc.base_url.base_path.clone()),
        );
        comp.insert(
            "baseURLLiveReload".into(),
            J::String(self.r(cc.base_url_live_reload.string())),
        );
        comp.insert(
            "serverInterface".into(),
            J::String(cc.server_interface.clone()),
        );
        comp.insert(
            "defaultOutputFormat".into(),
            J::String(cc.default_output_format.name.clone()),
        );
        comp.insert("disabledKinds".into(), bool_keys(&cc.disabled_kinds));
        comp.insert(
            "disabledLanguages".into(),
            bool_keys(&cc.disabled_languages),
        );
        comp.insert("ignoredLogs".into(), bool_keys(&cc.ignored_logs));
        comp.insert(
            "mainSections".into(),
            match cc.main_sections() {
                None => J::Null,
                Some(v) => self.strs(&v),
            },
        );
        comp.insert(
            "isMainSectionsSet".into(),
            J::Bool(cc.is_main_sections_set()),
        );
        comp.insert(
            "clock".into(),
            J::String(go_time::GoTimeExt::format(&cc.clock, go_time::RFC3339_NANO)),
        );
        comp.insert("clockIsZero".into(), J::Bool(cc.clock.is_zero()));
        let mut kof = serde_json::Map::new();
        for (k, fs) in &cc.kind_output_formats {
            kof.insert(
                k.clone(),
                J::Array(fs.0.iter().map(|f| J::String(f.name.clone())).collect()),
            );
        }
        comp.insert("kindOutputFormats".into(), J::Object(kof));
        comp.insert(
            "createTitle".into(),
            J::Array(
                TITLE_PROBES
                    .iter()
                    .map(|p| J::String((cc.create_title)(p)))
                    .collect(),
            ),
        );
        comp.insert(
            "isUglyURLSection".into(),
            J::Array(
                UGLY_PROBES
                    .iter()
                    .map(|p| J::Bool((cc.is_ugly_url_section)(p)))
                    .collect(),
            ),
        );
        comp.insert(
            "ignoreFile".into(),
            J::Array(
                IGNORE_PROBES
                    .iter()
                    .map(|p| J::Bool((cc.ignore_file)(p)))
                    .collect(),
            ),
        );
        comp.insert(
            "segments".into(),
            J::Array(
                segment_probes()
                    .iter()
                    .map(|p| {
                        json!([
                            cc.segment_filter.should_exclude_coarse(p),
                            cc.segment_filter.should_exclude_fine(p)
                        ])
                    })
                    .collect(),
            ),
        );
        let hc: Vec<J> = URL_PROBES
            .iter()
            .map(|u| {
                let pc = cc.http_cache.poll_config_for(u);
                json!({
                    "url": u,
                    "for": (cc.http_cache.for_)(&u.to_string()),
                    "poll": {
                        "disable": pc.config.disable,
                        "low": pc.config.low.0,
                        "high": pc.config.high.0,
                        "ok": pc.for_.is_some(),
                    }
                })
            })
            .collect();
        comp.insert("httpCache".into(), J::Array(hc));
        comp.insert(
            "isKindEnabled".into(),
            J::Array(
                OUTPUT_KINDS
                    .iter()
                    .map(|k| json!([k, c.is_kind_enabled(k)]))
                    .collect(),
            ),
        );
        let mut ld: Vec<J> = c
            .languages
            .get()
            .keys()
            .map(|k| json!([k, c.is_lang_disabled(k)]))
            .collect();
        ld.push(json!(["xx", c.is_lang_disabled("xx")]));
        comp.insert("isLangDisabled".into(), J::Array(ld));
        out.insert("compiled".into(), J::Object(comp));

        let mut hashes = serde_json::Map::new();
        let mut h = |name: &str, v: Option<String>| {
            if let Some(v) = v {
                hashes.insert(name.into(), J::String(v));
            }
        };
        h("imaging", c.imaging.as_ref().map(|n| n.source_hash.clone()));
        h(
            "mediaTypes",
            c.media_types.as_ref().map(|n| n.source_hash.clone()),
        );
        h(
            "outputFormats",
            c.output_formats.as_ref().map(|n| n.source_hash.clone()),
        );
        h(
            "contentTypes",
            c.content_types.as_ref().map(|n| n.source_hash.clone()),
        );
        h("cascade", c.cascade.as_ref().map(|n| n.source_hash.clone()));
        h(
            "segments",
            c.segments.as_ref().map(|n| n.source_hash.clone()),
        );
        h("menus", c.menus.as_ref().map(|n| n.source_hash.clone()));
        out.insert("hashes".into(), J::Object(hashes));

        let mut caches = serde_json::Map::new();
        for (k, fc) in &c.caches {
            caches.insert(
                k.clone(),
                json!({"dirCompiled": self.r(&fc.dir_compiled), "isResourceDir": fc.is_resource_dir}),
            );
        }
        out.insert("cachesCompiled".into(), J::Object(caches));
        out.insert(
            "cacheDirModules".into(),
            J::String(
                self.r(
                    &nh_helpers::cache::filecache::filecache_config::cache_dir_modules(&c.caches),
                ),
            ),
        );
        let ordering: Vec<String> = c
            .deployment
            .ordering
            .iter()
            .map(|r| r.string().to_string())
            .collect();
        out.insert("deploymentOrdering".into(), self.strs_nil(&ordering));
        let hl: Vec<J> = c
            .markup
            .highlight
            .hl_lines_parsed
            .iter()
            .map(|r| json!([r[0], r[1]]))
            .collect();
        out.insert(
            "hlLinesParsed".into(),
            if hl.is_empty() { J::Null } else { J::Array(hl) },
        );
        out.insert(
            "autoHeadingIDType".into(),
            J::String(c.markup.goldmark.parser.auto_heading_id_type.clone()),
        );
        J::Object(out)
    }

    fn dirs(&self, cd: &CommonDirs) -> J {
        json!({
            "themesDir": self.r(&cd.themes_dir), "publishDir": self.r(&cd.publish_dir),
            "resourceDir": self.r(&cd.resource_dir), "workingDir": self.r(&cd.working_dir),
            "cacheDir": self.r(&cd.cache_dir), "contentDir": self.r(&cd.content_dir),
            "dataDir": self.r(&cd.data_dir), "layoutDir": self.r(&cd.layout_dir),
            "i18nDir": self.r(&cd.i18n_dir), "archeTypeDir": self.r(&cd.arche_type_dir),
            "assetDir": self.r(&cd.asset_dir),
        })
    }

    pub fn config_lang(&self, confs: &Configs, c: &dyn AllProvider) -> J {
        let lang = c.language();
        let pg = c.pagination();
        let ignored: Vec<String> = c
            .ignored_logs()
            .iter()
            .map(|k| format!("{k}=true"))
            .collect();
        json!({
            "lang": lang.lang,
            "languagePrefix": c.language_prefix(),
            "baseURL": self.r(c.base_url().string()),
            "isMultihost": c.is_multihost(),
            "isMultilingual": c.is_multilingual(),
            "environment": c.environment(),
            "workingDir": self.r(&c.working_dir()),
            "defaultContentLanguage": c.default_content_language(),
            "defaultContentLanguageInSubdir": c.default_content_language_in_subdir(),
            "timeout": c.timeout().as_nanos() as i64,
            "staticDirs": self.strs_nil(&c.static_dirs()),
            "summaryLength": c.summary_length(),
            "canonifyURLs": c.canonify_urls(),
            "disablePathToLower": c.disable_path_to_lower(),
            "removePathAccents": c.remove_path_accents(),
            "buildDrafts": c.build_drafts(),
            "buildFuture": c.build_future(),
            "buildExpired": c.build_expired(),
            "enableEmoji": c.enable_emoji(),
            "noBuildLock": c.no_build_lock(),
            "quiet": c.quiet(),
            "running": c.running(),
            "watching": c.watching(),
            "pagination": {"PagerSize": pg.pager_size, "Path": pg.path, "DisableAliases": pg.disable_aliases},
            "ignoredLogs": if ignored.is_empty() { J::Null } else { json!(ignored) },
            "dirs": self.dirs(&c.dirs()),
            "dirsBase": self.dirs(&c.dirs_base()),
            "isUglyURLs": UGLY_PROBES.iter().map(|p| c.is_ugly_urls(p)).collect::<Vec<_>>(),
            "isLangDisabled": confs.languages.iter().map(|l| json!([l.lang, c.is_lang_disabled(&l.lang)])).collect::<Vec<_>>(),
            "language": {
                "lang": lang.lang,
                "languageName": lang.config.language_name,
                "languageCode": lang.language_code(),
                "title": lang.config.title,
                "languageDirection": lang.config.language_direction,
                "weight": lang.config.weight,
                "disabled": lang.config.disabled,
                "location": lang.location().name,
            },
        })
    }

    pub fn module(&self, m: &Module) -> J {
        let mounts: Vec<J> = m
            .mounts()
            .iter()
            .map(|mnt| {
                json!({
                    "source": self.r(&mnt.source), "target": mnt.target, "lang": mnt.lang,
                    "includeFiles": self.marshal(&nh_hugofs::modules::config::raw_files(&mnt.include_files)),
                    "excludeFiles": self.marshal(&nh_hugofs::modules::config::raw_files(&mnt.exclude_files)),
                    "disableWatch": mnt.disable_watch,
                })
            })
            .collect();
        let mut out = json!({
            "path": self.r(m.path()),
            "dir": self.r(m.dir()),
            "version": m.version(),
            "vendor": m.vendor(),
            "isGoMod": m.is_go_mod(),
            "watch": m.watch(),
            "configFilenames": self.strs_nil(m.config_filenames()),
            "owner": self.r(&m.owner().map(|o| o.path().to_string()).unwrap_or_default()),
            "mounts": if mounts.is_empty() { J::Null } else { J::Array(mounts) },
            "config": self.marshal(&nh_allconfig::json::module_config_value(m.config())),
        });
        if let Some(cfg) = m.cfg() {
            out["cfg"] = J::String(self.marshal(&cfg.get("")));
        }
        out
    }
}

/// The language keys of `LanguageConfigSlice`, in its (weight, lang) order.
pub fn language_slice_keys(confs: &Configs) -> Vec<String> {
    let mut keys: Vec<String> = confs.language_config_map.keys().cloned().collect();
    let w = |k: &String| {
        confs.language_config_map[k]
            .languages
            .get_lang(k)
            .map(|l| l.weight)
            .unwrap_or(0)
    };
    keys.sort_by(|a, b| w(a).cmp(&w(b)).then(a.cmp(b)));
    keys
}

/// Compares two JSON values, returning the paths that differ.
pub fn json_diff(want: &J, got: &J, path: &str, out: &mut Vec<String>) {
    match (want, got) {
        (J::Object(a), J::Object(b)) => {
            let mut keys: Vec<&String> = a.keys().chain(b.keys()).collect();
            keys.sort();
            keys.dedup();
            for k in keys {
                let p = format!("{path}.{k}");
                match (a.get(k), b.get(k)) {
                    (Some(x), Some(y)) => json_diff(x, y, &p, out),
                    (x, y) => out.push(format!("{p}: want {x:?} got {y:?}")),
                }
            }
        }
        (J::Array(a), J::Array(b)) if a.len() == b.len() => {
            for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
                json_diff(x, y, &format!("{path}[{i}]"), out);
            }
        }
        (J::String(a), J::String(b)) if a != b && (a.starts_with('{') || a.starts_with('[')) => {
            // Embedded JSON: diff it structurally for a readable message.
            match (serde_json::from_str::<J>(a), serde_json::from_str::<J>(b)) {
                (Ok(x), Ok(y)) if x != y => json_diff(&x, &y, &format!("{path}<json>"), out),
                _ => out.push(format!("{path}: want {a:?}\n   got {b:?}")),
            }
        }
        (a, b) if a != b => out.push(format!("{path}: want {a}\n   got {b}")),
        _ => {}
    }
}
