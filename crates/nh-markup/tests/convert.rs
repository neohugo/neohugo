//! The `convert` oracle (tools/go-oracle/nh-markup/convert): every corpus document rendered with
//! every markup config without link/image/heading/blockquote hooks (table and code block
//! replicas), plus the table of contents (Fragments and ToHTML) and the decoded configs.

mod common;

use std::sync::Arc;

use common::*;
use nh_markup::tableofcontents::{Fragments, Heading};
use serde_json::Value as J;

fn heading_matches(h: &Heading, j: &J) -> bool {
    let kids = j["headings"].as_array().cloned().unwrap_or_default();
    h.id.as_bytes() == bytes(&j["id"]).as_slice()
        && j["level"].as_i64() == Some(h.level)
        && h.title.as_bytes() == bytes(&j["title"]).as_slice()
        && h.headings.len() == kids.len()
        && h.headings
            .iter()
            .zip(kids.iter())
            .all(|(a, b)| heading_matches(a, b))
}

fn headings_match(h: &[Arc<Heading>], j: &J) -> bool {
    let a = j.as_array().cloned().unwrap_or_default();
    h.len() == a.len() && h.iter().zip(a.iter()).all(|(x, y)| heading_matches(x, y))
}

const TOC_LEVELS: [(i64, i64); 4] = [(1, -1), (2, 3), (3, 4), (0, 2)];

fn check_toc(
    name: &str,
    f: Option<&Arc<Fragments>>,
    want: &J,
    start: i64,
    end: i64,
    ordered: bool,
) -> Result<(), String> {
    let present = want["present"].as_bool().unwrap_or(false);
    let Some(f) = f else {
        return if present {
            Err(format!("{name}: TOC missing"))
        } else {
            Ok(())
        };
    };
    if !present {
        return Err(format!("{name}: unexpected TOC"));
    }
    if !headings_match(&f.headings, &want["headings"]) {
        return Err(format!(
            "{name}: TOC headings differ\n got {:?}\nwant {}",
            f.headings, want["headings"]
        ));
    }
    let want_ids: Vec<Vec<u8>> = want["identifiers"]
        .as_array()
        .map(|a| a.iter().map(bytes).collect())
        .unwrap_or_default();
    let got_ids: Vec<Vec<u8>> = f
        .identifiers
        .iter()
        .map(|x| x.as_bytes().to_vec())
        .collect();
    if got_ids != want_ids {
        return Err(format!(
            "{name}: identifiers differ\n got {:?}\nwant {:?}",
            got_ids.iter().map(|x| show(x)).collect::<Vec<_>>(),
            want_ids.iter().map(|x| show(x)).collect::<Vec<_>>()
        ));
    }
    let want_keys: Vec<Vec<u8>> = want["map_keys"]
        .as_array()
        .map(|a| a.iter().map(bytes).collect())
        .unwrap_or_default();
    let got_keys: Vec<Vec<u8>> = f
        .headings_map
        .keys()
        .map(|x| x.as_bytes().to_vec())
        .collect();
    if got_keys != want_keys {
        return Err(format!("{name}: headings map keys differ"));
    }
    let got_vals: Vec<Arc<Heading>> = f.headings_map.values().cloned().collect();
    if !headings_match(&got_vals, &want["map_values"]) {
        return Err(format!("{name}: headings map values differ"));
    }
    let mut levels = vec![(start, end, ordered)];
    for (i, (a, b)) in TOC_LEVELS.iter().enumerate() {
        levels.push((*a, *b, (i + 1) % 2 == 0));
    }
    let want_html = want["html"].as_array().expect("html");
    for (i, (a, b, o)) in levels.iter().enumerate() {
        let got = f.to_html(*a, *b, *o);
        let w = bytes(&want_html[i]);
        if got != w {
            return Err(format!(
                "{name}: ToHTML({a}, {b}, {o}) differs\n got {}\nwant {}",
                show(&got),
                show(&w)
            ));
        }
    }
    Ok(())
}

#[test]
fn convert_corpus() {
    common::big_stack(run_convert_corpus);
}

fn run_convert_corpus() {
    let fx = read_fixture("convert/convert.json.gz");
    let docs = fx["docs"].as_array().unwrap();
    let configs = fx["configs"].as_array().unwrap();
    let mut providers = Vec::new();
    for c in configs {
        let (p, m) = provider(c["toml"].as_str().unwrap());
        assert_eq!(
            dump_markup_config(&m),
            c["dump"].as_str().unwrap(),
            "config {} dump",
            c["name"]
        );
        providers.push((p, m));
    }

    let results = fx["results"].as_array().unwrap();
    let mut html_by: std::collections::HashMap<(usize, usize), Vec<u8>> = Default::default();
    let mut toc_by: std::collections::HashMap<(usize, usize), J> = Default::default();
    let mut failures = Vec::new();
    let (mut n_html, mut n_toc) = (0, 0);
    for r in results {
        let di = r["doc"].as_u64().unwrap() as usize;
        let ci = r["cfg"].as_u64().unwrap() as usize;
        let doc = &docs[di];
        let name = format!(
            "{} [{}]",
            doc["name"].as_str().unwrap(),
            configs[ci]["name"].as_str().unwrap()
        );
        let src = bytes(&doc["src"]);
        let (p, m) = &providers[ci];
        let conv = converter(p, document_context(doc["name"].as_str().unwrap()));
        let out = convert(&*conv, &src, true, replica_renderers());

        let want_html = match r.get("html_same").and_then(|v| v.as_u64()) {
            Some(first) => html_by[&(di, first as usize)].clone(),
            None => bytes(&r["html"]),
        };
        let want_toc = match r.get("toc_same").and_then(|v| v.as_u64()) {
            Some(first) => toc_by[&(di, first as usize)].clone(),
            None => r["toc"].clone(),
        };
        html_by.insert((di, ci), want_html.clone());
        toc_by.insert((di, ci), want_toc.clone());

        match out {
            Outcome::Html(rr) => {
                if let Some(e) = r.get("err").and_then(|v| v.as_str()) {
                    failures.push(format!("{name}: want error {e}"));
                    continue;
                }
                if rr.bytes != want_html {
                    failures.push(format!(
                        "{name}: HTML differs\n--- got\n{}\n--- want\n{}",
                        show(&rr.bytes),
                        show(&want_html)
                    ));
                    continue;
                }
                n_html += 1;
                let t = &m.table_of_contents;
                match check_toc(
                    &name,
                    rr.table_of_contents.as_ref(),
                    &want_toc,
                    t.start_level,
                    t.end_level,
                    t.ordered,
                ) {
                    Ok(()) => n_toc += 1,
                    Err(e) => failures.push(e),
                }
            }
            Outcome::Err(e) => failures.push(format!("{name}: error {e}")),
            Outcome::Panic(p) => failures.push(format!("{name}: panic {p}")),
        }
    }
    eprintln!(
        "convert: {n_html}/{} HTML identical, {n_toc} TOCs identical",
        results.len()
    );
    if !failures.is_empty() {
        let n = failures.len();
        for f in failures.iter().take(8) {
            eprintln!("{f}\n");
        }
        panic!("{n} failures");
    }
}
