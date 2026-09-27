//! Replays `tests/fixtures/html/internal.txt.gz` (written by
//! `tools/go-oracle/gotemplate/htmlexec.go:writeInternal` from the fork's
//! unexported functions):
//!
//! - `X`: escape_test.go TestEscapeText — `escapeText` of one text node from
//!   the start context: the output context (Go `String()`) and that the node
//!   text is not modified;
//! - `P`: TestEnsurePipelineContains — the rewritten pipeline;
//! - `F`: the `redundantFuncs` table, plus TestRedundantFuncs' property.

use go_value::{SafeKind, Value};

use super::super::context::Context;
use super::super::escape::{Escaper, ensure_pipeline_contains, redundant_funcs};
use super::super::{ESC_FUNC_NAMES, esc_func};
use super::common::{q, read_gz_fixture, unquote};
use crate::parse::{Node, NodeLike, TextNode};

fn lines(kind: &str) -> Vec<Vec<String>> {
    read_gz_fixture("html/internal.txt.gz")
        .lines()
        .filter(|l| l.starts_with(kind) && l.as_bytes().get(kind.len()) == Some(&b'\t'))
        .map(|l| l.split('\t').map(str::to_string).collect())
        .collect()
}

// Go: escape_test.go:TestEscapeText
#[test]
fn escape_text_contexts() {
    let cases = lines("X");
    assert!(cases.len() > 100, "{} cases", cases.len());
    let mut bad = Vec::new();
    for f in &cases {
        let text = unquote(&f[1]);
        let want = String::from_utf8(unquote(&f[2])).unwrap();
        let n = TextNode::new(None, 0, text.clone());
        let mut e = Escaper::new();
        let c = e.escape_text(Context::default(), &n);
        if c.string() != want || (n.text == text) != (f[3] == "true") {
            bad.push(format!("{}: want {want} got {}", q(&text), c.string()));
        }
    }
    assert!(
        bad.is_empty(),
        "{} of {} differ:\n{}",
        bad.len(),
        cases.len(),
        bad.join("\n")
    );
}

// Go: escape_test.go:TestEnsurePipelineContains
#[test]
fn ensure_pipeline_contains_table() {
    let cases = lines("P");
    assert!(cases.len() > 15, "{} cases", cases.len());
    for f in &cases {
        let src = unquote(&f[1]);
        let ids: Vec<String> = if f[2].is_empty() {
            Vec::new()
        } else {
            f[2].split(',')
                .map(|s| String::from_utf8(unquote(s)).unwrap())
                .collect()
        };
        let want = String::from_utf8(unquote(&f[3])).unwrap();
        // Go: template.Must(template.New("test").Parse(test.input))
        let t = crate::text::Template::new("test");
        t.parse(&src).expect("parse");
        let tree = t.tree().expect("tree").get();
        let root = tree.root.as_ref().expect("root");
        let Node::Action(action) = &root.nodes[0] else {
            panic!("first node is not an action: {}", q(&src));
        };
        let mut pipe = action.pipe.clone();
        ensure_pipeline_contains(&mut pipe, ids.clone());
        let got = String::from_utf8(pipe.to_bytes()).unwrap();
        assert_eq!(got, want, "{} {ids:?}", q(&src));
    }
}

// Go: escape.go:redundantFuncs (the table) and escape_test.go:TestRedundantFuncs
#[test]
fn redundant_funcs_table_and_property() {
    let pairs: Vec<(String, String)> = lines("F")
        .into_iter()
        .map(|f| (f[1].clone(), f[2].clone()))
        .collect();
    assert_eq!(pairs.len(), 7);
    for a in ESC_FUNC_NAMES {
        for b in ESC_FUNC_NAMES {
            let want = pairs.iter().any(|(x, y)| x == a && y == b);
            assert_eq!(redundant_funcs(a, b), want, "{a} {b}");
        }
    }

    let mut first = String::new();
    for c in 0u8..0x80 {
        first.push(c as char);
    }
    first.push_str(
        "\u{A0}\u{100}\u{2028}\u{2029}\u{feff}\u{fdec}\u{fffd}\u{ffff}\u{1D11E}&amp;%22\\",
    );
    let inputs = [
        Value::string(first.as_str()),
        Value::Safe(SafeKind::Css, r#"a[href =~ "//example.com"]#foo"#.into()),
        Value::Safe(SafeKind::Html, "Hello, <b>World</b> &amp;tc!".into()),
        Value::Safe(SafeKind::HtmlAttr, r#" dir="ltr""#.into()),
        Value::Safe(SafeKind::Js, r#"c && alert("Hello, World!");"#.into()),
        Value::Safe(SafeKind::JsStr, r"Hello, World & O'Reilly\x21".into()),
        Value::Safe(SafeKind::Url, "greeting=H%69&addressee=(World)".into()),
    ];
    for (n0, n1) in &pairs {
        let f0 = esc_func(n0).unwrap();
        let f1 = esc_func(n1).unwrap();
        for input in &inputs {
            let want = f0(std::slice::from_ref(input));
            let got = f1(&[Value::String(want.clone().into())]);
            assert_eq!(got, want, "{n0} {n1} with {input:?}");
        }
    }
}
