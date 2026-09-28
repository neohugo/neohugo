//! Differential tests of the page lexer and front matter split against
//! `tools/go-oracle/nh-parser/pageparser` (fixture `pageparser/pages.json.gz`).

mod support;

use nh_parser::pageparser::item::{Item, ItemType};
use nh_parser::pageparser::pagelexer::Config;
use nh_parser::pageparser::pageparser::{
    has_shortcode, is_probably_source_of_items, new_iterator, parse_bytes,
    parse_front_matter_and_content_bytes,
};
use serde_json::{Value as J, json};
use support::{Diffs, enc, j_bytes, str_j};

const CONFIGS: [Config; 3] = [
    Config {
        no_front_matter: false,
        no_summary_divider: false,
    },
    Config {
        no_front_matter: true,
        no_summary_divider: false,
    },
    Config {
        no_front_matter: false,
        no_summary_divider: true,
    },
];

fn enc_item(src: &[u8], it: &Item, with_strings: bool) -> J {
    let segs = if it.segments().is_empty() {
        J::Null
    } else {
        json!(
            it.segments()
                .iter()
                .map(|&(l, h)| json!([l, h]))
                .collect::<Vec<_>>()
        )
    };
    let err = match &it.err_bytes {
        Some(e) => str_j(e),
        None => J::Null,
    };
    let typed = if it.is_shortcode_param()
        || it.is_shortcode_param_val()
        || it.is_shortcode_name()
        || it.is_text()
    {
        enc(&it.val_typed(src))
    } else {
        J::Null
    };
    let (ts, val) = if with_strings || !it.segments().is_empty() {
        (str_j(&it.to_string_bytes(src)), str_j(&it.val(src)))
    } else {
        (J::Null, J::Null)
    };
    json!([
        it.typ.as_int(),
        it.low(),
        it.high(),
        it.first_byte(),
        it.is_string(),
        segs,
        err,
        it.pos(),
        typed,
        ts,
        val
    ])
}

#[test]
fn lexer_and_front_matter_match_go() {
    let fx = support::fixture_or_env("pageparser/pages.json.gz", "NH_PARSER_PAGES");
    let cases = fx["cases"].as_array().unwrap();
    assert_eq!(cases.len() as u64, fx["inputs"].as_u64().unwrap());
    let mut d = Diffs::default();
    let mut items_checked = 0usize;
    let mut fm_decoded = 0usize;

    for c in cases {
        let name = c["name"].as_str().unwrap();
        let src = j_bytes(&c["src"]);
        let generated = name == "test" || name == "shape" || name.starts_with("soup#");

        for (ci, cfg) in CONFIGS.iter().enumerate() {
            let want = &c["lex"][ci];
            let items = parse_bytes(&src, *cfg).unwrap();
            let want_items = want["items"].as_array().unwrap();
            d.eq(items.len(), want_items.len(), || {
                format!("{name} cfg{ci}: item count; got {:?}", items)
            });
            for (i, (it, w)) in items.iter().zip(want_items).enumerate() {
                items_checked += 1;
                let got = enc_item(&src, it, generated);
                d.check(&got == w, || {
                    format!("{name} cfg{ci} item {i}:\n  got:  {got}\n  want: {w}")
                });
            }
            d.eq(
                J::Bool(is_probably_source_of_items(&src, &items)),
                want["probably"].clone(),
                || format!("{name} cfg{ci}: IsProbablySourceOfItems"),
            );
            if ci == 0 {
                let mut it = new_iterator(&items);
                let mut lines = Vec::new();
                for _ in 0..items.len() {
                    it.next_item();
                    lines.push(json!(it.line_number(&src)));
                }
                d.eq(J::Array(lines), want["lines"].clone(), || {
                    format!("{name}: LineNumber")
                });
                let mut it2 = new_iterator(&items);
                it2.consume(3);
                d.eq(json!(it2.pos()), want["consume3"].clone(), || {
                    format!("{name}: Consume(3)")
                });
                d.eq(
                    json!(it2.is_value_next()),
                    want["valueNext"].clone(),
                    || format!("{name}: IsValueNext"),
                );
            }
        }

        d.eq(
            J::Bool(has_shortcode(&src)),
            c["hasShortcode"].clone(),
            || format!("{name}: HasShortcode"),
        );

        // ParseFrontMatterAndContent.
        let want = &c["fm"];
        let res = parse_front_matter_and_content_bytes(&src);
        if want["format"] == "org" {
            // go-org decoding is not ported: an explicit unsupported error.
            let err = res.expect_err("org front matter must fail");
            d.check(err.message().contains("neohugo-rs:"), || {
                format!("{name}: org error {err}")
            });
            continue;
        }
        match res {
            Ok(cf) => {
                d.eq(
                    json!(cf.front_matter_format.as_str()),
                    want["format"].clone(),
                    || format!("{name}: FM format"),
                );
                let content_off = if want["content"].is_null() {
                    // Go: nil content (no front matter item).
                    d.check(cf.content.is_empty(), || format!("{name}: content not nil"));
                    J::Null
                } else {
                    json!(src.len() - cf.content.len())
                };
                if !want["content"].is_null() {
                    d.eq(content_off, want["content"].clone(), || {
                        format!("{name}: content offset")
                    });
                }
                if want.get("err").is_some() {
                    d.check(false, || format!("{name}: Go failed: {}", want["err"]));
                } else {
                    fm_decoded += 1;
                    let got = if cf.front_matter_nil {
                        json!({"t": "nil:map[string]interface {}"})
                    } else {
                        enc(&go_value::Value::map(cf.front_matter))
                    };
                    d.check(got == want["fm"], || {
                        format!(
                            "{name}: front matter\n  got:  {got}\n  want: {}",
                            want["fm"]
                        )
                    });
                }
            }
            Err(e) => {
                let cause = want
                    .get("cause")
                    .or_else(|| want.get("err"))
                    .map(|j| String::from_utf8_lossy(&j_bytes(j)).into_owned());
                d.eq(Some(e.message().to_string()), cause, || {
                    format!("{name}: front matter error")
                });
            }
        }
    }
    eprintln!(
        "{} inputs, {items_checked} items, {fm_decoded} front matters",
        cases.len()
    );
    d.finish("pageparser");
}

#[test]
fn item_types_match_go_stringer() {
    let names = [
        (ItemType::Error, "tError"),
        (ItemType::Eof, "tEOF"),
        (ItemType::LeadSummaryDivider, "TypeLeadSummaryDivider"),
        (ItemType::FrontMatterOrg, "TypeFrontMatterORG"),
        (ItemType::Text, "tText"),
        (ItemType::KeywordMarker, "tKeywordMarker"),
    ];
    for (t, n) in names {
        assert_eq!(t.string(), n);
    }
    assert_eq!(ItemType::KeywordMarker.as_int(), 19);
}
