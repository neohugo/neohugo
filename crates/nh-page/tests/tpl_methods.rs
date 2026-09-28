//! The template method tables of the T12 types (`page.Pages`, `page.PagesGroup`,
//! `page.PageGroup`, `*page.Pager`, `page.WeightedPages`, `page.WeightedPage`, `page.Taxonomy`,
//! `page.OrderedTaxonomy`, `navigation.Menu`, `*navigation.MenuEntry`): dispatch, argument checks
//! and results against the direct functions, over the ties build of the collections fixture.

mod csupport;
mod support;

use std::sync::Arc;

use csupport::load;
use go_value::{Object, Value};
use nh_page::page::{Pages, pages_from_value, pages_to_value};
use nh_page::pagegroup::{PAGES_GROUP_METHODS, pages_group_from_value};
use nh_page::pages::PAGES_METHODS;
use nh_page::pagination::{PagerRef, paginate_with};
use nh_page::taxonomy::{
    TAXONOMY_METHODS, Taxonomy, ordered_taxonomy_from_value, taxonomy_to_value,
};
use nh_page::weighted::{WEIGHTED_PAGES_METHODS, WeightedPage, weighted_pages_to_value};
use support::fixture;

fn call(v: &Value, name: &str, args: &[Value]) -> go_value::Result<Value> {
    (PAGES_METHODS.call)(&(), v, name, args).expect("method")
}

fn ids(v: &Value) -> Vec<u64> {
    pages_from_value(v)
        .unwrap()
        .iter()
        .map(|p| p.0.page_id())
        .collect()
}

fn page_ids(p: &Pages) -> Vec<u64> {
    p.iter().map(|p| p.0.page_id()).collect()
}

#[test]
fn pages_methods() {
    let fx = fixture("collections/ties.json.gz");
    let b = load(&fx);
    let p = b.list(&fx["lists"][0]["pages"]);
    let v = pages_to_value(&p);
    assert!(!(PAGES_METHODS.has_method)("Nope"));
    for name in [
        "ByTitle",
        "ByLinkTitle",
        "ByDate",
        "ByWeight",
        "Reverse",
        "ByLanguage",
    ] {
        assert!((PAGES_METHODS.has_method)(name));
        let got = call(&v, name, &[]).unwrap();
        assert_eq!(got.go_type_name(), "page.Pages");
        let want = match name {
            "ByTitle" => nh_page::pages_sort::by_title(&p),
            "ByLinkTitle" => nh_page::pages_sort::by_link_title(&p),
            "ByDate" => nh_page::pages_sort::by_date(&p),
            "ByWeight" => nh_page::pages_sort::by_weight(&p),
            "Reverse" => nh_page::pages_sort::reverse(&p),
            _ => nh_page::pages_sort::by_language(&p),
        };
        assert_eq!(ids(&got), page_ids(&want), "{name}");
        let err = call(&v, name, &[Value::int(1)]).unwrap_err();
        assert_eq!(
            err.message(),
            format!("wrong number of args for {name}: want 0 got 1")
        );
    }
    assert_eq!(call(&v, "Len", &[]).unwrap().go_type_name(), "int");
    let s = call(&v, "String", &[]).unwrap();
    assert_eq!(
        s.as_go_string().unwrap().as_bytes(),
        format!("Pages({})", p.len()).as_bytes()
    );
    let lim = call(&v, "Limit", &[Value::int(3)]).unwrap();
    assert_eq!(ids(&lim), page_ids(&p[..3].to_vec()));
    assert!(call(&v, "Limit", &[Value::string("3")]).is_err());
    let byp = call(&v, "ByParam", &[Value::string("rating")]).unwrap();
    assert_eq!(
        ids(&byp),
        page_ids(&nh_page::pages_sort::by_param(&p, &Value::string("rating")))
    );

    // Groups.
    let g = call(
        &v,
        "GroupBy",
        &[Value::string("Section"), Value::string("desc")],
    )
    .unwrap();
    assert_eq!(g.go_type_name(), "page.PagesGroup");
    let groups = pages_group_from_value(&g).unwrap();
    assert!(!groups.is_empty());
    let first = &groups[0];
    // A PageGroup has Key and Pages fields and the promoted Pages methods.
    let gv = first.to_value();
    let o = gv.as_object().unwrap();
    assert_eq!(o.type_name(), "page.PageGroup");
    assert!(o.field("Key").is_some());
    assert_eq!(ids(&o.field("Pages").unwrap()), page_ids(&first.pages));
    let r = o.call_method(&(), "Reverse", &[]).unwrap().unwrap();
    assert_eq!(
        ids(&r),
        page_ids(&nh_page::pages_sort::reverse(&first.pages))
    );
    assert_eq!(
        page_ids(&pages_from_value(&gv).unwrap()),
        page_ids(&first.pages)
    );
    let len = (PAGES_GROUP_METHODS.call)(&(), &g, "Len", &[])
        .unwrap()
        .unwrap();
    assert!(matches!(len, Value::Int(n, _) if n as usize == p.len()));
    let err = call(&v, "GroupBy", &[Value::string("Params")]).unwrap_err();
    assert_eq!(err.message(), "reflect.MapOf: invalid key type maps.Params");
    let empty = call(
        &pages_to_value(&Vec::new()),
        "GroupBy",
        &[Value::string("Section")],
    )
    .unwrap();
    assert!(matches!(empty, Value::TypedNil(t) if &*t == "page.PagesGroup"));

    // Next/Prev take a page.
    let n = call(&v, "Next", &[p[1].to_value()]).unwrap();
    assert_eq!(
        nh_page::page::page_from_value(&n).unwrap().0.page_id(),
        p[0].0.page_id()
    );
    let n = call(&v, "Next", &[p[0].to_value()]).unwrap();
    assert!(matches!(n, Value::TypedNil(t) if &*t == "page.Page"));
    assert!(call(&v, "Next", &[Value::string("x")]).is_err());
}

