//! The `load` oracle (tools/go-oracle/nh-allconfig/load): `allconfig.LoadConfig` of site trees
//! recreated in a temporary directory, compared section by section with Go.

mod support;

use serde_json::Value as J;
use support::*;

/// Mount lists whose `_jsconfig` auto-mounts come from a directory listing in OS order (Go's
/// `File.ReadDir`): the Go run and this test may see different orders on different file
/// systems, so those tails are compared sorted.
fn normalize_dir_order(v: &mut J) {
    match v {
        J::Object(o) => {
            for (k, x) in o.iter_mut() {
                if k == "mounts"
                    && let J::Array(a) = x
                {
                    sort_jsconfig_tail(a);
                }
                normalize_dir_order(x);
            }
        }
        J::Array(a) => {
            for x in a.iter_mut() {
                normalize_dir_order(x);
            }
        }
        J::String(s) if s.contains("_jsconfig") && (s.starts_with('{') || s.starts_with('[')) => {
            if let Ok(mut inner) = serde_json::from_str::<J>(s) {
                normalize_dir_order(&mut inner);
                // Keep the canonical (serde) form for the comparison.
                *s = serde_json::to_string(&inner).unwrap();
            }
        }
        _ => {}
    }
}

fn sort_jsconfig_tail(a: &mut [J]) {
    let is_js = |m: &J| {
        m.get("target")
            .or_else(|| m.get("Target"))
            .and_then(|t| t.as_str())
            .is_some_and(|t| t.starts_with("assets/_jsconfig/"))
    };
    let start = a.iter().position(is_js).unwrap_or(a.len());
    let end = start + a[start..].iter().take_while(|m| is_js(m)).count();
    a[start..end].sort_by_key(|m| serde_json::to_string(m).unwrap());
}

/// Go's output-format decoder replaces a `mediaType` string of the config tree with the
/// decoded `media.Type` when the format's map is the tree's own (a theme's or a language's
/// format map); the port's provider values are copies. Only the raw tree shows it: compare
/// such values by their type string.
fn normalize_media_type_objects(v: &mut J) {
    match v {
        J::Object(o) => {
            for (k, x) in o.iter_mut() {
                if k.eq_ignore_ascii_case("mediatype")
                    && let Some(t) = x.get("type").and_then(|t| t.as_str())
                {
                    *x = J::String(t.to_string());
                    continue;
                }
                normalize_media_type_objects(x);
            }
        }
        J::Array(a) => a.iter_mut().for_each(normalize_media_type_objects),
        _ => {}
    }
}

fn normalize_raw_trees(v: &mut J) {
    let fix = |s: &mut J| {
        if let Some(t) = s.as_str()
            && let Ok(mut j) = serde_json::from_str::<J>(t)
        {
            normalize_media_type_objects(&mut j);
            *s = j;
        }
    };
    if let Some(p) = v.get_mut("providerRoot") {
        fix(p);
    }
    if let Some(J::Array(mods)) = v.get_mut("modules") {
        for m in mods {
            if let Some(c) = m.get_mut("cfg") {
                fix(c);
                // A theme's config maps are merged into the site's tree by reference in
                // Go, and fromLoadConfigResult then writes into the languages maps of the
                // tree (the `params` of each language): the theme's own `languages` map
                // shows those writes. The port's module config is the theme file as read.
                if let Some(o) = c.as_object_mut() {
                    o.remove("languages");
                }
            }
        }
    }
}

/// The text dumps with mounts (`neohugo config mounts`, `neohugo config`) are reparsed and
/// normalised the same way.
fn normalize_text_dumps(v: &mut J) {
    if let Some(md) = v.get_mut("mountsDump")
        && let Some(s) = md.as_str()
    {
        // One JSON document per module.
        let docs: Vec<J> = serde_json::Deserializer::from_str(s)
            .into_iter::<J>()
            .map(|d| d.unwrap())
            .collect();
        let mut docs = J::Array(docs);
        normalize_dir_order(&mut docs);
        *md = docs;
    }
    if let Some(J::Object(dumps)) = v.get_mut("configDumps") {
        for (_, d) in dumps.iter_mut() {
            if let Some(s) = d.as_str() {
                let mut j: J = serde_json::from_str(s).unwrap();
                normalize_dir_order(&mut j);
                *d = j;
            }
        }
    }
}

fn run_group(name: &str) {
    let path = fixture_dir("load").join(format!("{name}.json.gz"));
    let fx = load_fixture(&path);
    let mut n = 0;
    let mut nondet = 0;
    let mut failures: Vec<String> = Vec::new();
    for row in fx["cases"].as_array().unwrap() {
        let c = &row["case"];
        let case_name = c["name"].as_str().unwrap();
        let loaded = load_case(c);
        let d = Dumper {
            tmp: loaded.tmp.str(),
        };
        let log = String::from_utf8_lossy(&loaded.log.lock().unwrap()).into_owned();
        let mut got = match &loaded.result {
            Ok(confs) => d.configs(confs),
            Err(e) => serde_json::json!({ "err": d.r(&e.to_string()) }),
        };
        got["log"] = J::String(d.r(&log));
        normalize_dir_order(&mut got);
        normalize_text_dumps(&mut got);
        normalize_raw_trees(&mut got);
        // Go's map order can change the result (`nondet`): every variant Go produced is
        // recorded, and the port must produce one of them.
        let mut wants = vec![row["result"].clone()];
        if let Some(v) = row["variants"].as_array() {
            wants.extend(v.iter().cloned());
        }
        let mut best: Option<Vec<String>> = None;
        for mut want in wants {
            normalize_dir_order(&mut want);
            normalize_text_dumps(&mut want);
            normalize_raw_trees(&mut want);
            let mut diffs = Vec::new();
            json_diff(&want, &got, "", &mut diffs);
            if best.as_ref().is_none_or(|b| diffs.len() < b.len()) {
                best = Some(diffs);
            }
        }
        let diffs = best.unwrap_or_default();
        if row["nondet"].as_bool() == Some(true) && diffs.is_empty() {
            nondet += 1;
        }
        if !diffs.is_empty() {
            failures.push(format!(
                "{case_name}: {} difference(s):\n  {}",
                diffs.len(),
                diffs
                    .iter()
                    .take(40)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("\n  ")
            ));
        }
        n += 1;
    }
    assert!(
        failures.is_empty(),
        "{name}: {} of {n} case(s) differ:\n{}",
        failures.len(),
        failures.join("\n")
    );
    eprintln!("{name}: {n} cases identical ({nondet} match one of several Go map-order variants)");
}

#[test]
fn basic() {
    run_group("basic");
}

#[test]
fn configdir() {
    run_group("configdir");
}

#[test]
fn env() {
    run_group("env");
}

#[test]
fn languages() {
    run_group("languages");
}

#[test]
fn merge() {
    run_group("merge");
}

#[test]
fn mounts() {
    run_group("mounts");
}

#[test]
fn repo() {
    run_group("repo");
}

#[test]
fn sections() {
    run_group("sections");
}

#[test]
fn seeksnack() {
    run_group("seeksnack");
}

#[test]
fn themes() {
    run_group("themes");
}
