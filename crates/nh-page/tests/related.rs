//! Oracle test for `related.InvertedIndex`, `related.DecodeConfig` and `Pages.Related` against
//! `tools/go-oracle/nh-page/related` (fixtures/related/*.json.gz): `Related` for every regular page
//! of every site with the site's config, option maps, and direct index searches with the default
//! and five custom configs (documents, index subsets, fragments, named slices).

mod csupport;
mod support;

use std::sync::Arc;

use csupport::{Build, FakePage, load};
use go_value::{GoString, Map, MapType, Value};
use nh_common::types::types::KeyValues;
use nh_page::page::{PageRef, Pages};
use nh_page::pages_related::{PageDocument, RelatedDocsHandler, related_opt};
use nh_page::related::{Config, Document, IndexConfig, InvertedIndex, SearchOpts, decode_config};
use serde_json::Value as J;
use support::*;

fn config_from(j: &J) -> Config {
    Config {
        threshold: j["threshold"].as_i64().unwrap(),
        include_newer: j["includeNewer"].as_bool().unwrap(),
        to_lower: j["toLower"].as_bool().unwrap(),
        indices: j["indices"]
            .as_array()
            .map(|a| a.as_slice())
            .unwrap_or(&[])
            .iter()
            .map(|i| IndexConfig {
                name: gostring(&i["name"]),
                type_: i["type"].as_str().unwrap().to_string(),
                apply_filter: i["applyFilter"].as_bool().unwrap(),
                pattern: i["pattern"].as_str().unwrap().to_string(),
                weight: i["weight"].as_i64().unwrap(),
                cardinality_threshold: i["cardinalityThreshold"].as_i64().unwrap(),
                to_lower: i["toLower"].as_bool().unwrap(),
            })
            .collect(),
    }
}

fn config_json(c: &Config) -> J {
    let idx: Vec<J> = c
        .indices
        .iter()
        .map(|i| {
            serde_json::json!({
                "name": i.name, "type": i.type_, "applyFilter": i.apply_filter, "pattern": i.pattern,
                "weight": i.weight, "cardinalityThreshold": i.cardinality_threshold, "toLower": i.to_lower,
            })
        })
        .collect();
    serde_json::json!({
        "threshold": c.threshold, "includeNewer": c.include_newer, "toLower": c.to_lower,
        "indices": if idx.is_empty() { J::Null } else { J::Array(idx) },
    })
}

/// Encodes results like the oracle: entry, filtered headings, and runs of equal (publish date,
/// name) sorted by entry (Go's order within them is random).
fn docs_json(b: &Build, pages: &[PageRef]) -> J {
    struct E {
        i: usize,
        h: Option<Vec<String>>,
        key: (i64, i64, String),
    }
    let mut es: Vec<E> = pages
        .iter()
        .map(|p| {
            let f = p.0.as_any().downcast_ref::<FakePage>().unwrap();
            let d = p.0.publish_date();
            E {
                i: f.entry,
                h: f.headings_filtered.clone(),
                key: (
                    go_time::GoTimeExt::go_unix(&d),
                    go_time::GoTimeExt::nanosecond(&d),
                    nh_resource::resourcetypes::Resource::name(&*p.0),
                ),
            }
        })
        .collect();
    let mut ties = false;
    let mut i = 0;
    while i < es.len() {
        let mut j = i + 1;
        while j < es.len() && es[j].key == es[i].key {
            j += 1;
        }
        if j - i > 1 {
            ties = true;
            es[i..j].sort_by_key(|e| e.i);
        }
        i = j;
    }
    let _ = b;
    let docs: Vec<J> = es
        .iter()
        .map(|e| match &e.h {
            Some(h) => serde_json::json!({"i": e.i, "h": h}),
            None => serde_json::json!({"i": e.i}),
        })
        .collect();
    let mut m = serde_json::json!({ "docs": docs });
    if ties {
        m["ties"] = J::Bool(true);
    }
    m
}

fn docs_of(ds: &[Arc<dyn Document>]) -> Vec<PageRef> {
    ds.iter()
        .map(|d| d.as_any().downcast_ref::<PageDocument>().unwrap().0.clone())
        .collect()
}

fn result<T>(r: nh_common::Result<T>, f: impl FnOnce(T) -> J) -> J {
    match r {
        Ok(v) => serde_json::json!({"ok": f(v)}),
        Err(e) => serde_json::json!({"err": e.to_string()}),
    }
}

fn opts_value(i: u64, doc: &PageRef) -> Value {
    let mut m = Map::new(MapType::StringAny);
    let s = |x: &str| Value::string(x);
    match i {
        0 => {
            m.insert("document", doc.to_value());
            m.insert("indices", Value::string_list(["keywords"]));
        }
        1 => {
            m.insert("Document", doc.to_value());
            m.insert("indices", Value::any_list(vec![s("tags"), s("date")]));
        }
        2 => {
            m.insert("document", doc.to_value());
            m.insert("fragments", Value::string_list(["common", "heading-1"]));
        }
        3 => {
            m.insert("fragments", Value::any_list(vec![s("common")]));
        }
        4 => {
            m.insert("indices", Value::string_list(["nope"]));
            m.insert("document", doc.to_value());
        }
        _ => {
            m.insert(GoString::from("document"), s("not a page"));
        }
    }
    Value::map(m)
}

