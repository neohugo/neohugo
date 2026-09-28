//! Template lookups against the Go oracle (`tools/go-oracle/nh-tplimpl/lookup`): every
//! `LookupPagesLayout` (the layout of every page and output format of the builds, paginator
//! pages, aliases, markdown render hooks), `LookupPartial`, `LookupShortcode` and
//! `LookupShortcodeByName` Go made while building the sites, plus grid lookups, replayed on the
//! Rust store built from the same layouts. A query's Consider func answers what Go's answered
//! for the same candidate, and the Rust store must ask about exactly the candidates Go asked
//! about.

mod support;

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use nh_tplimpl::templatestore::{TemplInfo, TemplateQuery};
use serde_json::Value as J;
use support::{
    Ids, category_from_string, desc_from_json, integration_fixtures, load_fixture, site,
    site_from_fixture,
};

/// Go's shortcode error lists the candidates in map order: compare them sorted.
fn norm_err(e: &str) -> String {
    if let Some(i) = e.find(" in [")
        && let Some(j) = e[i..].find(']')
    {
        let inner = &e[i + 5..i + j];
        let mut parts: Vec<&str> = inner.split(' ').collect();
        parts.sort();
        return format!("{} in [{}]{}", &e[..i], parts.join(" "), &e[i + j + 1..]);
    }
    e.to_string()
}

fn check_site(name: &'static str) {
    check(
        name,
        site(name),
        load_fixture("lookup", &format!("{name}.json.gz")),
    );
}

fn check(name: &str, s: support::Site, go: J) {
    let store = s.store();
    let ids = Arc::new(Ids::new(&store));

    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut failures = Vec::new();
    for r in go["records"].as_array().unwrap() {
        let op = r["op"].as_str().unwrap();
        *counts.entry(op.to_string()).or_default() += 1;
        let want = r["result"].as_str().unwrap();
        match op {
            "pages" | "shortcode" => {
                let q = &r["query"];
                let calls: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
                let consider = if r["hadConsider"].as_bool().unwrap() {
                    let answers: BTreeMap<String, bool> = r["considered"]
                        .as_array()
                        .map(|a| {
                            a.iter()
                                .map(|c| {
                                    (c[0].as_str().unwrap().to_string(), c[1].as_bool().unwrap())
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    let ids = ids.clone();
                    let calls = calls.clone();
                    let f: Arc<dyn Fn(&TemplInfo) -> bool + Send + Sync> =
                        Arc::new(move |c: &TemplInfo| {
                            let id = ids.id_of_ref(c);
                            let v = answers.get(&id).copied().unwrap_or(true);
                            calls.lock().unwrap().push(id);
                            v
                        });
                    Some(f)
                } else {
                    None
                };
                let tq = TemplateQuery {
                    path: q["path"].as_str().unwrap().to_string(),
                    name: q["name"].as_str().unwrap().to_string(),
                    category: category_from_string(q["category"].as_str().unwrap()),
                    desc: desc_from_json(&q["desc"]),
                    consider,
                };
                let (got, err) = if op == "pages" {
                    (store.lookup_pages_layout(&tq), String::new())
                } else {
                    match store.lookup_shortcode(&tq) {
                        Ok(t) => (t, String::new()),
                        Err(e) => (None, e.message().to_string()),
                    }
                };
                let got_id = ids.id(got.as_ref());
                if got_id != want {
                    failures.push(format!("{op} {q}:\n  go   {want}\n  rust {got_id}"));
                }
                if op == "shortcode" {
                    let want_err = norm_err(r["err"].as_str().unwrap());
                    if norm_err(&err) != want_err {
                        failures.push(format!("{op} {q}: error\n  go   {want_err}\n  rust {err}"));
                    }
                }
                if r["hadConsider"].as_bool().unwrap() {
                    let mut want_calls: Vec<String> = r["considered"]
                        .as_array()
                        .map(|a| {
                            a.iter()
                                .map(|c| c[0].as_str().unwrap().to_string())
                                .collect()
                        })
                        .unwrap_or_default();
                    want_calls.sort();
                    let mut got_calls = calls.lock().unwrap().clone();
                    got_calls.sort();
                    if got_calls != want_calls {
                        failures.push(format!(
                            "{op} {q}: considered\n  go   {want_calls:?}\n  rust {got_calls:?}"
                        ));
                    }
                }
            }
            "partial" => {
                let n = r["name"].as_str().unwrap();
                let got_id = ids.id(store.lookup_partial(n).as_ref());
                if got_id != want {
                    failures.push(format!("partial {n}:\n  go   {want}\n  rust {got_id}"));
                }
            }
            "byName" => {
                let n = r["name"].as_str().unwrap();
                let got_id = ids.id(store.lookup_shortcode_by_name(n).as_ref());
                if got_id != want {
                    failures.push(format!("byName {n}:\n  go   {want}\n  rust {got_id}"));
                }
            }
            _ => panic!("unknown op {op}"),
        }
    }
    eprintln!("{name}: {counts:?}");
    assert!(
        failures.is_empty(),
        "{name}: {} of {} lookups differ:\n{}",
        failures.len(),
        go["records"].as_array().unwrap().len(),
        failures[..failures.len().min(20)].join("\n")
    );
}

/// The layout trees of tplimpl's integration tests (tsupport.IntegrationSites).
#[test]
fn lookup_integration() {
    support::big_stack(|| {
        let stores = integration_fixtures("store");
        let mut lookups = integration_fixtures("lookup");
        assert_eq!(stores.len(), lookups.len());
        for (name, fx) in stores {
            let go = lookups.remove(&name).unwrap();
            check(&name, site_from_fixture(fx), go);
        }
    });
}

#[test]
fn lookup_docs() {
    support::big_stack(|| check_site("docs"));
}

#[test]
fn lookup_testsite() {
    support::big_stack(|| check_site("testsite"));
}

#[test]
fn lookup_legacy() {
    support::big_stack(|| check_site("legacy"));
}

#[test]
fn lookup_modern() {
    support::big_stack(|| check_site("modern"));
}

#[test]
fn lookup_themes() {
    support::big_stack(|| check_site("themes"));
}
