//! Differential tests against the Go oracle (tools/go-oracle/tdewolff-parse-js):
//! every string literal of the upstream parse/js and minify/js tests plus
//! hand-written edge cases (full dumps), seeded fuzz mutations of them
//! (FNV digests of the dumps), the red-team sample (the minify-js oracle's
//! generated inputs, `reparse`) and an exhaustive enumeration of short
//! whitespace/line-terminator/comment token sequences (`enumerate`).

mod common;

use common::*;

fn check_literals() -> (usize, Vec<String>) {
    let recs = read_records("literals.rec.gz");
    let mut fails = Vec::new();
    for rec in &recs {
        let src = &rec[0];
        for (k, mode) in MODES.iter().enumerate() {
            let got = run_mode(mode, src);
            if got != rec[k + 1] {
                fails.push(format!(
                    "{} {:?}: {}",
                    mode,
                    String::from_utf8_lossy(src),
                    first_diff(&got, &rec[k + 1])
                ));
            }
        }
    }
    (recs.len(), fails)
}

#[test]
fn literals() {
    let (n, fails) = big_stack(check_literals);
    for f in fails.iter().take(20) {
        eprintln!("{}", f);
    }
    assert!(
        fails.is_empty(),
        "{} of {} records x {} modes differ",
        fails.len(),
        n,
        MODES.len()
    );
    eprintln!("literals: {} inputs x {} modes identical", n, MODES.len());
}

fn check_digests(name: &str) -> (usize, Vec<String>) {
    let recs = read_records(name);
    let mut fails = Vec::new();
    for rec in &recs {
        let src = &rec[0];
        for (k, mode) in MODES.iter().enumerate() {
            let got = digest(&run_mode(mode, src));
            if got.as_bytes() != &rec[k + 1][..] {
                fails.push(format!("{} {:?}", mode, String::from_utf8_lossy(src)));
            }
        }
    }
    (recs.len(), fails)
}

fn digests_test(name: &'static str) {
    let (n, fails) = big_stack(move || check_digests(name));
    for f in fails.iter().take(20) {
        eprintln!("{}", f);
    }
    assert!(
        fails.is_empty(),
        "{}: {} of {} records x {} modes differ",
        name,
        fails.len(),
        n,
        MODES.len()
    );
    eprintln!("{}: {} inputs x {} modes identical", name, n, MODES.len());
}

#[test]
fn fuzz() {
    digests_test("fuzz.rec.gz");
}

/// Mutated windows of the JS corpus.
#[test]
fn fuzz_corpus() {
    digests_test("fuzzcorpus.rec.gz");
}

/// The red-team sample: literal-heavy programs, logic trees, numbers, token
/// soup and generated programs of the minify-js oracle (`gen`), reparsed.
#[test]
fn redteam() {
    digests_test("redteam.rec.gz");
}

/// Every sequence of <= 3 tokens of whitespace, line terminators (CR,
/// CRLF, LF, U+2028/9), NBSP, BOM, comments, HTML-like comments and
/// hashbangs mixed with a few expression tokens.
#[test]
fn enumerate_ws() {
    let (n, bad) = big_stack(|| check_enumerate(&fixtures_dir().join("enum-ws.txt.gz")));
    for src in bad.iter().take(20) {
        eprintln!("{:?}", String::from_utf8_lossy(src));
    }
    assert!(
        bad.is_empty(),
        "enum-ws: {} of {} inputs differ",
        bad.len(),
        n
    );
    eprintln!("enum-ws: {} inputs x {} modes identical", n, MODES.len());
}
