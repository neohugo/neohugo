//! Differential test of the data namespaces (cast, collections, compare, crypto, encoding, fmt,
//! hash, math, reflect, safe) against `tools/go-oracle/nh-tplfuncs/data` (linux/arm64 Go under
//! qemu; see PORTING.md): every case calls the namespace method through its `go_methods!` table
//! and compares the typed result (value and Go type) or the error text.

mod support;

use std::collections::BTreeMap;
use std::sync::Arc;

use go_value::{Object, Value};
use nh_deps::deps::Deps;
use nh_tplimpl::engine::FuncMap;
use serde_json::{Value as J, json};

use support::*;

fn deps(lang: &str) -> Arc<Deps> {
    Arc::new(Deps::for_tests(TestCfg::new(lang)))
}

/// The func map of the T18 namespaces (the aliases and the namespace funcs), for `apply`.
fn func_map(d: &Arc<Deps>) -> FuncMap {
    let mut m = FuncMap::new();
    for ns in [
        nh_tplfuncs::cast::init::namespace(d),
        nh_tplfuncs::collections::init::namespace(d),
        nh_tplfuncs::compare::init::namespace(d),
        nh_tplfuncs::crypto::init::namespace(d),
        nh_tplfuncs::encoding::init::namespace(d),
        nh_tplfuncs::fmt::init::namespace(d),
        nh_tplfuncs::hash::init::namespace(d),
        nh_tplfuncs::math::init::namespace(d),
        nh_tplfuncs::reflect::init::namespace(d),
        nh_tplfuncs::safe::init::namespace(d),
    ] {
        m.insert(ns.name.to_string(), ns.context.clone());
        for (alias, f) in ns.aliases {
            m.insert(alias, f);
        }
    }
    m
}

fn namespaces() -> BTreeMap<&'static str, Arc<dyn Object>> {
    use nh_tplfuncs::*;
    let en = deps("en");
    let th = deps("th");
    let site = nh_tplfuncs::collections::collections::Namespace::new(en.clone());
    let _ = site.funcs.set(Arc::new(func_map(&en)));
    let mut m: BTreeMap<&'static str, Arc<dyn Object>> = BTreeMap::new();
    m.insert("cast", Arc::new(cast::cast::Namespace::new(en.clone())));
    m.insert(
        "collections",
        Arc::new(collections::collections::Namespace::new(en.clone())),
    );
    m.insert(
        "collections@th",
        Arc::new(collections::collections::Namespace::new(th)),
    );
    m.insert("collections@site", Arc::new(site));
    m.insert(
        "compare",
        Arc::new(compare::compare::Namespace::new(en.clone())),
    );
    m.insert(
        "crypto",
        Arc::new(crypto::crypto::Namespace::new(en.clone())),
    );
    m.insert(
        "encoding",
        Arc::new(encoding::encoding::Namespace::new(en.clone())),
    );
    m.insert("fmt", Arc::new(fmt::fmt::Namespace::new(en.clone())));
    m.insert("hash", Arc::new(hash::hash::Namespace::new(en.clone())));
    m.insert("math", Arc::new(math::math::Namespace::new(en.clone())));
    m.insert(
        "reflect",
        Arc::new(reflect::reflect::Namespace::new(en.clone())),
    );
    m.insert("safe", Arc::new(safe::safe::Namespace::new(en)));
    m
}

fn result_json(r: Option<go_value::Result<Value>>) -> J {
    match r {
        None => json!({"err": "no method"}),
        Some(Ok(v)) => json!({"ok": encode_value(&v)}),
        Some(Err(e)) => json!({"err": str_enc(e.message().as_bytes())}),
    }
}

/// Whether `v` holds a value whose printed form can include an address (the oracle's
/// `hasPtr`): a test object, a page, a time outside UTC.
fn has_ptr(v: &Value) -> bool {
    match v {
        Value::Object(o) => {
            v.downcast::<TstObj>().is_some()
                || v.downcast::<nh_page::page::PageRef>().is_some()
                || o.type_name() == "*main.tstObj"
        }
        Value::Time(t) => go_time::location_string(t.loc.as_ref()) != "UTC",
        Value::List(l) => l.items.iter().any(has_ptr),
        Value::Map(m) => m.entries.values().any(has_ptr),
        _ => false,
    }
}

