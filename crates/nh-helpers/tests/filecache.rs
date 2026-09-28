//! Oracle test: cache/filecache (DecodeConfig, Cache operations with expiry, AsHTTPCache,
//! NewCaches) and helpers.GetCacheDir against `tools/go-oracle/nh-helpers/filecache`
//! (fixtures/filecache/filecache.json.gz).

mod support;

use std::collections::BTreeMap;
use std::io::Read;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration as StdDuration, SystemTime};

use go_value::{Map, MapType, Value};
use nh_common::herrors::{Error, ErrorKind};
use nh_config::common_config::BaseConfig;
use nh_config::config_provider::AllProvider;
use nh_helpers::cache::filecache::filecache::{Cache, new_caches};
use nh_helpers::cache::filecache::filecache_config::{Configs, cache_dir_modules, decode_config};
use nh_helpers::path::{CacheDirEnv, get_cache_dir_env};
use nh_helpers::pathspec::PathSpec;
use nh_hugofs::afero::Fs;
use serde_json::{Value as J, json};
use support::goval::decode_goval_map;
use support::*;

fn configs_dump(c: &Configs, root: &str) -> J {
    J::Array(
        c.iter()
            .map(|(k, v)| {
                json!([k, {
                    "maxAge": v.max_age.0,
                    "dir": v.dir.replace(root, "$ROOT"),
                    "dirCompiled": v.dir_compiled.replace(root, "$ROOT"),
                    "isResourceDir": v.is_resource_dir,
                }])
            })
            .collect(),
    )
}

fn err_json(e: &Error) -> J {
    json!({ "err": e.message() })
}

#[test]
fn decode_config_matches_go() {
    let fx = fixture("filecache", "filecache.json.gz");
    let inputs = fx["decodeInputs"]["inputs"].as_array().unwrap();
    let bcfgs: Vec<BaseConfig> = fx["decodeInputs"]["bcfgs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| BaseConfig {
            working_dir: b["workingDir"].as_str().unwrap().to_string(),
            cache_dir: b["cacheDir"].as_str().unwrap().to_string(),
            ..Default::default()
        })
        .collect();
    let mut n = 0;
    let mut nondet = 0;
    for c in fx["decode"].as_array().unwrap() {
        let m = decode_goval_map(&inputs[c["in"].as_u64().unwrap() as usize]);
        let bcfg = &bcfgs[c["bcfg"].as_u64().unwrap() as usize];
        let fs: Arc<dyn Fs> = match c["fs"].as_str().unwrap() {
            "os" => nh_hugofs::afero::new_os_fs(),
            _ => nh_hugofs::afero::new_mem_map_fs(),
        };
        n += 1;
        match decode_config(fs.as_ref(), bcfg, &m) {
            Ok(cfg) => {
                assert_eq!(configs_dump(&cfg, "\u{0}"), c["ok"], "{c}");
                assert_eq!(json!(cache_dir_modules(&cfg)), c["modulesDir"], "{c}");
            }
            Err(e) => {
                if let Some(errs) = c["nondetErrs"].as_array() {
                    // Go ranges over maps in random order: any of its errors (deviation).
                    assert!(errs.contains(&json!(e.message())), "{c}: {}", e.message());
                    nondet += 1;
                } else {
                    assert_eq!(json!(e.message()), c["err"], "{c}");
                }
            }
        }
    }
    eprintln!("DecodeConfig: {n} cases ({nondet} with Go's random map order)");
}

fn env_map(e: &J, root: &str) -> BTreeMap<String, String> {
    e.as_object()
        .unwrap()
        .iter()
        .map(|(k, v)| (k.clone(), v.as_str().unwrap().replace("$ROOT", root)))
        .collect()
}

