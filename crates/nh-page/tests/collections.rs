//! Oracle test for the page collection functions (pages_sort, pages_cache, pagegroup,
//! pages_prev_next, pages_sort_search, pages_language_merge, taxonomy, weighted) against
//! `tools/go-oracle/nh-page/collections` (fixtures/collections/*.json.gz): every sort, group,
//! Next/Prev and merge over the page lists of real builds, with every site as the current site,
//! the cache sequences, and the taxonomies.

mod csupport;
mod support;

use std::collections::BTreeMap;

use csupport::{Build, key_json, load};
use go_value::Value;
use nh_page::page::{PageRef, Pages};
use nh_page::pagegroup::{GROUP_BY_METHODS, GroupByCall, GroupByKey, PagesGroup};
use nh_page::pages_sort as ps;
use nh_page::taxonomy::{self, Taxonomy};
use nh_page::weighted::{WeightedPage, WeightedPages};
use serde_json::Value as J;
use support::{fixture, fixture_files, gostring};

fn groups_json(b: &Build, g: &PagesGroup) -> Vec<J> {
    g.iter()
        .map(|pg| serde_json::json!({"key": key_json(b, &pg.key), "pages": b.indices(&pg.pages)}))
        .collect()
}

fn strs(c: &J, from: usize) -> Vec<String> {
    c["args"]
        .as_array()
        .map(|a| {
            a[from..]
                .iter()
                .map(|v| v.as_str().unwrap().to_string())
                .collect()
        })
        .unwrap_or_default()
}

/// Compares a Go group result (`CallRaw` of the encoded groups) with the Rust one.
fn check_groups(
    b: &Build,
    fails: &mut Vec<String>,
    c: &J,
    got: nh_common::Result<Option<PagesGroup>>,
) {
    let want = &c["res"];
    match got {
        Err(e) => {
            let w = want
                .get("panic")
                .or_else(|| want.get("err"))
                .and_then(|v| v.as_str());
            if w != Some(e.to_string().as_str()) {
                fails.push(format!("{c}\n   got error {e}"));
            }
        }
        Ok(g) => {
            let Some(w) = want.get("ok") else {
                fails.push(format!("{c}\n   got ok"));
                return;
            };
            let got = match &g {
                None => J::Null,
                Some(g) => J::Array(groups_json(b, g)),
            };
            let wg = if w.is_null() {
                J::Null
            } else {
                w["groups"].clone()
            };
            let unordered = w.get("unordered").is_some() || w.get("ties").is_some();
            let (mut g1, mut g2) = (got.clone(), wg.clone());
            if unordered {
                for v in [&mut g1, &mut g2] {
                    if let J::Array(a) = v {
                        a.sort_by_key(|x| x.to_string());
                    }
                }
            }
            if g1 != g2 {
                fails.push(format!("{c}\n   got {got}"));
            }
        }
    }
}