/// Go's `regexp.MustCompile("0x[0-9a-f]{6,}").ReplaceAllString(s, "0xADDR")`.
fn mask_str(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = String::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'0' && i + 1 < b.len() && b[i + 1] == b'x' {
            let mut j = i + 2;
            while j < b.len() && matches!(b[j], b'0'..=b'9' | b'a'..=b'f') {
                j += 1;
            }
            if j - (i + 2) >= 6 {
                out.push_str("0xADDR");
                i = j;
                continue;
            }
        }
        let ch = s[i..].chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

fn mask_addrs(v: &mut J) {
    match v {
        J::Object(m) => {
            for (k, e) in m.iter_mut() {
                if (k == "err" || k == "s")
                    && let J::String(s) = e
                {
                    *s = mask_str(s);
                    continue;
                }
                mask_addrs(e);
            }
        }
        J::Array(a) => a.iter_mut().for_each(mask_addrs),
        _ => {}
    }
}

/// The items of an encoded list, sorted by their JSON text (a shuffle's multiset).
fn sorted_items(v: &J) -> Option<(J, Vec<String>)> {
    let ok = v.get("ok")?;
    let mut items: Vec<String> = ok
        .get("items")?
        .as_array()?
        .iter()
        .map(|x| x.to_string())
        .collect();
    items.sort();
    Some((ok["t"].clone(), items))
}

#[test]
fn data_namespaces() {
    let header = fixture("data/values.json.gz");
    let pages = load_pages(header["pages"].as_array().unwrap());
    let values: Vec<Value> = header["values"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| decode_value(v, &pages))
        .collect();
    let ns = namespaces();

    let mut total = 0usize;
    let mut failed = 0usize;
    let mut nondet = 0usize;
    let mut unordered = 0usize;
    let mut report: Vec<String> = Vec::new();
    let mut per_file: BTreeMap<String, usize> = BTreeMap::new();
    for file in fixture_files("data") {
        if file == "values.json.gz" || file == "counter.json.gz" {
            continue;
        }
        let fx = fixture(&format!("data/{file}"));
        for c in fx["cases"].as_array().unwrap() {
            total += 1;
            let ns_name = c["ns"].as_str().unwrap();
            let m = c["m"].as_str().unwrap();
            let args: Vec<Value> = c["a"]
                .as_array()
                .unwrap()
                .iter()
                .map(|i| values[i.as_u64().unwrap() as usize].clone())
                .collect();
            let obj = ns
                .get(ns_name)
                .unwrap_or_else(|| panic!("no namespace {ns_name}"));
            let mut got = result_json(obj.call_method(&(), m, &args));
            if args.iter().any(has_ptr) {
                mask_addrs(&mut got);
            }
            let want = &c["r"];
            let ok = if c.get("unordered").is_some() {
                unordered += 1;
                match sorted_items(want) {
                    Some(w) => Some(w) == sorted_items(&got),
                    None => &got == want,
                }
            } else if want.get("rand").is_some() {
                got["ok"]["t"] == "float64" && {
                    let f = f64::from_bits(
                        u64::from_str_radix(got["ok"]["v"].as_str().unwrap(), 16).unwrap(),
                    );
                    (0.0..1.0).contains(&f)
                }
            } else if let Some(all) = c.get("nondet") {
                nondet += 1;
                all.as_array().unwrap().contains(&got)
            } else {
                &got == want
            };
            if ok {
                continue;
            }
            failed += 1;
            *per_file.entry(file.clone()).or_insert(0) += 1;
            if per_file[&file] <= 3 && report.len() < 200 {
                let a: Vec<J> = args.iter().map(encode_value).collect();
                report.push(format!(
                    "{file}: {ns_name}.{m}({})\n    want {want}\n    got  {got}",
                    serde_json::to_string(&a).unwrap()
                ));
            }
        }
    }
    eprintln!(
        "data: {total} cases, {failed} failed, {nondet} nondeterministic and {unordered} unordered in Go"
    );
    eprintln!("{per_file:?}");
    for r in &report {
        eprintln!("{r}");
    }
    assert_eq!(failed, 0, "{failed} of {total} cases differ from Go");
}

#[test]
fn counter() {
    let fx = fixture("data/counter.json.gz");
    let want = fx["cases"][0]["r"]["seq"].as_array().unwrap().clone();
    let ns = nh_tplfuncs::math::math::Namespace::new(deps("en"));
    let got: Vec<J> = (0..want.len())
        .map(|_| encode_value(&ns.counter(&(), &[]).unwrap()))
        .collect();
    assert_eq!(got, want);
}