fn setup_dirs(root: &Path, mk: &str) {
    for d in ["home", "xdg", "tmp", "cfgcache", "a"] {
        let _ = std::fs::remove_dir_all(root.join(d));
    }
    std::fs::create_dir_all(root.join("home")).unwrap();
    for m in mk.split(',') {
        let d = match m {
            "home" => "home/.cache",
            "hugo" => "home/.cache/hugo_cache",
            "xdg" => "xdg",
            "tmphugo" => "tmp/hugo_cache",
            _ => continue,
        };
        std::fs::create_dir_all(root.join(d)).unwrap();
    }
}

#[test]
fn get_cache_dir_matches_go() {
    let fx = fixture("filecache", "filecache.json.gz");
    let tmp = TempDir::new("cachedir");
    let root = tmp.str();
    std::fs::write(tmp.path.join("afile"), "x").unwrap();
    let mut n = 0;
    for c in fx["getCacheDir"].as_array().unwrap() {
        let env = env_map(&c["env"], &root);
        setup_dirs(&tmp.path, env.get("_mk").map(|s| s.as_str()).unwrap_or(""));
        let getenv = |k: &str| env.get(k).cloned().unwrap_or_default();
        let fs: Arc<dyn Fs> = if c["mem"].as_bool().unwrap() {
            nh_hugofs::afero::new_mem_map_fs()
        } else {
            nh_hugofs::afero::new_os_fs()
        };
        let arg = c["arg"].as_str().unwrap().replace("$ROOT", &root);
        let got = get_cache_dir_env(
            fs.as_ref(),
            &arg,
            &CacheDirEnv {
                getenv: &getenv,
                is_test: false,
            },
        );
        n += 1;
        match got {
            Ok(dir) => {
                assert_eq!(json!(dir.replace(&root, "$ROOT")), c["ok"], "{c}");
                let is_dir = fs.stat(&dir).map(|fi| fi.is_dir()).unwrap_or(false);
                assert_eq!(json!(is_dir), c["isDir"], "{c}");
            }
            Err(e) => assert_eq!(json!(e.message().replace(&root, "$ROOT")), c["err"], "{c}"),
        }
    }
    eprintln!("GetCacheDir: {n} cases");

    // allconfig.LoadConfig: `HUGO_CACHEDIR` (the environment, when set, even to "") overrides
    // the `cacheDir` setting, which is then passed to GetCacheDir (the loader is T09's; this
    // pins down the order for it).
    let cases: [(Option<&str>, Option<&str>); 5] = [
        (None, None),
        (Some("fromconfig"), None),
        (None, Some("fromenv")),
        (Some("fromconfig"), Some("fromenv")),
        (Some("fromconfig"), Some("")),
    ];
    setup_dirs(&tmp.path, "");
    let env: BTreeMap<String, String> = [
        ("HOME".to_string(), format!("{root}/home")),
        ("TMPDIR".to_string(), format!("{root}/tmp")),
    ]
    .into_iter()
    .collect();
    let getenv = |k: &str| env.get(k).cloned().unwrap_or_default();
    for (i, c) in fx["loadConfig"].as_array().unwrap().iter().enumerate() {
        let (cfg, envv) = cases[i];
        let effective = match envv {
            Some("") => String::new(),
            Some(v) => format!("{root}/{v}"),
            None => cfg.map(|v| format!("{root}/{v}")).unwrap_or_default(),
        };
        let dir = get_cache_dir_env(
            nh_hugofs::afero::new_os_fs().as_ref(),
            &effective,
            &CacheDirEnv {
                getenv: &getenv,
                is_test: false,
            },
        )
        .unwrap();
        assert_eq!(json!(dir.replace(&root, "$ROOT")), c["cacheDir"], "{c}");
        let bcfg = BaseConfig {
            working_dir: format!("{root}/sites/lc{i}"),
            cache_dir: dir,
            ..Default::default()
        };
        let cfgs = decode_config(
            nh_hugofs::afero::new_os_fs().as_ref(),
            &bcfg,
            &Map::new(MapType::StringAny),
        )
        .unwrap();
        assert_eq!(
            json!(cfgs["getresource"].dir_compiled.replace(&root, "$ROOT")),
            c["getresource"]
        );
    }
}

