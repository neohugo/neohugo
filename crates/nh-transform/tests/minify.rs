//! Differential test of `minifiers.DecodeConfig` and the minifier client (`minifiers.New`,
//! `Client.Transformer`, `Client.Minify`) against the Go oracle
//! `tools/go-oracle/nh-transform/minify` (generated on linux/arm64, the golden build's FMA
//! behaviour).

mod common;

use std::collections::BTreeMap;

use go_value::{Map, MapType, SliceType, Value};
use nh_media::media::config::default_types;
use nh_media::media::media_type::MediaType;
use nh_media::output::output_format::{Formats, OutputFormat, default_formats};
use nh_transform::chain::FromTo;
use nh_transform::minifiers::config::{MinifyConfig, decode_config};
use nh_transform::minifiers::minifiers::Client;
use serde_json::{Value as J, json};

/// Rebuilds a Go config value from the oracle's typed encoding.
fn value(v: &J) -> Value {
    let J::Object(o) = v else {
        assert!(v.is_null(), "{v}");
        return Value::Invalid;
    };
    let (k, x) = o.iter().next().unwrap();
    match k.as_str() {
        "s" => Value::string(x.as_str().unwrap()),
        "i" => Value::int(x.as_i64().unwrap()),
        "f" => Value::float64(x.as_f64().unwrap()),
        "b" => Value::Bool(x.as_bool().unwrap()),
        "l" => Value::list(
            SliceType::Any,
            x.as_array().unwrap().iter().map(value).collect(),
        ),
        "m" | "p" | "ms" => {
            let ty = match k.as_str() {
                "m" => MapType::StringAny,
                "p" => MapType::Params,
                _ => MapType::StringString,
            };
            let mut m = Map::new(ty);
            for (key, e) in x.as_object().unwrap() {
                let ev = if k == "ms" {
                    Value::string(e.as_str().unwrap())
                } else {
                    value(e)
                };
                m.insert(key.as_str(), ev);
            }
            Value::map(m)
        }
        _ => panic!("encoding {k}"),
    }
}

fn dump(c: &MinifyConfig) -> J {
    let t = &c.tdewolff;
    json!({
        "MinifyOutput": c.minify_output, "DisableHTML": c.disable_html, "DisableCSS": c.disable_css,
        "DisableJS": c.disable_js, "DisableJSON": c.disable_json, "DisableSVG": c.disable_svg,
        "DisableXML": c.disable_xml,
        "HTML": {
            "KeepComments": t.html.keep_comments, "KeepConditionalComments": t.html.keep_conditional_comments,
            "KeepSpecialComments": t.html.keep_special_comments, "KeepDefaultAttrVals": t.html.keep_default_attr_vals,
            "KeepDocumentTags": t.html.keep_document_tags, "KeepEndTags": t.html.keep_end_tags,
            "KeepQuotes": t.html.keep_quotes, "KeepWhitespace": t.html.keep_whitespace,
            "TemplateDelims": [t.html.template_delims[0], t.html.template_delims[1]],
        },
        "CSS": {"KeepCSS2": t.css.keep_css2, "Precision": t.css.precision, "Inline": t.css.inline},
        "JS": {"Precision": t.js.precision, "KeepVarNames": t.js.keep_var_names, "Version": t.js.version},
        "JSON": {"Precision": t.json.precision, "KeepNumbers": t.json.keep_numbers},
        "SVG": {"KeepComments": t.svg.keep_comments, "Precision": t.svg.precision, "Inline": t.svg.inline},
        "XML": {"KeepWhitespace": t.xml.keep_whitespace},
    })
}

fn media_type(t: &str) -> MediaType {
    let mut m = MediaType::default();
    m.typ = t.to_string();
    m
}

/// Runs `f` on a thread with a large stack (the minifiers recurse like Go's; see the
/// tdewolff-minify PORTING.md).
fn big_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(f)
        .unwrap()
        .join()
        .unwrap()
}

