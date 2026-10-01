//! `diagrams_goat` (feature `goat`) on every GoAT diagram of the Hugo documentation site
//! (the ```` ```goat ```` code blocks of `docs/content/**/*.md`).

use std::path::Path;

use tera::{Context, Value};

use crate::support::{Harness, crate_dir};

fn collect(dir: &Path, out: &mut Vec<(String, String)>) {
    let re = regex::Regex::new(r"(?ms)^```goat[^\n]*\n(.*?)^```").expect("valid");
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .expect("docs dir")
        .flatten()
        .collect();
    entries.sort_by_key(std::fs::DirEntry::path);
    for e in entries {
        let p = e.path();
        if p.is_dir() {
            collect(&p, out);
        } else if p.extension().is_some_and(|x| x == "md") {
            let text = std::fs::read_to_string(&p).expect("utf-8 markdown");
            for caps in re.captures_iter(&text) {
                out.push((p.display().to_string(), caps[1].to_owned()));
            }
        }
    }
}

fn field(d: &Value, key: &str) -> serde_json::Value {
    serde_json::to_value(d).expect("serializable")[key].clone()
}

#[test]
fn renders_every_docs_diagram() {
    let docs = crate_dir().join("../../docs/content");
    let mut diagrams = Vec::new();
    collect(&docs, &mut diagrams);
    assert!(
        diagrams.len() >= 10,
        "found only {} diagrams",
        diagrams.len()
    );
    let h = Harness::new();
    for (file, text) in &diagrams {
        let mut ctx = Context::new();
        ctx.insert("text", text);
        let d = h
            .eval("diagrams_goat(text=text)", &ctx)
            .unwrap_or_else(|e| panic!("{file}: {e}"));
        let lines = text.lines().count();
        let columns = text.lines().map(|l| l.chars().count()).max().unwrap_or(0);
        assert_eq!(
            field(&d, "width").as_u64(),
            u64::try_from((columns + 1) * 8).ok(),
            "{file}"
        );
        assert_eq!(
            field(&d, "height").as_u64(),
            u64::try_from((lines + 1) * 16).ok(),
            "{file}"
        );
        let inner = field(&d, "inner");
        let inner = inner.as_str().unwrap_or_default();
        assert!(
            inner.starts_with("<g class='svgbob'") && inner.contains("<text"),
            "{file}: {inner}"
        );
    }
}

#[test]
fn diagram() {
    let h = Harness::new();
    let out = h
        .render(
            "{% set d = diagrams_goat(text='+--+\n| A|-->\n+--+') %}\
             {{ d.width }}x{{ d.height }}\n{{ d.inner }}\n{{ d.wrapped }}",
            &Context::new(),
        )
        .expect("renders");
    neohugo_testkit::snapshot::settings().bind(|| {
        insta::assert_snapshot!("diagrams_goat", out);
    });
}
