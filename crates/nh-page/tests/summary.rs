//! Oracle test: `page.ExtractSummaryFromHTML`, `ExtractSummaryFromHTMLWithDivider` and the
//! `HtmlSummary` methods against `tools/go-oracle/nh-page/summary`
//! (fixtures/summary/*.json.gz): the rendered HTML of every page of the Go builds (recorded
//! through an overlaid hook), variants of it, and adversarial HTML.

mod support;

use nh_page::page_markup::{
    HtmlSummary, extract_summary_from_html, extract_summary_from_html_with_divider,
};
use serde_json::Value as J;
use sha2::{Digest, Sha256};
use support::*;

fn digest(s: &[u8]) -> J {
    if s.len() <= 160 {
        return enc(s);
    }
    let sum = Sha256::digest(s);
    serde_json::json!({ "len": s.len(), "sha256": hex(&sum) })
}

fn lh(l: nh_common::types::types::LowHigh) -> J {
    serde_json::json!([l.low, l.high])
}

fn dump(r: &HtmlSummary) -> J {
    serde_json::json!({
        "summary": digest(&r.summary()),
        "contentWithoutSummary": digest(&r.content_without_summary()),
        "content": digest(&r.content()),
        "truncated": r.truncated(),
        "summaryLowHigh": lh(r.summary_low_high),
        "summaryEndTag": lh(r.summary_end_tag),
        "wrapperStart": lh(r.wrapper_start),
        "wrapperEnd": lh(r.wrapper_end),
        "divider": lh(r.divider),
    })
}

/// Go stores a divider that was not found as `[-1, len-1]`; the port stores the zero range.
fn normalize(want: &J) -> J {
    let mut w = want.clone();
    if let Some(ok) = w.get_mut("ok")
        && ok["divider"][0] == -1
    {
        ok["divider"] = serde_json::json!([0, 0]);
    }
    w
}

fn run_file(name: &str) -> (usize, Vec<String>) {
    let fx = fixture(&format!("summary/{name}"));
    let types: Vec<_> = fx["types"]
        .as_array()
        .unwrap()
        .iter()
        .map(media_type)
        .collect();
    let inputs: Vec<Vec<u8>> = fx["inputs"].as_array().unwrap().iter().map(gostr).collect();
    let mut fails = Vec::new();
    let mut n = 0;
    for c in fx["cases"].as_array().unwrap() {
        let mt = &types[c["mt"].as_u64().unwrap() as usize];
        let input = &inputs[c["input"].as_u64().unwrap() as usize];
        let got = match catch(|| {
            if c["kind"] == "manual" {
                dump(&extract_summary_from_html_with_divider(
                    mt,
                    input,
                    &gostr(&c["divider"]),
                ))
            } else {
                dump(&extract_summary_from_html(
                    mt,
                    input,
                    c["numWords"].as_i64().unwrap(),
                    c["isCJK"].as_bool().unwrap(),
                ))
            }
        }) {
            Ok(v) => serde_json::json!({ "ok": v }),
            Err(p) => serde_json::json!({ "panic": p }),
        };
        n += 1;
        let want = normalize(&c["want"]);
        // A Go runtime panic (slice bounds; the reStructuredText wrapper with a divider before
        // it) is a Rust slice panic here; the messages differ.
        if want.get("panic").is_some() && got.get("panic").is_some() {
            continue;
        }
        if got != want {
            fails.push(format!(
                "{name} {} {} input {:?}: got {got}\n   want {want}",
                c["kind"],
                c["numWords"],
                String::from_utf8_lossy(&input[..input.len().min(200)])
            ));
        }
    }
    (n, fails)
}

#[test]
fn extract_summary_matches_go() {
    let mut total = 0;
    let mut fails = Vec::new();
    for f in fixture_files("summary") {
        let (n, fl) = run_file(&f);
        eprintln!("summary {f}: {n} cases, {} failures", fl.len());
        total += n;
        fails.extend(fl);
    }
    for f in fails.iter().take(20) {
        eprintln!("  {f}");
    }
    assert!(total > 15000);
    assert!(fails.is_empty());
}
