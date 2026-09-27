//! Differential tests against the Go oracle fixtures
//! (tools/go-oracle/tdewolff-minify-js `fixtures`): the output, the error
//! string and the input buffer after the call (in-place rewrites, the
//! unrestored NUL sentinel) must all match Go.
//!
//! * `literals` — every string literal of the upstream minify/js and
//!   parse/js tests plus hand-written cases (seeksnack's inline scripts and
//!   handlers, Hugo's embedded templates, in-place/aliasing/hoisting/merging
//!   edge cases) through 9 configurations, full outputs;
//! * `embedded` — scripts minified from a window of a larger buffer (as the
//!   HTML minifier does), the whole buffer is compared afterwards;
//! * `html` — HTML documents through neohugo's full `M` (tdewolff-minify's
//!   HTML/CSS/JSON/SVG/XML minifiers plus this JS minifier), the literals
//!   embedded in `<script>`s and `on*` attributes, and the html_test.go
//!   literals (also through the upstream TestHTMLCSSJS `M`);
//! * `adversarial` — long `var` lists (the position-dependent
//!   `sort.SliceStable` of `minifyVarDecl`), many locals with tied use
//!   counts (pdqsort in `renameScope`), multi-character names, deep nesting
//!   and long chains, long escape sequences;
//! * `grammar` — generated programs (digests);
//! * `fuzz` — mutated literals and programs (digests);
//! * `repo` — the neohugo repository's own JS files, 4 configurations;
//! * `corpuswin` — mutated windows of the JS corpus in the Go module cache.
//!
//! `TDEWOLFF_MINIFY_JS_FIXTURES=<dir>` points the tests at a larger set
//! generated with a higher scale (see tools/go-oracle/tdewolff-minify-js/gen.sh).

mod common;

use common::*;

/// Checks `min`, `sub` and `multi` records; `digests` when the expected
/// fields are FNV digests.
fn check(name: &'static str, digests: bool) {
    with_big_stack(move || {
        let recs = records(name);
        let mut mm = Mismatches::new(name);
        for r in &recs {
            let kind = r[0].as_slice();
            match kind {
                b"min" | b"sub" | b"html" => {
                    let cfg = String::from_utf8(r[1].clone()).unwrap();
                    let (got, want, input) = match kind {
                        b"min" => (run_min(&cfg, &r[2]), &r[3..6], r[2].clone()),
                        b"html" => (run_html(&cfg, &r[2]), &r[3..6], r[2].clone()),
                        _ => (run_sub(&cfg, &r[2], &r[3], &r[4]), &r[5..8], r[3].clone()),
                    };
                    let got: Vec<Vec<u8>> = if digests {
                        got.iter().map(|f| digest(f)).collect()
                    } else {
                        got.to_vec()
                    };
                    // on a Go panic only the panic itself is compared
                    let panic = if digests {
                        digest(b"PANIC")
                    } else {
                        b"PANIC".to_vec()
                    };
                    let ok = if want[1] == panic {
                        got[1] == panic
                    } else {
                        got == want
                    };
                    mm.check(ok, || {
                        format!(
                            "[{}] in={:?}\n  go  out={:?} err={:?} after={}\n  rs  out={:?} err={:?} after={}",
                            cfg,
                            lossy(&input),
                            lossy(&want[0]),
                            lossy(&want[1]),
                            want[2].len(),
                            lossy(&got[0]),
                            lossy(&got[1]),
                            got[2].len(),
                        )
                    });
                }
                b"multi" => {
                    let file = String::from_utf8_lossy(&r[1]).into_owned();
                    let input = &r[2];
                    for c in r[3..].chunks(4) {
                        let cfg = String::from_utf8(c[0].clone()).unwrap();
                        let got: Vec<Vec<u8>> =
                            run_min(&cfg, input).iter().map(|f| digest(f)).collect();
                        mm.check(got == c[1..4], || format!("[{}] {}", cfg, file));
                    }
                }
                k => panic!("unknown record kind {:?}", lossy(k)),
            }
        }
        mm.finish();
    });
}

#[test]
fn literals() {
    check("literals", false);
}

#[test]
fn embedded() {
    check("embedded", false);
}

#[test]
fn html() {
    check("html", false);
}

#[test]
fn adversarial() {
    check("adversarial", false);
}

#[test]
fn grammar() {
    check("grammar", true);
}

#[test]
fn fuzz() {
    check("fuzz", true);
}

#[test]
fn repo() {
    check("repo", true);
}

#[test]
fn corpuswin() {
    check("corpuswin", true);
}
