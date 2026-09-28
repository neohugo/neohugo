//! Differential tests against `tools/go-oracle/nh-hugofs/walk`: BaseFs built from recorded
//! module mounts over recreated trees; the component walks (Walkway order), Stat/Open/ReadDir on
//! the component views and the RootMappingFs, MakePathRelative, ReverseLookup, RealDirs, Glob,
//! and Go's error texts.

mod support;

use std::sync::{Arc, Mutex};

use nh_config::config_provider::AllProvider;
use nh_hugofs::afero::Fs;
use nh_hugofs::fileinfo::FileMetaInfo;
use nh_hugofs::filesystems::basefs::{BaseFs, SourceFilesystem};
use nh_hugofs::paths::Paths;
use nh_hugofs::walk::{Walkway, WalkwayConfig};
use serde_json::{Value as J, json};
use support::*;

struct Ctx {
    root: String,
}

impl Ctx {
    fn rel(&self, s: &str) -> String {
        s.replace(&self.root, PLACEHOLDER)
    }
    fn unrel(&self, s: &str) -> String {
        s.replace(PLACEHOLDER, &self.root)
    }

    fn fi(&self, fi: &FileMetaInfo) -> J {
        let m = fi.meta();
        let pi = m
            .path_info
            .as_ref()
            .map(|p| p.path().to_string())
            .unwrap_or_default();
        json!([
            fi.name(),
            fi.is_dir(),
            self.rel(&m.filename),
            m.path_info.is_some(),
            pi,
            m.lang,
            m.lang_index,
            m.weight,
            m.module_ordinal,
            m.component,
            m.module,
            m.is_project,
            m.watch,
            self.rel(&m.base_dir),
            self.rel(&m.source_root),
            m.name,
            fi.type_bits(),
        ])
    }
}

fn sort_rows(rows: &mut [J]) {
    rows.sort_by_key(|r| r.to_string());
}

fn views(b: &BaseFs) -> Vec<(String, Arc<SourceFilesystem>, bool)> {
    let mut v = vec![
        ("content".to_string(), b.content.clone(), false),
        ("data".to_string(), b.data.clone(), false),
        ("i18n".to_string(), b.i18n.clone(), false),
        ("layouts".to_string(), b.layouts.clone(), false),
        ("archetypes".to_string(), b.archetypes.clone(), false),
        ("assets".to_string(), b.assets.clone(), false),
        (
            "assetsDup".to_string(),
            b.assets_with_duplicates_preserved.clone(),
            false,
        ),
    ];
    for (k, s) in &b.static_ {
        v.push((format!("static:{k}"), s.clone(), true));
    }
    v
}

fn walk(ctx: &Ctx, fs: Arc<dyn Fs>, sorted: bool) -> (Vec<J>, Option<String>) {
    let mut rows = Vec::new();
    let mut wfn = |path: &str, fi: &FileMetaInfo| -> nh_common::Result<()> {
        let mut row = vec![J::String(ctx.rel(path))];
        row.extend(ctx.fi(fi).as_array().unwrap().iter().cloned());
        rows.push(J::Array(row));
        Ok(())
    };
    let mut w = Walkway::new(WalkwayConfig::new(fs, &mut wfn));
    let err = w.walk().err().map(|e| ctx.rel(&e.to_string()));
    drop(w);
    if sorted {
        sort_rows(&mut rows);
    }
    (rows, err)
}

fn stat(ctx: &Ctx, fs: &dyn Fs, name: &str) -> J {
    match fs.stat(name) {
        Ok(fi) => json!({"fi": ctx.fi(&fi)}),
        Err(e) => json!({"err": ctx.rel(&e.to_string())}),
    }
}

fn read_dir(ctx: &Ctx, fs: &dyn Fs, name: &str, sorted: bool) -> J {
    let mut f = match fs.open(name) {
        Ok(f) => f,
        Err(e) => return json!({"err": ctx.rel(&e.to_string())}),
    };
    let r = f.read_dir(-1);
    let _ = f.close();
    match r {
        Err(e) => json!({"err": ctx.rel(&e.to_string())}),
        Ok(fis) => {
            let mut rows: Vec<J> = fis.iter().map(|fi| ctx.fi(fi)).collect();
            if sorted {
                sort_rows(&mut rows);
            }
            json!({"entries": rows})
        }
    }
}

fn opt(c: &J, k: &str) -> J {
    c.get(k).cloned().unwrap_or(J::Null)
}

