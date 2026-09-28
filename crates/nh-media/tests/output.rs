//! Differential tests against `tools/go-oracle/nh-media/output` (fixture `output/output.json.gz`):
//! the built-in output formats, output.DefaultFormats, DecodeConfig (formats and their sort
//! order, SourceHash, the `hugo config` JSON dump) and the Formats/Format methods.

mod support;

use go_value::{Map, MapType, Value};
use nh_config::config_provider::Provider;
use nh_config::default_config_provider::DefaultConfigProvider;
use nh_media::media::config::{decode_types, default_types};
use nh_media::media::media_type::Types;
use nh_media::output::config::decode_config;
use nh_media::output::output_format::{Formats, OutputFormat, builtin_formats, default_formats};
use serde_json::{Map as JMap, Value as J, json};
use support::*;

fn format_methods(fs: &Formats) -> J {
    J::Array(
        fs.0.iter()
            .map(|f| {
                json!({
                    "BaseFilename": s(&f.base_filename()), "IsZero": f.is_zero(),
                    "MarshalJSON": bytes_result(f.marshal_json_bytes()),
                })
            })
            .collect(),
    )
}

fn found((f, ok): (OutputFormat, bool)) -> J {
    json!([dump_format(&f), ok])
}

fn opt(f: Option<OutputFormat>) -> (OutputFormat, bool) {
    let ok = f.is_some();
    (f.unwrap_or_default(), ok)
}

fn by_names(fs: &Formats, names: &[&str]) -> J {
    match fs.get_by_names_partial(names) {
        Ok(f) => json!([dump_formats(&f), J::Null]),
        Err((f, e)) => json!([dump_formats(&f), s(&e.to_string())]),
    }
}

/// Go's `goval.CallRaw` record of a decode.
fn call_raw(f: impl FnOnce() -> nh_common::Result<J>) -> J {
    match catch(f) {
        Ok(Ok(v)) => json!({ "ok": v }),
        Ok(Err(e)) => json!({ "err": e.to_string() }),
        Err(p) => json!({ "panic": p }),
    }
}

fn input_map(v: &Value) -> Map {
    match v {
        Value::Map(m) => (**m).clone(),
        _ => Map::new(MapType::StringAny),
    }
}

fn record(c: &J) -> J {
    let mut o = c.as_object().unwrap().clone();
    for k in ["op", "name", "in", "viaProvider"] {
        o.remove(k);
    }
    if c["op"] == "DecodeConfig" {
        o.remove("types");
    }
    if c["op"] == "FormatsQuery" || c["op"] == "FromFilename" {
        o.remove("formats");
    }
    J::Object(o)
}

#[test]
fn output_oracle() {
    let fx = fixture("output/output.json.gz");
    let custom_types = decode_types(&input_map(&decode(&fx["customTypesInput"])))
        .unwrap()
        .config;
    let types_by = |n: &str| -> Types {
        match n {
            "default" => default_types(),
            "custom" => custom_types.clone(),
            other => panic!("types {other}"),
        }
    };
    let custom_formats = decode_config(&custom_types, &decode(&fx["customFormatsInput"]))
        .unwrap()
        .config;
    let formats_by = |n: &str| -> Formats {
        match n {
            "default" => default_formats(),
            "custom" => custom_formats.clone(),
            other => panic!("formats {other}"),
        }
    };

    let cases = fx["cases"].as_array().unwrap();
    let mut failures = Vec::new();
    let mut compared = 0;
    for (i, c) in cases.iter().enumerate() {
        if c.get("nondet").is_some() {
            continue;
        }
        compared += 1;
        let op = c["op"].as_str().unwrap();
        let want = record(c);
        let got: J = match op {
            "Builtin" => {
                let b = builtin_formats();
                let all = Formats(vec![
                    b.amp.clone(),
                    b.calendar.clone(),
                    b.css.clone(),
                    b.csv.clone(),
                    b.html.clone(),
                    b.alias_html.clone(),
                    b.markdown.clone(),
                    b.json.clone(),
                    b.web_app_manifest.clone(),
                    b.robots_txt.clone(),
                    b.rss.clone(),
                    b.sitemap.clone(),
                    b.sitemap_index.clone(),
                    b.gotmpl.clone(),
                    b.http_status_404_html.clone(),
                ]);
                json!({ "formats": dump_formats(&all) })
            }
            "DefaultFormats" | "CustomFormats" => {
                let f = formats_by(if op == "DefaultFormats" {
                    "default"
                } else {
                    "custom"
                });
                json!({ "formats": dump_formats(&f), "methods": format_methods(&f) })
            }
            "FormatsQuery" => {
                let fs = formats_by(c["formats"].as_str().unwrap());
                let q = j_string(&c["q"]);
                json!({
                    "q": s(&q),
                    "GetBySuffix": found(fs.get_by_suffix_found(&q)),
                    "GetByName": found(opt(fs.get_by_name(&q))),
                    "GetByNames": by_names(&fs, &[&q, "html"]),
                    "GetByNames2": by_names(&fs, &["rss", &q]),
                })
            }
            "FromFilename" => {
                let fs = formats_by(c["formats"].as_str().unwrap());
                let q = j_string(&c["q"]);
                json!({ "q": s(&q), "out": found(fs.from_filename_found(&q)) })
            }
            "DecodeConfig" => {
                let input = decode(&c["in"]);
                let via = c["viaProvider"].as_bool().unwrap();
                let types = types_by(c["types"].as_str().unwrap());
                call_raw(|| {
                    let input = if via {
                        let cfg = DefaultConfigProvider::new();
                        if !input.is_invalid() {
                            cfg.set("outputFormats", input.clone());
                        }
                        cfg.get("outputformats")
                    } else {
                        input
                    };
                    let ns = decode_config(&types, &input)?;
                    let mut o = JMap::new();
                    o.insert("formats".into(), dump_formats(&ns.config));
                    o.insert("hash".into(), s(&ns.source_hash));
                    dump_json(&ns.source_structure, &mut o);
                    Ok(J::Object(o))
                })
            }
            other => panic!("op {other}"),
        };
        if got != want {
            failures.push(format!(
                "case {i} {op} {}:\n  in   {}\n  want {want}\n  got  {got}",
                c.get("name").map(|n| n.to_string()).unwrap_or_default(),
                c.get("in")
                    .map(|n| n.to_string())
                    .unwrap_or_default()
                    .chars()
                    .take(400)
                    .collect::<String>(),
            ));
        }
    }
    report("output", compared, &failures);
    assert!(compared > 1500, "{compared}");
}
