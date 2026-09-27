//! Port of Go's `texttemplate/parse/parse_test.go` (the fork's copy, which
//! is go1.24's): `numberTests`, `parseTests` (plain and copied),
//! `TestParseWithComments`, `TestKeywordsAndFuncs`, `TestSkipFuncCheck`,
//! `isEmptyTests`, `TestErrorContextWithTreeCopy`, `errorTests`,
//! `TestBlock` and `TestLineNum`. The `parseTests`/`errorTests` tables were
//! converted mechanically from the Go source.
//!
//! Go's tests set the package variable `textFormat = "%q"` so that text
//! nodes print quoted; [`quoted_string`] does the same on a copy of the
//! tree.

use std::collections::{BTreeMap, HashSet};

use gotemplate::parse::{
    self, FuncNames, ItemType, ListNode, Mode, Node, NodeLike, NumberNode, Tree, is_empty_tree,
};

const NO_ERROR: bool = true;
const HAS_ERROR: bool = false;

struct ParseTest {
    name: &'static str,
    input: &'static [u8],
    ok: bool,
    /// what the user would see in an error message.
    result: &'static [u8],
}

fn pt(name: &'static str, input: &'static [u8], ok: bool, result: &'static [u8]) -> ParseTest {
    ParseTest {
        name,
        input,
        ok,
        result,
    }
}

