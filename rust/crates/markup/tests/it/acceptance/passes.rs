//! The passes the plan names (definition-term ids, alert title and sign, block attributes,
//! passthrough, emoji, linkify), on small documents with Hugo's expected output. The corpus
//! tests (`docs`, `hooks`) measure the same passes at scale.

use std::sync::Mutex;

use neohugo_markup::{
    AlertSign, BlockquoteCtx, BlockquoteKind, Delimiters, Extensions, HookEnv, HookError, HookOut,
    Hooks, LinkCtx, LinkifyProtocol, MarkdownOptions, PassthroughCtx, PassthroughKind,
};

use super::super::comrak_spike::normalize::{Fold, normalize};
use super::{CONVERT, HugoCfg, Row, html, options, print, render_with, show};

#[derive(Default)]
struct Capture {
    quotes: Mutex<Vec<BlockquoteCtx>>,
    math: Mutex<Vec<PassthroughCtx>>,
    links: Mutex<Vec<LinkCtx>>,
}

impl Hooks for Capture {
    fn blockquote(&self, _: &HookEnv, c: &BlockquoteCtx) -> Result<HookOut, HookError> {
        self.quotes.lock().expect("lock").push(c.clone());
        Ok(HookOut::Default)
    }
    fn passthrough(&self, _: &HookEnv, c: &PassthroughCtx) -> Result<HookOut, HookError> {
        self.math.lock().expect("lock").push(c.clone());
        Ok(HookOut::Html(format!("<math>{}</math>", c.inner)))
    }
    fn link(&self, _: &HookEnv, c: &LinkCtx) -> Result<HookOut, HookError> {
        self.links.lock().expect("lock").push(c.clone());
        Ok(HookOut::Default)
    }
}

/// The heading ids of docs/rust-port/specs/markdown.md §7, verified there against the Go
/// build of seeksnack (Thai headings, the first-child quirk, entities, dedupe, setext).
#[test]
fn seeksnack_heading_ids() {
    let md = "### **Snack Jack - Sour Cream Flavor (Green Pea Snack)**\n\n### รสชาติ\n\n### สแน็คแจ๊ค รสดั้งเดิม (ขนมถั่วลั่นเตาอบกรอบ)\n\n### Squidy - Seasoned Roller Squid Hot&Spicy\n\n### White Koala's March (Chocolate Filling )\n\n### Pocky Cookies & Cream taste ( Chocolate biscuit stick) Glico brand ([Thai Glico 50th anniversaries](https://www.glico.com/th/en/article/thaiglico-50th/))\n\n## **Strong *em* more** tail\n\n## ![alt *x*](img.png \"T\") img\n\n## &amp; &copy; entity\n\n## Ünïcödé İstanbul ǅ\n\n## 🍫\n\n## Dup\n\n## Dup\n\n## dup-1\n\nSetext line one\nline two\n===\n\n### Edit layouts/_default/index.JSON\n";
    let got = render_with(md, &options(HugoCfg::Seeksnack), &neohugo_markup::NoHooks);
    let mut ids = got.fragments.identifiers.clone();
    let order: Vec<String> = {
        fn walk(h: &[neohugo_markup::Heading], out: &mut Vec<String>) {
            for x in h {
                if !x.id.is_empty() {
                    out.push(x.id.clone());
                }
                walk(&x.children, out);
            }
        }
        let mut v = Vec::new();
        walk(&got.toc.headings, &mut v);
        v
    };
    let want = [
        "snack-jack---sour-cream-flavor-green-pea-snack",
        "รสชาต",
        "สแนคแจค-รสดงเดม-ขนมถวลนเตาอบกรอบ",
        "squidy---seasoned-roller-squid-hotspicy",
        "white-koalas-march-chocolate-filling-",
        "pocky-cookies--cream-taste--chocolate-biscuit-stick-glico-brand-thai-glico-50th-anniversaries",
        "strong--tail",
        "alt--img",
        "--entity",
        "ünïcödé-istanbul-ǆ",
        "heading",
        "dup",
        "dup-1",
        "dup-1-1",
        "line-two",
        "edit-layouts_defaultindexjson",
    ];
    assert_eq!(order, want);
    ids.sort();
    let mut sorted: Vec<String> = want.iter().map(|s| (*s).to_owned()).collect();
    sorted.sort();
    assert_eq!(ids, sorted);
}

