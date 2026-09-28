//! Oracle test: source.File and source.SourceSpec.IgnoreFile against
//! `tools/go-oracle/nh-helpers/srcfile` (fixtures/srcfile/srcfile.json.gz).

mod support;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use nh_common::glob::filename_filter::FilenameFilter;
use nh_config::config_provider::AllProvider;
use nh_helpers::pathspec::PathSpec;
use nh_helpers::source::file_info::{File, FileObject};
use nh_helpers::source::source_spec::SourceSpec;
use nh_hugofs::fileinfo::{FileMeta, new_file_meta_info};
use serde_json::{Value as J, json};
use support::*;

fn dump(f: &Arc<File>) -> J {
    json!({
        "Filename": enc(f.filename().as_bytes()),
        "Path": call(|| f.path()),
        "Dir": call(|| f.dir()),
        "Ext": call(|| f.ext()),
        "LogicalName": call(|| f.logical_name()),
        "BaseFileName": call(|| f.base_file_name()),
        "TranslationBaseName": call(|| f.translation_base_name()),
        "ContentBaseName": call(|| f.content_base_name()),
        "Section": call(|| f.section()),
        "UniqueID": call(|| f.unique_id().to_string()),
        "String": call(|| f.string()),
        "IsContentAdapter": f.is_content_adapter(),
        "IsZero": go_value::Object::is_zero(&FileObject(Some(f.clone()))).unwrap(),
    })
}

fn strs(v: &J) -> Vec<String> {
    match v {
        J::Null => Vec::new(),
        J::Array(a) => a.iter().map(|s| s.as_str().unwrap().to_string()).collect(),
        _ => panic!(),
    }
}

#[test]
fn source_file_matches_go() {
    let fx = fixture("srcfile", "srcfile.json.gz");
    let misses: Misses = Arc::new(Mutex::new(Vec::new()));
    let pp = build_parser(&fx["parser"], &misses);

    let mut fails = Vec::new();
    let mut n = 0;
    for c in fx["cases"].as_array().unwrap() {
        let p = c["path"].as_str().unwrap();
        let pi = pp.parse(nh_common::files::COMPONENT_FOLDER_CONTENT, &format!("/{p}"));
        assert_eq!(pi.lang(), c["lang"].as_str().unwrap(), "{p}");
        let meta = FileMeta {
            filename: go_path::filepath::join(&["/site/content", p]),
            lang: pi.lang().to_string(),
            path_info: Some(Arc::new(pi)),
            ..Default::default()
        };
        let f = File::new(new_file_meta_info("", false, meta));
        let got = dump(&f);
        n += 1;
        if got != c["file"] {
            fails.push(format!("{p}: got {got}, want {}", c["file"]));
        }
        let from = File::new_content_file_info_from(p, &format!("/x/{p}"));
        let got = dump(&from);
        n += 1;
        if got != c["from"] {
            fails.push(format!("from {p}: got {got}, want {}", c["from"]));
        }
    }

    // (*source.File)(nil).IsZero() and the template object.
    assert_eq!(fx["nil"]["nilIsZero"], json!(true));
    let nil_obj = FileObject(None);
    assert_eq!(
        go_value::Object::is_zero(&nil_obj),
        Some(fx["nil"]["nilIsZero"].as_bool().unwrap())
    );

    let m = misses.lock().unwrap();
    eprintln!(
        "source.File: {n} files checked, {} failures, {} parser misses",
        fails.len(),
        m.len()
    );
    for f in fails.iter().take(20) {
        eprintln!("  {f}");
    }
    assert!(m.is_empty(), "{m:?}");
    assert!(fails.is_empty());
}

#[test]
fn ignore_file_matches_go() {
    let fx = fixture("srcfile", "srcfile.json.gz");
    let names = strs(&fx["ignoreNames"]);
    let log: HashMap<String, bool> = fx["ignoreLog"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| (e[0].as_str().unwrap().to_string(), e[1].as_bool().unwrap()))
        .collect();
    let misses: Misses = Arc::new(Mutex::new(Vec::new()));

    let tmp = TempDir::new("ignore");
    let mut cfg = TestCfg::new(&tmp.str());
    let m2 = misses.clone();
    cfg.ignore_file = Some(Arc::new(move |s: &str| {
        log.get(s).copied().unwrap_or_else(|| {
            m2.lock().unwrap().push(format!("IgnoreFile({s:?})"));
            false
        })
    }));
    let fs = nh_hugofs::fs::new_from(nh_hugofs::afero::new_mem_map_fs(), &cfg.base_config());
    let ps: Arc<PathSpec> = PathSpec::new(fs, Arc::new(cfg)).unwrap();

    let filters: HashMap<String, (Vec<String>, Vec<String>)> = fx["filters"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            (
                f["name"].as_str().unwrap().to_string(),
                (strs(&f["includes"]), strs(&f["excludes"])),
            )
        })
        .collect();

    let mut checks = 0;
    for ic in fx["ignore"].as_array().unwrap() {
        let fname = ic["filter"].as_str().unwrap();
        let filter = if fname == "nil" {
            None
        } else {
            let (inc, exc) = &filters[fname];
            FilenameFilter::new(inc, exc).unwrap()
        };
        let sfs = match ic["fs"].as_str().unwrap() {
            "os" => nh_hugofs::afero::new_os_fs(),
            _ => nh_hugofs::afero::new_mem_map_fs(),
        };
        let ss = SourceSpec::new(ps.clone(), filter, sfs);
        let want = ic["r"].as_array().unwrap();
        for (name, w) in names.iter().zip(want) {
            checks += 1;
            assert_eq!(
                ss.ignore_file(name),
                w.as_bool().unwrap(),
                "{fname}/{}: {name:?}",
                ic["fs"]
            );
        }
    }
    let m = misses.lock().unwrap();
    eprintln!("IgnoreFile: {checks} checks, {} misses", m.len());
    assert!(m.is_empty(), "{m:?}");
}
