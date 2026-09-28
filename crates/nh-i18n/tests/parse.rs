//! Oracle test: go-i18n message file parsing (ParseMessageFileBytes with Hugo's toml/yaml/yml/
//! json unmarshalers) against `tools/go-oracle/nh-i18n/parse` (fixtures/parse/parse.json.gz).

mod common;

use nh_i18n::goi18n::parse::parse_message_file_bytes;
use serde_json::{Value as J, json};

fn parse(content: &str, path: &str) -> J {
    match parse_message_file_bytes(content.as_bytes(), path) {
        Err(e) => json!({ "err": e.message }),
        Ok(mf) => {
            let mut msgs: Vec<J> = mf
                .messages
                .iter()
                .map(|m| {
                    json!({
                        "id": m.id, "hash": m.hash, "description": m.description,
                        "leftDelim": m.left_delim, "rightDelim": m.right_delim,
                        "zero": m.zero, "one": m.one, "two": m.two, "few": m.few,
                        "many": m.many, "other": m.other,
                    })
                })
                .collect();
            // The oracle sorts by the JSON text of each message (Go's json.Marshal sorts map keys).
            msgs.sort_by_key(go_json_text);
            json!({"tag": mf.tag.string(), "format": mf.format, "messages": msgs})
        }
    }
}

/// Go `json.Marshal` of a `map[string]string`: keys sorted, HTML-escaped strings.
fn go_json_text(m: &J) -> String {
    let o = m.as_object().unwrap();
    let mut keys: Vec<&String> = o.keys().collect();
    keys.sort();
    let mut s = String::from("{");
    for (i, k) in keys.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&go_str(k));
        s.push(':');
        s.push_str(&go_str(o[*k].as_str().unwrap()));
    }
    s.push('}');
    s
}

fn go_str(s: &str) -> String {
    let v = go_value::Value::string(s);
    String::from_utf8(go_json::marshal(&v).unwrap()).unwrap()
}

#[test]
fn parse_matches_go() {
    let fx = common::load_fixture("parse", "parse.json.gz");
    let mut bad = Vec::new();
    let cases = fx["cases"].as_array().unwrap();
    for c in cases {
        let path = c["path"].as_str().unwrap();
        let content = c["content"].as_str().unwrap();
        let got = parse(content, path);
        if got != c["result"] {
            bad.push(format!(
                "{path} {content:?}:\n  got  {got}\n  want {}",
                c["result"]
            ));
        }
    }
    assert!(
        bad.is_empty(),
        "{} of {} differ:\n{}",
        bad.len(),
        cases.len(),
        bad.join("\n")
    );
    assert!(cases.len() > 100);
}
