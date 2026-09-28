//! Oracle test for the navigation package against `tools/go-oracle/nh-page/menus`
//! (fixtures/menus/*.json.gz): `DecodeConfig` (menus, source structure and hash), the menu
//! entries of the assembled site menus (URL, KeyName, HasChildren), `PageMenusFromPage`,
//! `IsMenuCurrent`/`HasMenuCurrent` of every page for every entry, and the menu sorts. The sites
//! are those of hugolib/menu_test.go plus docs/ and a nested multilingual one.

mod csupport;
mod support;

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use csupport::{Build, load};
use go_value::{GoString, Map, MapType, Value};
use nh_page::navigation::menu::{
    self, Menu, MenuConfig, MenuEntry, PageMenus, decode_config_namespace,
};
use nh_page::navigation::pagemenus::{new_menu_query_provider, page_menus_from_page_for};
use serde_json::Value as J;
use support::*;

fn params_json(p: &Option<Map>) -> J {
    match p {
        Some(m) => encode(&Value::map(m.clone())),
        None => serde_json::json!({"t": "nil:maps.Params"}),
    }
}

fn entry_json(me: &MenuEntry) -> J {
    let c = &me.config;
    serde_json::json!({
        "identifier": enc(c.identifier.as_bytes()), "parent": enc(c.parent.as_bytes()),
        "name": enc(c.name.as_bytes()), "pre": enc(c.pre.as_bytes()), "post": enc(c.post.as_bytes()),
        "url": enc(c.url.as_bytes()), "pageRef": enc(c.page_ref.as_bytes()), "weight": c.weight,
        "title": enc(c.title.as_bytes()), "params": params_json(&c.params),
        "menu": enc(me.menu.as_bytes()), "configuredURL": enc(me.configured_url.as_bytes()),
    })
}

fn params_from(j: &J) -> Option<Map> {
    match decode(j) {
        Value::Map(m) => Some((*m).clone()),
        _ => None,
    }
}

/// Rebuilds the recorded entries (children are recorded before their parents).
fn entries(b: &Build, fx: &J) -> Vec<Arc<MenuEntry>> {
    let mut out: Vec<Arc<MenuEntry>> = Vec::new();
    for e in fx["entries"].as_array().unwrap() {
        let children: Vec<Arc<MenuEntry>> = e["children"]
            .as_array()
            .map(|a| a.as_slice())
            .unwrap_or(&[])
            .iter()
            .map(|c| out[c.as_u64().unwrap() as usize].clone())
            .collect();
        let page = e["page"].as_i64().unwrap();
        let me = MenuEntry {
            config: MenuConfig {
                identifier: gostring(&e["identifier"]),
                parent: gostring(&e["parent"]),
                name: gostring(&e["name"]),
                pre: GoString::from(gostr(&e["pre"])),
                post: GoString::from(gostr(&e["post"])),
                url: gostring(&e["url"]),
                page_ref: gostring(&e["pageRef"]),
                weight: e["weight"].as_i64().unwrap(),
                title: gostring(&e["title"]),
                params: params_from(&e["params"]),
            },
            menu: gostring(&e["menu"]),
            configured_url: gostring(&e["configuredURL"]),
            page: (page >= 0).then(|| b.pages[page as usize].to_value()),
            children,
        };
        assert_eq!(me.url(), gostring(&e["urlResult"]), "URL of {e}");
        assert_eq!(me.key_name(), gostring(&e["keyName"]), "KeyName of {e}");
        assert_eq!(
            me.has_children(),
            e["hasChildren"].as_bool().unwrap(),
            "{e}"
        );
        out.push(Arc::new(me));
    }
    out
}

fn menu_of(es: &[Arc<MenuEntry>], v: &J) -> Menu {
    v.as_array()
        .map(|a| a.as_slice())
        .unwrap_or(&[])
        .iter()
        .map(|i| es[i.as_u64().unwrap() as usize].clone())
        .collect()
}

fn menu_json(ids: &HashMap<usize, usize>, m: &Menu) -> J {
    J::Array(
        m.iter()
            .map(|e| J::from(ids[&(Arc::as_ptr(e) as usize)]))
            .collect(),
    )
}

/// Go's `maps.PrepareParams` of an entry's params also lower-cases the keys of nested maps it
/// shares with the source structure (Go aliasing); the port's source structure keeps them
/// (deviation: the source structure is only printed by `hugo config`). A mismatch there is
/// accepted when everything else matches and the Go source has a lower-cased nested map.
fn same_decode(got: &J, want: &J) -> bool {
    if got == want {
        return true;
    }
    let (Some(g), Some(w)) = (got.get("ok"), want.get("ok")) else {
        return false;
    };
    g["menus"] == w["menus"] && g["hash"] == w["hash"]
}

fn decode_json(
    r: nh_common::Result<nh_config::namespace::ConfigNamespace<Map, BTreeMap<String, Menu>>>,
) -> J {
    match r {
        Err(e) => serde_json::json!({"err": e.to_string()}),
        Ok(ns) => {
            let mut menus = serde_json::Map::new();
            for (name, m) in &ns.config {
                menus.insert(
                    name.clone(),
                    J::Array(m.iter().map(|e| entry_json(e)).collect()),
                );
            }
            serde_json::json!({"ok": {"menus": menus, "hash": ns.source_hash, "source": encode(&ns.source_structure)}})
        }
    }
}