fn run_site(file: &str, fails: &mut Vec<String>) -> usize {
    let fx = fixture(&format!("related/{file}"));
    let b = load(&fx);
    for (si, s) in b.sites.iter().enumerate() {
        let cfg = config_from(&fx["siteConfigs"][si]);
        let _ = s.related.set(RelatedDocsHandler::new(cfg));
    }
    let lists: Vec<Pages> = fx["lists"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| b.list(l))
        .collect();
    let configs: Vec<Config> = fx["configs"]
        .as_array()
        .unwrap()
        .iter()
        .map(config_from)
        .collect();
    let mut index: Option<(u64, u64, InvertedIndex)> = None;
    let mut n = 0;
    for c in fx["cases"].as_array().unwrap() {
        n += 1;
        let op = c["op"].as_str().unwrap();
        let li = c["list"].as_u64().unwrap();
        let ps = &lists[li as usize];
        let doc = c
            .get("doc")
            .map(|d| b.pages[d.as_u64().unwrap() as usize].clone());
        let got = match op {
            "Related" => result(related_opt(&(), ps, &doc.unwrap().to_value()), |r| {
                r.map(|r| docs_json(&b, &r)).unwrap_or(J::Null)
            }),
            "RelatedOpts" => {
                let v = opts_value(c["opts"].as_u64().unwrap(), &doc.unwrap());
                result(related_opt(&(), ps, &v), |r| {
                    r.map(|r| docs_json(&b, &r)).unwrap_or(J::Null)
                })
            }
            "RelatedBadArg" => result(related_opt(&(), ps, &Value::string("a string")), |r| {
                r.map(|r| docs_json(&b, &r)).unwrap_or(J::Null)
            }),
            "Add" => {
                let ci = c["cfg"].as_u64().unwrap();
                let mut idx = InvertedIndex::new(configs[ci as usize].clone());
                let mut errs = Vec::new();
                for p in ps {
                    let d: Arc<dyn Document> = Arc::new(PageDocument(p.clone()));
                    if let Err(e) = idx.add(&[d]) {
                        errs.push(serde_json::json!([b.index(p), e.to_string()]));
                    }
                }
                idx.finalize().unwrap();
                index = Some((ci, li, idx));
                let got = if errs.is_empty() {
                    J::Null
                } else {
                    J::Array(errs)
                };
                if got != c["errs"] {
                    fails.push(format!("{file}: {c}\n   got {got}"));
                }
                continue;
            }
            "Search" | "SearchNamed" => {
                let (ci, cli, idx) = index.as_ref().unwrap();
                assert_eq!((*ci, *cli), (c["cfg"].as_u64().unwrap(), li));
                let strings = |k: &str| -> Vec<String> {
                    c.get(k)
                        .and_then(|v| v.as_array())
                        .map(|a| a.iter().map(|x| x.as_str().unwrap().to_string()).collect())
                        .unwrap_or_default()
                };
                let mut opts = SearchOpts {
                    document: doc.map(|d| Arc::new(PageDocument(d)) as Arc<dyn Document>),
                    indices: strings("indices"),
                    fragments: strings("fragments"),
                    ..Default::default()
                };
                if op == "SearchNamed" {
                    let values = match decode(&c["values"]) {
                        Value::List(l) => l.items.clone(),
                        _ => Vec::new(),
                    };
                    opts.named_slices = vec![KeyValues {
                        key: decode(&c["key"]),
                        values,
                    }];
                }
                result(idx.search(&opts), |ds| docs_json(&b, &docs_of(&ds)))
            }
            _ => panic!("unknown op {op}"),
        };
        if got != c["res"] {
            fails.push(format!("{file}: {c}\n   got {got}"));
        }
    }
    n
}

#[test]
fn related_matches_go() {
    let mut fails = Vec::new();
    let mut total = 0;
    for f in fixture_files("related") {
        if f == "decode.json.gz" {
            continue;
        }
        let n = run_site(&f, &mut fails);
        eprintln!("{f}: {n} cases");
        total += n;
    }
    eprintln!("related: {total} cases, {} failures", fails.len());
    for f in fails.iter().take(10) {
        eprintln!("{f}");
    }
    assert!(fails.is_empty());
}

#[test]
fn decode_config_matches_go() {
    let fx = fixture("related/decode.json.gz");
    for c in fx["cases"].as_array().unwrap() {
        let got = result(decode_config(&decode(&c["in"])), |cfg| config_json(&cfg));
        assert_eq!(got, c["res"], "{}", c["in"]);
    }
}
