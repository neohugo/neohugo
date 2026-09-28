//! The parse trees of the template store after Hugo's AST transforms and html/template escaping
//! against the Go oracle (`tools/go-oracle/nh-tplimpl/escdump`): for every site, every tree of
//! the main html namespace (with the derived `name$htmltemplate_*` templates), of the main text
//! namespace and of every base-applied variant's namespace (its own trees; the number of trees
//! it shares with the main namespace) as `NewStore` leaves them, byte for byte (`Root.String()`).

mod support;

use nh_tplimpl::engine::parse::NodeLike;
use nh_tplimpl::engine::{Template, TextTemplate};
use nh_tplimpl::templatestore::TemplateStore;
use serde_json::{Value as J, json};
use support::{
    Ids, diff_json, integration_fixtures, load_fixture, norm_name, site, site_from_fixture,
};

fn dump_namespace(label: &str, tt: &TextTemplate, main: Option<&TextTemplate>) -> J {
    let mut list: Vec<(String, String)> = Vec::new();
    let mut shared = 0;
    for t in tt.all() {
        let name = t.name();
        if let Some(main) = main
            && let Some(mt) = main.lookup(&name)
        {
            let same = match (mt.tree(), t.tree()) {
                (Some(a), Some(b)) => a.ptr_eq(&b),
                (None, None) => true,
                _ => false,
            };
            if same {
                shared += 1;
                continue;
            }
        }
        let s = match t.tree() {
            Some(tree) => match &tree.get().root {
                Some(r) => String::from_utf8_lossy(&r.to_bytes()).into_owned(),
                None => "<nil>".to_string(),
            },
            None => "<nil>".to_string(),
        };
        list.push((norm_name(&name), s));
    }
    list.sort();
    json!({
        "label": label,
        "shared": shared,
        "templates": if list.is_empty() { J::Null } else { json!(list.into_iter().map(|(a, b)| json!([a, b])).collect::<Vec<_>>()) },
    })
}

fn dump_namespaces(store: &TemplateStore) -> J {
    let ids = Ids::new(store);
    let (html_text, text) = store.main_namespaces();
    let mut out = vec![
        dump_namespace("html", &html_text, None),
        dump_namespace("text", &text, None),
    ];
    let mut variants = Vec::new();
    for t in store.templates() {
        if t.base_template().is_none() {
            continue;
        }
        let label = ids.id(Some(&t));
        match t.template().expect("variant template") {
            Template::Html(h) => variants.push(dump_namespace(&label, &h.text(), Some(&html_text))),
            Template::Text(x) => variants.push(dump_namespace(&label, &x, Some(&text))),
        }
    }
    variants.sort_by(|a, b| a["label"].as_str().cmp(&b["label"].as_str()));
    out.extend(variants);
    J::Array(out)
}

fn check_site(name: &'static str) {
    check(
        name,
        site(name),
        load_fixture("escdump", &format!("{name}.json.gz")),
    );
}

fn check(name: &str, s: support::Site, go: J) {
    let store = s.store();
    let rust = dump_namespaces(&store);
    let go_ns = &go["namespaces"];
    let trees: usize = go_ns
        .as_array()
        .unwrap()
        .iter()
        .map(|n| n["templates"].as_array().map(|a| a.len()).unwrap_or(0))
        .sum();
    eprintln!(
        "{name}: {} namespaces, {trees} trees",
        go_ns.as_array().unwrap().len()
    );
    let mut diffs = Vec::new();
    diff_json("namespaces", go_ns, &rust, &mut diffs);
    assert!(diffs.is_empty(), "{name}:\n{}", diffs.join("\n"));
}

/// The layout trees of tplimpl's integration tests (tsupport.IntegrationSites).
#[test]
fn escdump_integration() {
    support::big_stack(|| {
        let stores = integration_fixtures("store");
        let mut dumps = integration_fixtures("escdump");
        assert_eq!(stores.len(), dumps.len());
        for (name, fx) in stores {
            let go = dumps.remove(&name).unwrap();
            check(&name, site_from_fixture(fx), go);
        }
    });
}

#[test]
fn escdump_docs() {
    support::big_stack(|| check_site("docs"));
}

#[test]
fn escdump_testsite() {
    support::big_stack(|| check_site("testsite"));
}

#[test]
fn escdump_legacy() {
    support::big_stack(|| check_site("legacy"));
}

#[test]
fn escdump_modern() {
    support::big_stack(|| check_site("modern"));
}

#[test]
fn escdump_themes() {
    support::big_stack(|| check_site("themes"));
}
