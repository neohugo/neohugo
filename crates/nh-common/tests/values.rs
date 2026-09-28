//! Differential tests against `tools/go-oracle/nh-common/values` (T01): common/maps (Params,
//! Scratch, KeyRenamer, conversions), common/collections (Append, Slice), common/hreflect,
//! common/types conversions and common/htime, over the decoded seeksnack config dumps, the front
//! matter of this repository's content and adversarial inputs.

mod support;

use std::sync::Arc;

use go_value::{GoString, Map, MapType, Object, Value};
use nh_common::collections::{append, slice};
use nh_common::htime::{self, TimeFormatter};
use nh_common::maps::maps as m;
use nh_common::maps::params as p;
use nh_common::maps::scratch::Scratch;
use nh_common::{hreflect, types};
use serde_json::{Value as J, json};
use support::*;

/// Mismatch collector: prints the first few and fails at the end.
struct Report {
    name: &'static str,
    n: usize,
    bad: Vec<String>,
    skipped: usize,
}

impl Report {
    fn new(name: &'static str) -> Self {
        Report {
            name,
            n: 0,
            bad: Vec::new(),
            skipped: 0,
        }
    }

    fn check(&mut self, what: impl FnOnce() -> String, want: &J, got: &J) {
        self.n += 1;
        if want.get("nondet").is_some() {
            self.skipped += 1;
            return;
        }
        if !same_result(want, got) {
            self.bad
                .push(format!("{}\n  want {want}\n  got  {got}", what()));
        }
    }

    fn finish(self) {
        eprintln!(
            "{}: {} checks, {} skipped (Go result depends on map order), {} mismatches",
            self.name,
            self.n,
            self.skipped,
            self.bad.len()
        );
        for b in self.bad.iter().take(12) {
            eprintln!("{b}");
        }
        assert!(
            self.bad.is_empty(),
            "{}: {} mismatches",
            self.name,
            self.bad.len()
        );
    }
}

fn ok(v: Value) -> J {
    json!({"ok": encode(&v)})
}

fn retyped(mut mm: Map, ty: MapType) -> Map {
    mm.ty = ty;
    mm
}

/// Resolves `{"same": name}` against the named reference results.
fn resolve<'a>(want: &'a J, refs: &'a [(&str, J)]) -> std::borrow::Cow<'a, J> {
    if let Some(name) = want.get("same").and_then(J::as_str) {
        for (n, r) in refs {
            if *n == name {
                return std::borrow::Cow::Owned(json!({"ok": r}));
            }
        }
        panic!("unknown ref {name}");
    }
    std::borrow::Cow::Borrowed(want)
}

fn split(s: &[u8], sep: &[u8]) -> Vec<Vec<u8>> {
    go_unicode::strings::split(s, sep)
        .into_iter()
        .map(|x| x.to_vec())
        .collect()
}

fn prepared_of(input: &Map) -> Map {
    let mut pm = retyped(input.clone(), MapType::Params);
    p::prepare_params(&mut pm);
    pm
}

fn key_renamer(patterns: &[&str]) -> m::KeyRenamer {
    m::KeyRenamer::new(patterns).unwrap()
}

