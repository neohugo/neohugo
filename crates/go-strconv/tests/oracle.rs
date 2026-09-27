//! Differential tests against fixtures produced by the Go oracle
//! (tools/go-oracle/go-strconv, go1.27.1).
//!
//! `hash_chunks` regenerates the oracle's deterministic inputs (SplitMix64)
//! and compares one FNV-1a hash per chunk (tests/fixtures/hashes.txt):
//! at least 1M float64 and 1M float32 values across all formats/precisions,
//! 1M generated float strings, 512K integer strings, every code point
//! through every quoting function, 512K random byte strings, and the
//! slow (multiprecision decimal) paths. On a mismatch set
//! GO_STRCONV_DUMP_DIR=<dir> to write the chunk's dump, and compare it with
//! `oracle -mode dump -test <name> -chunk <n>`.
//!
//! The `*_vectors` tests check readable explicit vectors.

mod common;

use common::*;
use go_strconv as sc;
use go_strconv::internal;

/// Runs every (test, chunk) in `expected` and compares hashes.
fn check_hashes(expected: &std::collections::HashMap<(String, usize), u64>) {
    let mut jobs: Vec<(usize, usize, u64)> = Vec::new();
    for ((name, c), &want) in expected {
        let ti = TESTS
            .iter()
            .position(|t| t.name == name)
            .unwrap_or_else(|| panic!("unknown test {name}"));
        jobs.push((ti, *c, want));
    }
    jobs.sort();
    let next = std::sync::atomic::AtomicUsize::new(0);
    let failures = std::sync::Mutex::new(Vec::new());
    let nthreads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    std::thread::scope(|scope| {
        for _ in 0..nthreads {
            scope.spawn(|| {
                loop {
                    let j = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    if j >= jobs.len() {
                        break;
                    }
                    let (ti, c, want) = jobs[j];
                    let t = &TESTS[ti];
                    let (h, _) = run_chunk(t, c, false);
                    if h != want {
                        failures.lock().unwrap().push((t.name, c, h, want));
                    }
                }
            });
        }
    });
    let mut failures = failures.into_inner().unwrap();
    failures.sort();
    if !failures.is_empty() {
        if let Ok(dir) = std::env::var("GO_STRCONV_DUMP_DIR") {
            for &(name, c, _, _) in failures.iter().take(5) {
                let t = TESTS.iter().find(|t| t.name == name).unwrap();
                let (_, d) = run_chunk(t, c, true);
                let p = format!("{dir}/rust-{name}-{c}.txt");
                std::fs::write(&p, d.unwrap()).unwrap();
                eprintln!("wrote {p}");
            }
        }
        for f in &failures {
            eprintln!(
                "MISMATCH test={} chunk={} got={:016x} want={:016x}",
                f.0, f.1, f.2, f.3
            );
        }
        panic!("{} of {} chunks mismatched", failures.len(), jobs.len());
    }
    eprintln!("all {} chunks match", jobs.len());
}

#[test]
fn hash_chunks() {
    let expected = expected_hashes(&fixture("hashes.txt"));
    let want_jobs: usize = TESTS.iter().map(|t| t.chunks).sum();
    assert_eq!(
        want_jobs,
        expected.len(),
        "fixture/test definition mismatch"
    );
    for t in TESTS {
        for c in 0..t.chunks {
            assert!(expected.contains_key(&(t.name.to_string(), c)));
        }
    }
    check_hashes(&expected);
}

/// Extended run: GO_STRCONV_HASHES=<file> produced by
/// `oracle -mode hashes -scale N -out <file>` (e.g. N=16: 16.7M float64 and
/// 16.7M float32 values). Run with `cargo test --release -- --ignored`.
#[test]
#[ignore]
fn hash_chunks_extended() {
    let path =
        std::env::var("GO_STRCONV_HASHES").expect("set GO_STRCONV_HASHES=<oracle hashes file>");
    let data = std::fs::read_to_string(&path).unwrap();
    check_hashes(&expected_hashes(&data));
}

#[test]
fn ftoa_vectors() {
    let mut n = 0;
    let mut bad = 0;
    for l in fixture_lines("ftoa_vectors.txt") {
        let f: Vec<&str> = l.split(' ').collect();
        let bits = u64::from_str_radix(f[0], 16).unwrap();
        let bs: i64 = f[1].parse().unwrap();
        let fm = f[2].as_bytes()[0];
        let prec: i64 = f[3].parse().unwrap();
        let want = f[4];
        let want_slow = if f[5] == "=" { f[4] } else { f[5] };
        let got = sc::format_float(f64::from_bits(bits), fm, prec, bs);
        if got != want {
            bad += 1;
            if bad < 20 {
                eprintln!(
                    "FormatFloat({bits:016x}, {}, {prec}, {bs}) = {got} want {want}",
                    fm as char
                );
            }
        }
        let old = internal::set_optimize(false);
        let got_slow = sc::format_float(f64::from_bits(bits), fm, prec, bs);
        internal::set_optimize(old);
        if got_slow != want_slow {
            bad += 1;
            if bad < 20 {
                eprintln!(
                    "slow FormatFloat({bits:016x}, {}, {prec}, {bs}) = {got_slow} want {want_slow}",
                    fm as char
                );
            }
        }
        let mut dst = b"abc".to_vec();
        sc::append_float(&mut dst, f64::from_bits(bits), fm, prec, bs);
        assert_eq!(&dst[3..], want.as_bytes());
        n += 1;
    }
    assert_eq!(bad, 0, "{bad} of {n} ftoa vectors failed");
    assert!(n > 10000);
}

