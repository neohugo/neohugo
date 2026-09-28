//! Randomized differential tests against `oracle -mode fuzz` vectors
//! (tools/go-oracle/go-fmt/fuzz.go).
//!
//! By default the checked-in fixtures (`fuzz_cases.txt`, `fuzz_formats.txt`,
//! `fuzz_matrix.txt`, default seed) are used. `GO_FMT_FUZZ_DIR=<dir>` runs
//! the same checks on a directory written by
//! `oracle -mode fuzz|redteam|flagperm -dir <dir> -seed N -n ... -nmatrix ...`.

mod common;

use std::path::PathBuf;

use common::*;

fn fuzz_file(name: &str) -> String {
    let path = match std::env::var_os("GO_FMT_FUZZ_DIR") {
        Some(d) => PathBuf::from(d).join(name),
        None => fixture_path(name),
    };
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// Random nested operands with random multi-directive formats, plus
/// Sprint/Sprintln operand lists and Errorf.
#[test]
fn fuzz_cases() {
    let (n, failures) = run_cases(&fuzz_file("fuzz_cases.txt"));
    assert!(
        n > 1000 || std::env::var_os("GO_FMT_FUZZ_DIR").is_some(),
        "too few fuzz cases"
    );
    eprintln!("fuzz cases: {n}, {} differ", failures.len());
    assert_no_failures("fuzz cases", &failures);
}

/// Hand-picked operands for the value-model mappings: named collection
/// types with String methods (`page.Pages`, `page.TaxonomyList`), typed
/// nils of interface types inside containers, GoStringer host objects,
/// nil `*time.Location`, `time.Time` fields (checked-in fixture only).
#[test]
fn model_cases() {
    let (n, failures) = run_cases(&read_fixture("model_cases.txt"));
    assert!(n > 300, "too few model cases");
    eprintln!("model cases: {n}, {} differ", failures.len());
    assert_no_failures("model cases", &failures);
}

/// The dense single-operand matrix (random floats, float32s, integers,
/// strings, byte slices; widths/precisions beyond the 68-byte intbuf).
/// On a mismatch, GO_FMT_DUMP_DIR=<dir> writes `<dir>/fuzz-<index>.txt`
/// with one Go-quoted `format => output` line per format.
#[test]
fn fuzz_matrix() {
    let formats: Vec<Vec<u8>> = fuzz_file("fuzz_formats.txt").lines().map(unquote).collect();
    let matrix = fuzz_file("fuzz_matrix.txt");
    let dump_dir = std::env::var_os("GO_FMT_DUMP_DIR");
    let mut failures = Vec::new();
    let mut hashed = 0usize;
    for (vi, line) in matrix.lines().enumerate() {
        let parts: Vec<&str> = line.split('\t').collect();
        assert_eq!(parts.len(), 3, "bad matrix line {line:?}");
        let v = spec_value(parts[0]);
        let want = u64::from_str_radix(parts[1], 16).unwrap();
        let skip = parse_ranges(parts[2]);
        let mut h = Fnv64::new();
        let mut outs = Vec::new();
        for (fi, f) in formats.iter().enumerate() {
            if skip.iter().any(|&(a, b)| a <= fi && fi <= b) {
                continue;
            }
            let out = go_fmt::sprintf(f, std::slice::from_ref(&v));
            h.write(&(out.len() as u64).to_le_bytes());
            h.write(&out);
            hashed += 1;
            if dump_dir.is_some() {
                outs.push(format!("{} => {}", q(f), q(&out)));
            }
        }
        if h.0 != want {
            failures.push(format!("#{vi} {}", parts[0]));
            if let Some(d) = &dump_dir {
                let mut s = outs.join("\n");
                s.push('\n');
                std::fs::write(std::path::Path::new(d).join(format!("fuzz-{vi}.txt")), s).unwrap();
            }
        }
    }
    eprintln!(
        "fuzz matrix: {hashed} outputs compared, {} operands differ",
        failures.len()
    );
    assert!(
        failures.is_empty(),
        "{} operands differ:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
