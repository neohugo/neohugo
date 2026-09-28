//! T22 acceptance: content rendering (`.Content`, `.Plain`, `.PlainWords`, `.Summary`,
//! `.Truncated`, `.WordCount`, `.FuzzyWordCount`, `.ReadingTime`, `.TableOfContents`, `.Len`,
//! `.Fragments`) per page and output format against the Go oracle
//! `tools/go-oracle/nh-hugolib/content`, with every render hook and shortcode template execution
//! REPLAYED from the Go recording through `template_exec::TemplateExecutor`. The replay fails on
//! a key the recording does not have and on a recorded key that is never used.
//!
//! The harness replaces the phases other tasks own: after T20's `process` it sets what Go's
//! assembly (T21) computes and content rendering reads (the page's kind, type, layout, markup,
//! content media type, front matter summary, `isCJKLanguage`; the sites' render formats), creates
//! the page outputs like `initLazyProviders` (one per format NAME, a content output on the first)
//! and shifts the pages of the rendering site like `shiftToOutputFormat(true, idx)` (reusing
//! another output's content output when `canReusePageOutputContent`).

mod content_support;
mod support;

use std::sync::atomic::Ordering;

use nh_hugolib::page::{PageHandle, PageId, PageWrapper};
use nh_tpl::template::TplContext;
use serde_json::{Value as J, json};

use content_support::*;
use support::*;

/// The values of one page in its current output format, in the oracle's call order (then, in
/// action mode, every page's `.HasShortcode` for the probe names).
fn content_values(st: &Setup, id: PageId, want: &J) -> J {
    let h = &st.h;
    let ps = h.page(id);
    let ctx = TplContext::default();
    let host = ctx.as_host();
    let handle = PageHandle {
        h: h.clone(),
        id,
        wrapper: PageWrapper::None,
    };
    let po = ps.current_output().clone();
    let pco = po
        .content_renderer()
        .expect("the rendering site's pages have a content output");
    let mut m = serde_json::Map::new();
    m.insert("page".into(), want["page"].clone());
    if let Some(cp) = po.pco() {
        m.insert("pcoFormat".into(), json!(cp.po.f.name));
    }
    m.insert(
        "toc".into(),
        str_j(&want["toc"], &value_bytes(&pco.table_of_contents(host))),
    );
    match pco.content(host, &handle) {
        Ok(v) => {
            m.insert("content".into(), str_j(&want["content"], &value_bytes(&v)));
        }
        Err(e) => {
            m.insert("contentErr".into(), json!(e.message()));
        }
    }
    match pco.content_without_summary(host) {
        Ok(v) => {
            m.insert(
                "contentWithoutSummary".into(),
                str_j(&want["contentWithoutSummary"], &value_bytes(&v)),
            );
            m.insert("contentWithoutSummaryErr".into(), J::Null);
        }
        Err(e) => {
            m.insert("contentWithoutSummary".into(), json!(""));
            m.insert("contentWithoutSummaryErr".into(), json!(e.message()));
        }
    }
    match pco.c().summary(host) {
        Ok(s) => {
            m.insert("summaryErr".into(), J::Null);
            m.insert("summaryType".into(), json!(s.type_));
        }
        Err(e) => {
            m.insert("summaryErr".into(), json!(e.message()));
            m.insert("summaryType".into(), json!(""));
        }
    }
    m.insert(
        "summary".into(),
        str_j(&want["summary"], &value_bytes(&pco.summary(host))),
    );
    m.insert("truncated".into(), json!(pco.truncated(host)));
    let plain = pco.plain(host, &handle).unwrap();
    m.insert("plain".into(), str_j(&want["plain"], &value_bytes(&plain)));
    let words: Vec<J> = pco
        .plain_words(host)
        .iter()
        .map(|w| J::String(String::from_utf8_lossy(w.as_bytes()).into_owned()))
        .collect();
    if want["plainWords"].is_object() {
        let joined: Vec<u8> = pco
            .plain_words(host)
            .iter()
            .flat_map(|w| {
                let mut b = w.as_bytes().to_vec();
                b.push(0);
                b
            })
            .collect();
        m.insert(
            "plainWords".into(),
            json!({"fnv": fnv(&joined), "len": words.len()}),
        );
    } else {
        m.insert("plainWords".into(), J::Array(words));
    }
    m.insert("wordCount".into(), json!(pco.word_count(host)));
    m.insert("fuzzyWordCount".into(), json!(pco.fuzzy_word_count(host)));
    m.insert("readingTime".into(), json!(pco.reading_time(host)));
    m.insert("len".into(), json!(pco.len(host)));
    if let Some(fr) = pco.fragments(host) {
        m.insert(
            "fragments".into(),
            J::Array(
                fr.identifiers
                    .iter()
                    .map(|s| J::String(String::from_utf8_lossy(s.as_bytes()).into_owned()))
                    .collect(),
            ),
        );
        m.insert(
            "fragmentsHTML13".into(),
            str_j(&want["fragmentsHTML13"], &fr.to_html(1, 3, true)),
        );
    }
    m.insert(
        "variations".into(),
        json!(
            ps.page_output_template_variations_state
                .load(Ordering::SeqCst)
        ),
    );
    m.insert(
        "fatal".into(),
        match h.fatal_error_handler.get_err() {
            Some(e) => json!(e.message()),
            None => J::Null,
        },
    );
    if want.get("hasShortcode").is_some() {
        m.insert(
            "hasShortcode".into(),
            has_shortcode_snapshot(h, &st.ids, &st.replay.probe_names),
        );
    }
    J::Object(m)
}