#[test]
fn decode_config_matches_go() {
    let recs = common::read_jsonl_gz(&common::fixture("minify/minify.jsonl.gz"));
    let mut n = 0;
    for r in recs.iter().filter(|r| r["t"] == "decode") {
        n += 1;
        let got = decode_config(&value(&r["in"]));
        match (&got, r.get("conf"), r.get("err")) {
            (Ok(c), Some(want), None) => assert_eq!(&dump(c), want, "{}", r["in"]),
            (Err(e), None, Some(want)) => {
                assert_eq!(e.to_string(), want.as_str().unwrap(), "{}", r["in"])
            }
            _ => panic!("{}: got {got:?}, want {r}", r["in"]),
        }
    }
    assert!(n > 25, "{n} decode cases");
}

#[test]
fn client_matches_go() {
    big_stack(client_matches_go_impl);
}

fn client_matches_go_impl() {
    let recs = common::read_jsonl_gz(&common::fixture("minify/minify.jsonl.gz"));
    let mut clients: BTreeMap<String, Client> = BTreeMap::new();
    let mut failures = Vec::new();
    let mut n = 0;
    let mut n_failed = 0;
    for r in &recs {
        match r["t"].as_str().unwrap() {
            "client" => {
                let mut types = default_types();
                for e in r["extraTypes"].as_array().unwrap() {
                    let exts: Vec<&str> = e["exts"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|x| x.as_str().unwrap())
                        .collect();
                    types.0.push(
                        MediaType::from_string_and_ext(e["type"].as_str().unwrap(), &exts).unwrap(),
                    );
                }
                let mut formats: Formats = default_formats();
                formats.0.push(OutputFormat {
                    name: "customhtml".into(),
                    media_type: types.get_by_type("text/x-custom-html").unwrap(),
                    is_html: true,
                    ..Default::default()
                });
                let conf = decode_config(&value(&r["conf"])).unwrap();
                let client = Client::from_config(&types, &formats, &conf).unwrap();
                assert_eq!(client.minify_output, r["minifyOutput"].as_bool().unwrap());
                clients.insert(r["variant"].as_str().unwrap().to_string(), client);
            }
            "min" => {
                n += 1;
                let client = &clients[r["variant"].as_str().unwrap()];
                let mt = media_type(r["mt"].as_str().unwrap());
                let input = common::bytes(&r["in"]);
                let mut problems = Vec::new();

                match client.transformer(&mt) {
                    None => {
                        if r.get("nil").is_none() {
                            problems.push("transformer is nil".to_string());
                        }
                    }
                    Some(tr) => {
                        let mut to = Vec::new();
                        let res = tr(&mut FromTo {
                            from: &input,
                            to: &mut to,
                        });
                        match (res, common::opt_bytes(r, "tr"), r.get("trErr")) {
                            (Ok(()), Some(want), None) if to == want => {}
                            (Err(e), None, Some(want))
                                if e.to_string() == want.as_str().unwrap() => {}
                            (res, _, _) => {
                                problems.push(format!("transformer: {res:?} {}", common::show(&to)))
                            }
                        }
                    }
                }
                let got = client.minify(&mt, &input);
                match (&got, common::opt_bytes(r, "min"), r.get("minErr")) {
                    (Ok(out), Some(want), None) if *out == want => {}
                    (Err(e), None, Some(want)) if e.to_string() == want.as_str().unwrap() => {}
                    _ => problems.push(format!(
                        "minify: {:?}",
                        got.as_ref().map(|o| common::show(o))
                    )),
                }
                if !problems.is_empty() {
                    n_failed += 1;
                    if failures.len() < 10 {
                        failures.push(format!(
                            "{} {} {}: {}\n  want {r}",
                            r["variant"],
                            r["mt"],
                            common::show(&input),
                            problems.join("; ")
                        ));
                    }
                }
            }
            _ => {}
        }
    }
    assert!(
        failures.is_empty(),
        "{n_failed} of {n} differ:\n{}",
        failures.join("\n")
    );
    assert!(n > 1000, "{n} cases");
}
