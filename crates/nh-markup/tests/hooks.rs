//! The `hooks` oracle (tools/go-oracle/nh-markup/hooks): the corpus rendered with recording hook
//! renderers for every hook type; the HTML (with the hooks' markers), every hook context dump,
//! and the errors and panics of failing hooks must match Go.

mod common;

use std::sync::Arc;

use common::*;
use go_value::Value;
use nh_markup::converter::converter::DocumentContext;
use nh_markup::goldmark::hugocontext;

#[test]
fn hooks_corpus() {
    common::big_stack(run_hooks_corpus);
}

fn run_hooks_corpus() {
    let fx = read_fixture("hooks/hooks.json.gz");
    let docs = fx["docs"].as_array().unwrap();
    let configs = fx["configs"].as_array().unwrap();
    let providers: Vec<_> = configs
        .iter()
        .map(|c| provider(c["toml"].as_str().unwrap()).0)
        .collect();

    let results = fx["results"].as_array().unwrap();
    let mut failures = Vec::new();
    let (mut n_html, mut n_rec, mut n_err) = (0, 0, 0);
    for r in results {
        let di = r["doc"].as_u64().unwrap() as usize;
        let ci = r["cfg"].as_u64().unwrap() as usize;
        let wrap = r["wrap"].as_bool().unwrap();
        let doc = &docs[di];
        let dname = doc["name"].as_str().unwrap();
        let name = format!(
            "{dname} [{}] wrap={wrap}",
            configs[ci]["name"].as_str().unwrap()
        );
        let mut src = bytes(&doc["src"]);
        if wrap {
            let mut s = b"Intro *text*\n\n".to_vec();
            s.extend_from_slice(&hugocontext::wrap(&src, 7));
            s.extend_from_slice(b"\nOutro [l](x)\n\n");
            s.extend_from_slice(&hugocontext::wrap(b"## Nil lookup\n\n![i](n.png)\n", 9));
            src = s;
        }
        let page = r["page"].as_str().unwrap();
        let dctx = DocumentContext {
            document: page_value(page),
            document_lookup: Some(Arc::new(|pid: u64| {
                if pid == 9 {
                    Value::Invalid
                } else {
                    page_value(&format!("inner:{pid}"))
                }
            })),
            document_id: dname.to_string(),
            document_name: String::new(),
            filename: dname.to_string(),
        };
        let conv = converter(&providers[ci], dctx);
        let rec = Arc::new(Recorder::default());
        let out = convert(
            &*conv,
            &src,
            r["toc"].as_bool().unwrap(),
            recording_renderers(rec.clone()),
        );

        let want_records: Vec<Vec<u8>> =
            r["records"].as_array().unwrap().iter().map(bytes).collect();
        let got_records = rec.records.lock().unwrap().clone();
        let mut ok = true;
        if got_records.len() != want_records.len() {
            failures.push(format!(
                "{name}: {} hook calls, want {}",
                got_records.len(),
                want_records.len()
            ));
            ok = false;
        }
        for (i, (g, w)) in got_records.iter().zip(want_records.iter()).enumerate() {
            if g != w {
                failures.push(format!(
                    "{name}: hook record {i} differs\n--- got\n{}\n--- want\n{}",
                    show(g),
                    show(w)
                ));
                ok = false;
                break;
            }
        }
        if ok {
            n_rec += 1;
        }

        match out {
            Outcome::Html(rr) => {
                if let Some(e) = r.get("err").and_then(|v| v.as_str()) {
                    failures.push(format!("{name}: want error {e}"));
                } else if let Some(p) = r.get("panic").and_then(|v| v.as_str()) {
                    failures.push(format!("{name}: want panic {p}"));
                } else {
                    let want = bytes(&r["html"]);
                    if rr.bytes != want {
                        failures.push(format!(
                            "{name}: HTML differs\n--- got\n{}\n--- want\n{}",
                            show(&rr.bytes),
                            show(&want)
                        ));
                    } else {
                        n_html += 1;
                    }
                }
            }
            Outcome::Err(e) => match r.get("err").and_then(|v| v.as_str()) {
                Some(w) if w == e => n_err += 1,
                w => failures.push(format!("{name}: error {e:?}, want {w:?}")),
            },
            Outcome::Panic(p) => match r.get("panic").and_then(|v| v.as_str()) {
                Some(w) if w == p => n_err += 1,
                w => failures.push(format!("{name}: panic {p:?}, want {w:?}")),
            },
        }
    }
    eprintln!(
        "hooks: {} conversions: {n_html} HTML identical, {n_err} errors/panics identical, {n_rec} hook dump lists identical ({} hook calls)",
        results.len(),
        results
            .iter()
            .map(|r| r["records"].as_array().unwrap().len())
            .sum::<usize>()
    );
    if !failures.is_empty() {
        let n = failures.len();
        for f in failures.iter().take(8) {
            eprintln!("{f}\n");
        }
        panic!("{n} failures");
    }
}
