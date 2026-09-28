//! Oracle test: the CLDR plural rules (go-i18n internal/plural + the generated rule_gen.rs)
//! against `tools/go-oracle/nh-i18n/plural` (fixtures/plural/plural.json.gz): every locale of
//! rule_gen.go × a grid of counts, and the CLDR samples of rule_gen_test.go.

mod common;

use std::sync::Arc;

use go_value::Value;
use nh_i18n::goi18n::bundle::Bundle;
use nh_i18n::goi18n::localizer::{LocalizeConfig, Localizer};
use nh_i18n::goi18n::message::Message;
use serde_json::{Value as J, json};
use xtext_collate::language as xl;

fn localizer(id: &str) -> Result<Localizer, J> {
    let tag = xl::DEFAULT.must_parse(id);
    let mut b = Bundle::new(xl::english());
    let m = Message {
        id: "m".into(),
        zero: "zero".into(),
        one: "one".into(),
        two: "two".into(),
        few: "few".into(),
        many: "many".into(),
        other: "other".into(),
        ..Default::default()
    };
    if let Err(e) = b.add_messages(tag.clone(), vec![m]) {
        return Err(json!({"adderr": e}));
    }
    Ok(Localizer::new(Arc::new(b), &[tag.string().as_str()]))
}

fn localize(l: &Result<Localizer, J>, count: &J) -> J {
    let l = match l {
        Ok(l) => l,
        Err(e) => return e.clone(),
    };
    let pc = common::value_from_spec(count);
    let (s, t, err) = l.localize_with_tag(&LocalizeConfig {
        message_id: "m".into(),
        template_data: Value::Invalid,
        plural_count: if matches!(pc, Value::Invalid) {
            None
        } else {
            Some(pc)
        },
        default_message: None,
    });
    if let Some(nh_i18n::goi18n::localizer::LocalizeError::Panic(p)) = &err {
        return json!({"panic": p});
    }
    let mut r = json!({"s": s, "tag": t.string()});
    if let Some(e) = err {
        r["err"] = J::String(e.to_string());
    }
    r
}

#[test]
fn plural_rules_match_go() {
    let fx = common::load_fixture("plural", "plural.json.gz");
    let counts = fx["counts"].as_array().unwrap();
    let mut n = 0;
    let mut bad = Vec::new();
    for c in fx["cases"].as_array().unwrap() {
        let locale = c["locale"].as_str().unwrap();
        let results = c["results"].as_array().unwrap();
        let inputs: Vec<J> = if c["kind"] == "grid" {
            counts.clone()
        } else {
            c["tests"].as_array().unwrap().clone()
        };
        assert_eq!(inputs.len(), results.len());
        let l = localizer(locale);
        for (input, want) in inputs.iter().zip(results) {
            n += 1;
            let got = localize(&l, input);
            if &got != want {
                bad.push(format!("{locale} {input}: got {got} want {want}"));
            }
            // The Go test's expected form (rule_gen_test.go) holds too.
            if let Some(form) = input.get("form") {
                assert_eq!(&got["s"], form, "{locale} {input}");
            }
        }
    }
    assert!(
        bad.is_empty(),
        "{} of {n} differ:\n{}",
        bad.len(),
        bad[..bad.len().min(40)].join("\n")
    );
    assert!(n > 40_000, "{n}");
}