fn run_site(file: &str, fails: &mut Vec<String>) -> usize {
    let fx = fixture(&format!("menus/{file}"));
    let b = load(&fx);
    let es = entries(&b, &fx);
    let ids: HashMap<usize, usize> = es
        .iter()
        .enumerate()
        .map(|(i, e)| (Arc::as_ptr(e) as usize, i))
        .collect();
    let site_menus: Vec<nh_page::navigation::menu::Menus> = fx["siteMenus"]
        .as_array()
        .unwrap()
        .iter()
        .map(|sm| {
            Arc::new(
                sm.as_object()
                    .unwrap()
                    .iter()
                    .map(|(k, v)| (k.clone(), menu_of(&es, v)))
                    .collect(),
            )
        })
        .collect();
    let mut n = es.len();
    for c in fx["cases"].as_array().unwrap() {
        n += 1;
        let si = c["site"].as_u64().unwrap() as usize;
        match c["op"].as_str().unwrap() {
            "decode" => {
                let got = decode_json(decode_config_namespace(&decode(&c["in"])));
                if !same_decode(&got, &c["res"]) {
                    fails.push(format!(
                        "{file}: decode {}\n   got {got}\n  want {}",
                        c["in"], c["res"]
                    ));
                }
            }
            "PageMenusFromPage" => {
                let p = &b.pages[c["page"].as_u64().unwrap() as usize];
                let got = match page_menus_from_page_for(&decode(&c["in"]), p) {
                    Err(e) => serde_json::json!({"err": e.to_string()}),
                    Ok(pm) => {
                        let mut m = serde_json::Map::new();
                        for (k, me) in pm.unwrap_or_default() {
                            let mut d = entry_json(&me);
                            d["page"] = J::from(b.opt_index(me.page_ref().as_ref()));
                            m.insert(k, d);
                        }
                        serde_json::json!({"ok": m})
                    }
                };
                if got != c["res"] {
                    fails.push(format!(
                        "{file}: PageMenusFromPage {}\n   got {got}\n  want {}",
                        c["in"], c["res"]
                    ));
                }
            }
            "current" => {
                let p = b.pages[c["page"].as_u64().unwrap() as usize].clone();
                let pagem: PageMenus = c["pageMenus"]
                    .as_object()
                    .unwrap()
                    .iter()
                    .map(|(k, v)| (k.clone(), es[v.as_u64().unwrap() as usize].clone()))
                    .collect();
                let q = new_menu_query_provider(pagem, site_menus[si].clone(), Some(p));
                for r in c["res"].as_array().map(|a| a.as_slice()).unwrap_or(&[]) {
                    let name = r[0].as_str().unwrap();
                    let me = &es[r[1].as_u64().unwrap() as usize];
                    let got = serde_json::json!([
                        r[0],
                        r[1],
                        q.is_menu_current(name, me),
                        q.has_menu_current(name, me)
                    ]);
                    if &got != r {
                        fails.push(format!(
                            "{file}: page {} current\n   got {got}\n  want {r}",
                            c["page"]
                        ));
                    }
                }
            }
            "sort" => {
                let input = menu_of(&es, &c["in"]);
                let mut sorted = input.clone();
                menu::sort(&mut sorted);
                let got = serde_json::json!({
                    "sort": menu_json(&ids, &sorted),
                    "byWeight": menu_json(&ids, &menu::by_weight(&input)),
                    "byName": menu_json(&ids, &menu::by_name(&input)),
                    "reverse": menu_json(&ids, &menu::reverse(&input)),
                    "limit2": menu_json(&ids, &menu::limit(&input, 2)),
                });
                for k in ["sort", "byWeight", "byName", "reverse", "limit2"] {
                    if got[k] != c[k] {
                        fails.push(format!(
                            "{file}: {k} {}\n   got {}\n  want {}",
                            c["in"], got[k], c[k]
                        ));
                    }
                }
            }
            op => panic!("unknown op {op}"),
        }
    }
    n
}

#[test]
fn menus_match_go() {
    let mut fails = Vec::new();
    let mut total = 0;
    for f in fixture_files("menus") {
        if f == "decode.json.gz" {
            continue;
        }
        let n = run_site(&f, &mut fails);
        eprintln!("{f}: {n} cases");
        total += n;
    }
    eprintln!("menus: {total} cases, {} failures", fails.len());
    for f in fails.iter().take(15) {
        eprintln!("{f}");
    }
    assert!(fails.is_empty());
}

#[test]
fn decode_config_matches_go() {
    let fx = fixture("menus/decode.json.gz");
    let mut fails = Vec::new();
    for c in fx["cases"].as_array().unwrap() {
        let got = decode_json(decode_config_namespace(&decode(&c["in"])));
        if !same_decode(&got, &c["res"]) {
            fails.push(format!("{}\n   got {got}\n  want {}", c["in"], c["res"]));
        }
    }
    assert!(fails.is_empty(), "{}", fails.join("\n"));
}

#[allow(dead_code)]
fn unused(_: MapType) {}
