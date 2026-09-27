//! Adversarial / randomized differential tests (tools/go-oracle/go-flate
//! `fuzz`, `genhash`): new data kinds (Fibonacci-skewed symbols, exact
//! periods around the 32 KiB window, sparse data, counters, block-boundary
//! segments, filtered PNG scanlines), block-boundary sizes, flush-heavy /
//! boundary / lifecycle programs and failing sinks.

mod common;

use std::collections::HashMap;
use std::path::Path;

use common::{check_cases, fnv64, gen_data, parse_cases};

fn fixture(name: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

fn report(failures: Vec<String>, total: usize) {
    if !failures.is_empty() {
        for f in failures.iter().take(40) {
            eprintln!("{f}");
        }
        panic!("{} of {} cases differ from Go", failures.len(), total);
    }
}

/// The Rust data generator must reproduce the oracle's byte for byte.
#[test]
fn gen_data_matches_oracle() {
    let text = fixture("genhash.txt");
    let mut n = 0;
    for line in text.lines() {
        let f: Vec<&str> = line.split(' ').collect();
        let (kind, size, seed): (usize, usize, u64) = (
            f[0].parse().unwrap(),
            f[1].parse().unwrap(),
            f[2].parse().unwrap(),
        );
        let want = u64::from_str_radix(f[3], 16).unwrap();
        assert_eq!(
            fnv64(&gen_data(kind, size, seed)),
            want,
            "gen_data({kind}, {size}, {seed})"
        );
        n += 1;
    }
    assert_eq!(n, 16 * 6 * 3);
}

/// `oracle fuzz -n 1500 -seed 1 -max 150000`.
#[test]
fn fuzz_cases() {
    let cases = parse_cases(&fixture("fuzz_cases.txt"));
    assert!(cases.len() >= 1500);
    let failures = check_cases(&cases, &HashMap::new());
    report(failures, cases.len());
}

/// Large fuzz corpora kept outside the repository:
/// `GO_FLATE_FUZZ=path cargo test --release --test fuzz_cases -- --ignored`
#[test]
#[ignore]
fn fuzz_corpus() {
    let Ok(paths) = std::env::var("GO_FLATE_FUZZ") else {
        eprintln!("GO_FLATE_FUZZ not set; skipping");
        return;
    };
    for path in paths.split(':') {
        let cases = parse_cases(&std::fs::read_to_string(path).unwrap());
        let failures = check_cases(&cases, &HashMap::new());
        eprintln!("{path}: {} cases, {} failures", cases.len(), failures.len());
        report(failures, cases.len());
    }
}

/// Go's own test programs as byte-exact cases (`oracle gotests`):
/// TestBestSpeed (every level, with/without Flush), TestBestSpeedMaxMatchOffset,
/// TestWriterReset/testResetOutput, TestWriteError (non-persistent and
/// persistent failing sinks), TestWriterPersistentWriteError,
/// TestDeflateFast_Reset, TestDeterministic (787 / 81761 byte writes),
/// TestRegression2508, TestMaxStackSize, TestVeryLongSparseChunk (64 MiB).
#[test]
fn go_test_programs() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/files");
    let mut files = HashMap::new();
    files.insert(
        "Isaac.Newton-Opticks.txt".to_string(),
        std::fs::read(dir.join("Isaac.Newton-Opticks.txt")).unwrap(),
    );
    let cases = parse_cases(&fixture("gotests.txt"));
    assert!(cases.len() > 3900);
    let failures = check_cases(&cases, &files);
    report(failures, cases.len());
}
