//! The static file copy (`copyStaticTo`: the spf13/fsync Syncer over the static filesystem)
//! against the Go oracle (`tools/go-oracle/nh-commands/staticcopy`): synthetic trees with
//! several mounts, overlapping files, symlinks, hidden files, Unicode names, modes and times,
//! and existing publish dirs (with and without `cleanDestinationDir`). The publish dir after
//! the sync (every entry, its bytes, and its mode and time when synced) must equal Go's.

mod support;

use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, UNIX_EPOCH};

use go_value::{Map, MapType, Value};
use nh_allconfig::load::{ConfigSourceDescriptor, load_config};
use nh_commands::hugobuilder::{StaticSyncOptions, copy_static_to_fs};
use nh_common::loggers::{Level, LogSink, Logger, Options};
use nh_config::config_provider::Provider;
use nh_config::default_config_provider::DefaultConfigProvider;
use nh_hugofs::filesystems::basefs::BaseFs;
use nh_hugofs::paths::Paths;
use serde_json::{Value as J, json};
use support::{TempDir, fixture};

const NOW_THRESHOLD: i64 = 1750000000;

fn hex_decode(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

fn hex_encode(b: &[u8]) -> String {
    b.iter().map(|c| format!("{c:02x}")).collect()
}

fn set_mtime(p: &Path, mtime: i64, base: i64) {
    let t = if mtime == 0 { base } else { mtime };
    let f = std::fs::File::open(p).unwrap();
    f.set_modified(UNIX_EPOCH + Duration::from_secs(t as u64))
        .unwrap();
}

/// The oracle's `materialize`.
fn materialize(root: &Path, entries: &[J], tmp: &str, base: i64) {
    for e in entries {
        let p = root.join(e["path"].as_str().unwrap());
        match e["kind"].as_str().unwrap() {
            "dir" => std::fs::create_dir_all(&p).unwrap(),
            "file" => {
                std::fs::create_dir_all(p.parent().unwrap()).unwrap();
                let b = match e.get("hex").and_then(|h| h.as_str()) {
                    Some(h) => hex_decode(h),
                    None => e
                        .get("content")
                        .and_then(|c| c.as_str())
                        .unwrap_or("")
                        .replace("$ROOT", tmp)
                        .into_bytes(),
                };
                std::fs::write(&p, b).unwrap();
            }
            "symlink" => {
                std::fs::create_dir_all(p.parent().unwrap()).unwrap();
                let target = e["target"].as_str().unwrap().replace("$ROOT", tmp);
                std::os::unix::fs::symlink(target, &p).unwrap();
            }
            k => panic!("kind {k}"),
        }
    }
    for e in entries {
        if e["kind"] != "file" {
            continue;
        }
        let p = root.join(e["path"].as_str().unwrap());
        let mode = e.get("mode").and_then(|m| m.as_u64()).unwrap_or(0o644) as u32;
        set_mtime(
            &p,
            e.get("mtime").and_then(|m| m.as_i64()).unwrap_or(0),
            base,
        );
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(mode)).unwrap();
    }
    // Every directory below root, deepest first (Go: longer paths first).
    let mut dirs = Vec::new();
    walk_dirs(root, &mut dirs);
    dirs.sort_by(|a, b| b.len().cmp(&a.len()).then(a.cmp(b)));
    for d in dirs {
        set_mtime(Path::new(&d), 0, base);
    }
}

fn walk_dirs(dir: &Path, out: &mut Vec<String>) {
    out.push(dir.to_str().unwrap().to_string());
    for e in std::fs::read_dir(dir).unwrap() {
        let e = e.unwrap();
        if e.file_type().unwrap().is_dir() {
            walk_dirs(&e.path(), out);
        }
    }
}

fn mtime_of(md: &std::fs::Metadata) -> J {
    let m = md
        .modified()
        .unwrap()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    if m > NOW_THRESHOLD {
        json!("now")
    } else {
        json!(m)
    }
}

/// The oracle's `dumpTree` (Go `filepath.WalkDir` order: names sorted by bytes).
fn dump_tree(dir: &Path, rel: &str, no_times: bool, no_chmod: bool, out: &mut Vec<J>) {
    let mut names: Vec<Vec<u8>> = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().as_bytes().to_vec())
        .collect();
    names.sort();
    for n in names {
        let name = String::from_utf8(n).unwrap();
        let path = dir.join(&name);
        let rel_path = if rel.is_empty() {
            name.clone()
        } else {
            format!("{rel}/{name}")
        };
        let md = std::fs::symlink_metadata(&path).unwrap();
        let mut e = serde_json::Map::new();
        e.insert("path".into(), json!(rel_path));
        if md.file_type().is_symlink() {
            e.insert("kind".into(), json!("symlink"));
            e.insert(
                "target".into(),
                json!(std::fs::read_link(&path).unwrap().to_str().unwrap()),
            );
            out.push(J::Object(e));
        } else if md.is_dir() {
            e.insert("kind".into(), json!("dir"));
            if !no_times {
                e.insert("mtime".into(), mtime_of(&md));
            }
            out.push(J::Object(e));
            dump_tree(&path, &rel_path, no_times, no_chmod, out);
        } else {
            e.insert("kind".into(), json!("file"));
            let b = std::fs::read(&path).unwrap();
            e.insert("size".into(), json!(b.len()));
            e.insert("hex".into(), json!(hex_encode(&b)));
            if !no_chmod {
                e.insert("mode".into(), json!(md.permissions().mode() & 0o777));
            }
            if !no_times {
                e.insert("mtime".into(), mtime_of(&md));
            }
            out.push(J::Object(e));
        }
    }
}