#[test]
fn pager_methods() {
    let fx = fixture("collections/ties.json.gz");
    let b = load(&fx);
    let p = b.list(&fx["lists"][0]["pages"]);
    let pag = paginate_with(Arc::new(|n| format!("/p/{n}/")), &pages_to_value(&p), 3).unwrap();
    let pager = PagerRef(pag.pagers()[0].clone());
    assert_eq!(pager.type_name(), "*page.Pager");
    let c = |name: &str| pager.call_method(&(), name, &[]).unwrap().unwrap();
    assert!(matches!(c("PageNumber"), Value::Int(1, _)));
    assert_eq!(c("URL").as_go_string().unwrap().as_bytes(), b"/p/1/");
    assert_eq!(ids(&c("Pages")), page_ids(&p[..3].to_vec()));
    assert!(matches!(c("PageGroups"), Value::TypedNil(t) if &*t == "page.PagesGroup"));
    assert!(matches!(c("Prev"), Value::TypedNil(t) if &*t == "*page.Pager"));
    let next = c("Next");
    assert_eq!(
        next.as_object().unwrap().identity(),
        Arc::as_ptr(&pag.pagers()[1]) as usize
    );
    // First returns the same pager object (pointer identity for `eq`).
    assert_eq!(c("First").as_object().unwrap().identity(), pager.identity());
    assert!(matches!(c("TotalPages"), Value::Int(n, _) if n == p.len().div_ceil(3) as i64));
    assert_eq!(c("Pagers").go_type_name(), "page.pagers");
    assert_eq!(pager.go_string().unwrap().as_bytes(), b"Pager 1");
}

#[test]
fn taxonomy_and_weighted_methods() {
    let fx = fixture("collections/ties.json.gz");
    let b = load(&fx);
    let tc = &fx["taxonomies"][0];
    let mut tax: Taxonomy = Taxonomy::new();
    for te in tc["terms"].as_array().unwrap() {
        let wp = te["entries"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| {
                WeightedPage::new(
                    e[1].as_i64().unwrap(),
                    b.pages[e[0].as_u64().unwrap() as usize].clone(),
                    None,
                )
            })
            .collect();
        tax.insert(support::gostring(&te["term"]), wp);
    }
    let tv = taxonomy_to_value(&tax);
    let c = |name: &str, args: &[Value]| {
        (TAXONOMY_METHODS.call)(&(), &tv, name, args)
            .unwrap()
            .unwrap()
    };
    let bc = c("ByCount", &[]);
    assert_eq!(bc.go_type_name(), "page.OrderedTaxonomy");
    let ot = ordered_taxonomy_from_value(&bc).unwrap();
    assert_eq!(ot.len(), tax.len());
    let e = ot[0].clone();
    let ev = Value::object(e.clone());
    let o = ev.as_object().unwrap();
    assert_eq!(
        o.call_method(&(), "Term", &[])
            .unwrap()
            .unwrap()
            .as_go_string()
            .unwrap()
            .as_bytes(),
        e.name.as_bytes()
    );
    // Promoted WeightedPages methods.
    assert!(o.has_method("Page"));
    let key = tax.keys().next().unwrap().clone();
    let got = c("Get", &[Value::string(key.to_uppercase())]);
    assert_eq!(got.go_type_name(), "page.WeightedPages");
    let missing = c("Get", &[Value::string("no such term")]);
    assert!(matches!(missing, Value::TypedNil(t) if &*t == "page.WeightedPages"));
    let wp = &tax[&key];
    let wv = weighted_pages_to_value(wp);
    let pages = (WEIGHTED_PAGES_METHODS.call)(&(), &wv, "Pages", &[])
        .unwrap()
        .unwrap();
    assert_eq!(
        ids(&pages),
        wp.iter().map(|w| w.page.0.page_id()).collect::<Vec<_>>()
    );
    let err = (WEIGHTED_PAGES_METHODS.call)(&(), &wv, "Sort", &[])
        .unwrap()
        .unwrap_err();
    assert_eq!(
        err.message(),
        "can't call method/function \"Sort\" with 0 results"
    );
    let w0 = Value::object(wp[0].clone());
    let wo = w0.as_object().unwrap();
    assert_eq!(wo.type_name(), "page.WeightedPage");
    assert!(matches!(wo.field("Weight"), Some(Value::Int(..))));
}
