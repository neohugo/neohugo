//! Differential tests of `metadecoders` (TOML port, YAML, JSON, format helpers) against
//! `tools/go-oracle/nh-parser/metadecoders` (fixtures `metadecoders/{decode,misc}.json.gz`).

mod support;

use go_value::{FloatKind, IntKind, Map, MapType, SliceType, Value};
use nh_parser::metadecoders::decoder::Decoder;
use nh_parser::metadecoders::format::{Format, format_from_string, format_from_strings};
use nh_parser::metadecoders::toml;
use serde_json::{Value as J, json};
use support::{Diffs, enc, j_bytes};

fn format_of(s: &str) -> Option<Format> {
    Some(match s {
        "" => Format::Unknown,
        "json" => Format::Json,
        "toml" => Format::Toml,
        "yaml" => Format::Yaml,
        "csv" => Format::Csv,
        "xml" => Format::Xml,
        "org" => Format::Org,
        // Go's Format is a string; the enum cannot hold an unknown name.
        _ => return None,
    })
}

fn lossy(j: &J) -> String {
    String::from_utf8_lossy(&j_bytes(j)).into_owned()
}

/// Compares a metadecoders result with Go's `{"ok"}` / `{"err","cause"}`.
fn check_result(
    d: &mut Diffs,
    unsupported: &mut usize,
    got: Result<J, nh_common::Error>,
    want: &J,
    ctx: &dyn Fn() -> String,
) {
    match got {
        Ok(v) => {
            let w = want.get("ok").cloned().unwrap_or(J::Null);
            d.check(v == w, || {
                format!("{}:\n  got:  {v}\n  want: {want}", ctx())
            });
        }
        Err(e) if e.message().starts_with("neohugo-rs:") => {
            // CSV, XML and ORG decoding are explicit stubs.
            *unsupported += 1;
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

#[test]
fn decoders_match_go() {
    let fx = support::fixture_or_env("metadecoders/decode.json.gz", "NH_PARSER_DECODE");
    let cases = fx["cases"].as_array().unwrap();
    assert_eq!(cases.len() as u64, fx["docs"].as_u64().unwrap());
    let mut d = Diffs::default();
    let mut unsupported = 0usize;
    let mut toml_ok = 0usize;
    let mut toml_err = 0usize;
    let dec = Decoder::default();

    for (i, c) in cases.iter().enumerate() {
        let name = c["name"].as_str().unwrap();
        let src = j_bytes(&c["src"]);
        let Some(format) = format_of(c["format"].as_str().unwrap()) else {
            continue;
        };
        let ctx = || {
            format!(
                "case {i} {name} {:?} {:?}",
                format,
                String::from_utf8_lossy(&src)
            )
        };

        let got = dec.unmarshal_to_map_nilable(Some(&src), format).map(map_j);
        check_result(&mut d, &mut unsupported, got, &c["toMap"], &|| {
            format!("{} UnmarshalToMap", ctx())
        });

        let got = dec.unmarshal(&src, format).map(|v| enc(&v));
        check_result(&mut d, &mut unsupported, got, &c["any"], &|| {
            format!("{} Unmarshal", ctx())
        });

        if format == Format::Toml {
            let want = &c["toml"];
            match toml::unmarshal(&src) {
                Ok(v) => {
                    toml_ok += 1;
                    let got = enc(&v);
                    let w = want.get("ok").cloned().unwrap_or(J::Null);
                    d.check(got == w, || {
                        format!("{} toml.Unmarshal:\n  got:  {got}\n  want: {want}", ctx())
                    });
                }
                Err(e) => {
                    toml_err += 1;
                    d.eq(Some(e.error_bytes()), want.get("err").map(j_bytes), || {
                        format!("{} toml error", ctx())
                    });
                    let pos = e.position().map(|(l, c)| json!([l, c]));
                    d.eq(pos, want.get("pos").cloned(), || {
                        format!("{} toml error position", ctx())
                    });
                    if let (Some(h), toml::Error::Decode(de)) = (want.get("human"), &e) {
                        d.eq(de.human().to_vec(), j_bytes(h), || {
                            format!("{} toml human error", ctx())
                        });
                    }
                }
            }
        }
    }
    eprintln!(
        "{} docs; TOML: {toml_ok} decoded, {toml_err} errors; {unsupported} unsupported (CSV/XML/ORG)",
        cases.len()
    );
    d.finish("metadecoders");
}

fn typ_value(name: &str) -> Value {
    match name {
        "string" => Value::string("s"),
        "map" => Value::map(Map::new(MapType::StringAny)),
        "params" => Value::map(Map::new(MapType::Params)),
        "slice" => Value::any_list(vec![]),
        "bool" => Value::Bool(true),
        "int" => Value::int(1),
        "int64" => Value::int64(1),
        "float64" => Value::Float(1.5, FloatKind::F64),
        "int32" => Value::Int(1, IntKind::Int32),
        "[]string" => Value::list(SliceType::String, vec![]),
        other => panic!("typ {other}"),
    }
}

#[test]
fn misc_match_go() {
    let fx = support::fixture("metadecoders/misc.json.gz");
    let mut d = Diffs::default();
    let mut unsupported = 0usize;
    for c in fx["cases"].as_array().unwrap() {
        let op = c["op"].as_str().unwrap();
        match op {
            "FormatFromString" => {
                let s = c["in"].as_str().unwrap();
                d.eq(
                    format_from_string(s).as_str(),
                    c["out"].as_str().unwrap(),
                    || format!("{op} {s:?}"),
                );
            }
            "FormatFromStrings" => {
                let ss: Vec<&str> = c["in"]
                    .as_array()
                    .map(|a| a.iter().map(|x| x.as_str().unwrap()).collect())
                    .unwrap_or_default();
                d.eq(
                    format_from_strings(&ss).as_str(),
                    c["out"].as_str().unwrap(),
                    || format!("{op} {ss:?}"),
                );
            }
            "FormatFromContentString" => {
                let s = j_bytes(&c["in"]);
                let delim = c["delim"].as_str().unwrap().chars().next().unwrap();
                let dec = Decoder {
                    delimiter: delim,
                    ..Decoder::default()
                };
                d.eq(
                    dec.format_from_content_string(&s).as_str(),
                    c["out"].as_str().unwrap(),
                    || format!("{op} {:?} {delim:?}", String::from_utf8_lossy(&s)),
                );
            }
            "OptionsKey" => {
                let ch = |k: &str| char::from_u32(c[k].as_u64().unwrap() as u32).unwrap();
                let comment = ch("comment");
                let dec = Decoder {
                    delimiter: ch("delim"),
                    comment: if comment == '\0' { None } else { Some(comment) },
                    lazy_quotes: c["lazy"].as_bool().unwrap(),
                    target_type: c["target"].as_str().unwrap().to_string(),
                };
                d.eq(dec.options_key().into_bytes(), j_bytes(&c["out"]), || {
                    format!("{op} {c}")
                });
            }
            "UnmarshalEmpty" => {
                let f = format_of(c["format"].as_str().unwrap()).unwrap();
                let dec = Decoder {
                    target_type: c["target"].as_str().unwrap().to_string(),
                    ..Decoder::default()
                };
                let got = dec.unmarshal(&[], f).map(|v| match &v {
                    Value::List(l) if l.ty == SliceType::Named("[][]string".into()) => {
                        json!({"t": "[][]string", "len": l.items.len()})
                    }
                    _ => enc(&v),
                });
                check_result(&mut d, &mut unsupported, got, &c["res"], &|| {
                    format!("{op} {c}")
                });
            }
            "UnmarshalToMapNil" => {
                let f = format_of(c["format"].as_str().unwrap()).unwrap();
                let got = Decoder::default()
                    .unmarshal_to_map_nilable(None, f)
                    .map(map_j);
                check_result(&mut d, &mut unsupported, got, &c["res"], &|| {
                    format!("{op} {c}")
                });
            }
            "UnmarshalStringTo" => {
                let typ = typ_value(c["typ"].as_str().unwrap());
                let s = c["in"].as_str().unwrap();
                let got = Decoder::default()
                    .unmarshal_string_to(s, &typ)
                    .map(|v| enc(&v));
                check_result(&mut d, &mut unsupported, got, &c["res"], &|| {
                    format!("{op} {c}")
                });
            }
            other => panic!("unknown op {other}"),
        }
    }
    d.finish("metadecoders misc");
}