#[test]
fn definition_term_ids() {
    let o = options(HugoCfg::Ascii);
    assert_eq!(
        html(
            "Term 1\n: Definition 1\n\nTerm *2*\n: 2a\n: 2b\n\n# Term 1\n",
            &o
        ),
        "<dl>\n<dt id=\"term-1\">Term 1</dt>\n<dd>Definition 1</dd>\n<dt id=\"term-2\">Term <em>2</em></dt>\n<dd>2a</dd>\n<dd>2b</dd>\n</dl>\n<h1 id=\"term-1-1\">Term 1</h1>\n"
    );
}

#[test]
fn alerts_with_title_and_sign() {
    let h = Capture::default();
    let md =
        "> [!WARNING]+ Be careful\n> with this.\n\n> [!note]\n> Plain.\n\n> [!TIP]\n\n> Regular.\n";
    let out = render_with(md, &MarkdownOptions::default(), &h);
    let q = h.quotes.into_inner().expect("lock");
    let got: Vec<_> = q
        .iter()
        .map(|c| {
            (
                c.kind,
                c.alert_type.as_str(),
                c.alert_title.as_str(),
                c.alert_sign,
                c.text.as_str(),
            )
        })
        .collect();
    assert_eq!(
        got,
        vec![
            (
                BlockquoteKind::Alert,
                "warning",
                "Be careful",
                AlertSign::Plus,
                "<p>with this.</p>"
            ),
            (
                BlockquoteKind::Alert,
                "note",
                "",
                AlertSign::None,
                "<p>Plain.</p>"
            ),
            (BlockquoteKind::Alert, "tip", "", AlertSign::None, ""),
            (
                BlockquoteKind::Regular,
                "",
                "",
                AlertSign::None,
                "<p>Regular.</p>"
            ),
        ]
    );
    // Without a hook an alert is a plain blockquote (Hugo's default renderer).
    assert!(
        out.html
            .starts_with("<blockquote>\n<p>[!WARNING]+ Be careful\nwith this.</p></blockquote>\n")
    );
}

#[test]
fn block_attributes() {
    let o = options(HugoCfg::Ascii);
    let md = "A paragraph\n{.para-class #para-id}\n\n> quote\n{.q}\n\n- item 1\n- item 2\n{.list-class}\n\n| a | b |\n|---|---|\n| 1 | 2 |\n{.table-class data-t=\"x\"}\n\n{.solitary}\n\nText\n\n```go\ncode\n```\n{.not-for-fences}\n";
    let got = html(md, &o);
    for want in [
        "<p class=\"para-class\" id=\"para-id\">A paragraph</p>",
        "<blockquote class=\"q\"><p>quote</p></blockquote>",
        "<ul class=\"list-class\">",
        "<table class=\"table-class\" data-t=\"x\">",
        "<p>Text</p>",
    ] {
        assert!(got.contains(want), "{want} in\n{got}");
    }
    assert!(
        !got.contains("solitary") && !got.contains("not-for-fences"),
        "{got}"
    );
    // Off by default: the line stays text.
    assert!(html("A\n{.x}\n", &MarkdownOptions::default()).contains("{.x}"));
}

