//! Differential tests of the CSV and XML decoders and the YAML, TOML and XML encoders
//! (`metadecoders::{csv, xml, mxj, toml::marshaler}`, go-yaml's `encode`) against
//! `tools/go-oracle/nh-parser/formats` (fixtures `formats/{csv,xml,encode}.json.gz`).

mod support;

use go_value::{IntKind, Map, MapType, Value};
use nh_parser::frontmatter::interface_to_config;
use nh_parser::metadecoders::decoder::Decoder;
use nh_parser::metadecoders::format::Format;
use serde_json::{Value as J, json};
use support::{Diffs, enc, j_bytes};

fn format_of(s: &str) -> Format {
    match s {
        "json" => Format::Json,
        "toml" => Format::Toml,
        "yaml" => Format::Yaml,
        "csv" => Format::Csv,
        "xml" => Format::Xml,
        other => panic!("format {other}"),
    }
}

fn lossy(j: &J) -> String {
    String::from_utf8_lossy(&j_bytes(j)).into_owned()
}

/// Compares a metadecoders result with Go's `{"ok"}` / `{"err","cause"}`.
fn check(d: &mut Diffs, got: Result<J, nh_common::Error>, want: &J, ctx: &dyn Fn() -> String) {
    match got {
        Ok(v) => {
            let w = want.get("ok").cloned().unwrap_or(J::Null);
            d.check(v == w, || {
                format!("{}:\n  got:  {v}\n  want: {want}", ctx())
            });
        }
        Err(e) => {
            let w = want.get("cause").map(lossy);
            d.eq(Some(e.message().to_string()), w, || {
                format!("{}: error", ctx())
            });
        }
    }
}

fn map_j(m: Option<Map>) -> J {
    match m {
        Some(m) => enc(&Value::map(m)),
        None => json!({"t": "nil:map[string]interface {}"}),
    }
}

fn decoder_of(c: &J) -> Decoder {
    let ch = |k: &str| char::from_u32(c[k].as_u64().unwrap() as u32).unwrap();
    let comment = ch("comment");
    Decoder {
        delimiter: ch("delim"),
        comment: if comment == '\0' { None } else { Some(comment) },
        lazy_quotes: c["lazy"].as_bool().unwrap(),
        target_type: c["target"].as_str().unwrap().to_string(),
    }
}

fn run_decode(topic: &str) {
    let fx = support::fixture_or_env(
        &format!("formats/{topic}.json.gz"),
        &format!("NH_PARSER_{}", topic.to_uppercase()),
    );
    let cases = fx["cases"].as_array().unwrap();
    assert_eq!(
        cases.len() as u64,
        fx["cases_count"].as_u64().unwrap_or(cases.len() as u64)
    );
    let mut d = Diffs::default();
    let (mut ok, mut err) = (0, 0);
    for (i, c) in cases.iter().enumerate() {
        let src = j_bytes(&c["src"]);
        let format = format_of(c["format"].as_str().unwrap());
        let dec = decoder_of(c);
        let ctx = || {
            format!(
                "case {i} {:?} {:?} {:?}",
                format,
                dec,
                String::from_utf8_lossy(&src)
            )
        };
        let got = dec.unmarshal(&src, format).map(|v| enc(&v));
        if got.is_ok() {
            ok += 1;
        } else {
            err += 1;
        }
        check(&mut d, got, &c["any"], &|| format!("{} Unmarshal", ctx()));
        let got = dec.unmarshal_to_map_nilable(Some(&src), format).map(map_j);
        check(&mut d, got, &c["toMap"], &|| {
            format!("{} UnmarshalToMap", ctx())
        });
    }
    eprintln!(
        "{topic}: {} cases ({ok} decoded, {err} errors)",
        cases.len()
    );
    d.finish(topic);
}

#[test]
fn csv_matches_go() {
    run_decode("csv");
}

#[test]
fn xml_matches_go() {
    run_decode("xml");
}

/// Go: `tpl/transform/remarshal.go:applyMarshalTypes` (Go's `int64(t)` as on arm64: Rust's
/// saturating `as`).
fn apply_marshal_types(m: &mut Map) {
    for v in m.entries.values_mut() {
        match v {
            Value::Map(mm) if mm.ty == MapType::StringAny => {
                let mut c = Map::clone(mm);
                apply_marshal_types(&mut c);
                *v = Value::map(c);
            }
            Value::Float(t, go_value::FloatKind::F64) => {
                let i = *t as i64;
                if *t == i as f64 {
                    *v = Value::Int(i, IntKind::Int64);
                }
            }
            _ => {}
        }
    }
}

fn encode_result(v: &Value, f: Format) -> J {
    let mut w = Vec::new();
    match interface_to_config(v, f, &mut w) {
        Ok(()) => json!({ "ok": support::str_j(&w) }),
        Err(e) => json!({ "err": e.message() }),
    }
}

/// Go's result, or one of its `alts` (nondeterministic Go results); `alts` counts those.
fn matches(got: &J, want: &J, alts: &mut usize) -> bool {
    let norm = |j: &J| -> J {
        match j.get("err") {
            Some(e) => json!({ "err": lossy(e) }),
            None => j.clone(),
        }
    };
    let got = norm(got);
    match want.get("alts") {
        Some(a) => {
            *alts += 1;
            a.as_array().unwrap().iter().any(|w| norm(w) == got)
        }
        None => norm(want) == got,
    }
}

#[test]
fn encoders_match_go() {
    let fx = support::fixture_or_env("formats/encode.json.gz", "NH_PARSER_ENCODE");
    let cases = fx["cases"].as_array().unwrap();
    let mut d = Diffs::default();
    let mut alts = 0usize;
    let mut outputs = 0usize;
    for (i, c) in cases.iter().enumerate() {
        let src = j_bytes(&c["src"]);
        let from = format_of(c["format"].as_str().unwrap());
        let mut m = Decoder::default()
            .unmarshal_to_map(&src, from)
            .unwrap_or_else(|e| panic!("case {i}: decode: {e}"));
        if c["apply"].as_bool().unwrap() {
            apply_marshal_types(&mut m);
        }
        let v = Value::map(m);
        for to in ["yaml", "toml", "xml", "json"] {
            let want = &c[to];
            let got = encode_result(&v, format_of(to));
            if got.get("ok").is_some() {
                outputs += 1;
            }
            d.check(matches(&got, want, &mut alts), || {
                format!(
                    "case {i} {from:?} apply={} -> {to}: {:?}\n  got:  {got}\n  want: {want}",
                    c["apply"],
                    String::from_utf8_lossy(&src)
                )
            });
        }
    }
    eprintln!(
        "encode: {} cases, {outputs} outputs, {alts} results with nondeterministic Go alternatives",
        cases.len()
    );
    d.finish("encode");
}
