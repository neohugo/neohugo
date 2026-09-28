//! Differential tests against `tools/go-oracle/nh-media/media` (fixture `media/media.json.gz`):
//! media.Builtin, DefaultTypes, DecodeTypes and DecodeContentTypes (types, SourceHash, the
//! `hugo config` JSON dump), the Types/Type/ContentTypes methods, FromString(AndExt),
//! http.DetectContentType and FromContent.

mod support;

use go_value::{Map, MapType, Value};
use nh_config::config_provider::Provider;
use nh_config::default_config_provider::DefaultConfigProvider;
use nh_media::media::builtin::builtin;
use nh_media::media::config::{
    ContentTypes, decode_content_types, decode_types, default_content_types, default_types,
};
use nh_media::media::media_type::{MediaType, SuffixInfo, Types, from_content};
use nh_media::media::sniff::detect_content_type;
use serde_json::{Map as JMap, Value as J, json};
use support::*;

const SUFFIX_PROBES: &[&str] = &[
    "html", "htm", "md", "markdown", "mdown", "adoc", "asciidoc", "ad", "pdc", "rst", "org", "xml",
    "json", "", "HTML", "MD", "cst", "CUS", "cus", "xhtml", "txt", ".md", "pandoc", "mmark",
    "thing",
];

const FILE_PROBES: &[&str] = &[
    "a.md",
    "index.md",
    "_index.md",
    "x/y/index.adoc",
    "index",
    "index.",
    "a.MD",
    ".md",
    "a.b.org",
    "index.html",
    "_index.htm",
    "/abs/_index.rst",
    "c:\\x\\index.md",
    "index.md/",
    "indexes.md",
    "_indexx.md",
    "p/index.cst",
    "a.pdc",
    "a.txt",
    "",
];

const HAS_SUFFIX_PROBES: &[&str] = &["html", "xml", "md", "", "json", "cst", "HTML"];

const HINT_PROBES: &[&[&str]] = &[
    &[],
    &["html"],
    &["js"],
    &[".svg"],
    &["xml"],
    &["json"],
    &["txt"],
    &["csv"],
    &["md"],
    &["png"],
    &["unknown", "json"],
    &["yaml"],
    &["toml"],
    &["css"],
    &[".rss"],
    &["ics"],
    &["webp"],
    &["jpg", "png"],
    &["cst"],
    &["svgz"],
];

fn content_types_dump(c: &ContentTypes) -> J {
    json!({
        "HTML": dump_type(&c.html), "Markdown": dump_type(&c.markdown), "AsciiDoc": dump_type(&c.ascii_doc),
        "Pandoc": dump_type(&c.pandoc), "ReStructuredText": dump_type(&c.re_structured_text),
        "EmacsOrgMode": dump_type(&c.emacs_org_mode), "Types": dump_types(c.types()),
        "IsContentSuffix": SUFFIX_PROBES.iter().map(|s| c.is_content_suffix(s)).collect::<Vec<_>>(),
        "IsHTMLSuffix": SUFFIX_PROBES.iter().map(|s| c.is_html_suffix(s)).collect::<Vec<_>>(),
        "IsContentFile": FILE_PROBES.iter().map(|s| c.is_content_file(s)).collect::<Vec<_>>(),
        "IsIndexContentFile": FILE_PROBES.iter().map(|s| c.is_index_content_file(s)).collect::<Vec<_>>(),
    })
}

fn type_methods(types: &Types) -> J {
    J::Array(
        types
            .0
            .iter()
            .map(|t| {
                let suffixes = t.suffixes();
                json!({
                    "String": s(t.string()),
                    "Suffixes": if suffixes.is_empty() { J::Null } else { J::Array(suffixes.iter().map(|x| s(x)).collect()) },
                    "IsText": t.is_text(), "IsHTML": t.is_html(), "IsMarkdown": t.is_markdown(), "IsZero": t.is_zero(),
                    "HasSuffix": HAS_SUFFIX_PROBES.iter().map(|x| t.has_suffix(x)).collect::<Vec<_>>(),
                    "MarshalJSON": bytes_result(t.marshal_json_bytes()),
                })
            })
            .collect(),
    )
}

fn found((t, ok): (MediaType, bool)) -> J {
    json!([dump_type(&t), ok])
}

fn found_opt(t: Option<MediaType>) -> J {
    let ok = t.is_some();
    found((t.unwrap_or_default(), ok))
}

fn found_si((t, si, ok): (MediaType, SuffixInfo, bool)) -> J {
    json!([dump_type(&t), dump_suffix_info(&si), ok])
}

/// Go's `goval.CallRaw` record of a decode: `{"ok": v}`, `{"err": msg}` or `{"panic": msg}`.
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

fn types_input(input: &Value, via_provider: bool) -> Map {
    if via_provider {
        let cfg = DefaultConfigProvider::new();
        if !input.is_invalid() {
            cfg.set("mediaTypes", input.clone());
        }
        cfg.get_string_map("mediatypes")
    } else {
        input_map(input)
    }
}