#[test]
fn passthrough_math() {
    let mut o = MarkdownOptions::default();
    for (open, close, kind) in [
        ("$$", "$$", PassthroughKind::Block),
        ("\\[", "\\]", PassthroughKind::Block),
        ("\\(", "\\)", PassthroughKind::Inline),
    ] {
        o.passthrough.push(Delimiters {
            open: open.into(),
            close: close.into(),
            kind,
        });
    }
    let h = Capture::default();
    let md = "Inline \\(a_1 * b_2\\) and `\\(code\\)`.\n\n$$\nx^2 *y* \\\\\nz\n$$\n\n\\[\\frac{1}{2}\\]\n\n```\n$$no$$\n```\n";
    let out = render_with(md, &o, &h).html;
    let m = h.math.into_inner().expect("lock");
    let got: Vec<_> = m.iter().map(|c| (c.kind, c.inner.as_str())).collect();
    assert_eq!(
        got,
        vec![
            (PassthroughKind::Inline, "a_1 * b_2"),
            (PassthroughKind::Block, "\nx^2 *y* \\\\\nz\n"),
            (PassthroughKind::Block, "\\frac{1}{2}"),
        ]
    );
    assert_eq!(
        out,
        "<p>Inline <math>a_1 * b_2</math> and <code>\\(code\\)</code>.</p>\n<math>\nx^2 *y* \\\\\nz\n</math>\n<math>\\frac{1}{2}</math>\n<pre><code>$$no$$\n</code></pre>\n"
    );
    // Without a hook the source is written as is.
    assert!(
        html("a \\(x_1\\) b\n", &o).contains("a \\(x_1\\) b"),
        "{}",
        html("a \\(x_1\\) b\n", &o)
    );
}

#[test]
fn emoji() {
    let mut o = MarkdownOptions::default();
    assert_eq!(html("a :smile: b\n", &o), "<p>a :smile: b</p>\n");
    o.extensions |= Extensions::EMOJI;
    assert_eq!(
        html("a :smile: :+1: `:smile:` :nosuch:\n", &o),
        "<p>a &#x1f604; &#x1f44d; <code>:smile:</code> :nosuch:</p>\n"
    );
}

#[test]
fn linkify() {
    let h = Capture::default();
    let mut o = MarkdownOptions::default();
    let out = render_with(
        "Go to www.example.org/a, https://x.org/b. or me@example.org! (http://p.org/q_(r))\n",
        &o,
        &h,
    );
    assert_eq!(
        out.html,
        "<p>Go to <a href=\"https://www.example.org/a\">www.example.org/a</a>, <a href=\"https://x.org/b\">https://x.org/b</a>. or <a href=\"mailto:me@example.org\">me@example.org</a>! (<a href=\"http://p.org/q_(r)\">http://p.org/q_(r)</a>)</p>\n"
    );
    let l = h.links.into_inner().expect("lock");
    assert_eq!(l[0].destination, "https://www.example.org/a");
    assert_eq!(l[0].text, "www.example.org/a");
    o.linkify_protocol = LinkifyProtocol::Http;
    assert!(html("www.example.org\n", &o).contains("href=\"http://www.example.org\""));
    o.extensions -= Extensions::LINKIFY;
    assert_eq!(html("www.example.org\n", &o), "<p>www.example.org</p>\n");
}

/// The adversarial documents of the convert oracle (attributes, tables, blockquotes, images,
/// code fences, raw HTML, typographer, linkify, extensions, lists, links, headings), whole
/// output normalised, per configuration.
#[test]
fn adversarial_documents() {
    let fold = Fold {
        auto_ids: false,
        typography: false,
        footnotes: false,
    };
    let mut rows = Vec::new();
    for cfg in HugoCfg::ALL {
        let o = options(cfg);
        let (mut total, mut ok) = (0, 0);
        for case in CONVERT
            .iter()
            .filter(|c| c.name.starts_with("adversarial/"))
        {
            let Some(want) = &case.html[cfg as usize] else {
                continue;
            };
            total += 1;
            let got = html(&case.md, &o);
            let (w, g) = (normalize(want, fold), normalize(&got, fold));
            show(&format!("adversarial {}", cfg.name()), &case.name, &w, &g);
            ok += usize::from(w == g);
        }
        rows.push(Row::new(
            format!("adversarial, cfg {}", cfg.name()),
            ok,
            total,
        ));
    }
    print("Adversarial documents (convert oracle, normalised)", &rows);
    // Residuals: Hugo's textual context markers (hugo-ctx-inline), invalid UTF-8 (the input
    // here is already `str`), the oracle's table replica (`s:` values) and CJK line breaks.
    for (r, min) in rows.iter().zip([18, 18, 17, 18, 15, 17]) {
        assert!(r.matched >= min, "{}: {} < {min}", r.what, r.matched);
    }
}
