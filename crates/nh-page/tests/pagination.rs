//! Oracle test for `page.Paginate`, the `Pager`/`Paginator` methods, the pagination URL factory
//! and `page.ResolvePagerSize` against `tools/go-oracle/nh-page/pagination`
//! (fixtures/pagination/*.json.gz): every Paginate call of the real builds (the paginator of every
//! list page), replayed with pager sizes 1, 3, 10 and 100 and with grouped pages, plus error cases.

mod csupport;
mod support;

use std::sync::{Arc, Mutex};

use csupport::{Build, key_json, load};
use go_value::Value;
use nh_config::common_config::Pagination;
use nh_page::page::pages_to_value;
use nh_page::page_paths::TargetPathDescriptor;
use nh_page::pagegroup::{PageGroup, PagesGroup, pages_group_to_value};
use nh_page::pagination::{Pager, Paginator, paginate, resolve_pager_size_option};
use serde_json::Value as J;
use support::*;

fn group_value(b: &Build, g: &J) -> PagesGroup {
    g.as_array()
        .map(|a| a.as_slice())
        .unwrap_or(&[])
        .iter()
        .map(|pg| PageGroup {
            key: key_value(b, &pg["key"]),
            pages: b.list(&pg["pages"]),
        })
        .collect()
}

fn key_value(b: &Build, k: &J) -> Value {
    if k["t"] == "page" {
        return b.pages[k["i"].as_u64().unwrap() as usize].to_value();
    }
    decode(k)
}

fn seq_value(b: &Build, s: &J) -> Value {
    match s["t"].as_str().unwrap() {
        "nil" => Value::Invalid,
        "pages" => pages_to_value(&b.list(&s["pages"])),
        "groups" => pages_group_to_value(&group_value(b, &s["groups"])),
        "any" => Value::any_list(
            s["items"]
                .as_array()
                .map(|a| a.as_slice())
                .unwrap_or(&[])
                .iter()
                .map(|it| match it["t"].as_str().unwrap() {
                    "page" => b.pages[it["i"].as_u64().unwrap() as usize].to_value(),
                    "group" => group_value(b, &it["group"])[0].to_value(),
                    _ => decode(it),
                })
                .collect(),
        ),
        _ => decode(&s["v"]),
    }
}

fn groups_json(b: &Build, g: &PagesGroup) -> J {
    J::Array(
        g.iter()
            .map(|pg| serde_json::json!({"key": key_json(b, &pg.key), "pages": b.indices(&pg.pages)}))
            .collect(),
    )
}

fn pager_num(p: Option<Arc<Pager>>) -> i64 {
    p.map(|p| p.page_number()).unwrap_or(-1)
}

fn dump(b: &Build, p: &Arc<Paginator>) -> J {
    let pagers = p.pagers();
    let first = &pagers[0];
    let ps: Vec<J> = pagers
        .iter()
        .map(|pg| {
            serde_json::json!({
                "number": pg.page_number(),
                "url": enc(pg.url().as_bytes()),
                "pages": pg.pages_opt().map(|x| b.indices(&x)).unwrap_or(J::Null),
                "groups": pg.page_groups_opt().map(|g| groups_json(b, &g)).unwrap_or(J::Null),
                "numberOfElements": pg.number_of_elements(),
                "hasPrev": pg.has_prev(),
                "hasNext": pg.has_next(),
                "prev": pager_num(pg.prev()),
                "next": pager_num(pg.next()),
                "first": pager_num(Some(pg.first())),
                "last": pager_num(Some(pg.last())),
                "string": pg.string(),
            })
        })
        .collect();
    serde_json::json!({
        "pagerSize": first.paginator.pager_size(),
        "totalPages": first.paginator.total_pages(),
        "totalNumberOfElements": first.paginator.total_number_of_elements(),
        "pagers": ps,
    })
}

fn run_site(file: &str, fails: &mut Vec<String>) -> usize {
    let fx = fixture(&format!("pagination/{file}"));
    let b = load(&fx);
    let misses: Misses = Arc::new(Mutex::new(Vec::new()));
    let pp = build_parser(&fx["parser"], &misses);
    let paths = build_paths(&fx["paths"], &pp);
    let specs: Vec<_> = fx["pathspecs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| {
            let p = &d["pagination"];
            build_path_spec_with_pagination(
                d,
                Some(Pagination {
                    pager_size: p["pagerSize"].as_i64().unwrap(),
                    path: gostring(&p["path"]),
                    disable_aliases: p["disableAliases"].as_bool().unwrap(),
                }),
            )
        })
        .collect();
    let formats: Vec<_> = fx["formats"]
        .as_array()
        .unwrap()
        .iter()
        .map(output_format)
        .collect();
    let path_at = |v: &J| {
        let i = v.as_i64().unwrap();
        (i >= 0).then(|| paths[i as usize].clone())
    };
    let mut n = 0;
    for c in fx["cases"].as_array().unwrap() {
        let d = &c["td"];
        let td = TargetPathDescriptor {
            path_spec: specs[d["ps"].as_u64().unwrap() as usize].clone(),
            type_: formats[d["type"].as_u64().unwrap() as usize].clone(),
            kind: d["kind"].as_str().unwrap().to_string(),
            path: path_at(&d["path"]).expect("descriptor path"),
            section: path_at(&d["section"]),
            base_name: gostring(&d["baseName"]),
            prefix_file_path: gostring(&d["prefixFilePath"]),
            prefix_link: gostring(&d["prefixLink"]),
            force_prefix: d["forcePrefix"].as_bool().unwrap(),
            url: gostring(&d["url"]),
            addends: gostring(&d["addends"]),
            expanded_permalink: gostring(&d["expandedPermalink"]),
            ugly_urls: d["uglyURLs"].as_bool().unwrap(),
        };
        let seq = seq_value(&b, &c["seq"]);
        let got = match paginate(&td, &seq, c["size"].as_i64().unwrap()) {
            Ok(p) => serde_json::json!({"ok": dump(&b, &p)}),
            Err(e) => serde_json::json!({"err": e.to_string()}),
        };
        n += 1;
        if got != c["res"] {
            fails.push(format!(
                "{file}: {} size {} seq {}\n   got {got}\n  want {}",
                c["src"], c["size"], c["seq"], c["res"]
            ));
        }
    }
    assert!(misses.lock().unwrap().is_empty(), "{file}: parser misses");
    n
}

#[test]
fn paginate_matches_go() {
    let mut fails = Vec::new();
    let mut total = 0;
    for f in fixture_files("pagination") {
        if f == "resolve.json.gz" {
            continue;
        }
        let n = run_site(&f, &mut fails);
        eprintln!("{f}: {n} cases");
        total += n;
    }
    eprintln!("pagination: {total} cases, {} failures", fails.len());
    for f in fails.iter().take(10) {
        eprintln!("{f}");
    }
    assert!(fails.is_empty());
}

#[test]
fn resolve_pager_size_matches_go() {
    let fx = fixture("pagination/resolve.json.gz");
    for c in fx["cases"].as_array().unwrap() {
        let opts: Vec<Value> = c["options"]
            .as_array()
            .unwrap()
            .iter()
            .map(decode)
            .collect();
        let got = match resolve_pager_size_option(&opts) {
            Ok(n) => serde_json::json!({"ok": {"t": "int", "v": n.to_string()}}),
            Err(e) => serde_json::json!({"err": e.to_string()}),
        };
        assert_eq!(got, c["res"], "{c}");
    }
}
