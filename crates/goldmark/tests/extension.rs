//! Differential tests for goldmark's extensions (crate::extension): Rust
//! output vs the Go oracle (tools/go-oracle/goldmark ext.go), byte-exact.

mod common;

use common::{check_records, fixture};

/// goldmark's extension/_test/*.txt with the configurations of the
/// extension _test.go files (+ "hugo" and "x-all"), and the DoTestCase cases
/// of those _test.go files. Also checks that Go matches the expected output
/// of goldmark's own tests (TrimSpace, as testutil does).
#[test]
fn extension_test_files() {
    let recs = fixture("ext.gmf");
    for r in &recs {
        if let Some(spec) = r.fields.get("spec") {
            assert_eq!(
                String::from_utf8_lossy(r.get("html")).trim(),
                String::from_utf8_lossy(spec).trim(),
                "{}: Go output differs from goldmark's expected output",
                r.name
            );
        }
    }
    let n = check_records(&recs);
    assert!(n > 200, "{n}");
}

#[test]
fn extension_edge_cases() {
    let n = check_records(&fixture("ext-edge.gmf.gz"));
    assert!(n > 5000, "{n}");
}

#[test]
fn extension_fuzz_fixed_seed() {
    let n = check_records(&fixture("ext-fuzz.gmf.gz"));
    assert_eq!(n, 10000);
}

/// The 251 seeksnack markdown bodies with the extension set and options
/// neohugo configures (plus the whole files).
#[test]
fn seeksnack_corpus_hugo_extensions() {
    let n = check_records(&fixture("corpus-ext.gmf.gz"));
    assert_eq!(n, 251 * 3);
}

// Go: extension/ast_test.go:TestASTBlockNodeText
#[test]
fn ast_block_node_text() {
    use goldmark::renderer::html;
    struct Case {
        name: &'static str,
        source: &'static str,
        t1: &'static str,
        t2: &'static str,
        c: bool,
    }
    let cases = [
        Case {
            name: "DefinitionList",
            source: "c1\n:   c2\n    c3\n\na\n\nc4\n:   c5\n    c6",
            t1: "c1c2\nc3",
            t2: "c4c5\nc6",
            c: false,
        },
        Case {
            name: "Table",
            source: "| h1 | h2 |\n| -- | -- |\n| c1 | c2 |\n\na\n\n\n| h3 | h4 |\n| -- | -- |\n| c3 | c4 |",
            t1: "h1h2c1c2",
            t2: "h3h4c3c4",
            c: false,
        },
    ];
    for cs in cases {
        let s = cs.source.as_bytes();
        let md = goldmark::new(vec![
            goldmark::with_renderer_options(vec![Box::new(html::with_unsafe())]),
            goldmark::with_extensions(vec![
                goldmark::extension::definition_list(),
                goldmark::extension::table(),
            ]),
        ]);
        let mut pc = goldmark::parser::new_context(vec![]);
        let doc = md.parse(s, &mut pc);
        let ast = &doc.ast;
        let mut c1 = ast.first_child(doc.root).unwrap();
        let mut c2 = ast.next_sibling(ast.next_sibling(c1).unwrap()).unwrap();
        if cs.c {
            c1 = ast.first_child(c1).unwrap();
            c2 = ast.first_child(c2).unwrap();
        }
        assert_eq!(
            String::from_utf8_lossy(&ast.text(c1, s)),
            cs.t1,
            "{} unmatch",
            cs.name
        );
        assert_eq!(
            String::from_utf8_lossy(&ast.text(c2, s)),
            cs.t2,
            "{}(EOF) unmatch",
            cs.name
        );
    }
}

// Go: extension/ast_test.go:TestASTInlineNodeText
#[test]
fn ast_inline_node_text() {
    use goldmark::renderer::html;
    let s = b"~c1 *c2*~";
    let md = goldmark::new(vec![
        goldmark::with_renderer_options(vec![Box::new(html::with_unsafe())]),
        goldmark::with_extensions(vec![goldmark::extension::strikethrough()]),
    ]);
    let mut pc = goldmark::parser::new_context(vec![]);
    let doc = md.parse(s, &mut pc);
    let ast = &doc.ast;
    let c1 = ast.first_child(ast.first_child(doc.root).unwrap()).unwrap();
    assert_eq!(ast.kind(c1), *goldmark::extension::ast::KIND_STRIKETHROUGH);
    assert_eq!(String::from_utf8_lossy(&ast.text(c1, s)), "c1 c2");
}

/// Go's extension parsers declare `CloseBlock(parent, pc)`, which does not
/// satisfy parser.CloseBlocker, so the typographer's unclosed-quote
/// counters carry over from one block to the next (bug-for-bug).
#[test]
fn typographer_counters_are_not_reset_per_block() {
    let md = common::new_markdown("x-typo");
    let mut out: Vec<u8> = Vec::new();
    md.convert(b"\"open\n\nsaturated: \"\"\n", &mut out)
        .unwrap();
    assert_eq!(
        String::from_utf8(out).unwrap(),
        "<p>&ldquo;open</p>\n<p>saturated: &quot;&rdquo;</p>\n"
    );
}
