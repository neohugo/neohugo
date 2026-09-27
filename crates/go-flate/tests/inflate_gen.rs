//! Decompressor parity on random *valid* DEFLATE streams that Go's encoder
//! never produces (tools/go-oracle/go-flate `inflate-gen`): stored / fixed /
//! dynamic blocks with random prefix codes (up to 15 bits, degenerate and
//! empty distance trees, RLE and plain code-length encodings, length 258 as
//! code 284+31), matches at distance 1 and at the window / dictionary limit,
//! zlib framing, optional truncation / bit flips / trailing garbage and
//! wrong or missing dictionaries. Compares output, the per-Read `(n, err)`
//! log, bytes consumed from the input and the final errors.

mod common;

use std::path::Path;

use common::fnv64;
use go_flate::{Error, flate, zlib};

fn err_string(e: &Option<Error>) -> String {
    match e {
        None => "<nil>".to_string(),
        Some(e) => e.to_string(),
    }
}

enum Rd<'a, 'b> {
    Flate(flate::Decompressor<&'a mut &'b [u8]>),
    Zlib(zlib::Reader<&'a mut &'b [u8]>),
}

/// Mirrors `decodeGo` in the oracle.
fn decode(
    zlib_wrapper: bool,
    stream: &[u8],
    dict: &[u8],
    read_size: usize,
) -> (Vec<u8>, u64, usize, String) {
    let mut cur: &[u8] = stream;
    let mut log = String::new();
    let mut out = Vec::new();
    let result;
    {
        let mut rd = if !zlib_wrapper {
            Rd::Flate(flate::new_reader_dict(&mut cur, dict))
        } else {
            match zlib::new_reader_dict(&mut cur, Some(dict)) {
                Ok(z) => Rd::Zlib(z),
                Err(e) => {
                    let remaining = cur.len();
                    return (Vec::new(), 0, stream.len() - remaining, format!("new: {e}"));
                }
            }
        };
        let mut buf = vec![0u8; read_size];
        let mut last: Option<Error> = None;
        for _ in 0..(1 << 22) {
            let (n, err) = match &mut rd {
                Rd::Flate(f) => f.read(&mut buf),
                Rd::Zlib(z) => z.read(&mut buf),
            };
            out.extend_from_slice(&buf[..n]);
            log.push_str(&format!("{}:{};", n, err_string(&err)));
            if err.is_some() {
                last = err;
                break;
            }
        }
        let close = match &mut rd {
            Rd::Flate(f) => f.close().err(),
            Rd::Zlib(z) => z.close().err(),
        };
        result = format!("{} | close: {}", err_string(&last), err_string(&close));
    }
    let remaining = cur.len();
    (out, fnv64(log.as_bytes()), stream.len() - remaining, result)
}

fn take<'a>(bin: &mut &'a [u8]) -> &'a [u8] {
    let n = u32::from_le_bytes(bin[..4].try_into().unwrap()) as usize;
    let b = &bin[4..4 + n];
    *bin = &bin[4 + n..];
    b
}

fn check(bin_path: &Path, txt_path: &Path) -> usize {
    let bin_all = std::fs::read(bin_path).unwrap();
    let txt = std::fs::read_to_string(txt_path).unwrap();
    let mut bin: &[u8] = &bin_all;
    let mut failures = Vec::new();
    let mut total = 0;
    for line in txt.lines() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        total += 1;
        let w = bin[0];
        bin = &bin[1..];
        let dict = take(&mut bin);
        let stream = take(&mut bin);
        let f: Vec<&str> = line.splitn(11, ' ').collect();
        assert_eq!(f[1].as_bytes()[0], w, "corpus out of sync at {line}");
        let read_size: usize = f[3].parse().unwrap();
        let want_len: usize = f[6].parse().unwrap();
        let want_fnv = u64::from_str_radix(f[7], 16).unwrap();
        let want_log = u64::from_str_radix(f[8], 16).unwrap();
        let want_consumed: usize = f[9].parse().unwrap();
        let want_err = f[10];
        let (out, log, consumed, err) = decode(w == b'z', stream, dict, read_size);
        let log_ok = want_err.starts_with("new: ") || log == want_log;
        if out.len() != want_len
            || fnv64(&out) != want_fnv
            || !log_ok
            || consumed != want_consumed
            || err != want_err
        {
            failures.push(format!(
                "{line}\n   got len={} fnv={:016x} log={:016x} consumed={} err={err}",
                out.len(),
                fnv64(&out),
                log,
                consumed
            ));
        }
    }
    assert!(bin.is_empty(), "trailing corpus data");
    for f in failures.iter().take(30) {
        eprintln!("{f}");
    }
    assert!(
        failures.is_empty(),
        "{} of {total} inflate-gen cases differ",
        failures.len()
    );
    total
}

/// `oracle inflate-gen -n 600 -seed 11 -maxblock 2000 -maxdict 3000`.
#[test]
fn inflate_gen_cases() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let n = check(&dir.join("inflate_gen.bin"), &dir.join("inflate_gen.txt"));
    assert!(n >= 600);
}

/// Large corpora outside the repository:
/// `GO_FLATE_INFLATE_GEN=a.bin:a.txt,b.bin:b.txt cargo test --release --test inflate_gen -- --ignored`
#[test]
#[ignore]
fn inflate_gen_corpus() {
    let Ok(spec) = std::env::var("GO_FLATE_INFLATE_GEN") else {
        eprintln!("GO_FLATE_INFLATE_GEN not set; skipping");
        return;
    };
    for pair in spec.split(',') {
        let (b, t) = pair.split_once(':').unwrap();
        let n = check(Path::new(b), Path::new(t));
        eprintln!("{t}: {n} cases ok");
    }
}
