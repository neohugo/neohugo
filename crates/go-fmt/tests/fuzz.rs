//! Randomized differential tests against `oracle -mode fuzz` vectors
//! (tools/go-oracle/go-fmt/fuzz.go).
//!
//! By default the checked-in fixtures (`fuzz_cases.txt`, `fuzz_formats.txt`,
//! `fuzz_matrix.txt`, default seed) are used. `GO_FMT_FUZZ_DIR=<dir>` runs
//! the same checks on a directory written by
//! `oracle -mode fuzz -dir <dir> -seed N -n ... -nmatrix ...`.

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

fn parse_ranges(s: &str) -> Vec<(usize, usize)> {
    if s.is_empty() {
        return Vec::new();
    }
    s.split(',')
        .map(|p| match p.split_once('-') {
            Some((a, b)) => (a.parse().unwrap(), b.parse().unwrap()),
            None => {
                let a = p.parse().unwrap();
                (a, a)
            }
        })
        .collect()
}

fn args_of(field: &str) -> Vec<go_value::Value> {
    if field.is_empty() {
        return Vec::new();
    }
    field.split(' ').map(spec_value).collect()
}

/// Runs "fn \t format \t operands \t output [\t wrapped]" lines and returns
/// the number of cases and the failure descriptions.
fn run_cases(cases: &str) -> (usize, Vec<String>) {
    let mut failures = Vec::new();
    let mut n = 0;
    for line in cases.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        let fname = parts[0];
        let format = unquote(parts[1]);
        let args = args_of(parts[2]);
        let want = unquote(parts[3]);
        let got = match fname {
            "sprint" => go_fmt::sprint(&args),
            "sprintln" => go_fmt::sprintln(&args),
            "sprintf" => go_fmt::sprintf(&format, &args),
            "errorf" => {
                let (msg, wrapped) = go_fmt::errorf(&format, &args);
                let want_wrapped: Vec<usize> = if parts[4].is_empty() {
                    vec![]
                } else {
                    parts[4].split(',').map(|x| x.parse().unwrap()).collect()
                };
                if wrapped != want_wrapped {
                    failures.push(format!(
                        "errorf {} {}: wrapped {:?} want {:?}",
                        parts[1], parts[2], wrapped, want_wrapped
                    ));
                }
                msg
            }
            other => panic!("unknown fn {other}"),
        };
        n += 1;
        if got != want {
            failures.push(format!(
                "{fname} {} [{}]:\n  got  {}\n  want {}",
                parts[1],
                parts[2],
                q(&got),
                q(&want)
            ));
        }
    }
    (n, failures)
}

fn assert_no_failures(what: &str, failures: &[String]) {
    if let Some(p) = std::env::var_os("GO_FMT_FAIL_FILE") {
        std::fs::write(p, failures.join("\n")).unwrap();
    }
    assert!(
        failures.is_empty(),
        "{} {what} differ:\n{}",
        failures.len(),
        failures
            .iter()
            .take(60)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// Random nested operands with random multi-directive formats, plus
/// Sprint/Sprintln operand lists and Errorf.
#[test]
fn fuzz_cases() {
    let (n, failures) = run_cases(&fuzz_file("fuzz_cases.txt"));
    assert!(n > 1000, "too few fuzz cases");
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