/// `b, _ := io.ReadAll(r)` (the oracle ignores read errors here).
fn read_all(r: &mut dyn Read) -> Vec<u8> {
    let mut b = Vec::new();
    let _ = r.read_to_end(&mut b);
    b
}

/// On an error, Go also returns the ItemInfo and a partial value, which the Result-returning
/// Rust API does not (PORTING.md, deviation); results with an error compare the error only.
fn normalize(v: &J) -> J {
    if let J::Array(a) = v
        && let Some(last) = a.last()
        && last.get("err").is_some()
    {
        return json!({ "err": last["err"] });
    }
    v.clone()
}

fn opt_bytes(b: Option<Vec<u8>>) -> J {
    b.map(|b| enc(&b)).unwrap_or(J::Null)
}

fn list_dir(dir: &Path) -> Vec<(String, J)> {
    fn walk(base: &Path, d: &Path, out: &mut Vec<(String, J)>) {
        let mut entries: Vec<_> = std::fs::read_dir(d).unwrap().map(|e| e.unwrap()).collect();
        entries.sort_by_key(|e| e.file_name());
        for e in entries {
            let p = e.path();
            if p.is_dir() {
                walk(base, &p, out);
            } else {
                let rel = p.strip_prefix(base).unwrap().to_string_lossy().into_owned();
                out.push((rel, enc(&std::fs::read(&p).unwrap())));
            }
        }
    }
    let mut out = Vec::new();
    walk(dir, dir, &mut out);
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

#[test]
fn cache_operations_match_go() {
    let fx = fixture("filecache", "filecache.json.gz");
    let tmp = TempDir::new("ops");
    let root = tmp.str();
    let mut nops = 0;
    for (seq, c) in fx["cases"].as_array().unwrap().iter().enumerate() {
        let dir = tmp.path.join(format!("ops{seq}"));
        std::fs::create_dir_all(&dir).unwrap();
        for p in c["pre"].as_array().unwrap() {
            let rel = p[0].as_str().unwrap();
            let age = p[1].as_u64().unwrap();
            let path = dir.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, format!("pre-{rel}")).unwrap();
            let f = std::fs::File::options().write(true).open(&path).unwrap();
            f.set_modified(SystemTime::now() - StdDuration::from_secs(age * 60))
                .unwrap();
        }
        let max_age = match c["setup"].as_str().unwrap() {
            "forever" => -1,
            "disabled" => 0,
            "hour" => 3_600_000_000_000,
            s => panic!("{s}"),
        };
        let bfs =
            nh_hugofs::fs::new_base_path_fs(nh_hugofs::afero::new_os_fs(), &dir.to_string_lossy());
        let cache = Cache::new(bfs, go_time::Duration(max_age), "");
        let hc = cache.as_http_cache();
        for op in c["ops"].as_array().unwrap() {
            let code = op[0].as_u64().unwrap();
            let id = op[1].as_str().unwrap().to_string();
            let sub = op[2].as_u64().unwrap();
            let want = &op[3];
            let got = call_json(|| match code {
                0 => match cache.get_bytes(&id) {
                    Ok((info, b)) => json!([info.name, opt_bytes(b), null]),
                    Err(e) => json!(["", null, err_json(&e)]),
                },
                1 => match cache.get(&id) {
                    Ok((info, r)) => {
                        let b = r.map(|mut r| {
                            let b = read_all(&mut *r);
                            let _ = r.close();
                            b
                        });
                        json!([info.name, opt_bytes(b), null])
                    }
                    Err(e) => json!(["", null, err_json(&e)]),
                },
                2 => match cache
                    .get_or_create_bytes(&id, || Ok(format!("created-{id}").into_bytes()))
                {
                    Ok((info, b)) => json!([info.name, enc(&b), null]),
                    Err(e) => json!(["", null, err_json(&e)]),
                },
                3 => match cache.get_or_create_bytes(&id, || Err(Error::new("create failed"))) {
                    Ok((info, b)) => json!([info.name, enc(&b), null]),
                    Err(e) => json!([
                        nh_helpers::cache::filecache::filecache::clean_id(&id),
                        null,
                        err_json(&e)
                    ]),
                },
                4 => match cache.get_or_create(&id, || {
                    Ok(Box::new(std::io::Cursor::new(
                        format!("streamed-{id}").into_bytes(),
                    )))
                }) {
                    Ok((info, mut r)) => json!([info.name, enc(&read_all(&mut *r)), null]),
                    Err(e) => json!(["", null, err_json(&e)]),
                },
                5..=7 => {
                    let mut read = Vec::new();
                    let read_err = code - 5;
                    let res = cache.read_or_create(
                        &id,
                        &mut |_info, r| {
                            read = read_all(r);
                            match read_err {
                                0 => Ok(()),
                                1 => Err(Error::new("corrupt")),
                                _ => {
                                    Err(Error::with_kind(ErrorKind::Fatal, "fatal filecache error"))
                                }
                            }
                        },
                        &mut |_info, mut w| {
                            let r =
                                std::io::Write::write_all(&mut w, format!("rc-{id}").as_bytes());
                            let _ = w.close();
                            r.map_err(|e| Error::new(e.to_string()))
                        },
                    );
                    match res {
                        Ok(info) => json!([read_err, info.name, enc(&read), null]),
                        Err(e) => json!([
                            read_err,
                            nh_helpers::cache::filecache::filecache::clean_id(&id),
                            enc(&read),
                            err_json(&e)
                        ]),
                    }
                }
                8 => match cache.write_closer(&id) {
                    Ok((info, mut w)) => {
                        let _ = std::io::Write::write_all(&mut w, format!("wc-{id}").as_bytes());
                        let _ = w.close();
                        json!([info.name, null])
                    }
                    Err(e) => json!([
                        nh_helpers::cache::filecache::filecache::clean_id(&id),
                        err_json(&e)
                    ]),
                },
                9 => {
                    let (b, ok) = hc.get(&id);
                    json!([opt_bytes(b), ok])
                }
                10 => match sub {
                    0 => {
                        hc.set(&id, format!("set-{id}").as_bytes());
                        json!(["set"])
                    }
                    1 => {
                        hc.delete(&id);
                        json!(["delete"])
                    }
                    _ => json!(["string", enc(&cache.get_string(&id))]),
                },
                _ => panic!("op {code}"),
            });
            let got = match got.get("panic") {
                Some(p) => json!({ "panic": p.as_str().unwrap().replace(&root, "$ROOT") }),
                None => relativize(got, &root),
            };
            nops += 1;
            assert_eq!(
                normalize(&got),
                normalize(want),
                "seq {seq} op {op}: pre {}",
                c["pre"]
            );
        }
        let files: Vec<J> = list_dir(&dir)
            .into_iter()
            .map(|(p, b)| json!([p, b]))
            .collect();
        let mut want: Vec<J> = c["files"].as_array().cloned().unwrap_or_default();
        want.sort_by(|a, b| a[0].as_str().unwrap().cmp(b[0].as_str().unwrap()));
        assert_eq!(J::Array(files), J::Array(want), "seq {seq} files");
    }
    eprintln!("Cache operations: {nops} ops");
}