#[test]
fn params() {
    init();
    let inputs_f = fixture("values/params_inputs.json.gz");
    assert_eq!(
        inputs_f["goarch"], "arm64",
        "the fixture must come from an arm64 oracle build"
    );
    let inputs: Vec<Map> = inputs_f["cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| decode_map(&c["v"]))
        .collect();
    let names: Vec<String> = inputs_f["cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| String::from_utf8_lossy(&bytes(&c["name"])).into_owned())
        .collect();
    assert!(inputs.len() > 900, "too few inputs: {}", inputs.len());
    assert_eq!(
        names.iter().filter(|n| n.starts_with("fm:")).count(),
        909,
        "front matter inputs"
    );
    let prepared: Vec<Map> = inputs.iter().map(prepared_of).collect();

    let f = fixture("values/params.json.gz");
    let rename1 = key_renamer(&["{menu,languages/*/menu}", "menus"]);
    let rename2 = key_renamer(&[
        "{ren1,sub/*/ren1}",
        "new1",
        "{Ren2,sub/ren2}",
        "new2",
        "**/b",
        "B",
        "a/?/c",
        "x",
        "l[a-j]st",
        "listed",
        "st[!a]",
        "sTR",
        "x/{y,yy}",
        "xy",
        "\\{}",
        "braces",
    ]);
    let mut r = Report::new("params");
    let mut ops = std::collections::BTreeMap::<&str, usize>::new();
    for c in f["cases"].as_array().unwrap() {
        let op = c["op"].as_str().unwrap();
        *ops.entry(op).or_default() += 1;
        match op {
            "single" => {
                let i = c["input"].as_u64().unwrap() as usize;
                let input = &inputs[i];
                let name = &names[i];
                let prep = &prepared[i];
                let in_enc = encode(&Value::map(input.clone()));
                let prep_enc = encode(&Value::map(prep.clone()));
                let mut prep_map = prep_enc.clone();
                prep_map["t"] = json!("map[string]interface {}");
                let refs = [
                    ("input", in_enc.clone()),
                    ("prepare", prep_enc.clone()),
                    ("prepareMap", prep_map),
                ];
                let chk = |r: &mut Report, field: &str, got: J| {
                    if let Some(w) = c.get(field) {
                        let w = resolve(w, &refs);
                        r.check(|| format!("{name} {field}"), &w, &got);
                    }
                };
                chk(&mut r, "prepare", ok(Value::map(prep.clone())));
                chk(
                    &mut r,
                    "prepareClone",
                    ok(Value::map(p::prepare_params_clone(input))),
                );
                chk(
                    &mut r,
                    "toParams",
                    result(p::to_params_and_prepare(&Value::map(input.clone())).map(Value::map)),
                );
                chk(
                    &mut r,
                    "clean",
                    ok(Value::map(p::clean_config_string_map(input))),
                );
                // Go's CleanConfigStringMap returns a map[string]interface {}.
                chk(
                    &mut r,
                    "cleanPrepared",
                    ok(Value::map(retyped(
                        p::clean_config_string_map(prep),
                        MapType::StringAny,
                    ))),
                );
                if let Some(z) = c.get("isZero") {
                    r.check(
                        || format!("{name} isZero"),
                        z,
                        &json!(p::params_is_zero(prep)),
                    );
                }
                let mut cf = input.clone();
                m::convert_float64_with_no_decimals_to_int(&mut cf);
                chk(&mut r, "convertFloat", ok(Value::map(cf)));
                chk(
                    &mut r,
                    "toStringMapString",
                    result(m::to_string_map_string_e(&Value::map(input.clone())).map(Value::map)),
                );
                let mut r1 = input.clone();
                rename1.rename(&mut r1);
                chk(&mut r, "rename", ok(Value::map(r1)));
                let mut r2 = input.clone();
                rename2.rename(&mut r2);
                chk(&mut r, "rename2", ok(Value::map(r2)));

                if let Some(lookups) = c.get("lookups").and_then(J::as_array) {
                    for l in lookups {
                        let key = bytes(&l[0]);
                        let sh = |v: Value| json!({"ok": shallow(&v)});
                        let param = sh(p::get_nested_param(&key, b".", &[prep]).unwrap());
                        let refs = [("param", param["ok"].clone())];
                        let what = || format!("{name} lookup {:?}", String::from_utf8_lossy(&key));
                        r.check(what, &l[1], &param);
                        r.check(
                            what,
                            &resolve(&l[2], &refs),
                            &sh(p::get_nested_param(&key, b"/", &[prep]).unwrap()),
                        );
                        let parts = split(&key, b".");
                        let idx: Vec<&[u8]> = parts.iter().map(|x| x.as_slice()).collect();
                        r.check(what, &resolve(&l[3], &refs), &sh(p::get_nested(prep, &idx)));
                        let (v, k, owner) =
                            p::get_nested_param_fn(&key, b".", |k| p::params_get(prep, k)).unwrap();
                        // Go's owner is a map[string]interface {} (the parameter type).
                        let owner = match owner {
                            Some(o) => Value::map(retyped((*o).clone(), MapType::StringAny)),
                            None => Value::TypedNil(Arc::from("map[string]interface {}")),
                        };
                        r.check(
                            what,
                            &l[4],
                            &json!({"ok": [shallow(&v), str_enc(&k), shallow(&owner)]}),
                        );
                    }
                }
            }
            "pair" => {
                let (a, b) = (
                    c["a"].as_u64().unwrap() as usize,
                    c["b"].as_u64().unwrap() as usize,
                );
                let what = |f: &str| format!("{} + {} {f}", names[a], names[b]);
                let pa = &prepared[a];
                let pb = &prepared[b];
                let mut merged = pa.clone();
                p::merge_params(&mut merged, pb);
                let mut set = pa.clone();
                p::set_params(&mut set, pb);
                let refs = [
                    ("a", encode(&Value::map(pa.clone()))),
                    ("merge", encode(&Value::map(merged.clone()))),
                    ("set", encode(&Value::map(set.clone()))),
                    ("inputA", encode(&Value::map(inputs[a].clone()))),
                ];
                r.check(
                    || what("merge"),
                    &resolve(&c["merge"], &refs),
                    &ok(Value::map(merged)),
                );
                r.check(
                    || what("set"),
                    &resolve(&c["set"], &refs),
                    &ok(Value::map(set)),
                );
                for (s, w) in ["", "none", "shallow", "deep", "bogus"]
                    .iter()
                    .zip(c["mergeWith"].as_array().unwrap())
                {
                    let mut dst = pa.clone();
                    p::merge_params_with_strategy(s, &mut dst, pb);
                    r.check(
                        || what(&format!("mergeWith {s:?}")),
                        &resolve(w, &refs),
                        &ok(Value::map(dst)),
                    );
                }
                let mut dst = inputs[a].clone();
                m::merge_shallow(&mut dst, &inputs[b]);
                r.check(
                    || what("mergeShallow"),
                    &resolve(&c["mergeShallow"], &refs),
                    &ok(Value::map(dst)),
                );
            }
            "fold" => {
                let i = c["input"].as_u64().unwrap() as usize;
                for l in c["lookups"].as_array().unwrap() {
                    let key = bytes(&l[0]);
                    let got = match m::lookup_equal_fold(&inputs[i], &key) {
                        Some((v, k)) => json!({"ok": [shallow(v), str_enc(k), true]}),
                        None => json!({"ok": [encode(&Value::Invalid), "", false]}),
                    };
                    r.check(
                        || format!("{} fold {:?}", names[i], String::from_utf8_lossy(&key)),
                        &l[1],
                        &got,
                    );
                }
            }
            "convert" => {
                let v = decode(&c["v"]);
                // The Rust results are `Map`s and `Vec`s: a Go nil map or slice is empty here.
                let want_ssm = nil_to_empty(&c["toSliceStringMap"]);
                let got = m::to_slice_string_map(&v).map(|ms| {
                    Value::list(
                        go_value::SliceType::MapStringAny,
                        ms.into_iter().map(Value::map).collect(),
                    )
                });
                r.check(
                    || format!("{v:?} toSliceStringMap"),
                    &want_ssm,
                    &result(got),
                );
                r.check(
                    || format!("{v:?} toStringMap"),
                    &nil_to_empty(&c["toStringMap"]),
                    &result(m::to_string_map_e(&v).map(Value::map)),
                );
                r.check(
                    || format!("{v:?} toParams"),
                    &c["toParams"],
                    &result(p::to_params_and_prepare(&v).map(Value::map)),
                );
            }
            other => panic!("unknown op {other}"),
        }
    }
    eprintln!("params cases by op: {ops:?}");
    r.finish();
}

/// Replaces Go's nil maps and slices (`{"t": "nil:map[...]"}`) by empty ones.
fn nil_to_empty(j: &J) -> J {
    match j {
        J::Object(o) => {
            if let Some(t) = o
                .get("t")
                .and_then(J::as_str)
                .and_then(|t| t.strip_prefix("nil:"))
            {
                if t.starts_with("map[") || t == "maps.Params" {
                    return json!({"t": t, "entries": []});
                }
                if t.starts_with("[]") {
                    return json!({"t": t, "items": []});
                }
            }
            J::Object(
                o.iter()
                    .map(|(k, v)| (k.clone(), nil_to_empty(v)))
                    .collect(),
            )
        }
        J::Array(a) => J::Array(a.iter().map(nil_to_empty).collect()),
        _ => j.clone(),
    }
}

fn scratch_call(s: &Scratch, op: &str, args: &[Value]) -> J {
    // Through the Object method table, as templates call it.
    match s.call_method(&(), op, args) {
        Some(Ok(v)) => json!({"ok": encode(&v)}),
        Some(Err(e)) => json!({"err": e.message()}),
        None => panic!("no method {op}"),
    }
}

#[test]
fn scratch() {
    init();
    let f = fixture("values/scratch.json");
    let mut r = Report::new("scratch");
    for c in f["cases"].as_array().unwrap() {
        let name = c["name"].as_str().unwrap();
        let s = Scratch::new();
        for o in c["ops"].as_array().unwrap() {
            let op = o["op"].as_str().unwrap();
            let args: Vec<Value> = o["args"]
                .as_array()
                .map(|a| a.iter().map(decode).collect())
                .unwrap_or_default();
            let got = scratch_call(&s, op, &args);
            r.check(|| format!("{name} {op}"), &o["ret"], &got);
            if let Some(state) = o.get("state") {
                r.check(
                    || format!("{name} {op} state"),
                    &json!({"ok": state}),
                    &ok(s.values()),
                );
            }
        }
    }
    r.finish();
}

#[test]
fn collections() {
    init();
    let f = fixture("values/collections.json");
    let mut r = Report::new("collections");
    for c in f["cases"].as_array().unwrap() {
        match c["op"].as_str().unwrap() {
            "append" => {
                let to = decode(&c["to"]);
                let from: Vec<Value> = c["from"]
                    .as_array()
                    .map(|a| a.iter().map(decode).collect())
                    .unwrap_or_default();
                let got = result(append::append(&(), &to, &from));
                r.check(|| format!("append {to:?} {from:?}"), &c["r"], &got);
            }
            "slice" => {
                let args: Vec<Value> = c["args"]
                    .as_array()
                    .map(|a| a.iter().map(decode).collect())
                    .unwrap_or_default();
                let got = ok(slice::slice(&(), &args));
                r.check(|| format!("slice {args:?}"), &c["r"], &got);
            }
            other => panic!("unknown op {other}"),
        }
    }
    r.finish();
}

#[test]
fn reflect() {
    init();
    let f = fixture("values/reflect.json");
    let mut r = Report::new("reflect");
    for c in f["cases"].as_array().unwrap() {
        let v = decode(&c["v"]);
        let what = |f: &str| format!("{v:?} {f}");
        r.check(
            || what("truthful"),
            &c["truthful"],
            &json!(hreflect::is_truthful(&v)),
        );
        r.check(
            || what("isSlice"),
            &c["isSlice"],
            &json!(hreflect::is_slice(&v)),
        );
        r.check(|| what("isMap"), &c["isMap"], &json!(hreflect::is_map(&v)));
        r.check(
            || what("isNil"),
            &c["isNil"],
            &json!(types::types::is_nil(&v)),
        );
        r.check(
            || what("isValid"),
            &c["isValid"],
            &json!(hreflect::is_valid(&v)),
        );
        r.check(
            || what("isNumber"),
            &c["isNumber"],
            &json!(hreflect::is_number(&v)),
        );
        // Go returns ([]any, bool); a nil []interface {} input is returned as it is (nil).
        let nil_any = || Value::TypedNil(Arc::from("[]interface {}"));
        let tsa = match hreflect::to_slice_any(&v) {
            Some(_) if matches!(&v, Value::TypedNil(t) if &**t == "[]interface {}") => {
                Value::any_list(vec![nil_any(), Value::Bool(true)])
            }
            Some(items) => Value::any_list(vec![Value::any_list(items), Value::Bool(true)]),
            None => Value::any_list(vec![nil_any(), Value::Bool(false)]),
        };
        r.check(|| what("toSliceAny"), &c["toSliceAny"], &ok(tsa));
        let sl = |x: Vec<GoString>| {
            Value::list(
                go_value::SliceType::String,
                x.into_iter().map(Value::String).collect(),
            )
        };
        // A nil []string result (nil input) cannot be told from an empty one.
        let mut want = c["toStringSlicePreserveString"].clone();
        if want["ok"]["t"] == "nil:[]string" {
            want = json!({"ok": {"t": "[]string", "items": []}});
        }
        let deref_err = |w: &J| -> J {
            if v.downcast::<Pg>().is_some() && w.get("err").is_some() {
                json!({"err": "<redacted>"})
            } else {
                w.clone()
            }
        };
        let want = deref_err(&want);
        r.check(
            || what("toStringSlicePreserveString"),
            &want,
            &result(types::convert::to_string_slice_preserve_string_e(&v).map(sl)),
        );
        r.check(
            || what("toString"),
            &deref_err(&c["toString"]),
            &result(types::convert::to_string_e(&v).map(Value::String)),
        );
        let tts = match types::convert::type_to_string(&v) {
            Some(s) => Value::any_list(vec![Value::String(s), Value::Bool(true)]),
            None => Value::any_list(vec![Value::string(""), Value::Bool(false)]),
        };
        r.check(|| what("typeToString"), &c["typeToString"], &ok(tts));
        let dur = types::convert::to_duration_e(&v).map(|d| {
            Value::object(NamedBasic {
                name: "time.Duration".into(),
                under: Value::int64(d.0),
            })
        });
        r.check(|| what("toDuration"), &c["toDuration"], &result(dur));
    }
    r.finish();
}

#[test]
fn htime() {
    init();
    let f = fixture("values/htime.json");
    let zones = [
        go_time::utc(),
        go_time::fixed_zone("ICT", 7 * 3600),
        go_time::fixed_zone("X", -5 * 3600),
    ];
    let mut r = Report::new("htime");
    for c in f["cases"].as_array().unwrap() {
        match c["op"].as_str().unwrap() {
            "toTime" => {
                let v = decode(&c["v"]);
                let z = &zones[c["zone"].as_u64().unwrap() as usize];
                let got = result(htime::to_time_in_default_location_e(&v, z).map(Value::Time));
                r.check(|| format!("toTime {v:?} {}", z.name), &c["r"], &got);
            }
            "format" => {
                let lang = c["lang"].as_str().unwrap();
                let tf =
                    TimeFormatter::new(nh_common::locales::get_translator(lang).unwrap().unwrap());
                let t = match decode(&c["t"]) {
                    Value::Time(t) => t,
                    other => panic!("{other:?}"),
                };
                for (l, want) in c["layouts"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .zip(c["out"].as_array().unwrap())
                {
                    let l = l.as_str().unwrap();
                    let got = str_enc(&tf.format_bytes(&t, l.as_bytes()));
                    r.check(
                        || format!("format {lang} {l:?}"),
                        &json!({"ok": want}),
                        &json!({"ok": got}),
                    );
                }
            }
            other => panic!("unknown op {other}"),
        }
    }
    r.finish();
}

/// spf13/cast's `indirect` dereferences a non-nil pointer to a struct before its type switch, so
/// the error prints the struct value (Go, via the T01 oracle: `main.pg{id:"p1"} of type main.pg`).
#[test]
fn cast_errors_dereference_pointer_objects() {
    use nh_common::cast::caste;
    let pg = Value::object(Pg("p1".into()));
    for (got, target) in [
        (caste::to_int_e(&pg).map(|_| ()), "int"),
        (caste::to_bool_e(&pg).map(|_| ()), "bool"),
        (caste::to_string_e(&pg).map(|_| ()), "string"),
        (caste::to_float64_e(&pg).map(|_| ()), "float64"),
    ] {
        assert_eq!(
            got.unwrap_err().to_string(),
            format!("unable to cast main.pg{{id:\"p1\"}} of type main.pg to {target}")
        );
    }
}
