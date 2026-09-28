//! Template execution through the store against the Go oracle
//! (`tools/go-oracle/nh-tplimpl/probe`): the template-engine spec's probe lines (Appendix A:
//! html/template; Appendix B: text/template; plus a plain text format) executed with
//! `TemplateStore::execute_with_context`, the MINIMAL TEST FuncMap defined here (safeHTML,
//! safeHTMLAttr, safeCSS, safeJS, safeURL, printf over go-fmt, jsonify over go-json, a
//! Go-faithful eq/ne/not for the probe's kinds, dict, slice, try) and a stub page, byte for
//! byte. Every other Hugo function name is a stub (only needed to parse the embedded
//! templates); text/template's builtins are the engine's.

mod support;

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::sync::Arc;

use go_time::Month;
use go_value::{GoString, HostCtx, Kind, Map, MapType, Object, SafeKind, SliceType, Value};
use nh_common::object::{NamedTypeRegistry, args};
use nh_tpl::template::TplContext;
use nh_tplimpl::category::Category;
use nh_tplimpl::engine::{FuncMap, TplFunc};
use nh_tplimpl::templatedescriptor::TemplateDescriptor;
use nh_tplimpl::templatestore::TemplateQuery;
use support::{Ids, load_fixture, site_from_fixture, stub_funcs};

/// text/template's builtins (Hugo overrides them; the probe uses the engine's).
const GO_BUILTINS: &[&str] = &[
    "and", "call", "html", "index", "slice", "js", "len", "not", "or", "print", "printf",
    "println", "urlquery", "eq", "ge", "gt", "le", "lt", "ne",
];