/// Error texts are not part of parity: only their presence is compared.
fn normalize_errors(v: &mut J) {
    if let J::Object(m) = v {
        for k in [
            "contentErr",
            "contentWithoutSummaryErr",
            "summaryErr",
            "fatal",
        ] {
            if let Some(e) = m.get_mut(k)
                && !e.is_null()
            {
                *e = json!("<error>");
            }
        }
    }
}

fn run_case(name: &str) {
    let fx = fixture(&format!("content/{name}.json.gz"));
    let st = setup(name, &fx);
    let dump = &fx["dump"];
    let pages = dump["pages"].as_array().unwrap();
    let h = &st.h;
    let ids = &st.ids;
    let norm = |s: &str| st.norm(s);

    let mut n_values = 0;
    let mut diffs = Vec::new();
    for step in dump["steps"].as_array().unwrap() {
        let site = step["site"].as_u64().unwrap() as usize;
        let idx = step["idx"].as_u64().unwrap() as usize;
        let format = step["format"].as_str().unwrap();
        h.current_site.store(site, Ordering::SeqCst);
        for &id in ids.iter() {
            if h.page(id).site_idx == site {
                shift_rendering(h, id, idx);
            }
        }
        for want in step["values"].as_array().unwrap() {
            let pi = want["page"].as_u64().unwrap() as usize;
            let mut got = content_values(&st, ids[pi], want);
            let mut want = want.clone();
            normalize_errors(&mut got);
            normalize_errors(&mut want);
            if let J::Object(m) = &mut got {
                for v in m.values_mut() {
                    if let J::String(s) = v {
                        *s = norm(s);
                    }
                }
            }
            n_values += 1;
            if let Some(d) = first_diff(
                &format!("{format}/{}", pages[pi]["file"].as_str().unwrap()),
                &want,
                &got,
            ) {
                diffs.push(d);
            }
        }
    }

    let used = st.check_replay(name);
    let (n_records, nested) = (st.n_records, st.nested);
    eprintln!(
        "{name}: {n_values} page/format values, {n_records} top-level records ({used} replayed), {nested} nested, {} diffs",
        diffs.len()
    );
    for d in diffs.iter().take(8) {
        eprintln!("{d}");
    }
    assert!(diffs.is_empty(), "{name}: {} differences", diffs.len());
}

/// Runs a case on a large stack (template execution and goldmark recurse like Go's growable
/// goroutine stacks, HANDOFF §5 item 3).
fn run(name: &'static str) {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || run_case(name))
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn content_site() {
    run("content");
}

#[test]
fn content_seeksnack() {
    run("seeksnack");
}

#[test]
fn content_testsite() {
    run("testsite");
}

#[test]
fn content_shortcodes() {
    run("shortcodes");
}

/// Action mode: `RenderString` and `.RenderShortcodes` made by the templates, `.HasShortcode`
/// probed before and after each call and after each page's values (`transferNames`).
#[test]
fn content_hasshortcode() {
    run("hasshortcode");
}

#[test]
fn content_docs() {
    run("docs");
}
