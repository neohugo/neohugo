//! Oracle test: neohugo langs/i18n (TranslationProvider.NewResource/CloneResource and the
//! translate funcs) against `tools/go-oracle/nh-i18n/translate`
//! (fixtures/translate/translate.json.gz).
//!
//! Every site of the fixture is recreated in a temporary directory, loaded with
//! `nh_allconfig::load::load_config`, and given a `Deps` (`Deps::for_tests(conf)` +
//! PathSpec + SourceSpec + a logger that records INFO and up) like the oracle's
//! `testconfig.GetTestDeps`. The provider runs on the first language's Deps and is cloned to the
//! others; every call goes through `Deps::translate` (the func stored in `Deps.translate`), and
//! its output, Go panic and log entries are compared with Go's.

mod common;

use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use go_value::Value;
use nh_allconfig::load::{ConfigSourceDescriptor, load_config};
use nh_common::loggers::{Level, Logger, Options};
use nh_config::config_provider::Provider;
use nh_config::default_config_provider::DefaultConfigProvider;
use nh_deps::deps::Deps;
use nh_helpers::pathspec::PathSpec;
use nh_helpers::source::source_spec::SourceSpec;
use nh_i18n::translation_provider::TranslationProvider;
use serde_json::{Value as J, json};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

struct TempDir(std::path::PathBuf);