fn func(f: impl Fn(&[Value]) -> go_value::Result<Value> + Send + Sync + 'static) -> TplFunc {
    Arc::new(move |_ctx: HostCtx<'_>, a: &[Value]| f(a))
}

fn safe(kind: SafeKind) -> TplFunc {
    func(move |a| {
        args::exactly(a, 1, "safe")?;
        let s = nh_common::cast::caste::to_string_e(&a[0])
            .map_err(|e| go_value::Error::new(e.message()))?;
        Ok(Value::Safe(kind, s))
    })
}

/// The value Hugo's `compare.Eq` compares after `normalize` (tpl/compare/compare.go).
#[derive(Debug, PartialEq)]
enum Norm {
    Nil,
    I(i64),
    F(f64),
    U(u64),
    S(Vec<u8>),
    Other(String),
}

fn normalize(v: &Value) -> Norm {
    let v = match v {
        Value::Object(o) => o.underlying().unwrap_or_else(|| v.clone()),
        _ => v.clone(),
    };
    match &v {
        Value::Invalid | Value::TypedNil(_) => Norm::Nil,
        Value::Int(i, _) => Norm::I(*i),
        Value::Float(f, _) => Norm::F(*f),
        Value::Uint(u, _) => {
            if *u <= i64::MAX as u64 {
                Norm::I(*u as i64)
            } else {
                Norm::U(*u)
            }
        }
        Value::String(s) | Value::Safe(_, s) => Norm::S(s.as_bytes().to_vec()),
        Value::Bool(b) => Norm::Other(format!("bool {b}")),
        other => Norm::Other(format!(
            "{} {}",
            other.go_type_name(),
            String::from_utf8_lossy(&go_fmt::sprint(std::slice::from_ref(other)))
        )),
    }
}

fn eq(first: &Value, others: &[Value]) -> bool {
    let nf = normalize(first);
    others.iter().any(|o| normalize(o) == nf)
}

/// The minimal test FuncMap (the Go oracle's `funcs`).
fn minimal_funcs() -> FuncMap {
    let mut m = FuncMap::new();
    m.insert("safeHTML".into(), safe(SafeKind::Html));
    m.insert("safeHTMLAttr".into(), safe(SafeKind::HtmlAttr));
    m.insert("safeCSS".into(), safe(SafeKind::Css));
    m.insert("safeJS".into(), safe(SafeKind::Js));
    m.insert("safeURL".into(), safe(SafeKind::Url));
    // Go: fmt.Sprintf(format string, a ...any) string.
    m.insert(
        "printf".into(),
        func(|a| {
            args::at_least(a, 1, "printf")?;
            let format = args::string(a, 0)?;
            Ok(Value::string(go_fmt::sprintf(format.as_bytes(), &a[1..])))
        }),
    );
    m.insert(
        "jsonify".into(),
        func(|a| {
            args::exactly(a, 1, "jsonify")?;
            let b = go_json::marshal(&a[0]).map_err(|e| go_value::Error::new(e.to_string()))?;
            Ok(Value::Safe(SafeKind::Html, GoString::from(b)))
        }),
    );
    m.insert(
        "eq".into(),
        func(|a| {
            args::at_least(a, 1, "eq")?;
            Ok(Value::Bool(eq(&a[0], &a[1..])))
        }),
    );
    m.insert(
        "ne".into(),
        func(|a| {
            args::exactly(a, 2, "ne")?;
            Ok(Value::Bool(!eq(&a[0], &a[1..])))
        }),
    );
    m.insert(
        "not".into(),
        func(|a| {
            args::exactly(a, 1, "not")?;
            Ok(Value::Bool(!nh_common::hreflect::is_truthful(&a[0])))
        }),
    );
    m.insert(
        "dict".into(),
        func(|a| {
            if a.len() % 2 != 0 {
                return Err(go_value::Error::new("invalid dictionary call"));
            }
            let mut map = Map::new(MapType::StringAny);
            for kv in a.chunks(2) {
                let Value::String(k) = &kv[0] else {
                    return Err(go_value::Error::new("dictionary keys must be strings"));
                };
                map.insert(k.clone(), kv[1].clone());
            }
            Ok(Value::map(map))
        }),
    );
    m.insert(
        "slice".into(),
        func(|a| Ok(Value::list(SliceType::Any, a.to_vec()))),
    );
    m.insert(
        "try".into(),
        func(|a| {
            args::exactly(a, 1, "try")?;
            Ok(a[0].clone())
        }),
    );
    m
}

fn utc_date(y: i64, mo: i64, d: i64, h: i64, mi: i64, s: i64, ns: i64) -> Value {
    Value::Time(go_time::date(
        y,
        Month(mo),
        d,
        h,
        mi,
        s,
        ns,
        &go_time::utc(),
    ))
}

fn params(entries: Vec<(&str, Value)>) -> Value {
    let mut m = Map::new(MapType::Params);
    for (k, v) in entries {
        m.insert(k, v);
    }
    Value::map(m)
}

fn strs(items: &[&str]) -> Value {
    Value::list(
        SliceType::Any,
        items.iter().map(|s| Value::string(*s)).collect(),
    )
}

/// The Go oracle's `probeParams`.
fn probe_params() -> Value {
    params(vec![
        ("yint", Value::int(5)),
        ("yfloat", Value::float64(4.5)),
        ("yfloat0", Value::float64(5.0)),
        ("ybig", Value::int(12345678901)),
        ("ystr", Value::string("007")),
        ("ybool", Value::Bool(true)),
        ("yfalse", Value::Bool(false)),
        ("ynull", Value::Invalid),
        ("yempty", Value::string("")),
        ("yzero", Value::int(0)),
        ("ylist", strs(&["a", "b", "c"])),
        (
            "ylistmixed",
            Value::list(
                SliceType::Any,
                vec![Value::int(1), Value::string("x"), Value::float64(2.5)],
            ),
        ),
        ("yemptylist", Value::list(SliceType::Any, vec![])),
        (
            "ymap",
            params(vec![
                ("b", Value::int(2)),
                ("a", Value::int(1)),
                (
                    "c",
                    Value::list(SliceType::Any, vec![Value::int(1), Value::int(2)]),
                ),
            ]),
        ),
        ("yemptymap", params(vec![])),
        ("ydate", utc_date(2021, 1, 2, 0, 0, 0, 0)),
        ("yneg", Value::int(-3)),
        ("yexp", Value::float64(1.5e10)),
        ("mixed_case", Value::string("mc")),
        ("yhtml", Value::string("<em>h</em>")),
    ])
}

/// The Go oracle's `siteParams`.
fn site_params() -> Value {
    params(vec![
        ("intv", Value::int(3)),
        ("floatv", Value::float64(1.0)),
        ("arr", strs(&["x", "y"])),
        ("nested", params(vec![("key", Value::string("v"))])),
    ])
}

/// The Go oracle's `probePage` (a struct value with value-receiver methods).
struct ProbePage;

/// The Go oracle's `probeSite`.
struct ProbeSite;

impl Object for ProbePage {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("main.probePage")
    }
    fn kind(&self) -> Kind {
        Kind::Struct
    }
    fn has_method(&self, name: &str) -> bool {
        matches!(
            name,
            "Title" | "Description" | "Kind" | "IsHome" | "Date" | "Params" | "Site"
        )
    }
    fn call_method(
        &self,
        _ctx: HostCtx<'_>,
        name: &str,
        a: &[Value],
    ) -> Option<go_value::Result<Value>> {
        let r = args::exactly(a, 0, name);
        if let Err(e) = r {
            return Some(Err(e));
        }
        Some(Ok(match name {
            "Title" => Value::string(r#"Home "Q" & 'A' <b>"#),
            "Description" => Value::string("desc + plus / slash"),
            "Kind" => Value::string("home"),
            "IsHome" => Value::Bool(true),
            "Date" => utc_date(2020, 9, 6, 15, 46, 26, 955000000),
            "Params" => probe_params(),
            "Site" => Value::object(ProbeSite),
            _ => return None,
        }))
    }
    fn struct_fields(&self) -> Option<Vec<(Cow<'_, str>, Value)>> {
        Some(Vec::new())
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Object for ProbeSite {
    fn type_name(&self) -> Cow<'_, str> {
        Cow::Borrowed("main.probeSite")
    }
    fn kind(&self) -> Kind {
        Kind::Struct
    }
    fn has_method(&self, name: &str) -> bool {
        matches!(name, "Title" | "Params")
    }
    fn call_method(
        &self,
        _ctx: HostCtx<'_>,
        name: &str,
        a: &[Value],
    ) -> Option<go_value::Result<Value>> {
        let r = args::exactly(a, 0, name);
        if let Err(e) = r {
            return Some(Err(e));
        }
        Some(Ok(match name {
            "Title" => Value::string("Site"),
            "Params" => site_params(),
            _ => return None,
        }))
    }
    fn struct_fields(&self) -> Option<Vec<(Cow<'_, str>, Value)>> {
        Some(Vec::new())
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[test]
fn probe_outputs() {
    support::big_stack(|| {
        let fx = load_fixture("probe", "probe.json.gz");
        let mut s = site_from_fixture(fx);
        let mut registry = NamedTypeRegistry::new();
        registry.register("maps.Params", nh_common::maps::params::PARAMS_METHODS);
        s.opts.named_types = Arc::new(registry);

        let names: Vec<String> = s
            .func_names
            .iter()
            .filter(|n| !GO_BUILTINS.contains(&n.as_str()))
            .cloned()
            .collect();
        let mut funcs = stub_funcs(&names);
        for (k, v) in minimal_funcs() {
            funcs.insert(k, v);
        }
        let store = s.store_with(funcs);
        let ids = Ids::new(&store);

        let outputs: BTreeMap<String, serde_json::Value> = s.fx["outputs"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let mut failures = Vec::new();
        for (name, media_type) in [
            ("html", "text/html"),
            ("json", "application/json"),
            ("plain", "text/plain"),
        ] {
            let want = &outputs[name];
            let ti = store
                .lookup_pages_layout(&TemplateQuery {
                    path: String::new(),
                    name: String::new(),
                    category: Category::Layout,
                    desc: TemplateDescriptor {
                        kind: "home".into(),
                        output_format: name.into(),
                        media_type: media_type.into(),
                        is_plain_text: name != "html",
                        ..Default::default()
                    },
                    consider: None,
                })
                .unwrap_or_else(|| panic!("no {name} template"));
            assert_eq!(ids.id(Some(&ti)), want["template"].as_str().unwrap());
            let mut buf = Vec::new();
            let res = store.execute_with_context(
                &TplContext::default(),
                &ti,
                &mut buf,
                &Value::object(ProbePage),
            );
            let got = String::from_utf8_lossy(&buf).into_owned();
            let want_out = want["output"].as_str().unwrap();
            let want_err = want["err"].as_str().unwrap_or("");
            let got_err = res
                .err()
                .map(|e| e.message().to_string())
                .unwrap_or_default();
            if got_err != want_err {
                failures.push(format!(
                    "{name}: error\n  go   {want_err}\n  rust {got_err}"
                ));
            }
            let (gl, wl): (Vec<&str>, Vec<&str>) =
                (got.lines().collect(), want_out.lines().collect());
            let mut lines = 0;
            for i in 0..gl.len().max(wl.len()) {
                let (g, w) = (gl.get(i).copied(), wl.get(i).copied());
                lines += 1;
                if g != w {
                    failures.push(format!("{name}: line {i}\n  go   {w:?}\n  rust {g:?}"));
                }
            }
            if got != want_out && failures.is_empty() {
                failures.push(format!("{name}: output differs (line endings)"));
            }
            eprintln!("{name}: {lines} lines, {} bytes", want_out.len());
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    });
}
