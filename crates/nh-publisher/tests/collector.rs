//! Differential test of the HTML elements collector (`publisher/htmlElementsCollector.go`)
//! against the Go oracle `tools/go-oracle/nh-publisher/collector`: `parseHTMLElement` of
//! adversarial element strings, whole documents (one Write each, per document and per group
//! through one collector), multi-Write streams (random splits and the Write calls of the HTML
//! minifier) and `isClosedByTag`.

mod common;

use std::collections::BTreeMap;

use nh_config::common_config::BuildStats;
use nh_publisher::html_elements_collector::{
    HtmlElements, HtmlElementsCollector, HtmlElementsCollectorWriter, is_closed_by_tag,
    parse_html_element,
};
use serde_json::Value;

fn conf(name: &str) -> BuildStats {
    let mut c = BuildStats {
        enable: true,
        ..Default::default()
    };
    match name {
        "all" => {}
        "noids" => c.disable_ids = true,
        "noclasses" => c.disable_classes = true,
        "notags" => c.disable_tags = true,
        "classesonly" => {
            c.disable_tags = true;
            c.disable_ids = true;
        }
        _ => panic!("conf {name}"),
    }
    c
}

fn list(v: &Value) -> Option<Vec<String>> {
    match v {
        Value::Null => None,
        Value::Array(a) => Some(a.iter().map(common::string).collect()),
        _ => panic!("list {v}"),
    }
}

fn elements(v: &Value) -> HtmlElements {
    HtmlElements {
        tags: list(&v["tags"]),
        classes: list(&v["classes"]),
        ids: list(&v["ids"]),
    }
}

#[test]
fn collector_matches_go() {
    let recs = common::read_jsonl_gz(&common::fixture("collector/collector.jsonl.gz"));
    assert!(recs.len() > 50_000, "{} records", recs.len());
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut failures = Vec::new();
    let mut n_failed = 0;
    // Group documents in record order (for the "group" records).
    let mut group_docs: BTreeMap<String, Vec<Vec<u8>>> = BTreeMap::new();

    for r in &recs {
        let t = r["t"].as_str().unwrap();
        *counts.entry(t.to_string()).or_default() += 1;
        let (ok, what) = match t {
            "el" => {
                let s = common::string(&r["s"]);
                let got = parse_html_element(&conf(r["conf"].as_str().unwrap()), &s);
                let ok = match (&got, r.get("panic").or(r.get("err"))) {
                    (Err(e), Some(p)) => e == p.as_str().unwrap(),
                    (Ok(el), None) => {
                        el.tag == common::string(&r["tag"])
                            && Some(el.classes.clone()).filter(|c| !c.is_empty())
                                == list(&r["classes"])
                            && Some(el.ids.clone()).filter(|c| !c.is_empty()) == list(&r["ids"])
                    }
                    _ => false,
                };
                (ok, format!("el {s:?}: got {got:?}"))
            }
            "doc" => {
                let doc = common::bytes(&r["doc"]);
                if r["conf"] == "all" {
                    group_docs
                        .entry(r["g"].as_str().unwrap().to_string())
                        .or_default()
                        .push(doc.clone());
                }
                let mut c = HtmlElementsCollector::new(conf(r["conf"].as_str().unwrap()));
                c.write(&doc);
                let got = c.get_html_elements();
                (
                    r.get("panic").is_none() && got == elements(&r["want"]),
                    format!("doc {}: got {got:?}", common::show(&doc)),
                )
            }
            "group" => {
                let g = r["g"].as_str().unwrap();
                let docs = &group_docs[g];
                assert_eq!(docs.len() as u64, r["n"].as_u64().unwrap(), "group {g}");
                let mut c = HtmlElementsCollector::new(conf(r["conf"].as_str().unwrap()));
                for d in docs {
                    c.write(d);
                }
                let got = c.get_html_elements();
                (
                    r.get("panic").is_none() && got == elements(&r["want"]),
                    format!("group {g} {}: got {got:?}", r["conf"]),
                )
            }
            "chunks" => {
                let chunks: Vec<Vec<u8>> = r["chunks"]
                    .as_array()
                    .map(|a| a.iter().map(common::bytes).collect())
                    .unwrap_or_default();
                let mut c = HtmlElementsCollector::new(conf(r["conf"].as_str().unwrap()));
                let err = {
                    let mut w = HtmlElementsCollectorWriter::new(&mut c);
                    for ch in &chunks {
                        w.write(ch);
                    }
                    w.err.clone().unwrap_or_default()
                };
                let got = c.get_html_elements();
                (
                    r.get("panic").is_none()
                        && got == elements(&r["want"])
                        && err == r["err"].as_str().unwrap(),
                    format!("chunks {chunks:?}: got {got:?} err {err:?}"),
                )
            }
            "closed" => {
                let b = common::bytes(&r["b"]);
                let tag = common::bytes(&r["tag"]);
                let got = is_closed_by_tag(&b, &tag);
                (
                    got == r["want"].as_bool().unwrap(),
                    format!("closed {}: got {got}", common::show(&b)),
                )
            }
            _ => panic!("record type {t}"),
        };
        if !ok {
            n_failed += 1;
            if failures.len() < 10 {
                failures.push(format!("{what}\n  want {r}"));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{n_failed} failures ({counts:?}):\n{}",
        failures.join("\n")
    );
    assert!(
        counts["group"] >= 25 && counts["chunks"] > 1000,
        "{counts:?}"
    );
}

/// The upstream `TestEndsWithTag` table.
#[test]
fn upstream_ends_with_tag() {
    for (s, tag, want) in [
        ("", "div", false),
        ("foo", "div", false),
        ("foo<div>", "div", false),
        ("foo/div>", "div", false),
        ("foo//div>", "div", false),
        ("foo</>", "div", false),
        ("foo</div>", "div", true),
        ("foo<  / div>", "div", true),
        ("foo<  / div   \n>", "div", true),
        ("foo</DIV>", "div", true),
        (
            r##"</defs><g><g><path fill="#010101" d=asdf"/>"##,
            "div",
            false,
        ),
    ] {
        assert_eq!(
            is_closed_by_tag(s.as_bytes(), tag.as_bytes()),
            want,
            "{s:?}"
        );
    }
}

/// `Merge` + `Sort` (the hugo_stats.json merge of the sites' collectors).
#[test]
fn merge_and_sort() {
    let mut h = HtmlElements::default();
    h.merge(&HtmlElements::default());
    assert_eq!(h, HtmlElements::default());
    h.merge(&HtmlElements {
        tags: Some(vec!["p".into(), "a".into()]),
        classes: None,
        ids: Some(vec!["x".into()]),
    });
    h.merge(&HtmlElements {
        tags: Some(vec!["a".into(), "b".into()]),
        classes: None,
        ids: None,
    });
    h.sort();
    assert_eq!(h.tags, Some(vec!["a".into(), "b".into(), "p".into()]));
    assert_eq!(h.classes, None);
    assert_eq!(h.ids, Some(vec!["x".into()]));
}