impl TempDir {
    fn new() -> TempDir {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let p = std::env::temp_dir().join(format!("nh-i18n-rs-{}-{n}", std::process::id()));
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

/// The log lines written since the last call, as Go's (level, message) pairs.
struct LogTap {
    buf: Arc<Mutex<Vec<u8>>>,
    site_dir: String,
}

impl LogTap {
    fn take(&self) -> J {
        let mut b = self.buf.lock().unwrap();
        let s = String::from_utf8_lossy(&b).into_owned();
        b.clear();
        let mut out = Vec::new();
        for line in s.lines() {
            let (level, msg) = line.split_at(5);
            let level = match level.trim() {
                "INFO" => "info",
                "WARN" => "warn",
                "ERROR" => "error",
                l => panic!("level {l}"),
            };
            let msg = msg
                .strip_prefix(' ')
                .unwrap_or(msg)
                .replace(&self.site_dir, "$SITE");
            out.push(json!([level, msg]));
        }
        if out.is_empty() {
            J::Null
        } else {
            J::Array(out)
        }
    }
}

static STATIC_TYPE_DIVERGENCES: AtomicUsize = AtomicUsize::new(0);

/// Go's text/template reports `can't evaluate field X in type T` with the STATIC type of the
/// receiver: `interface {}` for a map[string]any value, an `any` struct field or an `any` method
/// result. The gotemplate value model has no static types and names the dynamic type. The two
/// logs are equal once that type name is replaced.
fn static_type_divergence(got: &J, want: &J) -> bool {
    let (Some(g), Some(w)) = (got.as_array(), want.as_array()) else {
        return false;
    };
    g.len() == w.len()
        && g.iter().zip(w).all(|(g, w)| {
            if g == w {
                return true;
            }
            let (gs, ws) = (g[1].as_str().unwrap(), w[1].as_str().unwrap());
            let Some(i) = ws.rfind(" in type interface {}") else {
                return false;
            };
            let Some(j) = gs.rfind(" in type ") else {
                return false;
            };
            g[0] == w[0] && gs[..j] == ws[..i]
        })
}

fn panic_message(p: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = p.downcast_ref::<String>() {
        return s.clone();
    }
    if let Some(s) = p.downcast_ref::<&str>() {
        return s.to_string();
    }
    "<non-string panic>".to_string()
}

fn run_site(tmp: &TempDir, site: &J) -> Vec<String> {
    let mut bad = Vec::new();
    let name = site["name"].as_str().unwrap();
    let dir = tmp.0.join(name);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("hugo.toml"), site["toml"].as_str().unwrap()).unwrap();
    if let Some(files) = site["files"].as_object() {
        let mut names: Vec<&String> = files.keys().collect();
        names.sort();
        for n in names {
            let p = dir.join(n);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(&p, files[n].as_str().unwrap()).unwrap();
        }
    }
    let site_dir = dir.to_string_lossy().into_owned();

    let flags = DefaultConfigProvider::new();
    flags.set("workingDir", Value::string(site_dir.as_str()));
    let confs = load_config(ConfigSourceDescriptor {
        flags: Some(Arc::new(flags)),
        filename: dir.join("hugo.toml").to_string_lossy().into_owned(),
        environ: vec!["NEOHUGO_ORACLE=1".to_string()],
        ..Default::default()
    })
    .unwrap_or_else(|e| panic!("{name}: load: {e}"));

    let buf: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
    let sink: nh_common::loggers::LogSink = buf.clone();
    let logger = Logger::with_options(Options {
        level: Level::Info,
        std_out: Some(sink.clone()),
        std_err: Some(sink),
        ..Default::default()
    });
    let tap = LogTap {
        buf,
        site_dir: site_dir.clone(),
    };

    let conf0 = confs.get_first_language_config();
    let fs = nh_hugofs::fs::new_from(nh_hugofs::fs::os(), &conf0.base_config());
    let ps = PathSpec::new(fs.clone(), conf0.clone()).unwrap();
    let ss = SourceSpec::new(ps.clone(), None, fs.source.clone());
    let d0 = Deps::for_tests(conf0.clone())
        .with_fs(fs)
        .with_path_spec(ps)
        .with_source_spec(ss)
        .with_log(logger.clone());
    tap.take();

    let tp = TranslationProvider::new();
    if let Err(e) = tp.new_resource(&d0) {
        let got = e.to_string().replace(&site_dir, "$SITE");
        if J::String(got.clone()) != site["err"] {
            bad.push(format!(
                "{name}: NewResource error: got {got:?} want {}",
                site["err"]
            ));
        }
        return bad;
    }
    if !site["err"].is_null() {
        bad.push(format!(
            "{name}: NewResource succeeded, Go failed with {}",
            site["err"]
        ));
        return bad;
    }
    let log = tap.take();
    if log != site["log"] {
        bad.push(format!(
            "{name}: NewResource log: got {log} want {}",
            site["log"]
        ));
    }

    let lang0 = conf0.language().lang.clone();
    let mut by_lang: BTreeMap<String, Arc<Deps>> = BTreeMap::new();
    let d0 = Arc::new(d0);
    for c in site["calls"].as_array().unwrap() {
        let lang = c["lang"].as_str().unwrap();
        if by_lang.contains_key(lang) {
            continue;
        }
        if lang == lang0 {
            by_lang.insert(lang.to_string(), d0.clone());
            continue;
        }
        let conf = confs
            .get_by_lang(lang)
            .unwrap_or_else(|| panic!("{name}: no language {lang}"));
        let dl = Deps::for_tests(conf).with_log(logger.clone());
        tp.clone_resource(&dl, &d0).unwrap();
        by_lang.insert(lang.to_string(), Arc::new(dl));
    }
    let log = tap.take();
    if log != site["clonelog"] {
        bad.push(format!(
            "{name}: CloneResource log: got {log} want {}",
            site["clonelog"]
        ));
    }

    let translator = tp.translator().unwrap().clone();
    for c in site["calls"].as_array().unwrap() {
        let lang = c["lang"].as_str().unwrap();
        let id = c["id"].as_str().unwrap();
        let arg = common::value_from_spec(&c["arg"]);
        assert_eq!(
            arg.go_type_name(),
            c["argtype"].as_str().unwrap(),
            "{name}: spec {}",
            c["arg"]
        );
        let d = &by_lang[lang];
        let got = match catch_unwind(AssertUnwindSafe(|| d.translate(&(), id, &arg))) {
            Ok(s) => J::String(s),
            Err(p) => json!({"panic": panic_message(p)}),
        };
        let log = tap.take();
        if got == c["out"] && log != c["log"] && static_type_divergence(&log, &c["log"]) {
            // Known divergence (PORTING.md): the warning text names the dynamic type.
            STATIC_TYPE_DIVERGENCES.fetch_add(1, Ordering::SeqCst);
        } else if got != c["out"] || log != c["log"] {
            bad.push(format!(
                "{name} {lang} {id:?} {}:\n  got  {got} {log}\n  want {} {}",
                c["arg"], c["out"], c["log"]
            ));
        }
        // The error-returning func agrees (a Go panic is its error).
        let fe = translator.func_e(lang);
        tap.take();
        let alt = match fe(&(), id, &arg) {
            Ok(s) => J::String(s),
            Err(e) => json!({"panic": e.message()}),
        };
        tap.take();
        assert_eq!(alt, got, "{name} {lang} {id}: func_e");
    }
    bad
}

#[test]
fn translate_matches_go() {
    // Go panics are expected for a few inputs; keep the test output readable.
    std::panic::set_hook(Box::new(|_| {}));
    let fx = common::load_fixture("translate", "translate.json.gz");
    let tmp = TempDir::new();
    let mut bad = Vec::new();
    let mut calls = 0;
    for site in fx["cases"].as_array().unwrap() {
        calls += site["calls"].as_array().map_or(0, |c| c.len());
        bad.extend(run_site(&tmp, site));
    }
    let _ = std::panic::take_hook();
    assert!(
        bad.is_empty(),
        "{} differences:\n{}",
        bad.len(),
        bad[..bad.len().min(60)].join("\n")
    );
    assert!(calls > 3000, "{calls}");
    // Exactly the `{{ .Count.Foo }}` calls whose Count is an interface-typed value.
    assert_eq!(STATIC_TYPE_DIVERGENCES.load(Ordering::SeqCst), 21);
}