fn run_case(c: &J, base: i64) -> J {
    let tmp = TempDir::new(c["name"].as_str().unwrap());
    let tmps = tmp.str();
    let root = tmp.path.join("site");
    std::fs::create_dir_all(tmp.path.join("home")).unwrap();
    materialize(&root, c["entries"].as_array().unwrap(), &tmps, base);
    let roots = root.to_str().unwrap().to_string();

    // commands.hugoBuilder.loadConfig + rootCommand.ConfigFromProvider.
    let cfg = Arc::new(DefaultConfigProvider::new());
    cfg.set("renderToMemory", Value::Bool(false));
    cfg.set("environment", Value::string("production"));
    let mut internal = Map::new(MapType::StringAny);
    for k in ["running", "watch", "verbose", "fastRenderMode"] {
        internal.entries.insert(k.into(), Value::Bool(false));
    }
    cfg.set("internal", Value::Map(Arc::new(internal)));
    cfg.set("workingDir", Value::string(roots.as_str()));
    let log: LogSink = Arc::new(Mutex::new(Vec::<u8>::new()));
    let logger = Logger::with_options(Options {
        level: Level::Warn,
        std_out: Some(log.clone()),
        std_err: Some(log),
        ..Default::default()
    });
    let home = tmp.path.join("home").to_str().unwrap().to_string();
    let home2 = home.clone();
    let res = load_config(ConfigSourceDescriptor {
        flags: Some(cfg.clone()),
        config_dir: "config".to_string(),
        environment: "production".to_string(),
        environ: vec![format!("HOME={home}")],
        logger: Some(logger.clone()),
        getenv: Some(Arc::new(move |k: &str| {
            if k == "HOME" {
                home2.clone()
            } else {
                String::new()
            }
        })),
        ..Default::default()
    });
    let configs = match res {
        Ok(c) => c,
        Err(e) => return json!({"err": e.to_string().replace(&tmps, "$ROOT")}),
    };
    let base_conf = configs.base.clone();
    let publish_dir = base_conf.root.common_dirs.publish_dir.clone();
    for k in ["publishDir", "publishDirStatic", "publishDirDynamic"] {
        cfg.set(k, Value::string(publish_dir.as_str()));
    }
    let hfs = nh_hugofs::fs::new_from_source_and_destination(
        nh_hugofs::fs::os(),
        nh_hugofs::fs::os(),
        &*cfg,
    );
    let p = Paths::new(hfs.clone(), configs.get_first_language_config()).unwrap();
    let bfs = match BaseFs::new_with_base(&p, Some(logger), None) {
        Ok(b) => b,
        Err(e) => return json!({"err": e.to_string().replace(&tmps, "$ROOT")}),
    };

    let opts = StaticSyncOptions {
        no_times: base_conf.root.no_times,
        no_chmod: base_conf.root.no_chmod,
        clean_destination_dir: base_conf.root.clean_destination_dir,
    };
    let mut res = serde_json::Map::new();
    let mut counts = serde_json::Map::new();
    for (lang, sfs) in &bfs.source_filesystems.static_ {
        match copy_static_to_fs(sfs, hfs.publish_dir_static.clone(), opts) {
            Ok(n) => {
                counts.insert(lang.clone(), json!(n));
            }
            Err(e) => {
                assert!(
                    nh_common::herrors::is_not_exist(&e),
                    "{e}: copyStatic ignores only not-exist errors"
                );
                res.insert(
                    "syncErr".into(),
                    json!(e.to_string().replace(&tmps, "$ROOT")),
                );
                break;
            }
        }
    }
    res.insert("counts".into(), J::Object(counts));
    let abs_publish = nh_common::paths::path::abs_pathify(&roots, &publish_dir);
    res.insert(
        "publishDir".into(),
        json!(abs_publish.replace(&tmps, "$ROOT")),
    );
    let mut tree = Vec::new();
    if Path::new(&abs_publish).exists() {
        dump_tree(
            Path::new(&abs_publish),
            "",
            opts.no_times,
            opts.no_chmod,
            &mut tree,
        );
    }
    res.insert(
        "tree".into(),
        // Go's nil slice (no entry, or no publish dir) is null.
        if tree.is_empty() {
            J::Null
        } else {
            J::Array(tree)
        },
    );
    J::Object(res)
}

#[test]
fn staticcopy_cases() {
    let fx = fixture("staticcopy/staticcopy.json.gz");
    let base = fx["baseTime"].as_i64().unwrap();
    let cases = fx["cases"].as_array().unwrap();
    let mut failures = Vec::new();
    let mut files = 0;
    for c in cases {
        let name = c["name"].as_str().unwrap();
        let got = run_case(c, base);
        let mut want = c["result"].clone();
        if let Some(o) = want.as_object_mut() {
            o.remove("log");
        }
        files += want["tree"]
            .as_array()
            .map(|t| t.iter().filter(|e| e["kind"] == "file").count())
            .unwrap_or(0);
        if got != want {
            failures.push(format!("{name}:\n  got  {got}\n  want {want}"));
        }
    }
    eprintln!(
        "staticcopy: {} cases, {files} published files, {} failures",
        cases.len(),
        failures.len()
    );
    if !failures.is_empty() {
        panic!("{}", failures.join("\n"));
    }
}
