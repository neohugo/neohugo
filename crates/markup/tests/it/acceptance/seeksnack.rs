//! The 251 seeksnack bodies against plain goldmark (`corpus/goldmark/*.gmf.gz`), normalised.

use neohugo_markup::{Extensions, LineBreaks, MarkdownOptions, RawHtml, TocOptions};

use super::super::comrak_spike::corpus::seeksnack;
use super::super::comrak_spike::normalize::{Fold, normalize};
use super::{Row, html, print, show};

/// The goldmark instance of a corpus configuration (`tools/go-oracle/goldmark` at 44529028).
fn goldmark(cfg: &str) -> MarkdownOptions {
    let plain = MarkdownOptions {
        extensions: Extensions::empty(),
        typographer: None,
        heading_ids: None,
        toc: TocOptions::default(),
        ..MarkdownOptions::default()
    };
    match cfg {
        "default" => plain,
        "unsafe" => MarkdownOptions {
            raw_html: RawHtml::Pass,
            ..plain
        },
        "all" => MarkdownOptions {
            raw_html: RawHtml::Pass,
            line_breaks: LineBreaks::Hard,
            extensions: Extensions::HEADING_ATTRIBUTES,
            ..plain
        },
        _ => MarkdownOptions {
            raw_html: RawHtml::Pass,
            ..MarkdownOptions::default()
        },
    }
}

#[test]
fn seeksnack_bodies() {
    let docs = seeksnack();
    assert_eq!(docs.len(), 251);
    let fold = Fold {
        auto_ids: true,
        typography: false,
        footnotes: false,
    };
    let mut rows = Vec::new();
    for cfg in ["default", "unsafe", "all", "hugo", "hugo-autoid"] {
        let o = goldmark(cfg);
        let (mut total, mut ok) = (0, 0);
        for d in &docs {
            let Some((_, want)) = d.html.iter().find(|(c, _)| c == cfg) else {
                continue;
            };
            total += 1;
            let (w, g) = (normalize(want, fold), normalize(&html(&d.md, &o), fold));
            show(&format!("seeksnack {cfg}"), &d.name, &w, &g);
            ok += usize::from(w == g);
        }
        rows.push(Row::new(format!("bodies, goldmark {cfg}"), ok, total));
    }
    print("Seeksnack bodies (normalised)", &rows);
    // Plain goldmark `default` writes `<!-- raw HTML omitted -->` for HTML comments; Hugo
    // (and this crate) drop them, which is the only difference on those 11 bodies.
    for r in &rows {
        let floor = if r.what.ends_with("default") {
            240
        } else {
            251
        };
        assert!(r.matched >= floor, "{}: {} < {floor}", r.what, r.matched);
    }
}
