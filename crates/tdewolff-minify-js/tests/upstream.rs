//! The upstream `minify/v2@v2.23.8/js/js_test.go` tests, ported: the
//! `TestJS`, `TestJSVarRenaming` and `TestJSVersion` tables (extracted
//! literally into `upstream_tables.rs` by the Go oracle, which also checked
//! that every row passes with go1.27.1), `TestReaderError`,
//! `TestWriterError` and `ExampleMinify`. The `util_test.go` tests and
//! `TestRenamerIndices` need crate-private functions and live in
//! `src/unit_tests.rs`.

mod common;
#[allow(clippy::all)]
mod upstream_tables;

use std::sync::Arc;

use tdewolff_minify_js::{GoError, GoReader, M, Minifier, Writer};
use upstream_tables::*;

fn run(o: &Minifier, input: &[u8]) -> Result<Vec<u8>, GoError> {
    // Go: r := bytes.NewBufferString(tt.js); o.Minify(m, w, r, nil)
    let mut w = Vec::new();
    let mut r = tdewolff_parse::buffer::Reader::new(common::cp(input));
    o.minify(&M::new(), &mut w, &mut r, None)?;
    Ok(w)
}

fn check_table(name: &str, o: Minifier, rows: &'static [(&'static [u8], &'static [u8])]) {
    let name = name.to_string();
    common::with_big_stack(move || {
        let mut mm = common::Mismatches::new(&name);
        for (input, expected) in rows {
            let got = run(&o, input);
            mm.check(got.as_deref() == Ok(*expected), || {
                format!(
                    "{:?}\n  got  {:?}\n  want {:?}",
                    common::lossy(input),
                    got.as_ref().map(|g| common::lossy(g)),
                    common::lossy(expected)
                )
            });
        }
        mm.finish();
    });
}

// Go: js_test.go:TestJS
#[test]
fn test_js() {
    check_table(
        "TestJS",
        Minifier {
            keep_var_names: true,
            use_alphabet_var_names: true,
            ..Default::default()
        },
        TEST_JS,
    );
}

// Go: js_test.go:TestJSVarRenaming
#[test]
fn test_js_var_renaming() {
    check_table(
        "TestJSVarRenaming",
        Minifier {
            use_alphabet_var_names: true,
            ..Default::default()
        },
        TEST_JS_VAR_RENAMING,
    );
}

// Go: js_test.go:TestJSVersion
#[test]
fn test_js_version() {
    let versions = [2022, 2020, 2019, 2018, 2014];
    let mut mm = common::Mismatches::new("TestJSVersion");
    for &(version, input, before, after) in TEST_JS_VERSION {
        for v in versions {
            let o = Minifier {
                keep_var_names: true,
                use_alphabet_var_names: true,
                version: v,
                ..Default::default()
            };
            let want = if v < version { before } else { after };
            let got = run(&o, input);
            mm.check(got.as_deref() == Ok(want), || {
                format!(
                    "{}/{:?}: got {:?} want {:?}",
                    v,
                    common::lossy(input),
                    got,
                    want
                )
            });
        }
    }
    mm.finish();
}

// Go: html/html_test.go:TestHTMLCSSJS — the two rows tdewolff-minify could
// only check without a JS minifier, here with the real one.
#[test]
fn test_html_css_js() {
    let m = common::html_m("upstream");
    let mut mm = common::Mismatches::new("TestHTMLCSSJS");
    for (input, expected) in TEST_HTML_CSS_JS {
        let mut w = Vec::new();
        let mut r = tdewolff_parse::buffer::Reader::new(common::cp(input));
        let res = tdewolff_minify::html::minify(&m, &mut w, &mut r, None);
        mm.check(res.is_ok() && w == *expected, || {
            format!(
                "{:?}: got {:?} {:?} want {:?}",
                common::lossy(input),
                common::lossy(&w),
                res,
                common::lossy(expected)
            )
        });
    }
    mm.finish();
}

/// Go: tdewolff/test.ErrPlain
fn err_plain() -> GoError {
    GoError::Other(b"error".to_vec())
}

/// Go: tdewolff/test.ErrorReader — n successive one-byte reads, then ErrPlain.
struct ErrorReader {
    n: usize,
}

impl GoReader for ErrorReader {
    fn read(&mut self, p: &mut [u8]) -> (usize, Option<GoError>) {
        if p.is_empty() {
            return (0, None);
        }
        if self.n == 0 {
            return (0, Some(err_plain()));
        }
        self.n -= 1;
        p[0] = b'.';
        (1, None)
    }
}

/// Go: tdewolff/test.ErrorWriter — n successive writes, then ErrPlain.
struct ErrorWriter {
    n: usize,
}

impl Writer for ErrorWriter {
    fn write(&mut self, b: &[u8]) -> Result<usize, GoError> {
        if self.n == 0 {
            return Err(err_plain());
        }
        self.n -= 1;
        Ok(b.len())
    }
}

// Go: js_test.go:TestReaderError
#[test]
fn test_reader_error() {
    let mut r = ErrorReader { n: 0 };
    let mut w = Vec::new();
    let err = tdewolff_minify_js::minify(&M::new(), &mut w, &mut r, None);
    assert_eq!(err, Err(err_plain()));
}

// Go: js_test.go:TestWriterError
#[test]
fn test_writer_error() {
    let mut r = tdewolff_parse::buffer::Reader::new(common::cp(b"a"));
    let mut w = ErrorWriter { n: 0 };
    let err = tdewolff_minify_js::minify(&M::new(), &mut w, &mut r, None);
    assert_eq!(err, Err(err_plain()));
    // Write errors before the final w.Write(nil) are ignored, as in Go.
    let mut r = tdewolff_parse::buffer::Reader::new(common::cp(b"var a=1;b()"));
    let mut w = ErrorWriter { n: 3 };
    let err = tdewolff_minify_js::minify(&M::new(), &mut w, &mut r, None);
    assert_eq!(err, Err(err_plain()));
}

// Go: js_test.go:ExampleMinify (registered in an M, with neohugo's mimetype
// setup).
#[test]
fn example_minify() {
    let mut m = M::new();
    m.add_func("application/javascript", |m, w, r, params| {
        tdewolff_minify_js::minify(m, w, r, params)
    });
    let hugo = Arc::new(Minifier {
        version: 2022,
        ..Default::default()
    });
    m.add_regexp(
        tdewolff_minify::Regexp::must_compile("^(application|text)/(x-)?(java|ecma)script$"),
        hugo,
    );
    let out = m
        .minify_bytes("application/javascript", b"var a = 'b' ; c ( a )")
        .unwrap();
    assert_eq!(out, b"var a=\"b\";c(a)");
    let out = m
        .minify_bytes("text/javascript", b"function f(){ var name=1; return name}")
        .unwrap();
    assert_eq!(out, b"function f(){var e=1;return e}");
    // parse errors are the parser's *parse.Error, with position context
    let err = m.minify_bytes("text/javascript", b"a b").unwrap_err();
    assert_eq!(
        err.error_bytes(),
        b"unexpected b in expression on line 1 and column 3\n    1: a b\n         ^".to_vec()
    );
}
