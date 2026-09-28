//! T22: the rendered-content caches of a REAL Go build (`tools/go-oracle/nh-hugolib/hookrec`:
//! page layouts reading `.Content`, `.Summary`, `.Plain`, `.TableOfContents`, `.WordCount` in
//! every output format) against the port, with every render hook and shortcode template
//! execution the Go build performed replayed through `template_exec::TemplateExecutor`.
//!
//! Each Go cache entry is (page, output format name of the content output that rendered it:
//! the `/cont/ren|toc|pla/<site>` key `sourceKey + "/" + format`); the port renders the same
//! entry with a content output of that format and must produce the same content, summary,
//! table of contents and plain text, consuming exactly the recorded executions.

mod content_support;
mod support;

use std::sync::atomic::Ordering;

use nh_hugolib::page__per_output::PageContentOutput;
use nh_tpl::template::TplContext;
use serde_json::{Value as J, json};

use content_support::*;
use support::*;

fn run_case(name: &str) {
    let fx = fixture(&format!("hookrec/{name}.json.gz"));
    if let Some(err) = fx["err"].as_str() {
        panic!("{name}: the Go oracle failed: {err}");
    }
    let st = setup(name, &fx);
    let dump = &fx["dump"];
    let pages = dump["pages"].as_array().unwrap();
    let h = &st.h;

    let mut diffs = Vec::new();
    let caches = dump["caches"].as_array().unwrap();
    for want in caches {
        let pi = want["page"].as_u64().unwrap() as usize;
        let format = want["format"].as_str().unwrap();
        let id = st.ids[pi];
        let ps = h.page(id);
        let lazy = ps.lazy.get().unwrap().as_ref().unwrap();
        let idx = h
            .render_formats
            .0
            .iter()
            .position(|f| f.name == format)
            .unwrap_or_else(|| panic!("{name}: format {format}"));
        let idx = if lazy.outputs.len() == 1 { 0 } else { idx };
        ps.current_output_idx.store(idx, Ordering::SeqCst);
        h.current_site.store(ps.site_idx, Ordering::SeqCst);
        let po = &lazy.outputs[idx];
        let pco = PageContentOutput::new(po, idx).unwrap();
        po.set_content_provider(Some(pco.clone()));

        let ctx = TplContext::default();
        let scope = pco.c();
        let mut got = serde_json::Map::new();
        got.insert("page".into(), json!(pi));
        got.insert("format".into(), json!(format));
        if want.get("content").is_some() {
            let cr = scope.content_rendered(&ctx).unwrap();
            let s = |b: &[u8]| J::String(st.norm(&String::from_utf8_lossy(b)));
            got.insert("content".into(), s(&cr.content));
            got.insert(
                "contentWithoutSummary".into(),
                s(&cr.content_without_summary),
            );
            got.insert("summary".into(), s(cr.summary.text.as_bytes()));
            got.insert("summaryType".into(), json!(cr.summary.type_));
            got.insert("truncated".into(), json!(cr.summary.truncated));
        }
        if want.get("toc").is_some() {
            let ct = scope.content_to_c(&ctx).unwrap();
            got.insert(
                "toc".into(),
                J::String(String::from_utf8_lossy(&ct.table_of_contents_html).into_owned()),
            );
        }
        if want.get("plain").is_some() {
            let p = scope.content_plain(&ctx).unwrap();
            got.insert(
                "plain".into(),
                J::String(st.norm(&String::from_utf8_lossy(&p.plain))),
            );
            got.insert("wordCount".into(), json!(p.word_count));
            got.insert("readingTime".into(), json!(p.reading_time));
        }
        if let Some(d) = first_diff(
            &format!("{format}/{}", pages[pi]["file"].as_str().unwrap()),
            want,
            &J::Object(got),
        ) {
            diffs.push(d);
        }
    }

    let used = st.check_replay(name);
    eprintln!(
        "{name}: {} cache entries, {} top-level records ({used} replayed), {} nested, {} diffs",
        caches.len(),
        st.n_records,
        st.nested,
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
fn hookrec_content() {
    run("content");
}

#[test]
fn hookrec_seeksnack() {
    run("seeksnack");
}

#[test]
fn hookrec_testsite() {
    run("testsite");
}

#[test]
fn hookrec_shortcodes() {
    run("shortcodes");
}