/// The fields of a record other than the case description.
fn record(c: &J) -> J {
    let mut o = c.as_object().unwrap().clone();
    for k in ["op", "name", "in", "viaProvider"] {
        o.remove(k);
    }
    if c["op"] != "DefaultTypes" {
        // The name of the types used ("default", "custom").
        o.remove("types");
    }
    J::Object(o)
}

#[test]
fn media_oracle() {
    let fx = fixture("media/media.json.gz");
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

    let cases = fx["cases"].as_array().unwrap();
    let mut failures = Vec::new();
    let mut compared = 0;
    for (i, c) in cases.iter().enumerate() {
        if c.get("nondet").is_some() || c.get("skip").is_some() {
            continue;
        }
        compared += 1;
        let op = c["op"].as_str().unwrap();
        let want = record(c);
        let got: J = match op {
            "Builtin" => {
                let fields: Vec<J> = builtin()
                    .fields()
                    .into_iter()
                    .map(|(n, t)| json!([n, dump_type(t)]))
                    .collect();
                json!({ "fields": fields })
            }
            "DefaultTypes" => json!({ "types": dump_types(&default_types()) }),
            "DefaultContentTypes" => json!({ "out": content_types_dump(&default_content_types()) }),
            "DecodeTypes" => {
                let input = decode(&c["in"]);
                let via = c["viaProvider"].as_bool().unwrap();
                call_raw(|| {
                    let ns = decode_types(&types_input(&input, via))?;
                    let mut o = JMap::new();
                    o.insert("types".into(), dump_types(&ns.config));
                    o.insert("hash".into(), s(&ns.source_hash));
                    o.insert("typeMethods".into(), type_methods(&ns.config));
                    dump_json(&ns.source_structure, &mut o);
                    Ok(J::Object(o))
                })
            }
            "DecodeContentTypes" => {
                let input = decode(&c["in"]);
                let types = types_by(c["types"].as_str().unwrap());
                call_raw(|| {
                    let ns = decode_content_types(&input_map(&input), &types)?;
                    let mut o = JMap::new();
                    o.insert("out".into(), content_types_dump(&ns.config));
                    o.insert("hash".into(), s(&ns.source_hash));
                    dump_json(&ns.source_structure, &mut o);
                    Ok(J::Object(o))
                })
            }
            "TypesQuery" => {
                let types = types_by(c["types"].as_str().unwrap());
                let q = j_string(&c["q"]);
                let by: Vec<J> = types.by_suffix(&q).iter().map(|t| s(&t.typ)).collect();
                let (main, sub) = q.split_once('/').unwrap_or((q.as_str(), ""));
                json!({
                    "q": s(&q),
                    "GetBestMatch": found_opt(types.get_best_match(&q)),
                    "GetByType": found(types.get_by_type_found(&q)),
                    "BySuffix": if by.is_empty() { J::Null } else { J::Array(by) },
                    "GetFirstBySuffix": found_si(types.get_first_by_suffix(&q).map_or_else(Default::default, |(t, si)| (t, si, true))),
                    "GetBySuffix": found_si(types.get_by_suffix_found(&q)),
                    "IsTextSuffix": types.is_text_suffix(&q),
                    "GetBySubType": found(types.get_by_sub_type_found(&q)),
                    "GetByMainSubType": found(types.get_by_main_sub_type_found(main, sub)),
                })
            }
            "TypeMethods" => {
                json!({ "out": type_methods(&types_by(c["types"].as_str().unwrap())) })
            }
            "FromString" => {
                let q = j_string(&c["s"]);
                let (t, e) = match MediaType::from_string(&q) {
                    Ok(t) => (t, None),
                    Err(e) => (MediaType::default(), Some(e.to_string())),
                };
                json!({ "s": s(&q), "out": dump_type(&t), "err": err_j(e) })
            }
            "FromStringAndExt" => {
                let q = j_string(&c["s"]);
                let ext = strs(&c["ext"]);
                let ext_refs: Vec<&str> = ext.iter().map(String::as_str).collect();
                let (t, e) = match MediaType::from_string_and_ext(&q, &ext_refs) {
                    Ok(t) => (t, None),
                    Err(e) => (MediaType::default(), Some(e.to_string())),
                };
                json!({
                    "s": s(&q), "ext": c["ext"].clone(), "out": dump_type(&t), "err": err_j(e),
                    "json": bytes_result(t.marshal_json_bytes()),
                })
            }
            "FromContent" => {
                let types = types_by(c["types"].as_str().unwrap());
                let content = goval::bytes(&c["content"]);
                let outs: Vec<J> = HINT_PROBES
                    .iter()
                    .map(|h| {
                        let h: Vec<String> = h.iter().map(|x| x.to_string()).collect();
                        dump_type(&from_content(&types, &h, &content))
                    })
                    .collect();
                json!({ "content": c["content"].clone(), "out": outs })
            }
            "DetectContentType" => {
                let ins = c["in"].as_array().unwrap();
                let outs: Vec<J> = ins
                    .iter()
                    .map(|b| J::String(detect_content_type(&goval::bytes(b)).to_string()))
                    .collect();
                json!({ "out": outs })
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
    report("media", compared, &failures);
    assert!(compared > 2000, "{compared}");
}