/// Error texts in results carry absolute paths; the oracle records them below `$ROOT`.
fn relativize(v: J, root: &str) -> J {
    match v {
        J::String(s) => J::String(s.replace(root, "$ROOT")),
        J::Array(a) => J::Array(a.into_iter().map(|x| relativize(x, root)).collect()),
        J::Object(m) => J::Object(
            m.into_iter()
                .map(|(k, x)| (k, relativize(x, root)))
                .collect(),
        ),
        x => x,
    }
}

/// Runs `f`, returning `{"panic": msg}` for a panic.
fn call_json(f: impl FnOnce() -> J) -> J {
    let r = call(|| serde_json::to_string(&f()).unwrap());
    match r.get("panic") {
        Some(_) => r,
        None => serde_json::from_str(r.as_str().unwrap()).unwrap(),
    }
}

#[test]
fn new_caches_matches_go() {
    let fx = fixture("filecache", "filecache.json.gz");
    let nc = fx["newCaches"].as_array().unwrap();
    let tmp = TempDir::new("newcaches");
    let root = tmp.str();
    let working_dir = format!("{root}/ncsites/seeksnack");
    std::fs::create_dir_all(&working_dir).unwrap();
    let cache_dir = format!("{root}/nccache/");
    assert_eq!(nc[0]["cacheDir"], json!("$ROOT/nccache/"));

    // The [caches] section of the oracle's hugo.toml.
    let params = |entries: &[(&str, Value)]| {
        let mut m = Map::new(MapType::Params);
        for (k, v) in entries {
            m.insert(*k, v.clone());
        }
        Value::map(m)
    };
    let mut m = Map::new(MapType::StringAny);
    m.insert(
        "getresource",
        params(&[
            ("dir", Value::string(":cacheDir/:project")),
            ("maxage", Value::Int(-1, go_value::IntKind::Int64)),
        ]),
    );
    m.insert(
        "images",
        params(&[("dir", Value::string(":resourceDir/_gen"))]),
    );
    m.insert(
        "misc",
        params(&[
            ("dir", Value::string(":resourceDir/misc")),
            ("maxage", Value::string("1h")),
        ]),
    );
    m.insert(
        "getjson",
        params(&[("dir", Value::string(format!("{root}/absjson")))]),
    );
    let bcfg = BaseConfig {
        working_dir: working_dir.clone(),
        cache_dir: cache_dir.clone(),
        publish_dir: "public".to_string(),
        ..Default::default()
    };
    let cfgs = decode_config(nh_hugofs::afero::new_os_fs().as_ref(), &bcfg, &m).unwrap();
    let got_cfgs = configs_dump(&cfgs, &root);
    let want_cfgs = nc[0]["configs"].clone();
    assert_eq!(got_cfgs, want_cfgs);

    let mut cfg = TestCfg::new(&working_dir);
    cfg.cache_dir = cache_dir;
    cfg.resource_dir = "myresources".to_string();
    cfg.sections.insert("caches".to_string(), Arc::new(cfgs));
    let fs = nh_hugofs::fs::new_from(nh_hugofs::afero::new_os_fs(), &cfg.base_config());
    let ps = PathSpec::new(fs, Arc::new(cfg)).unwrap();
    assert_eq!(
        json!(ps.abs_resources_dir.replace(&root, "$ROOT")),
        nc[0]["resources"]
    );
    let caches = new_caches(&ps).unwrap();
    for c in &nc[1..] {
        let k = c["cache"].as_str().unwrap();
        let cache = caches.get(&k.to_uppercase()).unwrap();
        cache
            .get_or_create_bytes(&format!("probe/{k}"), || Ok(k.as_bytes().to_vec()))
            .unwrap();
        let mut found = Vec::new();
        find(&tmp.path, k, &mut found);
        let found: Vec<String> = found.iter().map(|p| p.replace(&root, "$ROOT")).collect();
        assert_eq!(json!(found), c["found"], "{k}");
    }
}

fn find(d: &Path, name: &str, out: &mut Vec<String>) {
    let mut entries: Vec<_> = std::fs::read_dir(d).unwrap().map(|e| e.unwrap()).collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        let p = e.path();
        if p.is_dir() {
            find(&p, name, out);
        } else if p.file_name().unwrap() == name
            && p.parent().unwrap().file_name().unwrap() == "probe"
        {
            out.push(p.to_string_lossy().into_owned());
        }
    }
}