/// Go: `var builtins = map[string]any{"printf": fmt.Sprintf, "contains": strings.Contains}`.
fn builtins() -> HashSet<String> {
    ["printf", "contains"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

/// Rewrites every text node's text to its Go `%q` form (Go's tests set
/// `textFormat = "%q"`).
fn quote_texts_list(l: &mut ListNode) {
    for n in &mut l.nodes {
        quote_texts(n);
    }
}

fn quote_texts(n: &mut Node) {
    match n {
        Node::Text(t) => t.text = go_strconv::quote(&t.text).into_bytes(),
        Node::List(l) => quote_texts_list(l),
        Node::If(b) | Node::Range(b) | Node::With(b) => {
            quote_texts_list(&mut b.list);
            if let Some(e) = &mut b.else_list {
                quote_texts_list(e);
            }
        }
        _ => {}
    }
}

/// `list.String()` with Go's test-only `textFormat = "%q"`.
fn quoted_string(list: &ListNode) -> Vec<u8> {
    let mut l = list.clone();
    quote_texts_list(&mut l);
    l.to_bytes()
}

fn parse_mode(
    name: &str,
    input: &[u8],
    mode: Mode,
    funcs: &[&dyn FuncNames],
) -> Result<BTreeMap<String, Tree>, String> {
    parse::parse_with_mode(name, input, "", "", mode, funcs).map_err(|e| e.to_string())
}

fn show(b: &[u8]) -> String {
    format!("{:?}", String::from_utf8_lossy(b))
}

// Go: parse_test.go:testParse
fn test_parse_impl(do_copy: bool) {
    let b = builtins();
    let mut bad = 0;
    for test in parse_tests() {
        let r = parse_mode(test.name, test.input, Mode::default(), &[&b]);
        match (&r, test.ok) {
            (Ok(_), false) => {
                bad += 1;
                eprintln!("{:?}: expected error; got none", test.name);
                continue;
            }
            (Err(e), true) => {
                bad += 1;
                eprintln!("{:?}: unexpected error: {e}", test.name);
                continue;
            }
            (Err(_), false) => continue, // expected error, got one
            (Ok(_), true) => {}
        }
        let trees = r.unwrap();
        let tmpl = &trees[test.name];
        let root = tmpl.root.as_ref().unwrap();
        let result = if do_copy {
            quoted_string(&root.copy_list())
        } else {
            quoted_string(root)
        };
        if result != test.result {
            bad += 1;
            eprintln!(
                "{}=({}): got\n\t{}\nexpected\n\t{}",
                test.name,
                show(test.input),
                show(&result),
                show(test.result)
            );
        }
    }
    assert_eq!(bad, 0);
}

// Go: parse_test.go:TestParse
#[test]
fn test_parse() {
    test_parse_impl(false);
}

// Go: parse_test.go:TestParseCopy
/// Same as TestParse, but we copy the node first
#[test]
fn test_parse_copy() {
    test_parse_impl(true);
}

// Go: parse_test.go:TestParseWithComments
#[test]
fn test_parse_with_comments() {
    let tests = [
        pt("comment", b"{{/*\n\n\n*/}}", NO_ERROR, b"{{/*\n\n\n*/}}"),
        pt(
            "comment trim left",
            b"x \r\n\t{{- /* hi */}}",
            NO_ERROR,
            b"\"x\"{{/* hi */}}",
        ),
        pt(
            "comment trim right",
            b"{{/* hi */ -}}\n\n\ty",
            NO_ERROR,
            b"{{/* hi */}}\"y\"",
        ),
        pt(
            "comment trim left and right",
            b"x \r\n\t{{- /* */ -}}\n\n\ty",
            NO_ERROR,
            b"\"x\"{{/* */}}\"y\"",
        ),
    ];
    for test in tests {
        let trees = parse_mode(test.name, test.input, Mode::PARSE_COMMENTS, &[]).unwrap();
        let result = quoted_string(trees[test.name].root.as_ref().unwrap());
        assert_eq!(show(&result), show(test.result), "{}", test.name);
    }
}

// Go: parse_test.go:TestKeywordsAndFuncs
/// Check collisions between functions and new keywords like 'break'. When a
/// break function is provided, the parser should treat 'break' as a
/// function, not a keyword.
#[test]
fn test_keywords_and_funcs() {
    let inp = b"{{range .X}}{{break 20}}{{end}}";
    // 'break' is a defined function, don't treat it as a keyword: it should
    // accept an argument successfully.
    let funcs_with_keyword_func: HashSet<String> = ["break".to_string()].into_iter().collect();
    let r = parse_mode("", inp, Mode::default(), &[&funcs_with_keyword_func]);
    assert!(
        r.is_ok(),
        "with break func: unexpected error: {:?}",
        r.err()
    );
    // No function called 'break'; treat it as a keyword. Results in a parse
    // error.
    let empty: HashSet<String> = HashSet::new();
    let r = parse_mode("", inp, Mode::default(), &[&empty]);
    assert!(r.is_err(), "without break func: expected error; got none");
}

// Go: parse_test.go:TestSkipFuncCheck
#[test]
fn test_skip_func_check() {
    let trees = parse_mode("skip func check", b"{{fn 1 2}}", Mode::SKIP_FUNC_CHECK, &[]).unwrap();
    let result = quoted_string(trees["skip func check"].root.as_ref().unwrap());
    assert_eq!(show(&result), show(b"{{fn 1 2}}"));
}

// Go: parse_test.go:TestIsEmpty
#[test]
fn test_is_empty() {
    assert!(is_empty_tree(None), "nil tree is not empty");
    let is_empty_tests: [(&str, &[u8], bool); 8] = [
        ("empty", b"", true),
        ("nonempty", b"hello", false),
        ("spaces only", b" \t\n \t\n", true),
        ("comment only", b"{{/* comment */}}", true),
        ("definition", b"{{define \"x\"}}something{{end}}", true),
        (
            "definitions and space",
            b"{{define `x`}}something{{end}}\n\n{{define `y`}}something{{end}}\n\n",
            true,
        ),
        (
            "definitions and text",
            b"{{define `x`}}something{{end}}\nx\n{{define `y`}}something{{end}}\ny\n",
            false,
        ),
        (
            "definition and action",
            b"{{define `x`}}something{{end}}{{if 3}}foo{{end}}",
            false,
        ),
    ];
    for (name, input, empty) in is_empty_tests {
        let trees = parse_mode("root", input, Mode::default(), &[]).unwrap();
        let got = is_empty_tree(trees["root"].root.as_ref());
        assert_eq!(got, empty, "{name:?}");
    }
}

// Go: parse_test.go:TestErrorContextWithTreeCopy
#[test]
fn test_error_context_with_tree_copy() {
    let trees = parse_mode("root", b"{{if true}}{{end}}", Mode::default(), &[]).unwrap();
    let tree = &trees["root"];
    let tree_copy = tree.copy();
    let want = tree.error_context(&tree.root.as_ref().unwrap().nodes[0]);
    let got = tree_copy.error_context(&tree_copy.root.as_ref().unwrap().nodes[0]);
    assert_eq!(want, got);
    assert_eq!(
        want,
        ("root:1:5".to_string(), "{{if true}}{{end}}".to_string())
    );
}

// Go: parse_test.go:TestErrors
/// All failures, and the result is a string that must appear in the error
/// message.
#[test]
fn test_errors() {
    let mut bad = 0;
    for test in error_tests() {
        match parse_mode(test.name, test.input, Mode::default(), &[]) {
            Ok(_) => {
                bad += 1;
                eprintln!(
                    "{}: expected error {}, got nil",
                    test.name,
                    show(test.result)
                );
            }
            Err(e) => {
                if !e.contains(std::str::from_utf8(test.result).unwrap()) {
                    bad += 1;
                    eprintln!(
                        "{}: error {e:?} does not contain {}",
                        test.name,
                        show(test.result)
                    );
                }
            }
        }
    }
    assert_eq!(bad, 0);
}

// Go: parse_test.go:TestBlock
#[test]
fn test_block() {
    let input = b"a{{block \"inner\" .}}bar{{.}}baz{{end}}b";
    let outer = "a{{template \"inner\" .}}b";
    let inner = "bar{{.}}baz";
    let trees = parse_mode("outer", input, Mode::default(), &[]).unwrap();
    let got = trees["outer"].root.as_ref().unwrap().to_string_lossy();
    assert_eq!(got, outer, "outer template");
    let in_tmpl = trees.get("inner").expect("block did not define template");
    assert_eq!(
        in_tmpl.root.as_ref().unwrap().to_string_lossy(),
        inner,
        "inner template"
    );
}

// Go: parse_test.go:TestLineNum
#[test]
fn test_line_num() {
    let count = 3;
    let text = "{{printf 1234}}\n".repeat(count);
    let trees = parse_mode("bench", text.as_bytes(), Mode::default(), &[&builtins()]).unwrap();
    // Check the line numbers. Each line is an action containing a template,
    // followed by text. That's two nodes per line.
    let nodes = &trees["bench"].root.as_ref().unwrap().nodes;
    for i in (0..nodes.len()).step_by(2) {
        let line = 1 + i / 2;
        // Action first.
        let Node::Action(action) = &nodes[i] else {
            panic!("not an action");
        };
        assert_eq!(action.line, line, "line {line}: action");
        assert_eq!(action.pipe.line, line, "line {line}: pipe");
    }
}

struct NumberTest {
    text: &'static str,
    is_int: bool,
    is_uint: bool,
    is_float: bool,
    is_complex: bool,
    int64: i64,
    uint64: u64,
    float64: f64,
    complex128: (f64, f64),
}

#[allow(clippy::too_many_arguments)]
fn nt(
    text: &'static str,
    is_int: bool,
    is_uint: bool,
    is_float: bool,
    is_complex: bool,
    int64: i64,
    uint64: u64,
    float64: f64,
    complex128: (f64, f64),
) -> NumberTest {
    NumberTest {
        text,
        is_int,
        is_uint,
        is_float,
        is_complex,
        int64,
        uint64,
        float64,
        complex128,
    }
}

/// `{text: "..."}`: some broken syntax.
fn bad(text: &'static str) -> NumberTest {
    nt(text, false, false, false, false, 0, 0, 0.0, (0.0, 0.0))
}

const T: bool = true;
const F: bool = false;

fn number_tests() -> Vec<NumberTest> {
    vec![
        // basics
        nt("0", T, T, T, F, 0, 0, 0.0, (0.0, 0.0)),
        nt("-0", T, T, T, F, 0, 0, 0.0, (0.0, 0.0)), // check that -0 is a uint.
        nt("73", T, T, T, F, 73, 73, 73.0, (0.0, 0.0)),
        nt("7_3", T, T, T, F, 73, 73, 73.0, (0.0, 0.0)),
        nt("0b10_010_01", T, T, T, F, 73, 73, 73.0, (0.0, 0.0)),
        nt("0B10_010_01", T, T, T, F, 73, 73, 73.0, (0.0, 0.0)),
        nt("073", T, T, T, F, 0o73, 0o73, 0o73 as f64, (0.0, 0.0)),
        nt("0o73", T, T, T, F, 0o73, 0o73, 0o73 as f64, (0.0, 0.0)),
        nt("0O73", T, T, T, F, 0o73, 0o73, 0o73 as f64, (0.0, 0.0)),
        nt("0x73", T, T, T, F, 0x73, 0x73, 0x73 as f64, (0.0, 0.0)),
        nt("0X73", T, T, T, F, 0x73, 0x73, 0x73 as f64, (0.0, 0.0)),
        nt("0x7_3", T, T, T, F, 0x73, 0x73, 0x73 as f64, (0.0, 0.0)),
        nt("-73", T, F, T, F, -73, 0, -73.0, (0.0, 0.0)),
        nt("+73", T, F, T, F, 73, 0, 73.0, (0.0, 0.0)),
        nt("100", T, T, T, F, 100, 100, 100.0, (0.0, 0.0)),
        nt(
            "1e9",
            T,
            T,
            T,
            F,
            1_000_000_000,
            1_000_000_000,
            1e9,
            (0.0, 0.0),
        ),
        nt("-1e9", T, F, T, F, -1_000_000_000, 0, -1e9, (0.0, 0.0)),
        nt("-1.2", F, F, T, F, 0, 0, -1.2, (0.0, 0.0)),
        nt(
            "1e19",
            F,
            T,
            T,
            F,
            0,
            10_000_000_000_000_000_000,
            1e19,
            (0.0, 0.0),
        ),
        nt(
            "1e1_9",
            F,
            T,
            T,
            F,
            0,
            10_000_000_000_000_000_000,
            1e19,
            (0.0, 0.0),
        ),
        nt(
            "1E19",
            F,
            T,
            T,
            F,
            0,
            10_000_000_000_000_000_000,
            1e19,
            (0.0, 0.0),
        ),
        nt("-1e19", F, F, T, F, 0, 0, -1e19, (0.0, 0.0)),
        nt("0x_1p4", T, T, T, F, 16, 16, 16.0, (0.0, 0.0)),
        nt("0X_1P4", T, T, T, F, 16, 16, 16.0, (0.0, 0.0)),
        nt("0x_1p-4", F, F, T, F, 0, 0, 1.0 / 16.0, (0.0, 0.0)),
        nt("4i", F, F, F, T, 0, 0, 0.0, (0.0, 4.0)),
        nt("-1.2+4.2i", F, F, F, T, 0, 0, 0.0, (-1.2, 4.2)),
        nt("073i", F, F, F, T, 0, 0, 0.0, (0.0, 73.0)), // not octal!
        // complex with 0 imaginary are float (and maybe integer)
        nt("0i", T, T, T, T, 0, 0, 0.0, (0.0, 0.0)),
        nt("-1.2+0i", F, F, T, T, 0, 0, -1.2, (-1.2, 0.0)),
        nt("-12+0i", T, F, T, T, -12, 0, -12.0, (-12.0, 0.0)),
        nt("13+0i", T, T, T, T, 13, 13, 13.0, (13.0, 0.0)),
        // funny bases
        nt("0123", T, T, T, F, 0o123, 0o123, 0o123 as f64, (0.0, 0.0)),
        nt("-0x0", T, T, T, F, 0, 0, 0.0, (0.0, 0.0)),
        nt(
            "0xdeadbeef",
            T,
            T,
            T,
            F,
            0xdeadbeef,
            0xdeadbeef,
            0xdeadbeef_u32 as f64,
            (0.0, 0.0),
        ),
        // character constants
        nt(
            "'a'",
            T,
            T,
            T,
            F,
            'a' as i64,
            'a' as u64,
            'a' as u32 as f64,
            (0.0, 0.0),
        ),
        nt("'\\n'", T, T, T, F, 10, 10, 10.0, (0.0, 0.0)),
        nt("'\\\\'", T, T, T, F, 92, 92, 92.0, (0.0, 0.0)),
        nt("'\\''", T, T, T, F, 39, 39, 39.0, (0.0, 0.0)),
        nt("'\\xFF'", T, T, T, F, 0xFF, 0xFF, 255.0, (0.0, 0.0)),
        nt(
            "'パ'",
            T,
            T,
            T,
            F,
            0x30d1,
            0x30d1,
            0x30d1 as f64,
            (0.0, 0.0),
        ),
        nt(
            "'\\u30d1'",
            T,
            T,
            T,
            F,
            0x30d1,
            0x30d1,
            0x30d1 as f64,
            (0.0, 0.0),
        ),
        nt(
            "'\\U000030d1'",
            T,
            T,
            T,
            F,
            0x30d1,
            0x30d1,
            0x30d1 as f64,
            (0.0, 0.0),
        ),
        // some broken syntax
        bad("+-2"),
        bad("0x123."),
        bad("1e."),
        bad("0xi."),
        bad("1+2."),
        bad("'x"),
        bad("'xx'"),
        bad("'433937734937734969526500969526500'"), // Integer too large - issue 10634.
        // Issue 8622 - 0xe parsed as floating point. Very embarrassing.
        nt("0xef", T, T, T, F, 0xef, 0xef, 0xef as f64, (0.0, 0.0)),
    ]
}

// Go: parse_test.go:TestNumberParse
#[test]
fn test_number_parse() {
    // Go decides the item type with fmt.Sscan: if it can scan the text as a
    // complex128 the item is itemComplex. These are the table's texts that
    // Sscan accepts.
    let sscan_complex = ["-1.2+4.2i", "-1.2+0i", "-12+0i", "13+0i"];
    let mut bad = 0;
    for test in number_tests() {
        let typ = if test.text.starts_with('\'') {
            ItemType::CharConstant
        } else if sscan_complex.contains(&test.text) {
            ItemType::Complex
        } else {
            ItemType::Number
        };
        let r = NumberNode::new(None, 0, test.text.as_bytes(), typ);
        let ok = test.is_int || test.is_uint || test.is_float || test.is_complex;
        let n = match r {
            Err(e) => {
                if ok {
                    bad += 1;
                    eprintln!("unexpected error for {:?}: {e}", test.text);
                }
                continue;
            }
            Ok(n) => {
                if !ok {
                    bad += 1;
                    eprintln!("expected error for {:?}", test.text);
                    continue;
                }
                n
            }
        };
        let mut errs = Vec::new();
        if n.is_complex != test.is_complex {
            errs.push(format!("complex incorrect; should be {}", test.is_complex));
        }
        if n.is_int != test.is_int || (test.is_int && n.int64 != test.int64) {
            errs.push(format!(
                "int: {} {} want {} {}",
                n.is_int, n.int64, test.is_int, test.int64
            ));
        }
        if n.is_uint != test.is_uint || (test.is_uint && n.uint64 != test.uint64) {
            errs.push(format!(
                "uint: {} {} want {} {}",
                n.is_uint, n.uint64, test.is_uint, test.uint64
            ));
        }
        if n.is_float != test.is_float || (test.is_float && n.float64 != test.float64) {
            errs.push(format!(
                "float: {} {} want {} {}",
                n.is_float, n.float64, test.is_float, test.float64
            ));
        }
        if test.is_complex && n.complex128 != test.complex128 {
            errs.push(format!(
                "complex128 {:?} want {:?}",
                n.complex128, test.complex128
            ));
        }
        if !errs.is_empty() {
            bad += 1;
            eprintln!("{:?}: {}", test.text, errs.join("; "));
        }
    }
    assert_eq!(bad, 0);
}

fn parse_tests() -> Vec<ParseTest> {
    vec![
        pt("empty", b"", NO_ERROR, b""),
        pt("comment", b"{{/*\n\n\n*/}}", NO_ERROR, b""),
        pt("spaces", b" \t\n", NO_ERROR, b"\" \\t\\n\""),
        pt("text", b"some text", NO_ERROR, b"\"some text\""),
        pt("emptyAction", b"{{}}", HAS_ERROR, b"{{}}"),
        pt("field", b"{{.X}}", NO_ERROR, b"{{.X}}"),
        pt("simple command", b"{{printf}}", NO_ERROR, b"{{printf}}"),
        pt("$ invocation", b"{{$}}", NO_ERROR, b"{{$}}"),
        pt("variable invocation", b"{{with $x := 3}}{{$x 23}}{{end}}", NO_ERROR, b"{{with $x := 3}}{{$x 23}}{{end}}"),
        pt("variable with fields", b"{{$.I}}", NO_ERROR, b"{{$.I}}"),
        pt("multi-word command", b"{{printf `%d` 23}}", NO_ERROR, b"{{printf `%d` 23}}"),
        pt("pipeline", b"{{.X|.Y}}", NO_ERROR, b"{{.X | .Y}}"),
        pt("pipeline with decl", b"{{$x := .X|.Y}}", NO_ERROR, b"{{$x := .X | .Y}}"),
        pt("nested pipeline", b"{{.X (.Y .Z) (.A | .B .C) (.E)}}", NO_ERROR, b"{{.X (.Y .Z) (.A | .B .C) (.E)}}"),
        pt("field applied to parentheses", b"{{(.Y .Z).Field}}", NO_ERROR, b"{{(.Y .Z).Field}}"),
        pt("simple if", b"{{if .X}}hello{{end}}", NO_ERROR, b"{{if .X}}\"hello\"{{end}}"),
        pt("if with else", b"{{if .X}}true{{else}}false{{end}}", NO_ERROR, b"{{if .X}}\"true\"{{else}}\"false\"{{end}}"),
        pt("if with else if", b"{{if .X}}true{{else if .Y}}false{{end}}", NO_ERROR, b"{{if .X}}\"true\"{{else}}{{if .Y}}\"false\"{{end}}{{end}}"),
        pt("if else chain", b"+{{if .X}}X{{else if .Y}}Y{{else if .Z}}Z{{end}}+", NO_ERROR, b"\"+\"{{if .X}}\"X\"{{else}}{{if .Y}}\"Y\"{{else}}{{if .Z}}\"Z\"{{end}}{{end}}{{end}}\"+\""),
        pt("simple range", b"{{range .X}}hello{{end}}", NO_ERROR, b"{{range .X}}\"hello\"{{end}}"),
        pt("chained field range", b"{{range .X.Y.Z}}hello{{end}}", NO_ERROR, b"{{range .X.Y.Z}}\"hello\"{{end}}"),
        pt("nested range", b"{{range .X}}hello{{range .Y}}goodbye{{end}}{{end}}", NO_ERROR, b"{{range .X}}\"hello\"{{range .Y}}\"goodbye\"{{end}}{{end}}"),
        pt("range with else", b"{{range .X}}true{{else}}false{{end}}", NO_ERROR, b"{{range .X}}\"true\"{{else}}\"false\"{{end}}"),
        pt("range over pipeline", b"{{range .X|.M}}true{{else}}false{{end}}", NO_ERROR, b"{{range .X | .M}}\"true\"{{else}}\"false\"{{end}}"),
        pt("range []int", b"{{range .SI}}{{.}}{{end}}", NO_ERROR, b"{{range .SI}}{{.}}{{end}}"),
        pt("range 1 var", b"{{range $x := .SI}}{{.}}{{end}}", NO_ERROR, b"{{range $x := .SI}}{{.}}{{end}}"),
        pt("range 2 vars", b"{{range $x, $y := .SI}}{{.}}{{end}}", NO_ERROR, b"{{range $x, $y := .SI}}{{.}}{{end}}"),
        pt("range with break", b"{{range .SI}}{{.}}{{break}}{{end}}", NO_ERROR, b"{{range .SI}}{{.}}{{break}}{{end}}"),
        pt("range with continue", b"{{range .SI}}{{.}}{{continue}}{{end}}", NO_ERROR, b"{{range .SI}}{{.}}{{continue}}{{end}}"),
        pt("constants", b"{{range .SI 1 -3.2i true false 'a' nil}}{{end}}", NO_ERROR, b"{{range .SI 1 -3.2i true false 'a' nil}}{{end}}"),
        pt("template", b"{{template `x`}}", NO_ERROR, b"{{template \"x\"}}"),
        pt("template with arg", b"{{template `x` .Y}}", NO_ERROR, b"{{template \"x\" .Y}}"),
        pt("with", b"{{with .X}}hello{{end}}", NO_ERROR, b"{{with .X}}\"hello\"{{end}}"),
        pt("with with else", b"{{with .X}}hello{{else}}goodbye{{end}}", NO_ERROR, b"{{with .X}}\"hello\"{{else}}\"goodbye\"{{end}}"),
        pt("with with else with", b"{{with .X}}hello{{else with .Y}}goodbye{{end}}", NO_ERROR, b"{{with .X}}\"hello\"{{else}}{{with .Y}}\"goodbye\"{{end}}{{end}}"),
        pt("with else chain", b"{{with .X}}X{{else with .Y}}Y{{else with .Z}}Z{{end}}", NO_ERROR, b"{{with .X}}\"X\"{{else}}{{with .Y}}\"Y\"{{else}}{{with .Z}}\"Z\"{{end}}{{end}}{{end}}"),
        pt("trim left", b"x \r\n\t{{- 3}}", NO_ERROR, b"\"x\"{{3}}"),
        pt("trim right", b"{{3 -}}\n\n\ty", NO_ERROR, b"{{3}}\"y\""),
        pt("trim left and right", b"x \r\n\t{{- 3 -}}\n\n\ty", NO_ERROR, b"\"x\"{{3}}\"y\""),
        pt("trim with extra spaces", b"x\n{{-  3   -}}\ny", NO_ERROR, b"\"x\"{{3}}\"y\""),
        pt("comment trim left", b"x \r\n\t{{- /* hi */}}", NO_ERROR, b"\"x\""),
        pt("comment trim right", b"{{/* hi */ -}}\n\n\ty", NO_ERROR, b"\"y\""),
        pt("comment trim left and right", b"x \r\n\t{{- /* */ -}}\n\n\ty", NO_ERROR, b"\"x\"\"y\""),
        pt("block definition", b"{{block \"foo\" .}}hello{{end}}", NO_ERROR, b"{{template \"foo\" .}}"),
        pt("newline in assignment", b"{{ $x \n := \n 1 \n }}", NO_ERROR, b"{{$x := 1}}"),
        pt("newline in empty action", b"{{\n}}", HAS_ERROR, b"{{\n}}"),
        pt("newline in pipeline", b"{{\n\"x\"\n|\nprintf\n}}", NO_ERROR, b"{{\"x\" | printf}}"),
        pt("newline in comment", b"{{/*\nhello\n*/}}", NO_ERROR, b""),
        pt("newline in comment", b"{{-\n/*\nhello\n*/\n-}}", NO_ERROR, b""),
        pt("spaces around continue", b"{{range .SI}}{{.}}{{ continue }}{{end}}", NO_ERROR, b"{{range .SI}}{{.}}{{continue}}{{end}}"),
        pt("spaces around break", b"{{range .SI}}{{.}}{{ break }}{{end}}", NO_ERROR, b"{{range .SI}}{{.}}{{break}}{{end}}"),
        pt("unclosed action", b"hello{{range", HAS_ERROR, b""),
        pt("unmatched end", b"{{end}}", HAS_ERROR, b""),
        pt("unmatched else", b"{{else}}", HAS_ERROR, b""),
        pt("unmatched else after if", b"{{if .X}}hello{{end}}{{else}}", HAS_ERROR, b""),
        pt("multiple else", b"{{if .X}}1{{else}}2{{else}}3{{end}}", HAS_ERROR, b""),
        pt("missing end", b"hello{{range .x}}", HAS_ERROR, b""),
        pt("missing end after else", b"hello{{range .x}}{{else}}", HAS_ERROR, b""),
        pt("undefined function", b"hello{{undefined}}", HAS_ERROR, b""),
        pt("undefined variable", b"{{$x}}", HAS_ERROR, b""),
        pt("variable undefined after end", b"{{with $x := 4}}{{end}}{{$x}}", HAS_ERROR, b""),
        pt("variable undefined in template", b"{{template $v}}", HAS_ERROR, b""),
        pt("declare with field", b"{{with $x.Y := 4}}{{end}}", HAS_ERROR, b""),
        pt("template with field ref", b"{{template .X}}", HAS_ERROR, b""),
        pt("template with var", b"{{template $v}}", HAS_ERROR, b""),
        pt("invalid punctuation", b"{{printf 3, 4}}", HAS_ERROR, b""),
        pt("multidecl outside range", b"{{with $v, $u := 3}}{{end}}", HAS_ERROR, b""),
        pt("too many decls in range", b"{{range $u, $v, $w := 3}}{{end}}", HAS_ERROR, b""),
        pt("dot applied to parentheses", b"{{printf (printf .).}}", HAS_ERROR, b""),
        pt("adjacent args", b"{{printf 3`x`}}", HAS_ERROR, b""),
        pt("adjacent args with .", b"{{printf `x`.}}", HAS_ERROR, b""),
        pt("extra end after if", b"{{if .X}}a{{else if .Y}}b{{end}}{{end}}", HAS_ERROR, b""),
        pt("break outside range", b"{{range .}}{{end}} {{break}}", HAS_ERROR, b""),
        pt("continue outside range", b"{{range .}}{{end}} {{continue}}", HAS_ERROR, b""),
        pt("break in range else", b"{{range .}}{{else}}{{break}}{{end}}", HAS_ERROR, b""),
        pt("continue in range else", b"{{range .}}{{else}}{{continue}}{{end}}", HAS_ERROR, b""),
        pt("bug0a", b"{{$x := 0}}{{$x}}", NO_ERROR, b"{{$x := 0}}{{$x}}"),
        pt("bug0b", b"{{$x += 1}}{{$x}}", HAS_ERROR, b""),
        pt("bug0c", b"{{$x ! 2}}{{$x}}", HAS_ERROR, b""),
        pt("bug0d", b"{{$x % 3}}{{$x}}", HAS_ERROR, b""),
        pt("bug0e", b"{{range $x := $y := 3}}{{end}}", HAS_ERROR, b""),
        pt("bug1a", b"{{$x:=.}}{{$x!2}}", HAS_ERROR, b""),
        pt("bug1b", b"{{$x:=.}}{{$x+2}}", HAS_ERROR, b""),
        pt("bug1c", b"{{$x:=.}}{{$x +2}}", NO_ERROR, b"{{$x := .}}{{$x +2}}"),
        pt("bug2a", b"{{range $x := 0}}{{$x}}{{end}}", NO_ERROR, b"{{range $x := 0}}{{$x}}{{end}}"),
        pt("bug2b", b"{{range $x = 0}}{{$x}}{{end}}", NO_ERROR, b"{{range $x = 0}}{{$x}}{{end}}"),
        pt("dot after integer", b"{{1.E}}", HAS_ERROR, b""),
        pt("dot after float", b"{{0.1.E}}", HAS_ERROR, b""),
        pt("dot after boolean", b"{{true.E}}", HAS_ERROR, b""),
        pt("dot after char", b"{{'a'.any}}", HAS_ERROR, b""),
        pt("dot after string", b"{{\"hello\".guys}}", HAS_ERROR, b""),
        pt("dot after dot", b"{{..E}}", HAS_ERROR, b""),
        pt("dot after nil", b"{{nil.E}}", HAS_ERROR, b""),
        pt("wrong pipeline dot", b"{{12|.}}", HAS_ERROR, b""),
        pt("wrong pipeline number", b"{{.|12|printf}}", HAS_ERROR, b""),
        pt("wrong pipeline string", b"{{.|printf|\"error\"}}", HAS_ERROR, b""),
        pt("wrong pipeline char", b"{{12|printf|'e'}}", HAS_ERROR, b""),
        pt("wrong pipeline boolean", b"{{.|true}}", HAS_ERROR, b""),
        pt("wrong pipeline nil", b"{{'c'|nil}}", HAS_ERROR, b""),
        pt("empty pipeline", b"{{printf \"%d\" ( ) }}", HAS_ERROR, b""),
        pt("block definition", b"{{block \"foo\"}}hello{{end}}", HAS_ERROR, b""),
    ]
}

fn error_tests() -> Vec<ParseTest> {
    vec![
        pt(
            "unclosed1",
            b"line1\n{{",
            HAS_ERROR,
            b"unclosed1:2: unclosed action",
        ),
        pt(
            "unclosed2",
            b"line1\n{{define `x`}}line2\n{{",
            HAS_ERROR,
            b"unclosed2:3: unclosed action",
        ),
        pt(
            "unclosed3",
            b"line1\n{{\"x\"\n\"y\"\n",
            HAS_ERROR,
            b"unclosed3:4: unclosed action started at unclosed3:2",
        ),
        pt(
            "unclosed4",
            b"{{\n\n\n\n\n",
            HAS_ERROR,
            b"unclosed4:6: unclosed action started at unclosed4:1",
        ),
        pt(
            "var1",
            b"line1\n{{\nx\n}}",
            HAS_ERROR,
            b"var1:3: function \"x\" not defined",
        ),
        pt(
            "function",
            b"{{foo}}",
            HAS_ERROR,
            b"function \"foo\" not defined",
        ),
        pt(
            "comment1",
            b"{{/*}}",
            HAS_ERROR,
            b"comment1:1: unclosed comment",
        ),
        pt(
            "comment2",
            b"{{/*\nhello\n}}",
            HAS_ERROR,
            b"comment2:1: unclosed comment",
        ),
        pt(
            "lparen",
            b"{{.X (1 2 3}}",
            HAS_ERROR,
            b"unclosed left paren",
        ),
        pt(
            "rparen",
            b"{{.X 1 2 3 ) }}",
            HAS_ERROR,
            b"unexpected right paren",
        ),
        pt("rparen2", b"{{(.X 1 2 3", HAS_ERROR, b"unclosed action"),
        pt("space", b"{{`x`3}}", HAS_ERROR, b"in operand"),
        pt("idchar", b"{{a#}}", HAS_ERROR, b"'#'"),
        pt(
            "charconst",
            b"{{'a}}",
            HAS_ERROR,
            b"unterminated character constant",
        ),
        pt(
            "stringconst",
            b"{{\"a}}",
            HAS_ERROR,
            b"unterminated quoted string",
        ),
        pt(
            "rawstringconst",
            b"{{`a}}",
            HAS_ERROR,
            b"unterminated raw quoted string",
        ),
        pt("number", b"{{0xi}}", HAS_ERROR, b"number syntax"),
        pt(
            "multidefine",
            b"{{define `a`}}a{{end}}{{define `a`}}b{{end}}",
            HAS_ERROR,
            b"multiple definition of template",
        ),
        pt("eof", b"{{range .X}}", HAS_ERROR, b"unexpected EOF"),
        pt(
            "variable",
            b"{{$x := 23}}{{with $x.y := 3}}{{$x 23}}{{end}}",
            HAS_ERROR,
            b"unexpected \":=\"",
        ),
        pt(
            "multidecl",
            b"{{$a,$b,$c := 23}}",
            HAS_ERROR,
            b"too many declarations",
        ),
        pt("undefvar", b"{{$a}}", HAS_ERROR, b"undefined variable"),
        pt(
            "wrongdot",
            b"{{true.any}}",
            HAS_ERROR,
            b"unexpected . after term",
        ),
        pt(
            "wrongpipeline",
            b"{{12|false}}",
            HAS_ERROR,
            b"non executable command in pipeline",
        ),
        pt(
            "emptypipeline",
            b"{{ ( ) }}",
            HAS_ERROR,
            b"missing value for parenthesized pipeline",
        ),
        pt(
            "multilinerawstring",
            b"{{ $v := `\n` }} {{",
            HAS_ERROR,
            b"multilinerawstring:2: unclosed action",
        ),
        pt(
            "rangeundefvar",
            b"{{range $k}}{{end}}",
            HAS_ERROR,
            b"undefined variable",
        ),
        pt(
            "rangeundefvars",
            b"{{range $k, $v}}{{end}}",
            HAS_ERROR,
            b"undefined variable",
        ),
        pt(
            "rangemissingvalue1",
            b"{{range $k,}}{{end}}",
            HAS_ERROR,
            b"missing value for range",
        ),
        pt(
            "rangemissingvalue2",
            b"{{range $k, $v := }}{{end}}",
            HAS_ERROR,
            b"missing value for range",
        ),
        pt(
            "rangenotvariable1",
            b"{{range $k, .}}{{end}}",
            HAS_ERROR,
            b"range can only initialize variables",
        ),
        pt(
            "rangenotvariable2",
            b"{{range $k, 123 := .}}{{end}}",
            HAS_ERROR,
            b"range can only initialize variables",
        ),
    ]
}