fn err_code(e: Option<internal::Error>) -> &'static str {
    match e {
        None => "nil",
        Some(internal::Error::Range) => "range",
        Some(internal::Error::Syntax) => "syntax",
        Some(_) => "other",
    }
}

#[test]
fn atof_vectors() {
    let mut n = 0;
    let mut bad = 0;
    for l in fixture_lines("atof_vectors.txt") {
        let mut f: Vec<&str> = l.split('\t').collect();
        if f[5] == "=" {
            f.truncate(5);
            f.extend_from_within(1..5);
        }
        let s = f[0].as_bytes();
        for (bs, bi, ei, sbi, sei) in [(64, 1, 2, 5, 6), (32, 3, 4, 7, 8)] {
            let want_bits = u64::from_str_radix(f[bi], 16).unwrap();
            let (v, err) = internal::parse_float(s, bs);
            if v.to_bits() != want_bits || err_code(err) != f[ei] {
                bad += 1;
                if bad < 20 {
                    eprintln!(
                        "ParseFloat({:?}, {bs}) = {:016x} {} want {want_bits:016x} {}",
                        f[0],
                        v.to_bits(),
                        err_code(err),
                        f[ei]
                    );
                }
            }
            // Slow path (optimize=false) against Go's slow path.
            let want_slow = u64::from_str_radix(f[sbi], 16).unwrap();
            let old = internal::set_optimize(false);
            let (v2, err2) = internal::parse_float(s, bs);
            internal::set_optimize(old);
            if v2.to_bits() != want_slow || err_code(err2) != f[sei] {
                bad += 1;
                if bad < 20 {
                    eprintln!("slow ParseFloat({:?}, {bs}) = {:016x}", f[0], v2.to_bits());
                }
            }
        }
        n += 1;
    }
    assert_eq!(bad, 0, "{bad} failures in {n} atof vectors");
    assert!(n > 5000);
}

fn err_str<E: std::fmt::Display>(e: Option<E>) -> String {
    match e {
        None => "nil".to_string(),
        Some(e) => e.to_string(),
    }
}

#[test]
fn atoi_vectors() {
    let mut n = 0;
    for l in fixture_lines("atoi_vectors.txt") {
        let f: Vec<&str> = l.split('\t').collect();
        let s = unhex(f[0]);
        let base: i64 = f[1].parse().unwrap();
        let bits: i64 = f[2].parse().unwrap();
        let (v, _) = internal::parse_int(&s, base, bits);
        let r = sc::parse_int(&s, base, bits);
        assert_eq!(
            (v.to_string(), err_str(r.err())),
            (f[3].to_string(), f[4].to_string()),
            "ParseInt({:?}, {base}, {bits})",
            String::from_utf8_lossy(&s)
        );
        let (u, _) = internal::parse_uint(&s, base, bits);
        let r = sc::parse_uint(&s, base, bits);
        assert_eq!(
            (u.to_string(), err_str(r.err())),
            (f[5].to_string(), f[6].to_string()),
            "ParseUint({:?}, {base}, {bits})",
            String::from_utf8_lossy(&s)
        );
        let (a, _) = internal::atoi(&s);
        let r = sc::atoi(&s);
        assert_eq!(
            (a.to_string(), err_str(r.err())),
            (f[7].to_string(), f[8].to_string()),
            "Atoi({:?})",
            String::from_utf8_lossy(&s)
        );
        n += 1;
    }
    assert!(n > 2000);
}

#[test]
fn quote_vectors() {
    let mut n = 0;
    for l in fixture_lines("quote_vectors.txt") {
        let f: Vec<&str> = l.split('\t').collect();
        let s = unhex(f[0]);
        assert_eq!(sc::quote(&s), f[1], "Quote({:?})", f[0]);
        assert_eq!(sc::quote_to_ascii(&s), f[2], "QuoteToASCII({:?})", f[0]);
        assert_eq!(sc::quote_to_graphic(&s), f[3], "QuoteToGraphic({:?})", f[0]);
        assert_eq!(
            sc::can_backquote(&s).to_string(),
            f[4],
            "CanBackquote({:?})",
            f[0]
        );
        let u = sc::unquote(&s);
        assert_eq!(
            hex(u.as_deref().unwrap_or(b"")),
            f[5],
            "Unquote({:?})",
            f[0]
        );
        assert_eq!(err_str(u.err()), f[6], "Unquote err({:?})", f[0]);
        n += 1;
    }
    assert!(n > 2000);
}

#[test]
fn numerror_messages() {
    for l in fixture_lines("numerror.txt") {
        let f: Vec<&str> = l.split('\t').collect();
        let s = unhex(f[1]);
        let base: i64 = f[2].parse().unwrap();
        let bits: i64 = f[3].parse().unwrap();
        let got = match f[0] {
            "ParseInt" => err_str(sc::parse_int(&s, base, bits).err()),
            "ParseUint" => err_str(sc::parse_uint(&s, base, bits).err()),
            "ParseFloat" => err_str(sc::parse_float(&s, bits).err()),
            "ParseComplex" => err_str(sc::parse_complex(&s, bits).err()),
            "ParseBool" => err_str(sc::parse_bool(&s).err()),
            "Atoi" => err_str(sc::atoi(&s).err()),
            other => panic!("unknown func {other}"),
        };
        assert_eq!(got, f[4], "{} {:?}", f[0], f[1]);
    }
}