fn run_case(b: &Build, lists: &[Pages], c: &J, fails: &mut Vec<String>) {
    let op = c["op"].as_str().unwrap();
    if op == "cache" {
        let p = &lists[c["list"].as_u64().unwrap() as usize];
        nh_page::page::clear();
        b.set_current(0);
        let a = ps::by_title(p);
        let bb = ps::by_link_title(p);
        b.set_current(1);
        let a2 = ps::by_title(p);
        let b2 = ps::by_link_title(p);
        nh_page::page::clear();
        let a3 = ps::by_title(p);
        let b3 = ps::by_link_title(p);
        let got = J::Array(
            [a, bb, a2, b2, a3, b3]
                .iter()
                .map(|x| b.indices(x))
                .collect(),
        );
        if got != c["res"] {
            fails.push(format!("{c}\n   got {got}"));
        }
        return;
    }
    if op == "MergeByLanguage" {
        nh_page::page::clear();
        b.set_current(c["current"].as_u64().unwrap() as usize);
        let p1 = b.list(&c["p1"]);
        let p2 = b.list(&c["p2"]);
        let got = b.indices(&nh_page::pages_language_merge::merge_by_language(&p1, &p2));
        if got != c["res"] {
            fails.push(format!(
                "MergeByLanguage current {}\n   got {got}\n  want {}",
                c["current"], c["res"]
            ));
        }
        return;
    }
    let p = &lists[c["list"].as_u64().unwrap() as usize];
    b.set_current(c["current"].as_u64().unwrap() as usize);
    let args = c.get("args").cloned().unwrap_or(J::Null);
    let pages_res = |r: Pages| b.indices(&r);
    let got = match op {
        "ByWeight" => pages_res(ps::by_weight(p)),
        "ByTitle" => pages_res(ps::by_title(p)),
        "ByLinkTitle" => pages_res(ps::by_link_title(p)),
        "ByDate" => pages_res(ps::by_date(p)),
        "ByPublishDate" => pages_res(ps::by_publish_date(p)),
        "ByExpiryDate" => pages_res(ps::by_expiry_date(p)),
        "ByLastmod" => pages_res(ps::by_lastmod(p)),
        "ByLanguage" => pages_res(ps::by_language(p)),
        "ByLength" => pages_res(ps::by_length(&(), p)),
        "Reverse" => pages_res(ps::reverse(p)),
        "ByParam" => pages_res(ps::by_param(p, &Value::string(args[0].as_str().unwrap()))),
        "Limit" => pages_res(ps::limit(p, args[0].as_u64().unwrap() as usize)),
        "SortByDefault" => {
            let mut cp = p.clone();
            ps::sort_by_default(&mut cp);
            pages_res(cp)
        }
        "SortByLanguage" => {
            let mut cp = p.clone();
            ps::sort_by_language(&mut cp);
            pages_res(cp)
        }
        "NextPrev" => {
            let cur = &b.pages[args[0].as_u64().unwrap() as usize];
            let n = nh_page::pages_prev_next::next(p, &*cur.0);
            let pr = nh_page::pages_prev_next::prev(p, &*cur.0);
            serde_json::json!([b.opt_index(n.as_ref()), b.opt_index(pr.as_ref())])
        }
        "GroupBy" => {
            let key = args[0].as_str().unwrap();
            let r = nh_page::pagegroup::group_by_ctx(&(), p, key, &strs(c, 1));
            return check_groups(b, fails, c, r);
        }
        "GroupByParam" => {
            let key = args[0].as_str().unwrap();
            let r = nh_page::pagegroup::group_by_param(p, key, &strs(c, 1));
            return check_groups(b, fails, c, r);
        }
        "GroupByDate" | "GroupByPublishDate" | "GroupByExpiryDate" | "GroupByLastmod" => {
            let f = args[0].as_str().unwrap();
            let order = strs(c, 1);
            let r = match op {
                "GroupByDate" => nh_page::pagegroup::group_by_date(p, f, &order),
                "GroupByPublishDate" => nh_page::pagegroup::group_by_publish_date(p, f, &order),
                "GroupByExpiryDate" => nh_page::pagegroup::group_by_expiry_date(p, f, &order),
                _ => nh_page::pagegroup::group_by_lastmod(p, f, &order),
            };
            return check_groups(b, fails, c, r);
        }
        "GroupByParamDate" => {
            let key = args[0].as_str().unwrap();
            let f = args[1].as_str().unwrap();
            let r = nh_page::pagegroup::group_by_param_date(p, key, f, &strs(c, 2));
            return check_groups(b, fails, c, r);
        }
        _ => panic!("unknown op {op}"),
    };
    if got != c["res"] {
        fails.push(format!(
            "list {} current {} {op} {args}\n   got {got}\n  want {}",
            c["list"], c["current"], c["res"]
        ));
    }
}

fn arr(v: &J) -> &[J] {
    v.as_array().map(|a| a.as_slice()).unwrap_or(&[])
}

