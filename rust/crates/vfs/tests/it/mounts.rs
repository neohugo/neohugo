//! The effective mounts against the Go oracle `oracle/allconfig/load/mounts.json.gz`
//! (`modules[0].mounts` after `Configs.Init`): defaults for unconfigured components, missing
//! sources, per-language `contentDir`/`staticDir`, multihost, legacy `*Dir` keys, the JS config
//! files, duplicates.

use std::fs;
use std::path::Path;

use neohugo_config::{LoadOptions, load};
use neohugo_testkit::fixture::oracle;
use neohugo_vfs::{Module, Mount, Vfs};
use serde_json::{Value as J, json};

/// Writes the case's files (`name/` is a directory) below `root/site`.
fn write_case(root: &Path, files: &serde_json::Map<String, J>) {
    let site = root.join("site");
    fs::create_dir_all(&site).unwrap();
    let root_str = root.to_str().unwrap();
    for (name, content) in files {
        let p = site.join(name);
        if name.ends_with('/') {
            fs::create_dir_all(&p).unwrap();
        } else {
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            let text = content.as_str().unwrap().replace("$ROOT", root_str);
            fs::write(&p, text).unwrap();
        }
    }
}

fn dump(m: &Mount, root: &str) -> J {
    let list = |v: &[String]| {
        if v.is_empty() {
            "null".to_owned()
        } else {
            serde_json::to_string(v).unwrap()
        }
    };
    json!({
        "source": m.source.replace(root, "$ROOT"),
        "target": m.target,
        "lang": m.lang.clone().unwrap_or_default(),
        "includeFiles": list(&m.include_files),
        "excludeFiles": list(&m.exclude_files),
        "disableWatch": m.disable_watch,
    })
}

/// The JS config mounts are compared as a set (Go lists them in directory order).
fn split(mounts: Vec<J>) -> (Vec<J>, Vec<String>) {
    let (mut js, rest): (Vec<J>, Vec<J>) = mounts.into_iter().partition(|m| {
        m["target"]
            .as_str()
            .unwrap()
            .starts_with("assets/_jsconfig")
    });
    js.sort_by_key(ToString::to_string);
    (rest, js.iter().map(ToString::to_string).collect())
}

#[test]
fn mounts_match_go() {
    let f: J = oracle("oracle/allconfig/load/mounts.json.gz");
    let cases = f["cases"].as_array().unwrap();
    let mut checked = 0;
    for c in cases {
        let name = c["case"]["name"].as_str().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        write_case(root, c["case"]["files"].as_object().unwrap());
        let options = LoadOptions {
            source: root.join("site"),
            env: vec![
                ("HOME".into(), root.join("home").to_str().unwrap().into()),
                (
                    "XDG_CACHE_HOME".into(),
                    root.join("xdg").to_str().unwrap().into(),
                ),
            ],
            ..LoadOptions::default()
        };
        let got = load(&options)
            .map_err(|e| e.to_string())
            .and_then(|cfg| Vfs::new(&cfg).map_err(|e| e.to_string()));
        if let Some(err) = c["result"].get("err") {
            assert!(got.is_err(), "{name}: want an error like {err}");
            checked += 1;
            continue;
        }
        let vfs = got.unwrap_or_else(|e| panic!("{name}: {e}"));
        let root_str = root.to_str().unwrap();
        let ours: Vec<J> = vfs
            .mounts()
            .iter()
            .filter(|m| m.module == Module::Project)
            .map(|m| dump(m, root_str))
            .collect();
        let want: Vec<J> = c["result"]["modules"][0]["mounts"]
            .as_array()
            .unwrap()
            .clone();
        let (ours, ours_js) = split(ours);
        let (want, want_js) = split(want);
        assert_eq!(ours, want, "{name}");
        assert_eq!(ours_js, want_js, "{name}: JS config mounts");
        checked += 1;
    }
    assert_eq!(checked, cases.len());
    eprintln!("mounts: {checked} cases equal");
}
