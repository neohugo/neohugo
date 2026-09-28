//! Differential tests of `Language::new` and `decode_config` against
//! `tools/go-oracle/nh-langs/language` (fixture `language/language.json.gz`, arm64 Go).

mod support;

use go_value::Value;
use nh_langs::config::{LanguageConfig, decode_config};
use nh_langs::language::Language;
use serde_json::{Value as J, json};
use support::{dec, j_string};

#[test]
fn language_matches_go() {
    let fx = support::fixture("language/language.json.gz");
    assert_eq!(fx["goarch"], "arm64");
    let mut mismatches = Vec::new();
    let mut checks = 0;
    let words = [
        "a", "A", "b", "é", "e", "z", "ä", "ö", "o", "ch", "c", "h", "ß", "ss", "ก", "เก", "ข",
        "1", "-", " ",
    ];

    for c in fx["cases"].as_array().unwrap() {
        checks += 1;
        match c["op"].as_str().unwrap() {
            "DecodeConfig" => {
                let Value::Map(m) = dec(&c["in"]) else {
                    panic!("not a map")
                };
                let got = match decode_config(&m) {
                    Ok(langs) => json!({
                        "out": langs
                            .iter()
                            .map(|(k, l)| {
                                json!([
                                    k,
                                    l.language_name,
                                    l.language_code,
                                    l.title,
                                    l.language_direction,
                                    l.weight,
                                    l.disabled
                                ])
                            })
                            .collect::<Vec<_>>()
                    }),
                    Err(e) => json!({ "err": e.message() }),
                };
                let want = match c.get("err") {
                    Some(e) => json!({ "err": j_string(e) }),
                    None => json!({
                        "out": c["out"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .map(|row| {
                                let r = row.as_array().unwrap();
                                json!([
                                    j_string(&r[0]),
                                    j_string(&r[1]),
                                    j_string(&r[2]),
                                    j_string(&r[3]),
                                    j_string(&r[4]),
                                    r[5],
                                    r[6]
                                ])
                            })
                            .collect::<Vec<_>>()
                    }),
                };
                if got != want {
                    mismatches.push(format!("{}:\n  got:  {got}\n  want: {want}", c["name"]));
                }
            }
            "NewLanguage" => {
                let lang = c["lang"].as_str().unwrap();
                let dcl = c["dcl"].as_str().unwrap();
                let tz = c["tz"].as_str().unwrap();
                let got = match Language::new(lang, dcl, tz, LanguageConfig::default()) {
                    Err(e) => json!({ "tag": tag_of(lang), "err": e.message() }),
                    Ok(l) => {
                        let mut signs = Vec::new();
                        for a in words {
                            for b in words {
                                let s1 = l.collator1().compare_strings(a.as_bytes(), b.as_bytes());
                                let s2 = l.collator2().compare_strings(a.as_bytes(), b.as_bytes());
                                assert_eq!(s1, s2);
                                signs.push(s1);
                            }
                        }
                        json!({
                            "tag": l.tag,
                            "languageCode": l.language_code(),
                            "string": l.string(),
                            "translator": l.translator().locale(),
                            "location": l.location().name,
                            "signs": signs,
                        })
                    }
                };
                let mut want = json!({ "tag": c["tag"] });
                if let Some(e) = c.get("err") {
                    want["err"] = J::String(j_string(e));
                } else {
                    for k in ["languageCode", "string", "translator", "location", "signs"] {
                        want[k] = c[k].clone();
                    }
                }
                if got != want {
                    mismatches.push(format!(
                        "NewLanguage({lang:?}, {dcl:?}, {tz:?}):\n  got:  {got}\n  want: {want}"
                    ));
                }
            }
            other => panic!("unknown op {other}"),
        }
    }
    eprintln!("language: {checks} cases, {} mismatches", mismatches.len());
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

/// The tag string Go's `language.Parse(lang)` returns (also on error).
fn tag_of(lang: &str) -> String {
    match xtext_collate::language::DEFAULT.parse(lang) {
        Ok(t) | Err((t, _)) => t.string(),
    }
}
