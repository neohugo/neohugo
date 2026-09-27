//! Differential tests against fixtures produced by tools/go-oracle/go-flate
//! (Go 1.27.1 compress/flate + compress/zlib).

mod common;

use std::collections::HashMap;
use std::path::Path;

use common::{check_cases, parse_cases};

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

/// Generated inputs x all levels x write/flush/reset/close programs.
#[test]
fn oracle_cases_small() {
    let cases = parse_cases(&fixture("cases_small.txt"));
    assert!(cases.len() > 6000);
    let failures = check_cases(&cases, &HashMap::new());
    report(failures, cases.len());
}

/// Real text files (Go testdata) x all levels x programs.
#[test]
fn oracle_cases_files() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/files");
    let mut files = HashMap::new();
    for e in std::fs::read_dir(&dir).unwrap() {
        let e = e.unwrap();
        files.insert(
            e.file_name().to_string_lossy().into_owned(),
            std::fs::read(e.path()).unwrap(),
        );
    }
    let cases = parse_cases(&fixture("cases_files.txt"));
    let failures = check_cases(&cases, &files);
    report(failures, cases.len());
}

/// Large corpora kept outside the repository:
/// `GO_FLATE_CASES=path/to/cases.txt [GO_FLATE_FILES=dir] cargo test --release -- --ignored big_corpus`
#[test]
#[ignore]
fn big_corpus() {
    let Ok(path) = std::env::var("GO_FLATE_CASES") else {
        eprintln!("GO_FLATE_CASES not set; skipping");
        return;
    };
    let mut files = HashMap::new();
    if let Ok(dir) = std::env::var("GO_FLATE_FILES") {
        for e in std::fs::read_dir(&dir).unwrap() {
            let e = e.unwrap();
            if e.path().is_file() {
                files.insert(
                    e.file_name().to_string_lossy().into_owned(),
                    std::fs::read(e.path()).unwrap(),
                );
            }
        }
    }
    let cases = parse_cases(&std::fs::read_to_string(&path).unwrap());
    let failures = check_cases(&cases, &files);
    eprintln!(
        "big corpus: {} cases, {} failures",
        cases.len(),
        failures.len()
    );
    report(failures, cases.len());
}
