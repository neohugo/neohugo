//! Differential tests: Rust output vs the Go oracle output (byte-exact) for
//! goldmark's own test inputs, hand-written edge cases, a fixed-seed fuzz
//! corpus and the seeksnack markdown corpus.

mod common;

use common::{check_records, fixture};

#[test]
fn spec_json() {
    let recs = fixture("spec.gmf");
    // goldmark's TestSpec compares TrimSpace'd outputs with the spec's HTML.
    for r in &recs {
        if let Some(spec) = r.fields.get("spec") {
            assert_eq!(
                String::from_utf8_lossy(r.get("html")).trim(),
                String::from_utf8_lossy(spec).trim(),
                "{}: Go output differs from spec.json",
                r.name
            );
        }
    }
    let n = check_records(&recs);
    assert!(n >= 652 * 3, "{n}");
}

#[test]
fn extra_txt() {
    let recs = fixture("extra.gmf");
    let n = check_records(&recs);
    assert!(n > 100, "{n}");
}

#[test]
fn options_txt() {
    let recs = fixture("options.gmf");
    let n = check_records(&recs);
    assert!(n > 10, "{n}");
}

#[test]
fn edge_cases() {
    let n = check_records(&fixture("edge.gmf"));
    assert!(n > 1000, "{n}");
}

#[test]
fn fuzz_fixed_seed() {
    let n = check_records(&fixture("fuzz.gmf.gz"));
    assert_eq!(n, 3000);
}

#[test]
fn seeksnack_corpus() {
    let n = check_records(&fixture("corpus.gmf.gz"));
    assert_eq!(n, 251 * 4);
}

/// Large fuzz corpora generated outside the repository:
/// `GOLDMARK_FUZZ=path1.gmf[,path2.gmf.gz...] cargo test --release -- --ignored`.
#[test]
#[ignore]
fn external_fuzz() {
    let Ok(paths) = std::env::var("GOLDMARK_FUZZ") else {
        return;
    };
    for p in paths.split(',') {
        let recs = common::parse_gmf(&common::read_file(std::path::Path::new(p)));
        let n = check_records(&recs);
        eprintln!("{p}: {n} records identical");
    }
}
