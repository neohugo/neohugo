//! Differential tests against the Go oracle (tools/go-oracle/tdewolff-parse-js):
//! every string literal of the upstream parse/js and minify/js tests plus
//! hand-written edge cases (full dumps), and seeded fuzz mutations of them
//! (FNV digests of the dumps).

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
