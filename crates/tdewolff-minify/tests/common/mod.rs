//! Shared helpers for the differential tests: fixture loading (gzip TSV
//! written by tools/go-oracle/tdewolff-minify) and minification runs that
//! mirror the oracle's `runM`.
#![allow(dead_code)]

pub mod configs;

use std::io::Read;
use std::path::PathBuf;

use tdewolff_minify::{GoBytes, GoError, M};

pub fn fixtures_dir() -> PathBuf {
    if let Ok(d) = std::env::var("TDEWOLFF_MINIFY_FIXTURES") {
        return PathBuf::from(d);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// Reads `<name>.txt.gz` as tab-separated records (comment lines skipped).
pub fn records(name: &str) -> Vec<Vec<String>> {
    let p = fixtures_dir().join(format!("{}.txt.gz", name));
    let f = std::fs::File::open(&p).unwrap_or_else(|e| panic!("{}: {}", p.display(), e));
    let mut s = String::new();
    flate2::read::GzDecoder::new(f)
        .read_to_string(&mut s)
        .unwrap_or_else(|e| panic!("{}: {}", p.display(), e));
    s.lines()
        .filter(|l| !l.starts_with('#'))
        .map(|l| l.split('\t').map(|f| f.to_string()).collect())
        .collect()
}

pub fn unhex(s: &str) -> Vec<u8> {
    assert!(s.len().is_multiple_of(2), "odd hex {}", s);
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

pub fn hex(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for c in b {
        s.push_str(&format!("{:02x}", c));
    }
    s
}

/// Go `errStr`: `-` for nil, else hex of `err.Error()`.
pub fn err_str(e: &Option<GoError>) -> String {
    match e {
        None => "-".to_string(),
        Some(e) => hex(&e.error_bytes()),
    }
}

/// Go `sameOr`: `=` when equal to the reference.
pub fn same_or(b: &[u8], reference: &[u8]) -> String {
    if b == reference {
        "=".to_string()
    } else {
        hex(b)
    }
}

/// Go `cp`: `append([]byte(nil), s...)` (size-class capacity).
pub fn cp(s: &[u8]) -> GoBytes {
    GoBytes::nil().append(s)
}

/// Go `runM`: minifies a private copy of `input`; returns output, error and
/// the input copy after minification.
pub fn run_m(m: &M, mediatype: &[u8], input: &[u8]) -> (Vec<u8>, Option<GoError>, Vec<u8>) {
    let buf = cp(input);
    let mut out: Vec<u8> = Vec::new();
    let mut r = tdewolff_parse::buffer::Reader::new(buf.clone());
    let err = m.minify(mediatype, &mut out, &mut r).err();
    (out, err, buf.to_vec())
}

/// FNV-1a 64 as `%016x` (Go `hash/fnv`).
pub fn fnv64(b: &[u8]) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for &c in b {
        h ^= c as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{:016x}", h)
}

/// Collects mismatches and panics with the first few at the end.
pub struct Mismatches {
    pub name: String,
    pub total: usize,
    pub failed: Vec<String>,
}

impl Mismatches {
    pub fn new(name: &str) -> Self {
        Mismatches {
            name: name.to_string(),
            total: 0,
            failed: Vec::new(),
        }
    }

    pub fn check(&mut self, ok: bool, msg: impl FnOnce() -> String) {
        self.total += 1;
        if !ok {
            self.failed.push(msg());
        }
    }

    pub fn finish(self) {
        eprintln!(
            "{}: {}/{} passed",
            self.name,
            self.total - self.failed.len(),
            self.total
        );
        if !self.failed.is_empty() {
            let shown: Vec<_> = self.failed.iter().take(15).cloned().collect();
            panic!(
                "{}: {} of {} mismatched; first:\n{}",
                self.name,
                self.failed.len(),
                self.total,
                shown.join("\n")
            );
        }
    }
}

pub fn lossy(b: &[u8]) -> String {
    format!("{:?}", String::from_utf8_lossy(b))
}
