//! Oracle test: x/text language matching (NewMatcher/Match, ParseAcceptLanguage) and go-i18n's
//! bundle/localizer resolution against `tools/go-oracle/nh-i18n/matcher`
//! (fixtures/matcher/matcher.json.gz).

mod common;

use std::sync::Arc;

use nh_i18n::goi18n::bundle::Bundle;
use nh_i18n::goi18n::localizer::{LocalizeConfig, Localizer};
use nh_i18n::goi18n::message::Message;
use nh_i18n::xlanguage::{Confidence, Matcher, parse_accept_language};
use serde_json::{Value as J, json};
use xtext_collate::language as xl;

fn conf_str(c: Confidence) -> &'static str {
    // Go: language.Confidence.String()
    match c {
        Confidence::No => "No",
        Confidence::Low => "Low",
        Confidence::High => "High",
        Confidence::Exact => "Exact",
    }
}

fn strs(v: &J) -> Vec<String> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap().to_string())
        .collect()
}

fn tags(ss: &[String]) -> Vec<xl::Tag> {
    ss.iter().map(|s| xl::make(s)).collect()
}

#[test]
fn matcher_matches_go() {
    let fx = common::load_fixture("matcher", "matcher.json.gz");
    let mut n = 0;
    let mut bad = Vec::new();
    for c in fx["cases"].as_array().unwrap() {
        match c["kind"].as_str().unwrap() {
            "match" => {
                let supported = strs(&c["supported"]);
                let m = Matcher::new(&tags(&supported));
                for r in c["results"].as_array().unwrap() {
                    n += 1;
                    let want = strs(&r["want"]);
                    let (i, conf) = m.match_tags(&tags(&want));
                    let got = json!({"want": r["want"], "index": i, "conf": conf_str(conf)});
                    if &got != r {
                        bad.push(format!("match {supported:?} {want:?}: got {got} want {r}"));
                    }
                }
            }
            "bundle" => {
                n += 1;
                let supported = strs(&c["supported"]);
                let mut b = Bundle::new(xl::make(&supported[0]));
                let mut added = Vec::new();
                for s in &supported {
                    let t = xl::make(s);
                    let m = Message {
                        id: "m".into(),
                        other: format!("msg:{}", t.string()),
                        ..Default::default()
                    };
                    match b.add_messages(t.clone(), vec![m]) {
                        Ok(()) => added.push(t.string()),
                        Err(e) => added.push(format!("err:{e}")),
                    }
                }
                let got_tags: Vec<String> = b.language_tags().iter().map(|t| t.string()).collect();
                if J::from(added.clone()) != c["added"] || J::from(got_tags.clone()) != c["tags"] {
                    bad.push(format!(
                        "bundle {supported:?}: added {added:?} tags {got_tags:?} want {} {}",
                        c["added"], c["tags"]
                    ));
                    continue;
                }
                let b = Arc::new(b);
                for r in c["results"].as_array().unwrap() {
                    n += 1;
                    let lang = r["lang"].as_str().unwrap();
                    let l = Localizer::new(b.clone(), &[lang]);
                    let (s, t, err) = l.localize_with_tag(&LocalizeConfig {
                        message_id: "m".into(),
                        template_data: go_value::Value::Invalid,
                        plural_count: None,
                        default_message: None,
                    });
                    let mut got = json!({"lang": lang, "s": s, "tag": t.string()});
                    if let Some(e) = err {
                        got["err"] = J::String(e.to_string());
                    }
                    if &got != r {
                        bad.push(format!("bundle {supported:?} {lang}: got {got} want {r}"));
                    }
                }
            }
            "accept" => {
                n += 1;
                let s = c["s"].as_str().unwrap();
                let mut got = json!({"kind": "accept", "s": s});
                match parse_accept_language(s) {
                    Ok((ts, qs)) => {
                        let ts: Vec<String> = ts.iter().map(|t| t.string()).collect();
                        let qs: Vec<String> = qs
                            .iter()
                            .map(|q| go_strconv::format_float(*q as f64, b'g', -1, 32))
                            .collect();
                        got["tags"] = J::String(ts.join(","));
                        got["q"] = J::String(qs.join(","));
                    }
                    Err(e) => got["err"] = J::String(e.to_string()),
                }
                if &got != c {
                    bad.push(format!("accept {s:?}: got {got} want {c}"));
                }
            }
            k => panic!("unknown case kind {k}"),
        }
    }
    assert!(
        bad.is_empty(),
        "{} of {n} differ:\n{}",
        bad.len(),
        bad[..bad.len().min(40)].join("\n")
    );
    assert!(n > 100_000, "{n}");
}