fn check_taxonomies(b: &Build, fx: &J, fails: &mut Vec<String>) -> usize {
    let mut n = 0;
    for tc in fx["taxonomies"].as_array().unwrap_or(&Vec::new()) {
        let mut tax: Taxonomy = BTreeMap::new();
        for te in arr(&tc["terms"]) {
            let owner = te["owner"].as_i64().unwrap();
            let owner: Option<PageRef> = (owner >= 0).then(|| b.pages[owner as usize].clone());
            let wp: WeightedPages = arr(&te["entries"])
                .iter()
                .map(|e| {
                    WeightedPage::new(
                        e[1].as_i64().unwrap(),
                        b.pages[e[0].as_u64().unwrap() as usize].clone(),
                        owner.clone(),
                    )
                })
                .collect();
            tax.insert(gostring(&te["term"]), wp);
        }
        let names = |o: &taxonomy::OrderedTaxonomy| -> J {
            J::Array(
                o.iter()
                    .map(|e| serde_json::json!([e.name, e.count()]))
                    .collect(),
            )
        };
        let ctx = format!("{} {}", tc["site"], tc["plural"]);
        n += 1;
        let got = names(&taxonomy::by_count(&tax));
        if got != tc["byCount"] {
            fails.push(format!(
                "{ctx} ByCount\n   got {got}\n  want {}",
                tc["byCount"]
            ));
        }
        for a in arr(&tc["alphabetical"]) {
            n += 1;
            b.set_current(a["current"].as_u64().unwrap() as usize);
            let mut got = names(&taxonomy::alphabetical(&tax));
            let mut want = a["names"].clone();
            if a["ties"].as_bool().unwrap() {
                // Go's order of collator-equal names is random (map order).
                for v in [&mut got, &mut want] {
                    if let J::Array(x) = v {
                        x.sort_by_key(|e| e.to_string());
                    }
                }
            }
            if got != want {
                fails.push(format!(
                    "{ctx} Alphabetical current {}\n   got {got}\n  want {want}",
                    a["current"]
                ));
            }
        }
        for g in arr(&tc["get"]) {
            n += 1;
            let k = gostring(&g[0]);
            let got = serde_json::json!([
                g[0],
                taxonomy::taxonomy_get(&tax, &k)
                    .map(|w| w.len())
                    .unwrap_or(0),
                taxonomy::taxonomy_count(&tax, &k)
            ]);
            if &got != g {
                fails.push(format!("{ctx} Get\n   got {got}\n  want {g}"));
            }
        }
        n += 1;
        let page = taxonomy::taxonomy_page(&tax).unwrap();
        let got = b.opt_index(page.as_ref());
        if got != tc["page"] {
            fails.push(format!("{ctx} Page got {got} want {}", tc["page"]));
        }
        for w in arr(&tc["weighted"]) {
            n += 1;
            let wp = &tax[&gostring(&w["term"])];
            let mut cp: WeightedPages = wp.iter().rev().cloned().collect();
            nh_page::weighted::sort(&mut cp);
            let got = J::Array(cp.iter().map(|x| J::from(b.index(&x.page))).collect());
            if got != w["sorted"] {
                fails.push(format!(
                    "{ctx} {} Sort\n   got {got}\n  want {}",
                    w["term"], w["sorted"]
                ));
            }
            let got = J::Array(
                wp.iter()
                    .map(|x| {
                        serde_json::json!([
                            b.opt_index(
                                nh_page::weighted::weighted_pages_next(wp, &x.page).as_ref()
                            ),
                            b.opt_index(
                                nh_page::weighted::weighted_pages_prev(wp, &x.page).as_ref()
                            )
                        ])
                    })
                    .collect(),
            );
            if got != w["nextPrev"] {
                fails.push(format!(
                    "{ctx} {} NextPrev\n   got {got}\n  want {}",
                    w["term"], w["nextPrev"]
                ));
            }
        }
    }
    n
}

#[test]
fn collections_match_go() {
    let mut fails = Vec::new();
    let mut total = 0;
    for f in fixture_files("collections") {
        if f == "methods.json.gz" {
            continue;
        }
        let fx = fixture(&format!("collections/{f}"));
        let b = load(&fx);
        let lists: Vec<Pages> = fx["lists"]
            .as_array()
            .unwrap()
            .iter()
            .map(|l| b.list(&l["pages"]))
            .collect();
        let mut last = (u64::MAX, u64::MAX);
        let mut n = 0;
        for c in fx["cases"].as_array().unwrap() {
            // The oracle clears the cache before the cases of each (list, current site).
            let k = (
                c["list"].as_u64().unwrap_or(u64::MAX),
                c["current"].as_u64().unwrap_or(u64::MAX),
            );
            if k != last {
                nh_page::page::clear();
                last = k;
            }
            run_case(&b, &lists, c, &mut fails);
            n += 1;
        }
        n += check_taxonomies(&b, &fx, &mut fails);
        eprintln!("{f}: {n} cases");
        total += n;
    }
    eprintln!("collections: {total} cases, {} failures", fails.len());
    for f in fails.iter().take(30) {
        eprintln!("{f}");
    }
    assert!(fails.is_empty(), "{} failures", fails.len());
}

#[test]
fn group_by_method_table_matches_go() {
    let fx = fixture("collections/methods.json.gz");
    let cases = fx["cases"].as_array().unwrap();
    assert_eq!(cases.len(), GROUP_BY_METHODS.len());
    for (c, (name, key, call)) in cases.iter().zip(GROUP_BY_METHODS) {
        assert_eq!(c["name"].as_str().unwrap(), *name);
        let k = match key {
            GroupByKey::Int => "int".to_string(),
            GroupByKey::String => "string".to_string(),
            GroupByKey::Other => "other".to_string(),
            GroupByKey::NotComparable(t) => format!("nc:{t}"),
        };
        assert_eq!(c["key"].as_str().unwrap(), k, "{name}");
        let cl = match call {
            GroupByCall::Ok => "ok",
            GroupByCall::TooFew => "toofew",
            GroupByCall::NotSupported => "notsupported",
        };
        assert_eq!(c["call"].as_str().unwrap(), cl, "{name}");
    }
}