fn run_site(name: &str) -> usize {
    let fixture = load_fixture(&fixture_dir("walk").join(format!("{name}.json.gz")));
    let tmp = TempDir::new(name);
    let root_path = tmp.path.join("site");
    std::fs::create_dir_all(&root_path).unwrap();
    materialize(&root_path, &fixture["tree"]);
    let root = root_path.to_str().unwrap().to_string();
    let ctx = Ctx { root: root.clone() };

    let misses: Misses = Arc::new(Mutex::new(Vec::new()));
    let cfg = Arc::new(TestCfg::from_fixture(&fixture, &root, &misses));
    let osfs: Arc<dyn Fs> = Arc::new(nh_hugofs::afero::OsFs);
    let fs = nh_hugofs::fs::new_from(osfs, &cfg.base_config());
    let p = Paths::new(fs, cfg.clone()).unwrap();
    assert_eq!(
        ctx.rel(&p.abs_publish_dir),
        fixture["config"]["absPublishDir"].as_str().unwrap()
    );
    assert_eq!(
        ctx.rel(&p.abs_resources_dir),
        fixture["config"]["absResourcesDir"].as_str().unwrap()
    );
    let b = BaseFs::new(&p).unwrap();

    let views = views(&b);
    let view = |id: &str| -> Arc<SourceFilesystem> {
        views
            .iter()
            .find(|v| v.0 == id)
            .unwrap_or_else(|| panic!("view {id}"))
            .1
            .clone()
    };

    let mut failures = Vec::new();
    let mut n = 0;
    for c in fixture["cases"].as_array().unwrap() {
        n += 1;
        let op = c["op"].as_str().unwrap();
        let fs_id = c["fs"].as_str().unwrap_or("");
        let cname = c["name"].as_str().unwrap_or("");
        let name = ctx.unrel(cname);
        let fs_for = |id: &str| -> Arc<dyn Fs> {
            if let Some(i) = id.strip_prefix("rfs") {
                let i: usize = i.parse().unwrap();
                return b.root_fss[i].clone();
            }
            match id {
                "work" => b.work.clone(),
                "source" => b.source_fs.clone(),
                _ => view(id).fs.clone(),
            }
        };
        let (want, got): (J, J) = match op {
            "walk" => {
                let sorted = c["sorted"].as_bool().unwrap();
                let (rows, err) = walk(&ctx, view(fs_id).fs.clone(), sorted);
                let mut want_rows = opt(c, "entries");
                if sorted && let J::Array(a) = &mut want_rows {
                    sort_rows(a);
                }
                (
                    json!({"entries": want_rows, "err": opt(c, "err")}),
                    json!({"entries": if rows.is_empty() { J::Null } else { J::Array(rows) }, "err": err}),
                )
            }
            "stat" => {
                let fs = fs_for(fs_id);
                let got = stat(&ctx, fs.as_ref(), &name);
                let want = match c.get("err") {
                    Some(e) => json!({"err": e}),
                    None => json!({"fi": c["fi"]}),
                };
                (want, got)
            }
            "readdir" => {
                let fs = fs_for(fs_id);
                let sorted = c["sorted"].as_bool().unwrap();
                let got = read_dir(&ctx, fs.as_ref(), &name, sorted);
                let want = match c.get("err") {
                    Some(e) => json!({"err": e}),
                    None => {
                        let mut e = c["entries"].as_array().cloned().unwrap_or_default();
                        if sorted {
                            sort_rows(&mut e);
                        }
                        json!({"entries": e})
                    }
                };
                (want, got)
            }
            "mounts" => {
                let i: usize = fs_id.strip_prefix("rfs").unwrap().parse().unwrap();
                let got = match b.root_fss[i].mounts(&name) {
                    Ok(fis) => {
                        let rows: Vec<J> = fis.iter().map(|fi| ctx.fi(fi)).collect();
                        json!({"entries": if rows.is_empty() { J::Null } else { J::Array(rows) }})
                    }
                    Err(e) => json!({"err": ctx.rel(&e.to_string())}),
                };
                let want = match c.get("err") {
                    Some(e) => json!({"err": e}),
                    None => json!({"entries": c["entries"]}),
                };
                (want, got)
            }
            "makePathRelative" => {
                let check = c["check"].as_bool().unwrap();
                let r = view(fs_id).make_path_relative(&name, check);
                (
                    json!([c["rel"], c["ok"]]),
                    json!([r.clone().unwrap_or_default(), r.is_some()]),
                )
            }
            "contains" => (c["res"].clone(), json!(view(fs_id).contains(&name))),
            "realDirs" => {
                let rd: Vec<String> = view(fs_id)
                    .real_dirs(cname)
                    .iter()
                    .map(|x| ctx.rel(x))
                    .collect();
                (
                    c["res"].clone(),
                    if rd.is_empty() { J::Null } else { json!(rd) },
                )
            }
            "resolvePaths" => {
                let cps: Vec<J> = b
                    .resolve_paths(&name)
                    .iter()
                    .map(|cp| json!([cp.component, cp.path, cp.lang, cp.watch]))
                    .collect();
                (
                    c["res"].clone(),
                    if cps.is_empty() {
                        J::Null
                    } else {
                        J::Array(cps)
                    },
                )
            }
            "isContent" => (c["res"].clone(), json!(b.is_content(&name))),
            "isStatic" => (c["res"].clone(), json!(b.is_static(&name))),
            "makeStaticPathRelative" => {
                (c["res"].clone(), json!(b.make_static_path_relative(&name)))
            }
            "resolveJSConfigFile" => (
                c["res"].clone(),
                json!(ctx.rel(&b.resolve_js_config_file(cname))),
            ),
            "absProjectContentDir" => {
                let got = match b.abs_project_content_dir(&name) {
                    Ok((rel, abs)) => json!({"res": [ctx.rel(&rel), ctx.rel(&abs)]}),
                    Err(e) => json!({"err": ctx.rel(&e.to_string())}),
                };
                let want = match c.get("err") {
                    Some(e) => json!({"err": e}),
                    None => json!({"res": c["res"]}),
                };
                (want, got)
            }
            "watchFilenames" => {
                let mut wf: Vec<String> = b.watch_filenames().iter().map(|x| ctx.rel(x)).collect();
                wf.sort();
                (
                    c["res"].clone(),
                    if wf.is_empty() { J::Null } else { json!(wf) },
                )
            }
            "statResource" => {
                let lang = c["lang"].as_str().unwrap();
                let (r, fs) = b.stat_resource(lang, cname);
                let which = if Arc::ptr_eq(&fs, &b.assets.fs) {
                    "assets"
                } else if Arc::ptr_eq(&fs, &b.content.fs) {
                    "content"
                } else {
                    "static"
                };
                let got = match r {
                    Ok(fi) => json!({"which": which, "fi": ctx.fi(&fi)}),
                    Err(e) => json!({"which": which, "err": ctx.rel(&e.to_string())}),
                };
                let want = match c.get("err") {
                    Some(e) => json!({"which": c["which"], "err": e}),
                    None => json!({"which": c["which"], "fi": c["fi"]}),
                };
                (want, got)
            }
            "glob" | "globStop" | "globMissing" | "globErr" => {
                let fs = view(fs_id).fs.clone();
                let mut matched: Vec<String> = Vec::new();
                let r = nh_hugofs::glob::glob(fs, cname, &mut |fi: &FileMetaInfo| {
                    if op == "globErr" {
                        return Err(nh_common::herrors::Error::new("handle failed"));
                    }
                    matched.push(ctx.rel(&fi.meta().filename));
                    Ok(op == "globStop" && matched.len() == 2)
                });
                let got = json!({
                    "res": if matched.is_empty() { J::Null } else { json!(matched) },
                    "err": r.err().map(|e| ctx.rel(&e.to_string())),
                });
                let want = json!({"res": opt(c, "res"), "err": opt(c, "err")});
                (want, got)
            }
            _ => panic!("unknown op {op}"),
        };
        if want != got {
            failures.push(format!(
                "{name}: {op} fs={fs_id} name={cname:?}\n  want: {want}\n   got: {got}"
            ));
        }
    }

    let misses = misses.lock().unwrap();
    assert!(
        misses.is_empty(),
        "{name}: PathParser callbacks Go never made: {:?}",
        &misses[..misses.len().min(20)]
    );
    if !failures.is_empty() {
        let shown: Vec<&String> = failures.iter().take(25).collect();
        panic!(
            "{name}: {} of {n} cases differ:\n{}",
            failures.len(),
            shown
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join("\n")
        );
    }
    n
}

#[test]
fn walk_docs() {
    run_site("docs");
}

#[test]
fn walk_testsite() {
    run_site("testsite");
}

#[test]
fn walk_multilang() {
    run_site("multilang");
}

#[test]
fn walk_mounts() {
    run_site("mounts");
}

#[test]
fn walk_multihost() {
    run_site("multihost");
}

#[test]
fn walk_empty() {
    run_site("empty");
}
